//! Restart-free lossless JPEG-92 color-component decoding.
//!
//! The DNG-specific tile assembly and metadata decisions are documented in
//! `docs/reference/decoders/dng-lossless-jpeg.md`.

use rayon::prelude::*;

use crate::error::invalid;
use crate::request::{DecodeRequest, EncodeRequest, EncodedChunks, SampleFormat};
use crate::samples::{Geometry, buffer_u8, buffer_u16, chunk};
use crate::{CodecError, Predictor, SampleBuffer};

const MARKER_PREFIX: u8 = 0xff;
const MARKER_SOF3: u8 = 0xc3;
const MARKER_DHT: u8 = 0xc4;
const MARKER_SOI: u8 = 0xd8;
const MARKER_EOI: u8 = 0xd9;
const MARKER_SOS: u8 = 0xda;
const HUFFMAN_LOOKAHEAD_BITS: usize = 16;
const HUFFMAN_LOOKAHEAD_SIZE: usize = 1 << HUFFMAN_LOOKAHEAD_BITS;

pub fn decode(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    if request.sample_format != SampleFormat::Integer
        || !(2..=16).contains(&request.bits_per_sample)
    {
        return Err(invalid("lossless JPEG requires 2-16-bit integer samples"));
    }
    if request.predictor != Predictor::None {
        return Err(invalid(
            "lossless JPEG prediction is encoded in the JPEG stream",
        ));
    }
    let geometry = Geometry::new(
        request.width,
        request.height,
        request.chunk_width,
        request.chunk_height,
        request.chunks.len(),
    )?;
    let sample_count = geometry
        .width
        .checked_mul(geometry.height)
        .ok_or_else(|| invalid("lossless JPEG sample count overflow"))?;
    let mut output = vec![0_u16; sample_count];
    let expected_samples = geometry
        .chunk_width
        .checked_mul(geometry.chunk_height)
        .ok_or_else(|| invalid("lossless JPEG tile sample count overflow"))?;
    output
        .par_chunks_mut(geometry.band_stride()?)
        .enumerate()
        .try_for_each(|(tile_y_index, band)| {
            let mut decoded = vec![0_u16; expected_samples];
            let tile_y = tile_y_index * geometry.chunk_height;
            let copy_height = geometry.chunk_height.min(geometry.height - tile_y);
            for tile_x_index in 0..geometry.across {
                let index = tile_y_index * geometry.across + tile_x_index;
                let encoded = chunk(request.source, request.chunks[index])?;
                let precision = u8::try_from(request.bits_per_sample)
                    .expect("lossless JPEG precision is at most 16");
                decode_into(
                    encoded,
                    geometry.chunk_width,
                    geometry.chunk_height,
                    precision,
                    &mut decoded,
                )
                .map_err(|detail| invalid(format!("lossless JPEG tile {index}: {detail}")))?;
                let tile_x = tile_x_index * geometry.chunk_width;
                let copy_width = geometry.chunk_width.min(geometry.width - tile_x);
                for row in 0..copy_height {
                    let source_start = row * geometry.chunk_width;
                    let target_start = row * geometry.width + tile_x;
                    band[target_start..target_start + copy_width]
                        .copy_from_slice(&decoded[source_start..source_start + copy_width]);
                }
            }
            Ok::<(), CodecError>(())
        })?;

    if request.bits_per_sample <= 8 {
        let samples = output
            .into_iter()
            .map(|value| {
                u8::try_from(value).map_err(|_| invalid("lossless JPEG sample exceeds u8 storage"))
            })
            .collect::<Result<Vec<_>, _>>()?;
        buffer_u8(geometry.width, geometry.height, samples)
    } else {
        buffer_u16(geometry.width, geometry.height, output)
    }
}

#[cfg(feature = "hasselblad")]
pub fn decode_hasselblad(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    if request.sample_format != SampleFormat::Integer
        || request.bits_per_sample != 16
        || request.predictor != Predictor::None
    {
        return Err(invalid(
            "Hasselblad lossless JPEG requires 16-bit integer samples",
        ));
    }
    let geometry = Geometry::new(
        request.width,
        request.height,
        request.chunk_width,
        request.chunk_height,
        request.chunks.len(),
    )?;
    if geometry.chunk_width != geometry.width
        || geometry.chunk_height != geometry.height
        || request.chunks.len() != 1
    {
        return Err(invalid(
            "Hasselblad lossless JPEG requires one full-image chunk",
        ));
    }
    let encoded = chunk(request.source, request.chunks[0])?;
    let decoded = decode_hasselblad_interleaved(encoded, request.width, request.height)
        .map_err(|detail| invalid(format!("Hasselblad lossless JPEG: {detail}")))?;
    buffer_u16(geometry.width, geometry.height, decoded)
}

