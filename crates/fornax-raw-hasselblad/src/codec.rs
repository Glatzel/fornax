use fornax_codec::{
    CodecError, DecodeCompression, DecodeRequest, ICodecDecode, Predictor, SampleBuffer,
    SampleFormat,
};

use crate::samples::{buffer_u16, chunk};

const MARKER_PREFIX: u8 = 0xff;
const MARKER_SOF3: u8 = 0xc3;
const MARKER_DHT: u8 = 0xc4;
const MARKER_SOI: u8 = 0xd8;
const MARKER_EOI: u8 = 0xd9;
const MARKER_SOS: u8 = 0xda;
const HUFFMAN_LOOKAHEAD_BITS: usize = 16;
const HUFFMAN_LOOKAHEAD_SIZE: usize = 1 << HUFFMAN_LOOKAHEAD_BITS;

#[derive(Clone, Copy, Debug)]
pub enum HasselbladCodec {
    Uncompressed,
    LosslessJpeg,
}

impl ICodecDecode for HasselbladCodec {
    fn decode(&self, request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
        match self {
            Self::Uncompressed => DecodeCompression::Uncompressed.decode(request),
            Self::LosslessJpeg => decode_lossless(request),
        }
    }
}

fn decode_lossless(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    if request.sample_format != SampleFormat::Integer
        || request.bits_per_sample != 16
        || request.predictor != Predictor::None
    {
        return Err(invalid(
            "Hasselblad lossless JPEG requires 16-bit integer samples",
        ));
    }
    if request.chunk_width != request.width
        || request.chunk_height != request.height
        || request.chunks.len() != 1
    {
        return Err(invalid(
            "Hasselblad lossless JPEG requires one full-image chunk",
        ));
    }
    let encoded = chunk(request.source, request.chunks[0])?;
    let decoded = decode_interleaved(encoded, request.width, request.height)
        .map_err(|detail| invalid(format!("Hasselblad lossless JPEG: {detail}")))?;
    buffer_u16(request.width, request.height, decoded)
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

    fn decode(&self, bits: &mut BitReader<'_>) -> Result<u8, String> {
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
    exhausted: bool,
}

impl<'a> BitReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            position: 0,
            buffer: 0,
            buffered_bits: 0,
            exhausted: false,
        }
    }

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

fn decode_interleaved(
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
            MARKER_EOI => return Err("stream reached EOI before SOS".to_owned()),
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
    let mut bits = BitReader::new(entropy);
    for row in 0..expected_height {
        let mut first = 0x8000_i32;
        let mut second = 0x8000_i32;
        for column in (0..expected_width).step_by(2) {
            let first_length = table
                .decode(&mut bits)
                .map_err(|detail| format!("{detail} at row {row}, column {column}"))?;
            let second_length = table
                .decode(&mut bits)
                .map_err(|detail| format!("{detail} at row {row}, column {}", column + 1))?;
            first = first.wrapping_add(decode_difference(first_length, &mut bits)?);
            second = second.wrapping_add(decode_difference(second_length, &mut bits)?);
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

fn decode_difference(symbol: u8, bits: &mut BitReader<'_>) -> Result<i32, String> {
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

fn invalid<T>(detail: T) -> CodecError
where
    T: Into<Box<str>>,
{
    CodecError::InvalidInput(detail.into())
}