pub fn encode(request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError> {
    if request.predictor != Predictor::None {
        return Err(invalid(
            "lossless JPEG prediction is selected by the encoder",
        ));
    }
    let width =
        usize::try_from(request.image.width()).expect("u32 fits usize on supported platforms");
    let height =
        usize::try_from(request.image.height()).expect("u32 fits usize on supported platforms");
    let across = width.div_ceil(request.chunk_width.max(1));
    let down = height.div_ceil(request.chunk_height.max(1));
    let chunk_count = across
        .checked_mul(down)
        .ok_or_else(|| invalid("lossless JPEG chunk count overflow"))?;
    let geometry = Geometry::new(
        width,
        height,
        request.chunk_width,
        request.chunk_height,
        chunk_count,
    )?;
    let bits = match request.image {
        SampleBuffer::U8(_) => 8_u8,
        SampleBuffer::U16(_) => 16_u8,
        _ => {
            return Err(invalid("lossless JPEG encoding requires U8 or U16 samples"));
        }
    };
    let width = u16::try_from(request.chunk_width)
        .map_err(|_| invalid("lossless JPEG chunk width exceeds u16"))?;
    let height = u16::try_from(request.chunk_height)
        .map_err(|_| invalid("lossless JPEG chunk height exceeds u16"))?;
    let mut tile = vec![0_u16; geometry.chunk_sample_count()?];
    let mut chunks = Vec::with_capacity(chunk_count);
    for tile_y_index in 0..geometry.down {
        let tile_y = tile_y_index * geometry.chunk_height;
        for tile_x_index in 0..geometry.across {
            let tile_x = tile_x_index * geometry.chunk_width;
            copy_tile_into(
                request.image,
                geometry.width,
                geometry.height,
                tile_x,
                tile_y,
                geometry.chunk_width,
                geometry.chunk_height,
                &mut tile,
            );
            let index = tile_y_index * geometry.across + tile_x_index;
            chunks.push(
                encode_tile(&tile, width, height, bits)
                    .map_err(|detail| invalid(format!("lossless JPEG chunk {index}: {detail}")))?,
            );
        }
    }
    Ok(EncodedChunks { chunks })
}

fn copy_tile_into(
    image: &SampleBuffer,
    source_width: usize,
    source_height: usize,
    tile_x: usize,
    tile_y: usize,
    tile_width: usize,
    tile_height: usize,
    output: &mut [u16],
) {
    debug_assert_eq!(output.len(), tile_width * tile_height);
    let copy_width = tile_width.min(source_width - tile_x);
    let copy_height = tile_height.min(source_height - tile_y);
    if copy_width != tile_width || copy_height != tile_height {
        output.fill(0);
    }

    match image {
        SampleBuffer::U8(samples) => {
            let samples = samples.as_raw();
            for row in 0..copy_height {
                let source_start = (tile_y + row) * source_width + tile_x;
                let target_start = row * tile_width;
                for (target, sample) in output[target_start..target_start + copy_width]
                    .iter_mut()
                    .zip(&samples[source_start..source_start + copy_width])
                {
                    *target = u16::from(*sample);
                }
            }
        }
        SampleBuffer::U16(samples) => {
            let samples = samples.as_raw();
            for row in 0..copy_height {
                let source_start = (tile_y + row) * source_width + tile_x;
                let target_start = row * tile_width;
                output[target_start..target_start + copy_width]
                    .copy_from_slice(&samples[source_start..source_start + copy_width]);
            }
        }
        SampleBuffer::U32(_) | SampleBuffer::F32(_) | SampleBuffer::F64(_) => {
            unreachable!("the encoder validates that samples are U8 or U16")
        }
    }
}

#[derive(Clone, Copy)]
struct Component {
    id: u8,
    huffman_table: usize,
}

#[derive(Clone, Copy)]
struct HuffmanEntry {
    symbol: u8,
    bits: u8,
}

impl HuffmanEntry {
    const INVALID: Self = Self { symbol: 0, bits: 0 };
}

struct HuffmanTable {
    lookup: Box<[HuffmanEntry]>,
}

impl HuffmanTable {
    fn parse(
        counts: &[u8; 16],
        symbols: &[u8],
        lookup: &mut [HuffmanEntry],
    ) -> Result<Self, String> {
        lookup.fill(HuffmanEntry::INVALID);

        let mut symbol_index = 0;
        let mut lookup_index = 0_usize;
        for (length_index, &count) in counts.iter().enumerate() {
            let length = u8::try_from(length_index + 1)
                .expect("lossless JPEG Huffman lengths are at most 16");
            for _ in 0..count {
                let symbol = *symbols
                    .get(symbol_index)
                    .ok_or_else(|| "lossless JPEG Huffman symbol table is truncated".to_owned())?;
                let width = 1_usize << (HUFFMAN_LOOKAHEAD_BITS - usize::from(length));
                let end = lookup_index
                    .checked_add(width)
                    .ok_or_else(|| "lossless JPEG Huffman table overflow".to_owned())?;
                let entries = lookup
                    .get_mut(lookup_index..end)
                    .ok_or_else(|| "lossless JPEG Huffman code lengths are invalid".to_owned())?;
                entries.fill(HuffmanEntry {
                    symbol,
                    bits: length,
                });
                lookup_index = end;
                symbol_index += 1;
            }
        }

        Ok(Self {
            lookup: lookup.into(),
        })
    }

    // Keep the DNG hot path concrete. Genericizing these bit-reader helpers
    // regressed decoder raw_mosaic and cropped_mosaic benchmarks.
    fn decode(&self, bits: &mut BitReader<'_>) -> Result<u8, String> {
        let entry = self.lookup[usize::from(bits.peek(16))];
        if entry.bits == 0 {
            return Err("lossless JPEG contains an invalid Huffman code".to_owned());
        }
        bits.skip(usize::from(entry.bits));
        Ok(entry.symbol)
    }

    #[cfg(feature = "hasselblad")]
    fn decode_hasselblad(&self, bits: &mut HasselbladBitReader<'_>) -> Result<u8, String> {
        let entry = self.lookup[usize::from(bits.peek(16))];
        if entry.bits == 0 {
            return Err("lossless JPEG contains an invalid Huffman code".to_owned());
        }
        bits.skip(usize::from(entry.bits));
        Ok(entry.symbol)
    }
}

struct Frame {
    width: usize,
    height: usize,
    precision: u8,
    components: Vec<Component>,
}

struct BitReader<'a> {
    bytes: &'a [u8],
    position: usize,
    buffer: u64,
    buffered_bits: u8,
    ended: bool,
}

#[cfg(feature = "hasselblad")]
struct HasselbladBitReader<'a> {
    bytes: &'a [u8],
    position: usize,
    buffer: u64,
    buffered_bits: u8,
    exhausted: bool,
}

#[cfg(feature = "hasselblad")]
impl<'a> HasselbladBitReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            position: 0,
            buffer: 0,
            buffered_bits: 0,
            exhausted: false,
        }
    }
}

#[cfg(feature = "hasselblad")]
impl HasselbladBitReader<'_> {
    fn peek(&mut self, count: u8) -> u16 {
        while self.buffered_bits < count {
            let end = self.position.saturating_add(4).min(self.bytes.len());
            if end == self.position {
                self.exhausted = true;
                self.buffer <<= 32;
                self.buffered_bits += 32;
                continue;
            }
            let mut word = [0_u8; 4];
            word[..end.saturating_sub(self.position)]
                .copy_from_slice(&self.bytes[self.position..end]);
            let loaded_bits = (end - self.position) * 8;
            let aligned = u32::from_le_bytes(word) << (32 - loaded_bits);
            self.buffer = (self.buffer << loaded_bits) | u64::from(aligned);
            self.buffered_bits +=
                u8::try_from(loaded_bits).expect("a bit-pump refill is at most 32");
            self.position = end;
        }
        let shift = self.buffered_bits - count;
        let mask = (1_u64 << count) - 1;
        u16::try_from((self.buffer >> shift) & mask).unwrap_or_default()
    }

    fn skip(&mut self, count: usize) {
        self.buffered_bits = self
            .buffered_bits
            .saturating_sub(u8::try_from(count).expect("lossless JPEG bit counts are at most 16"));
        self.buffer &= (1_u64 << self.buffered_bits) - 1;
    }
}

impl<'a> BitReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            position: 0,
            buffer: 0,
            buffered_bits: 0,
            ended: false,
        }
    }

    fn peek(&mut self, count: u8) -> u16 {
        while self.buffered_bits < count {
            let fast = self
                .bytes
                .get(self.position..self.position.saturating_add(4))
                .filter(|bytes| bytes.iter().all(|byte| *byte != MARKER_PREFIX));
            if let Some(bytes) = fast {
                let word = u32::from_be_bytes(bytes.try_into().expect("four bytes are checked"));
                self.buffer = (self.buffer << 32) | u64::from(word);
                self.position += 4;
                self.buffered_bits += 32;
            } else {
                let byte = self.next_byte();
                self.buffer = (self.buffer << 8) | u64::from(byte);
                self.buffered_bits += 8;
            }
        }
        let shift = self.buffered_bits - count;
        let mask = (1_u64 << count) - 1;
        u16::try_from((self.buffer >> shift) & mask).unwrap_or_default()
    }

    fn skip(&mut self, count: usize) {
        self.buffered_bits = self
            .buffered_bits
            .saturating_sub(u8::try_from(count).expect("lossless JPEG bit counts are at most 16"));
        self.buffer &= (1_u64 << self.buffered_bits) - 1;
    }

    fn next_byte(&mut self) -> u8 {
        if self.ended || self.position >= self.bytes.len() {
            self.ended = true;
            return 0;
        }

        let byte = self.bytes[self.position];
        if byte != MARKER_PREFIX {
            self.position += 1;
            return byte;
        }

        if self.bytes.get(self.position + 1) == Some(&0) {
            self.position += 2;
            return byte;
        }

        self.ended = true;
        0
    }
}

struct HeaderReader<'a> {
    bytes: &'a [u8],
    position: usize,
}

impl<'a> HeaderReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, position: 0 } }

    fn u8(&mut self) -> Result<u8, String> {
        let byte = *self
            .bytes
            .get(self.position)
            .ok_or_else(|| "lossless JPEG header is truncated".to_owned())?;
        self.position += 1;
        Ok(byte)
    }

    fn u16(&mut self) -> Result<u16, String> {
        let high = u16::from(self.u8()?);
        let low = u16::from(self.u8()?);
        Ok((high << 8) | low)
    }

    fn take(&mut self, count: usize) -> Result<&'a [u8], String> {
        let end = self
            .position
            .checked_add(count)
            .ok_or_else(|| "lossless JPEG segment length overflow".to_owned())?;
        let bytes = self
            .bytes
            .get(self.position..end)
            .ok_or_else(|| "lossless JPEG segment is truncated".to_owned())?;
        self.position = end;
        Ok(bytes)
    }

    fn marker(&mut self) -> Result<u8, String> {
        while self.u8()? != MARKER_PREFIX {}
        loop {
            let marker = self.u8()?;
            if marker != MARKER_PREFIX {
                return Ok(marker);
            }
        }
    }
}

pub struct InterleavedJpeg {
    pub width: usize,
    pub height: usize,
    #[cfg(feature = "arw")]
    pub components: usize,
    pub samples: Vec<u16>,
}

pub fn decode_interleaved(
    source: &[u8],
    expected_precision: u8,
) -> Result<InterleavedJpeg, String> {
    let mut header = HeaderReader::new(source);
    if header.marker()? != MARKER_SOI {
        return Err("lossless JPEG stream does not start with SOI".to_owned());
    }

    let mut frame = None;
    let mut tables: [Option<HuffmanTable>; 4] = std::array::from_fn(|_| None);
    let (predictor, point_transform) = loop {
        let marker = header.marker()?;
        match marker {
            MARKER_SOF3 => frame = Some(parse_frame(&mut header)?),
            MARKER_DHT => parse_huffman_tables(&mut header, &mut tables)?,
            MARKER_SOS => {
                let frame = frame
                    .as_mut()
                    .ok_or_else(|| "lossless JPEG SOS appears before SOF3".to_owned())?;
                break parse_scan(&mut header, frame)?;
            }
            MARKER_EOI => {
                return Err("lossless JPEG reached EOI before SOS".to_owned());
            }
            MARKER_PREFIX | MARKER_SOI => {
                return Err(format!(
                    "lossless JPEG contains unexpected marker 0x{marker:02x}"
                ));
            }
            _ => {
                let length = usize::from(header.u16()?);
                if length < 2 {
                    return Err("lossless JPEG segment length is invalid".to_owned());
                }
                header.take(length - 2)?;
            }
        }
    };

    let frame = frame.expect("SOF3 is parsed before the scan starts");
    if frame.precision > expected_precision {
        return Err(format!(
            "lossless JPEG precision is {}, exceeding the {expected_precision}-bit container",
            frame.precision
        ));
    }
    let output_width = frame
        .width
        .checked_mul(frame.components.len())
        .ok_or_else(|| "lossless JPEG output width overflow".to_owned())?;
    if !(1..=7).contains(&predictor) {
        return Err(format!(
            "lossless JPEG predictor {predictor} is unsupported"
        ));
    }
    if point_transform >= frame.precision {
        return Err("lossless JPEG point transform is invalid".to_owned());
    }

    let entropy = header
        .bytes
        .get(header.position..)
        .ok_or_else(|| "lossless JPEG entropy stream is missing".to_owned())?;
    let sample_count = output_width
        .checked_mul(frame.height)
        .ok_or_else(|| "lossless JPEG sample count overflow".to_owned())?;
    let mut output = vec![0_u16; sample_count];
    decode_scan_into(
        entropy,
        &frame,
        &tables,
        predictor,
        point_transform,
        output_width,
        &mut output,
    )?;
    if point_transform != 0 {
        for value in &mut output {
            *value = value.wrapping_shl(u32::from(point_transform));
        }
    }
    Ok(InterleavedJpeg {
        width: output_width,
        height: frame.height,
        #[cfg(feature = "arw")]
        components: frame.components.len(),
        samples: output,
    })
}

fn decode_into(
    source: &[u8],
    expected_width: usize,
    expected_height: usize,
    expected_precision: u8,
    output: &mut [u16],
) -> Result<(), String> {
    let decoded = decode_interleaved(source, expected_precision)?;
    if decoded.width != expected_width || decoded.height != expected_height {
        return Err(format!(
            "lossless JPEG frame is {}x{}, expected {expected_width}x{expected_height}",
            decoded.width, decoded.height
        ));
    }
    output.copy_from_slice(&decoded.samples);
    Ok(())
}

#[cfg(feature = "hasselblad")]
fn decode_hasselblad_interleaved(
    source: &[u8],
    expected_width: usize,
    expected_height: usize,
) -> Result<Vec<u16>, String> {
    let mut header = HeaderReader::new(source);
    if header.marker()? != MARKER_SOI {
        return Err("stream does not start with SOI".to_owned());
    }

    let mut frame = None;
    let mut tables: [Option<HuffmanTable>; 4] = std::array::from_fn(|_| None);
    let (predictor, point_transform) = loop {
        let marker = header.marker()?;
        match marker {
            MARKER_SOF3 => frame = Some(parse_frame(&mut header)?),
            MARKER_DHT => parse_huffman_tables(&mut header, &mut tables)?,
            0xdd => {
                let length = header.u16()?;
                if length != 4 || header.u16()? != 0 {
                    return Err("restart intervals are unsupported".to_owned());
                }
            }
            MARKER_SOS => {
                let frame = frame
                    .as_mut()
                    .ok_or_else(|| "SOS precedes SOF3".to_owned())?;
                break parse_scan(&mut header, frame)?;
            }
            MARKER_EOI => {
                return Err("stream reached EOI before SOS".to_owned());
            }
            _ => {
                let length = usize::from(header.u16()?);
                if length < 2 {
                    return Err("segment length is invalid".to_owned());
                }
                header.take(length - 2)?;
            }
        }
    };

    let frame = frame.ok_or_else(|| "stream has no SOF3".to_owned())?;
    if frame.precision != 16
        || frame.components.len() != 1
        || frame.width != expected_width
        || frame.height != expected_height
    {
        return Err("frame must be one-component 16-bit data matching the raw IFD".to_owned());
    }
    if predictor != 8 || point_transform != 0 {
        return Err("predictor 8 with no point transform is required".to_owned());
    }
    let table = tables[frame.components[0].huffman_table]
        .as_ref()
        .ok_or_else(|| "stream references a missing Huffman table".to_owned())?;
    let entropy = header
        .bytes
        .get(header.position..)
        .ok_or_else(|| "entropy stream is missing".to_owned())?;
    if entropy.is_empty() || (entropy[0] == MARKER_PREFIX && entropy.get(1) != Some(&0)) {
        return Err("entropy stream is truncated".to_owned());
    }
    let sample_count = expected_width
        .checked_mul(expected_height)
        .ok_or_else(|| "sample count overflow".to_owned())?;
    let mut output = Vec::with_capacity(sample_count);
    let mut bits = HasselbladBitReader::new(entropy);
    for _row in 0..expected_height {
        let mut first = 0x8000_i32;
        let mut second = 0x8000_i32;
        for column in (0..expected_width).step_by(2) {
            let first_length = table
                .decode_hasselblad(&mut bits)
                .map_err(|detail| format!("{detail} at row {_row}, column {column}"))?;
            let second_length = table
                .decode_hasselblad(&mut bits)
                .map_err(|detail| format!("{detail} at row {_row}, column {}", column + 1))?;
            first = first.wrapping_add(decode_hasselblad_difference(first_length, &mut bits)?);
            second = second.wrapping_add(decode_hasselblad_difference(second_length, &mut bits)?);
            output.push(first as u16);
            output.push(second as u16);
        }
    }
    if bits.exhausted {
        return Err("entropy stream is truncated".to_owned());
    }
    Ok(output)
}

fn parse_frame(header: &mut HeaderReader<'_>) -> Result<Frame, String> {
    let length = usize::from(header.u16()?);
    let precision = header.u8()?;
    let height = usize::from(header.u16()?);
    let width = usize::from(header.u16()?);
    let component_count = usize::from(header.u8()?);
    if length != 8 + component_count * 3 {
        return Err("lossless JPEG SOF3 length is invalid".to_owned());
    }
    if !(1..=4).contains(&component_count) {
        return Err(format!(
            "lossless JPEG component count {component_count} is unsupported"
        ));
    }
    if !(2..=16).contains(&precision) {
        return Err(format!(
            "lossless JPEG precision {precision} is unsupported"
        ));
    }
    if width == 0 || height == 0 {
        return Err("lossless JPEG dimensions must not be zero".to_owned());
    }

    let mut components = Vec::with_capacity(component_count);
    for _ in 0..component_count {
        let id = header.u8()?;
        let sampling = header.u8()?;
        let _quantization = header.u8()?;
        if sampling != 0x11 {
            return Err(format!(
                "lossless JPEG sampling {sampling:#04x} is unsupported"
            ));
        }
        components.push(Component {
            id,
            huffman_table: 0,
        });
    }

    Ok(Frame {
        width,
        height,
        precision,
        components,
    })
}

fn parse_huffman_tables(
    header: &mut HeaderReader<'_>,
    tables: &mut [Option<HuffmanTable>; 4],
) -> Result<(), String> {
    let segment_length = usize::from(header.u16()?);
    if segment_length < 2 {
        return Err("lossless JPEG DHT length is invalid".to_owned());
    }
    let mut remaining = segment_length - 2;
    while remaining > 0 {
        let selector = header.u8()?;
        remaining -= 1;
        let class = selector >> 4;
        let table_index = usize::from(selector & 0x0f);
        if class != 0 || table_index >= tables.len() {
            return Err(format!(
                "lossless JPEG Huffman selector {selector:#04x} is unsupported"
            ));
        }
        let counts: [u8; 16] = header
            .take(16)?
            .try_into()
            .expect("the Huffman count slice has a fixed length");
        remaining -= 16;
        let symbol_count = counts
            .iter()
            .map(|&count| usize::from(count))
            .sum::<usize>();
        if symbol_count > 256 || symbol_count > remaining {
            return Err("lossless JPEG DHT symbol table is invalid".to_owned());
        }
        let symbols = header.take(symbol_count)?;
        remaining -= symbol_count;
        let mut lookup = vec![HuffmanEntry::INVALID; HUFFMAN_LOOKAHEAD_SIZE];
        tables[table_index] = Some(HuffmanTable::parse(&counts, symbols, &mut lookup)?);
    }
    Ok(())
}

fn parse_scan(header: &mut HeaderReader<'_>, frame: &mut Frame) -> Result<(u8, u8), String> {
    let length = usize::from(header.u16()?);
    if length != 6 + frame.components.len() * 2 {
        return Err("lossless JPEG SOS length is invalid".to_owned());
    }
    let component_count = usize::from(header.u8()?);
    if component_count != frame.components.len() {
        return Err("lossless JPEG SOS component count does not match SOF3".to_owned());
    }
    for _ in 0..component_count {
        let selector = header.u8()?;
        let table_selector = header.u8()?;
        let table_index = usize::from(table_selector >> 4);
        if table_selector & 0x0f != 0 || table_index >= 4 {
            return Err(format!(
                "lossless JPEG scan selector {table_selector:#04x} is unsupported"
            ));
        }
        let component = frame
            .components
            .iter_mut()
            .find(|component| component.id == selector)
            .ok_or_else(|| format!("lossless JPEG scan component {selector} is invalid"))?;
        component.huffman_table = table_index;
    }
    let predictor = header.u8()?;
    let end_of_spectral = header.u8()?;
    let point_transform = header.u8()?;
    if end_of_spectral != 0 || point_transform & 0xf0 != 0 {
        return Err("lossless JPEG scan transformation is unsupported".to_owned());
    }
    Ok((predictor, point_transform & 0x0f))
}

fn decode_scan_into(
    entropy: &[u8],
    frame: &Frame,
    tables: &[Option<HuffmanTable>; 4],
    predictor: u8,
    point_transform: u8,
    output_width: usize,
    output: &mut [u16],
) -> Result<(), String> {
    let mut components = Vec::with_capacity(frame.components.len());
    for component in &frame.components {
        components.push(
            tables[component.huffman_table]
                .as_ref()
                .ok_or_else(|| "lossless JPEG references a missing Huffman table".to_owned())?,
        );
    }

    let sample_count = output_width
        .checked_mul(frame.height)
        .ok_or_else(|| "lossless JPEG sample count overflow".to_owned())?;
    if output.len() != sample_count {
        return Err(format!(
            "lossless JPEG output has {} samples, expected {sample_count}",
            output.len()
        ));
    }
    let mut bits = BitReader::new(entropy);
    let initial = 1_i32 << (frame.precision - point_transform - 1);
    let component_count = frame.components.len();

    for (component_index, table) in components.iter().enumerate() {
        let symbol = table
            .decode(&mut bits)
            .map_err(|detail| format!("{detail} at row 0, column 0"))?;
        output[component_index] = add_prediction(initial, symbol, &mut bits, frame.precision)?;
    }

    for row in 0..frame.height {
        let first_column = usize::from(row == 0);
        for column in first_column..frame.width {
            for (component_index, table) in components.iter().enumerate() {
                let index = row * output_width + column * component_count + component_index;
                let prediction = predict(
                    output,
                    output_width,
                    row,
                    column,
                    component_index,
                    component_count,
                    predictor,
                );
                let symbol = table.decode(&mut bits).map_err(|detail| {
                    format!("{detail} at row {row}, column {column}, component {component_index}")
                })?;
                output[index] = add_prediction(prediction, symbol, &mut bits, frame.precision)?;
            }
        }
    }

    Ok(())
}

fn predict(
    output: &[u16],
    output_width: usize,
    row: usize,
    column: usize,
    component: usize,
    components: usize,
    predictor: u8,
) -> i32 {
    let index = row * output_width + column * components + component;
    if column == 0 {
        debug_assert!(row > 0, "the first scan sample is initialized separately");
        return i32::from(output[index - output_width]);
    }

    let left = i32::from(output[index - components]);
    if row == 0 {
        return left;
    }

    let above = i32::from(output[index - output_width]);
    let above_left = i32::from(output[index - output_width - components]);
    match predictor {
        1 => left,
        2 => above,
        3 => above_left,
        4 => left + above - above_left,
        5 => left + ((above - above_left) >> 1),
        6 => above + ((left - above_left) >> 1),
        7 => (left + above) >> 1,
        _ => unreachable!("the predictor is validated before scanning"),
    }
}

fn add_prediction(
    prediction: i32,
    symbol: u8,
    bits: &mut BitReader<'_>,
    precision: u8,
) -> Result<u16, String> {
    let difference = match symbol {
        0 => 0,
        16 => -32_768,
        1..=15 => {
            let bit_count = symbol;
            let raw = i32::from(bits.peek(bit_count));
            bits.skip(usize::from(bit_count));
            let threshold = 1_i32 << (bit_count - 1);
            if raw < threshold {
                raw - ((1_i32 << bit_count) - 1)
            } else {
                raw
            }
        }
        _ => {
            return Err(format!(
                "lossless JPEG difference class {symbol} is unsupported"
            ));
        }
    };
    let mask = (1_u32 << precision) - 1;
    Ok(
        u16::try_from((prediction.wrapping_add(difference) as u32) & mask)
            .expect("the precision mask limits samples to u16"),
    )
}

#[cfg(feature = "hasselblad")]
fn decode_hasselblad_difference(
    symbol: u8,
    bits: &mut HasselbladBitReader<'_>,
) -> Result<i32, String> {
    // Hasselblad class 16 has an extra 16-bit payload that DNG class 16 omits.
    Ok(match symbol {
        0 => 0,
        16 => {
            bits.peek(16);
            bits.skip(16);
            -32_768
        }
        1..=15 => {
            let bit_count = symbol;
            let raw = i32::from(bits.peek(bit_count));
            bits.skip(usize::from(bit_count));
            let threshold = 1_i32 << (bit_count - 1);
            if raw < threshold {
                raw - ((1_i32 << bit_count) - 1)
            } else {
                raw
            }
        }
        _ => {
            return Err(format!(
                "lossless JPEG difference class {symbol} is unsupported"
            ));
        }
    })
}

const HUFFMAN_BITS: u8 = 5;

fn encode_tile(samples: &[u16], width: u16, height: u16, bits: u8) -> Result<Vec<u8>, String> {
    encode_interleaved(samples, width, height, 1, bits)
}

#[cfg(all(test, feature = "arw"))]
pub fn encode_interleaved_for_test(
    samples: &[u16],
    width: u16,
    height: u16,
    components: u8,
    bits: u8,
) -> Vec<u8> {
    encode_interleaved(samples, width, height, components, bits).unwrap()
}

fn encode_interleaved(
    samples: &[u16],
    width: u16,
    height: u16,
    components: u8,
    bits: u8,
) -> Result<Vec<u8>, String> {
    if !(2..=16).contains(&bits) {
        return Err(format!("lossless JPEG bit depth {bits} is invalid"));
    }
    if !(1..=4).contains(&components) {
        return Err(format!(
            "lossless JPEG component count {components} is invalid"
        ));
    }
    let width_usize = usize::from(width);
    let height_usize = usize::from(height);
    let components_usize = usize::from(components);
    let expected = usize::from(width)
        .checked_mul(usize::from(height))
        .and_then(|samples| samples.checked_mul(components_usize))
        .ok_or_else(|| "lossless JPEG tile size overflow".to_owned())?;
    if samples.len() != expected {
        return Err(format!(
            "lossless JPEG input has {} samples, expected {expected}",
            samples.len()
        ));
    }

    let mut encoded = Vec::with_capacity(samples.len() * 2);
    write_marker(&mut encoded, MARKER_SOI);
    write_marker(&mut encoded, MARKER_SOF3);
    push_u16(&mut encoded, 8 + 3 * u16::from(components));
    encoded.push(bits);
    push_u16(&mut encoded, height);
    push_u16(&mut encoded, width);
    encoded.push(components);
    for component in 1..=components {
        encoded.extend_from_slice(&[component, 0x11, 0]);
    }

    write_marker(&mut encoded, MARKER_DHT);
    push_u16(&mut encoded, 36);
    encoded.push(0);
    encoded.extend_from_slice(&[0, 0, 0, 0, 17, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
    encoded.extend(0_u8..=16);

    write_marker(&mut encoded, MARKER_SOS);
    push_u16(&mut encoded, 6 + 2 * u16::from(components));
    encoded.push(components);
    for component in 1..=components {
        encoded.extend_from_slice(&[component, 0]);
    }
    encoded.extend_from_slice(&[1, 0, 0]);

    let modulus = 1_i32 << bits;
    let half = modulus >> 1;
    let initial = half as u16;
    let mut writer = BitWriter::new(&mut encoded);
    let row_width = width_usize * components_usize;
    for row in 0..height_usize {
        for column in 0..width_usize {
            for component in 0..components_usize {
                let index = row * row_width + column * components_usize + component;
                let prediction = if row == 0 && column == 0 {
                    initial
                } else if column == 0 {
                    samples[index - row_width]
                } else {
                    samples[index - components_usize]
                };
                let raw_difference = i32::from(samples[index]) - i32::from(prediction);
                let difference = (raw_difference + half).rem_euclid(modulus) - half;
                let category = if difference == 0 {
                    0
                } else {
                    difference.unsigned_abs().ilog2() + 1
                };
                let category =
                    u8::try_from(category).expect("lossless JPEG category is at most 16");
                writer.write(u32::from(category), HUFFMAN_BITS);
                if category != 0 && category != 16 {
                    let value = if difference < 0 {
                        difference + ((1_i32 << category) - 1)
                    } else {
                        difference
                    };
                    writer.write(
                        u32::try_from(value).expect("lossless JPEG amplitude is non-negative"),
                        category,
                    );
                }
            }
        }
    }
    writer.finish();
    write_marker(&mut encoded, MARKER_EOI);
    Ok(encoded)
}

fn write_marker(encoded: &mut Vec<u8>, marker: u8) {
    encoded.extend_from_slice(&[MARKER_PREFIX, marker]);
}

fn push_u16(encoded: &mut Vec<u8>, value: u16) { encoded.extend_from_slice(&value.to_be_bytes()); }

struct BitWriter<'a> {
    encoded: &'a mut Vec<u8>,
    buffer: u64,
    used: u8,
}

impl<'a> BitWriter<'a> {
    const fn new(encoded: &'a mut Vec<u8>) -> Self {
        Self {
            encoded,
            buffer: 0,
            used: 0,
        }
    }

    fn write(&mut self, value: u32, bits: u8) {
        if bits == 0 {
            return;
        }
        let mask = (1_u64 << bits) - 1;
        self.buffer = (self.buffer << bits) | (u64::from(value) & mask);
        self.used += bits;
        while self.used >= 8 {
            self.used -= 8;
            self.flush_byte((self.buffer >> self.used) as u8);
        }
        if self.used == 0 {
            self.buffer = 0;
        } else {
            self.buffer &= (1_u64 << self.used) - 1;
        }
    }

    fn finish(&mut self) {
        while self.used != 0 {
            self.write(1, 1);
        }
    }

    fn flush_byte(&mut self, byte: u8) {
        self.encoded.push(byte);
        if byte == MARKER_PREFIX {
            self.encoded.push(0);
        }
    }
}

#[cfg(test)]
mod tests {
    use super::{BitReader, BitWriter, add_prediction, copy_tile_into, predict};
    use crate::SampleBuffer;

    #[test]
    fn fourteen_bit_difference_class_consumes_amplitude_bits() {
        let mut bits = BitReader::new(&[0x8c, 0xa0]);
        assert_eq!(add_prediction(100, 14, &mut bits, 14).unwrap(), 9_100);
    }

    #[test]
    fn all_lossless_jpeg_predictors_use_the_expected_neighbors() {
        let mut output = vec![0_u16; 9];
        output[0] = 4;
        output[1] = 20;
        output[3] = 10;

        assert_eq!(predict(&output, 3, 1, 1, 0, 1, 1), 10);
        assert_eq!(predict(&output, 3, 1, 1, 0, 1, 2), 20);
        assert_eq!(predict(&output, 3, 1, 1, 0, 1, 3), 4);
        assert_eq!(predict(&output, 3, 1, 1, 0, 1, 4), 26);
        assert_eq!(predict(&output, 3, 1, 1, 0, 1, 5), 18);
        assert_eq!(predict(&output, 3, 1, 1, 0, 1, 6), 23);
        assert_eq!(predict(&output, 3, 1, 1, 0, 1, 7), 15);
    }

    #[test]
    fn tile_copy_extracts_u8_and_u16_partial_edges() {
        let u8_image = SampleBuffer::from_u8(5, 3, (0_u8..15).collect()).unwrap();
        let mut u8_tile = vec![0_u16; 8];
        copy_tile_into(&u8_image, 5, 3, 4, 1, 4, 2, &mut u8_tile);
        assert_eq!(u8_tile, vec![9, 0, 0, 0, 14, 0, 0, 0]);

        let u16_image =
            SampleBuffer::from_u16(5, 3, (0_u16..15).map(|value| value * 1_000).collect()).unwrap();
        let mut u16_tile = vec![0_u16; 8];
        copy_tile_into(&u16_image, 5, 3, 4, 1, 4, 2, &mut u16_tile);
        assert_eq!(u16_tile, vec![9_000, 0, 0, 0, 14_000, 0, 0, 0]);
    }

    #[test]
    fn bit_writer_preserves_packed_bytes_padding_and_stuffing() {
        let mut encoded = Vec::new();
        let mut writer = BitWriter::new(&mut encoded);
        writer.write(0b101, 3);
        writer.write(0x1ff, 9);
        writer.finish();
        assert_eq!(encoded, [0xbf, 0xff, 0x00]);

        let mut encoded = Vec::new();
        let mut writer = BitWriter::new(&mut encoded);
        writer.write(0x1234, 16);
        writer.finish();
        assert_eq!(encoded, [0x12, 0x34]);
    }
}
