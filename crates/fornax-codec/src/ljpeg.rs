//! Restart-free lossless JPEG-92 color-component decoding.
//!
//! The DNG-specific tile assembly and metadata decisions are documented in
//! `docs/reference/decoders/reference/dng-lossless-jpeg.md`.

use rayon::prelude::*;

use crate::allocation::zeroed_samples;
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
const MARKER_DRI: u8 = 0xdd;
const HUFFMAN_LOOKAHEAD_BITS: usize = 16;
const FAST_HUFFMAN_LOOKAHEAD_BITS: usize = 10;
const FAST_HUFFMAN_LOOKAHEAD_BITS_U8: u8 = 10;
const FAST_HUFFMAN_LOOKAHEAD_SIZE: usize = 1 << FAST_HUFFMAN_LOOKAHEAD_BITS;
const UNSTUFF_THRESHOLD_BYTES: usize = 1 << 20;
const UNSTUFF_CHUNK_BYTES: usize = 4 * 1024 * 1024;

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
    let mut output = zeroed_samples(sample_count)?;
    let expected_samples = geometry
        .chunk_width
        .checked_mul(geometry.chunk_height)
        .ok_or_else(|| invalid("lossless JPEG tile sample count overflow"))?;
    output
        .par_chunks_mut(geometry.band_stride()?)
        .enumerate()
        .try_for_each(|(tile_y_index, band)| {
            let mut decoded = Vec::new();
            let mut row_scratch = FastRowScratch::default();
            let tile_y = tile_y_index * geometry.chunk_height;
            let copy_height = geometry.chunk_height.min(geometry.height - tile_y);
            for tile_x_index in 0..geometry.across {
                let index = tile_y_index * geometry.across + tile_x_index;
                let encoded = chunk(request.source, request.chunks[index])?;
                let precision = u8::try_from(request.bits_per_sample)
                    .expect("lossless JPEG precision is at most 16");
                let tile_x = tile_x_index * geometry.chunk_width;
                let copy_width = geometry.chunk_width.min(geometry.width - tile_x);
                let decoded_directly = decode_into_rows_with_scratch(
                    encoded,
                    precision,
                    geometry.chunk_width,
                    geometry.chunk_height,
                    &mut row_scratch,
                    |row, samples| {
                        if row < copy_height {
                            let target_start = row * geometry.width + tile_x;
                            let width = copy_width.min(samples.len());
                            band[target_start..target_start + width]
                                .copy_from_slice(&samples[..width]);
                        }
                        Ok(())
                    },
                )
                .map_err(|detail| invalid(format!("lossless JPEG tile {index}: {detail}")))?;
                if decoded_directly {
                    continue;
                }
                if decoded.len() != expected_samples {
                    decoded.resize(expected_samples, 0);
                }
                decode_into(
                    encoded,
                    geometry.chunk_width,
                    geometry.chunk_height,
                    precision,
                    &mut decoded,
                )
                .map_err(|detail| invalid(format!("lossless JPEG tile {index}: {detail}")))?;
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
        let mut samples = zeroed_samples(sample_count)?;
        for (sample, value) in samples.iter_mut().zip(output) {
            *sample = u8::try_from(value)
                .map_err(|_| invalid("lossless JPEG sample exceeds u8 storage"))?;
        }
        buffer_u8(geometry.width, geometry.height, samples)
    } else {
        buffer_u16(geometry.width, geometry.height, output)
    }
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
struct FastHuffmanCode(u16);

impl FastHuffmanCode {
    const INVALID: Self = Self(0);
    const LONG: Self = Self::new(31, 0);

    const fn new(length: u8, symbol: u8) -> Self { Self(u16::from_le_bytes([symbol, length])) }

    const fn length(self) -> u8 { self.0.to_le_bytes()[1] & 0x1f }

    const fn symbol(self) -> u8 { self.0.to_le_bytes()[0] }
}

struct HuffmanTable {
    fast_lookup: Box<[FastHuffmanCode; FAST_HUFFMAN_LOOKAHEAD_SIZE]>,
    counts: [u8; 16],
    symbols: Box<[u8]>,
}

impl HuffmanTable {
    fn parse(counts: &[u8; 16], symbols: &[u8]) -> Result<Self, String> {
        let mut fast_lookup = vec![FastHuffmanCode::INVALID; FAST_HUFFMAN_LOOKAHEAD_SIZE];

        let mut symbol_index = 0;
        let mut lookup_index = 0_usize;
        let mut code = 0_usize;
        for (length_index, &count) in counts.iter().enumerate() {
            let length = u8::try_from(length_index + 1)
                .expect("lossless JPEG Huffman lengths are at most 16");
            for _ in 0..count {
                let symbol = *symbols
                    .get(symbol_index)
                    .ok_or_else(|| "lossless JPEG Huffman symbol table is truncated".to_owned())?;
                let width = 1_usize << (HUFFMAN_LOOKAHEAD_BITS - usize::from(length));
                lookup_index = lookup_index
                    .checked_add(width)
                    .ok_or_else(|| "lossless JPEG Huffman table overflow".to_owned())?;
                if lookup_index > 1 << HUFFMAN_LOOKAHEAD_BITS {
                    return Err("lossless JPEG Huffman code lengths are invalid".to_owned());
                }

                let fast_length = usize::from(length).min(FAST_HUFFMAN_LOOKAHEAD_BITS);
                let fast_width = 1_usize << (FAST_HUFFMAN_LOOKAHEAD_BITS - fast_length);
                let fast_start = if usize::from(length) <= FAST_HUFFMAN_LOOKAHEAD_BITS {
                    code << (FAST_HUFFMAN_LOOKAHEAD_BITS - usize::from(length))
                } else {
                    code >> (usize::from(length) - FAST_HUFFMAN_LOOKAHEAD_BITS)
                };
                let fast_end = fast_start
                    .checked_add(fast_width)
                    .ok_or_else(|| "lossless JPEG fast Huffman table overflow".to_owned())?;
                let entries = fast_lookup.get_mut(fast_start..fast_end).ok_or_else(|| {
                    "lossless JPEG fast Huffman code lengths are invalid".to_owned()
                })?;
                entries.fill(
                    if symbol <= 15 && usize::from(length) <= FAST_HUFFMAN_LOOKAHEAD_BITS {
                        FastHuffmanCode::new(length, symbol)
                    } else {
                        FastHuffmanCode::LONG
                    },
                );
                symbol_index += 1;
                code += 1;
            }
            code <<= 1;
        }

        Ok(Self {
            fast_lookup: fast_lookup
                .into_boxed_slice()
                .try_into()
                .map_err(|_| "lossless JPEG fast Huffman table has an invalid size".to_owned())?,
            counts: *counts,
            symbols: symbols.into(),
        })
    }

    #[inline(always)]
    fn decode(&self, bits: &mut BitReader<'_>) -> Result<u8, String> {
        // The general path handles multi-component Sony and DNG scans. Keep the
        // hot symbol lookup inline so it does not pay a call for every sample.
        let entry = self.fast_lookup[usize::from(bits.peek(FAST_HUFFMAN_LOOKAHEAD_BITS_U8))];
        let length = entry.length();
        if length != 0 && usize::from(length) <= FAST_HUFFMAN_LOOKAHEAD_BITS {
            bits.skip(usize::from(length));
            return Ok(entry.symbol());
        }
        self.decode_slow(bits)
    }

    #[cold]
    #[inline(never)]
    fn decode_slow(&self, bits: &mut BitReader<'_>) -> Result<u8, String> {
        self.decode_symbol_slow(bits)
            .ok_or_else(|| "lossless JPEG contains an invalid Huffman code".to_owned())
    }

    fn decode_symbol_slow(&self, bits: &mut BitReader<'_>) -> Option<u8> {
        let mut code = 0_u32;
        let mut first_code = 0_u32;
        let mut symbol_index = 0_usize;
        for &count in &self.counts {
            code = (code << 1) | u32::from(bits.peek(1));
            bits.skip(1);
            let count = u32::from(count);
            if code < first_code + count {
                let offset = usize::try_from(code - first_code).ok()?;
                return self.symbols.get(symbol_index + offset).copied();
            }
            symbol_index += usize::try_from(count).ok()?;
            first_code = (first_code + count) << 1;
        }
        None
    }

    fn decode_difference_fast<B: FastBits>(&self, bits: &mut B) -> i32 {
        let index = usize::from(bits.peek_no_fill(FAST_HUFFMAN_LOOKAHEAD_BITS));
        let entry = self.fast_lookup[index];
        let length = entry.length();
        if length == 0 {
            bits.record_error("lossless JPEG contains an invalid Huffman code");
            return 0;
        }
        if usize::from(entry.length()) > FAST_HUFFMAN_LOOKAHEAD_BITS {
            return self.decode_difference_fast_slow(bits);
        }

        let symbol = entry.symbol();
        let cache = bits.cache_word();
        let raw = ((cache << length) >> 1) >> (63 - usize::from(symbol));
        bits.skip_no_fill(usize::from(length) + usize::from(symbol));
        let raw = i32::from_le_bytes(low_u32_from_u64(raw).to_le_bytes());
        let top = (raw >> (usize::from(symbol).max(1) - 1)) & 1;
        let negative = top - 1;
        raw.wrapping_add(negative & (1 - (1_i32 << symbol)))
    }

    #[cold]
    #[inline(never)]
    fn decode_difference_fast_slow<B: FastBits>(&self, bits: &mut B) -> i32 {
        let symbol = self.decode_symbol_fast_slow(bits);
        decode_difference_fast(symbol, bits)
    }

    #[cold]
    #[inline(never)]
    fn decode_symbol_fast_slow<B: FastBits>(&self, bits: &mut B) -> u8 {
        let mut code = 0_u32;
        let mut first_code = 0_u32;
        let mut symbol_index = 0_usize;
        for &count in &self.counts {
            code = (code << 1) | u32::from(bits.peek_no_fill(1));
            bits.skip_no_fill(1);
            let count = u32::from(count);
            if code < first_code + count {
                let offset =
                    usize::try_from(code - first_code).expect("Huffman symbol offset fits usize");
                return self.symbols[symbol_index + offset];
            }
            symbol_index += usize::try_from(count).expect("Huffman symbol count fits usize");
            first_code = (first_code + count) << 1;
        }
        bits.record_error("lossless JPEG contains an invalid Huffman code");
        0
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
    real_bits_loaded: u64,
    synthetic_bits_loaded: u64,
    ended: bool,
}

impl<'a> BitReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            position: 0,
            buffer: 0,
            buffered_bits: 0,
            real_bits_loaded: 0,
            synthetic_bits_loaded: 0,
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
                self.real_bits_loaded += 32;
            } else {
                let (byte, valid) = self.next_byte();
                self.buffer = (self.buffer << 8) | u64::from(byte);
                self.buffered_bits += 8;
                if valid {
                    self.real_bits_loaded += 8;
                } else {
                    self.synthetic_bits_loaded += 8;
                }
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

    fn next_byte(&mut self) -> (u8, bool) {
        if self.ended || self.position >= self.bytes.len() {
            self.ended = true;
            return (0, false);
        }

        let byte = self.bytes[self.position];
        if byte != MARKER_PREFIX {
            self.position += 1;
            return (byte, true);
        }

        if self.bytes.get(self.position + 1) == Some(&0) {
            self.position += 2;
            return (byte, true);
        }

        self.ended = true;
        (0, false)
    }

    fn take_error(&mut self) -> Option<&'static str> {
        let consumed =
            self.real_bits_loaded + self.synthetic_bits_loaded - u64::from(self.buffered_bits);
        (consumed > self.real_bits_loaded).then_some("lossless JPEG bitstream is exhausted")
    }
}

struct FastBitReader<'a> {
    bytes: &'a [u8],
    position: usize,
    cache: u64,
    buffered_bits: usize,
    real_bits_loaded: usize,
    synthetic_bits_loaded: usize,
    ended: bool,
    error: Option<&'static str>,
}

impl<'a> FastBitReader<'a> {
    const fn new(bytes: &'a [u8]) -> Self {
        Self {
            bytes,
            position: 0,
            cache: 0,
            buffered_bits: 0,
            real_bits_loaded: 0,
            synthetic_bits_loaded: 0,
            ended: false,
            error: None,
        }
    }

    fn refill(&mut self) {
        if self.buffered_bits >= 32 {
            return;
        }
        if let Some(bytes) = self.bytes.get(self.position..)
            && let Some(bytes) = bytes.first_chunk::<8>()
        {
            let word = u64::from_be_bytes(*bytes);
            let has_prefix =
                (!word).wrapping_sub(0x0101_0101_0101_0101) & word & 0x8080_8080_8080_8080 != 0;
            if !has_prefix {
                self.cache |= word >> self.buffered_bits;
                let taken = (63 - self.buffered_bits) >> 3;
                self.position += taken;
                self.buffered_bits += taken * 8;
                self.real_bits_loaded += taken * 8;
                return;
            }
        }
        self.refill_slow();
    }

    #[cold]
    #[inline(never)]
    fn refill_slow(&mut self) {
        while self.buffered_bits < 32 {
            let (byte, valid) = self.next_byte();
            let shift = 64 - self.buffered_bits - 8;
            self.cache |= u64::from(byte) << shift;
            self.buffered_bits += 8;
            if valid {
                self.real_bits_loaded += 8;
            } else {
                self.synthetic_bits_loaded += 8;
            }
        }
    }

    fn peek_no_fill(&self, count: usize) -> u16 {
        debug_assert!(count != 0 && count <= self.buffered_bits && count <= 16);
        low_u16_from_u64(self.cache >> (64 - count))
    }

    fn skip_no_fill(&mut self, count: usize) {
        debug_assert!(count <= self.buffered_bits);
        self.cache <<= count;
        self.buffered_bits -= count;
    }

    fn next_byte(&mut self) -> (u8, bool) {
        if self.ended || self.position >= self.bytes.len() {
            self.ended = true;
            return (0, false);
        }
        let byte = self.bytes[self.position];
        if byte != MARKER_PREFIX {
            self.position += 1;
            return (byte, true);
        }
        if self.bytes.get(self.position + 1) == Some(&0) {
            self.position += 2;
            return (byte, true);
        }
        self.ended = true;
        (0, false)
    }

    const fn record_error(&mut self, error: &'static str) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }

    #[cfg(test)]
    const fn has_error(&self) -> bool { self.error.is_some() }

    const fn take_error(&mut self) -> Option<&'static str> {
        let consumed = self.real_bits_loaded + self.synthetic_bits_loaded - self.buffered_bits;
        let error = if consumed > self.real_bits_loaded {
            Some("lossless JPEG bitstream is exhausted")
        } else {
            self.error
        };
        self.error = None;
        error
    }
}

trait FastBits {
    fn refill(&mut self);
    fn peek_no_fill(&self, count: usize) -> u16;
    fn skip_no_fill(&mut self, count: usize);
    fn cache_word(&self) -> u64;
    fn record_error(&mut self, error: &'static str);
    fn take_error(&mut self) -> Option<&'static str>;
}

impl FastBits for FastBitReader<'_> {
    #[inline(always)]
    fn refill(&mut self) { FastBitReader::refill(self); }

    #[inline(always)]
    fn peek_no_fill(&self, count: usize) -> u16 { FastBitReader::peek_no_fill(self, count) }

    #[inline(always)]
    fn skip_no_fill(&mut self, count: usize) { FastBitReader::skip_no_fill(self, count); }

    #[inline(always)]
    fn cache_word(&self) -> u64 { self.cache }

    #[inline(always)]
    fn record_error(&mut self, error: &'static str) { FastBitReader::record_error(self, error); }

    fn take_error(&mut self) -> Option<&'static str> { FastBitReader::take_error(self) }
}

/// Unstuffs `0xFF 0x00` sequences across independent chunks and stops before
/// the earliest real JPEG marker or dangling `0xFF`.
///
/// A byte `entropy[start]` is the stuffed `0x00` of chunk `c - 1`'s `0xFF` iff
/// `entropy[start - 1] == 0xFF && entropy[start] == 0x00`; if `entropy[start -
/// 2]` was also `0xFF`, chunk `c - 1` already stopped at `start - 2 < start`.
struct UnstuffedEntropy {
    storage: Vec<u8>,
    spans: Vec<(usize, usize)>,
}

/// Unstuffs `0xFF 0x00` sequences across independent chunks and stops before
/// the earliest real JPEG marker or dangling `0xFF`.
///
/// A byte `entropy[start]` is the stuffed `0x00` of chunk `c - 1`'s `0xFF` iff
/// `entropy[start - 1] == 0xFF && entropy[start] == 0x00`; if `entropy[start -
/// 2]` was also `0xFF`, chunk `c - 1` already stopped at `start - 2 < start`.
fn unstuff_entropy_parallel(entropy: &[u8]) -> Result<UnstuffedEntropy, String> {
    unstuff_entropy_with_chunk_size(entropy, UNSTUFF_CHUNK_BYTES)
}

fn unstuff_entropy_with_chunk_size(
    entropy: &[u8],
    chunk_size: usize,
) -> Result<UnstuffedEntropy, String> {
    if entropy.is_empty() {
        return Ok(UnstuffedEntropy {
            storage: Vec::new(),
            spans: Vec::new(),
        });
    }
    let chunk_size = chunk_size.max(1);
    let mut storage = zeroed_samples::<u8>(entropy.len())
        .map_err(|error| format!("lossless JPEG allocation: {error}"))?;
    let chunk_meta: Vec<(usize, usize, bool)> = storage
        .par_chunks_mut(chunk_size)
        .enumerate()
        .map(|(chunk_index, dst)| {
            let start = chunk_index * chunk_size;
            let chunk_end = start + dst.len();
            let skip_first =
                start > 0 && entropy[start - 1] == MARKER_PREFIX && entropy[start] == 0;
            let mut pos = start + usize::from(skip_first);
            let mut out_pos = 0_usize;
            let mut stopped = false;

            while pos + 8 <= chunk_end {
                let bytes: [u8; 8] = entropy[pos..pos + 8]
                    .try_into()
                    .expect("eight-byte unstuff window is in bounds");
                dst[out_pos..out_pos + 8].copy_from_slice(&bytes);
                let word = u64::from_le_bytes(bytes);
                let has_ff =
                    (!word).wrapping_sub(0x0101_0101_0101_0101) & word & 0x8080_8080_8080_8080;
                if has_ff == 0 {
                    out_pos += 8;
                    pos += 8;
                } else {
                    let ff_offset = (has_ff.trailing_zeros() as usize) >> 3;
                    let ff_pos = pos + ff_offset;
                    if entropy.get(ff_pos + 1) == Some(&0) {
                        out_pos += ff_offset + 1;
                        pos = ff_pos + 2;
                    } else {
                        out_pos += ff_offset;
                        stopped = true;
                        break;
                    }
                }
            }
            if !stopped {
                while pos < chunk_end {
                    let byte = entropy[pos];
                    if byte != MARKER_PREFIX {
                        dst[out_pos] = byte;
                        out_pos += 1;
                        pos += 1;
                    } else if entropy.get(pos + 1) == Some(&0) {
                        dst[out_pos] = MARKER_PREFIX;
                        out_pos += 1;
                        pos += 2;
                    } else {
                        stopped = true;
                        break;
                    }
                }
            }
            (start, out_pos, stopped)
        })
        .collect();

    let mut spans = Vec::with_capacity(chunk_meta.len());
    for (start, len, stopped) in chunk_meta {
        if len > 0 {
            spans.push((start, start + len));
        }
        if stopped {
            break;
        }
    }
    Ok(UnstuffedEntropy { storage, spans })
}

struct UnstuffedBitReader<'a> {
    storage: &'a [u8],
    spans: &'a [(usize, usize)],
    span_index: usize,
    current: &'a [u8],
    position: usize,
    cache: u64,
    buffered_bits: usize,
    real_bits_loaded: usize,
    synthetic_bits_loaded: usize,
    error: Option<&'static str>,
}

impl<'a> UnstuffedBitReader<'a> {
    fn new(unstuffed: &'a UnstuffedEntropy) -> Self {
        let current = unstuffed
            .spans
            .first()
            .map_or(&[][..], |&(start, end)| &unstuffed.storage[start..end]);
        Self {
            storage: &unstuffed.storage,
            spans: &unstuffed.spans,
            span_index: 0,
            current,
            position: 0,
            cache: 0,
            buffered_bits: 0,
            real_bits_loaded: 0,
            synthetic_bits_loaded: 0,
            error: None,
        }
    }

    #[inline(always)]
    fn refill(&mut self) {
        if self.buffered_bits >= 32 {
            return;
        }
        if let Some(bytes) = self
            .current
            .get(self.position..)
            .and_then(|s| s.first_chunk::<8>())
        {
            let word = u64::from_be_bytes(*bytes);
            self.cache |= word >> self.buffered_bits;
            let taken = (63 - self.buffered_bits) >> 3;
            self.position += taken;
            self.buffered_bits += taken * 8;
            self.real_bits_loaded += taken * 8;
            return;
        }
        self.refill_tail();
    }

    #[cold]
    #[inline(never)]
    fn refill_tail(&mut self) {
        while self.buffered_bits < 32 {
            let (byte, valid) = self.next_byte();
            let shift = 64 - self.buffered_bits - 8;
            self.cache |= u64::from(byte) << shift;
            self.buffered_bits += 8;
            if valid {
                self.real_bits_loaded += 8;
            } else {
                self.synthetic_bits_loaded += 8;
            }
        }
    }

    fn next_byte(&mut self) -> (u8, bool) {
        loop {
            if let Some(&byte) = self.current.get(self.position) {
                self.position += 1;
                return (byte, true);
            }
            self.span_index += 1;
            let Some(&(start, end)) = self.spans.get(self.span_index) else {
                self.current = &[];
                return (0, false);
            };
            self.current = &self.storage[start..end];
            self.position = 0;
        }
    }
}

impl FastBits for UnstuffedBitReader<'_> {
    #[inline(always)]
    fn refill(&mut self) { UnstuffedBitReader::refill(self); }

    #[inline(always)]
    fn peek_no_fill(&self, count: usize) -> u16 {
        debug_assert!(count != 0 && count <= self.buffered_bits && count <= 16);
        low_u16_from_u64(self.cache >> (64 - count))
    }

    #[inline(always)]
    fn skip_no_fill(&mut self, count: usize) {
        debug_assert!(count <= self.buffered_bits);
        self.cache <<= count;
        self.buffered_bits -= count;
    }

    #[inline(always)]
    fn cache_word(&self) -> u64 { self.cache }

    #[inline(always)]
    fn record_error(&mut self, error: &'static str) {
        if self.error.is_none() {
            self.error = Some(error);
        }
    }

    fn take_error(&mut self) -> Option<&'static str> {
        let consumed = self.real_bits_loaded + self.synthetic_bits_loaded - self.buffered_bits;
        let error = if consumed > self.real_bits_loaded {
            Some("lossless JPEG bitstream is exhausted")
        } else {
            self.error
        };
        self.error = None;
        error
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
    pub components: usize,
    pub samples: Vec<u16>,
}

struct ParsedJpeg<'a> {
    frame: Frame,
    tables: [Option<HuffmanTable>; 4],
    predictor: u8,
    point_transform: u8,
    entropy: &'a [u8],
}

pub fn decode_interleaved(
    source: &[u8],
    expected_precision: u8,
) -> Result<InterleavedJpeg, String> {
    decode_interleaved_impl(source, expected_precision, true)
}

fn decode_interleaved_impl(
    source: &[u8],
    expected_precision: u8,
    fast: bool,
) -> Result<InterleavedJpeg, String> {
    let parsed = parse_jpeg(source, expected_precision)?;
    let frame = parsed.frame;
    let output_width = frame
        .width
        .checked_mul(frame.components.len())
        .ok_or_else(|| "lossless JPEG output width overflow".to_owned())?;
    let sample_count = output_width
        .checked_mul(frame.height)
        .ok_or_else(|| "lossless JPEG sample count overflow".to_owned())?;
    let mut output = zeroed_samples(sample_count)
        .map_err(|error| format!("lossless JPEG allocation: {error}"))?;
    let fast_path = decode_scan_into(
        parsed.entropy,
        &frame,
        &parsed.tables,
        parsed.predictor,
        parsed.point_transform,
        output_width,
        &mut output,
        fast,
    )?;
    if parsed.point_transform != 0 && !fast_path {
        for value in &mut output {
            *value = value.wrapping_shl(u32::from(parsed.point_transform));
        }
    }
    Ok(InterleavedJpeg {
        width: output_width,
        height: frame.height,
        components: frame.components.len(),
        samples: output,
    })
}

/// Streams four-component predictor-one scan rows without retaining a decoded
/// image.
///
/// The callback receives each interleaved JPEG row. Returns `Ok(false)` when
/// the scan does not meet the four-component fast-path contract.
///
/// # Errors
///
/// Returns an error when the JPEG stream, scan geometry, or Huffman data is
/// invalid.
#[doc(hidden)]
pub fn decode_interleaved_rows<F>(
    source: &[u8],
    expected_precision: u8,
    expected_width: usize,
    expected_height: usize,
    mut sink: F,
) -> Result<bool, String>
where
    F: FnMut(usize, &[u16]) -> Result<(), String>,
{
    let parsed = parse_jpeg(source, expected_precision)?;
    let components = parsed.frame.components.len();
    if parsed.predictor != 1 || components != 4 || parsed.point_transform != 0 {
        return Ok(false);
    }
    let decoded_width = parsed
        .frame
        .width
        .checked_mul(components)
        .ok_or_else(|| "lossless JPEG output width overflow".to_owned())?;
    let decoded_samples = decoded_width
        .checked_mul(parsed.frame.height)
        .ok_or_else(|| "lossless JPEG decoded sample count overflow".to_owned())?;
    let expected_samples = expected_width
        .checked_mul(expected_height)
        .ok_or_else(|| "lossless JPEG expected sample count overflow".to_owned())?;
    let tile_width = parsed
        .frame
        .width
        .checked_mul(2)
        .ok_or_else(|| "lossless JPEG Bayer width overflow".to_owned())?;
    let tile_height = parsed
        .frame
        .height
        .checked_mul(2)
        .ok_or_else(|| "lossless JPEG Bayer height overflow".to_owned())?;
    if decoded_samples != expected_samples
        || (tile_width, tile_height) != (expected_width, expected_height)
    {
        return Err(format!(
            "lossless JPEG frame is {}x{} with four components, expected \
             {expected_width}x{expected_height}",
            parsed.frame.width, parsed.frame.height
        ));
    }
    let tables = parsed
        .frame
        .components
        .iter()
        .map(|component| {
            parsed.tables[component.huffman_table]
                .as_ref()
                .ok_or_else(|| "lossless JPEG references a missing Huffman table".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    let mut scratch = FastRowScratch::default();
    decode_scan_four_rows_with_scratch(
        parsed.entropy,
        &parsed.frame,
        &tables,
        parsed.predictor,
        parsed.point_transform,
        decoded_width,
        &mut scratch,
        &mut sink,
    )?;
    Ok(true)
}

fn decode_scan_four_rows_with_scratch(
    entropy: &[u8],
    frame: &Frame,
    tables: &[&HuffmanTable],
    predictor: u8,
    point_transform: u8,
    output_width: usize,
    scratch: &mut FastRowScratch,
    sink: impl FnMut(usize, &[u16]) -> Result<(), String>,
) -> Result<(), String> {
    if entropy.len() >= UNSTUFF_THRESHOLD_BYTES {
        let unstuffed = unstuff_entropy_parallel(entropy)?;
        let bits = UnstuffedBitReader::new(&unstuffed);
        decode_scan_four_rows_inner(
            bits,
            frame,
            tables,
            predictor,
            point_transform,
            output_width,
            scratch,
            sink,
        )
    } else {
        let bits = FastBitReader::new(entropy);
        decode_scan_four_rows_inner(
            bits,
            frame,
            tables,
            predictor,
            point_transform,
            output_width,
            scratch,
            sink,
        )
    }
}

fn decode_scan_four_rows_inner<B: FastBits>(
    mut bits: B,
    frame: &Frame,
    tables: &[&HuffmanTable],
    predictor: u8,
    point_transform: u8,
    output_width: usize,
    scratch: &mut FastRowScratch,
    mut sink: impl FnMut(usize, &[u16]) -> Result<(), String>,
) -> Result<(), String> {
    if predictor != 1 || frame.components.len() != 4 || tables.len() != 4 {
        return Err("fast lossless JPEG four-component scan is invalid".to_owned());
    }
    let expected_width = frame
        .width
        .checked_mul(4)
        .ok_or_else(|| "lossless JPEG output width overflow".to_owned())?;
    if output_width != expected_width {
        return Err("lossless JPEG row width does not match the frame".to_owned());
    }
    let initial = 1_i32 << (frame.precision - point_transform - 1);
    let mask = (1_u32 << frame.precision) - 1;
    scratch.resize(output_width, point_transform != 0);

    if point_transform == 0 {
        for row in 0..frame.height {
            let mut predictions = [initial; 4];
            if row != 0 {
                for (prediction, &previous) in predictions.iter_mut().zip(&scratch.previous) {
                    *prediction = i32::from(previous);
                }
            }
            for chunk in scratch.current.as_chunks_mut::<4>().0 {
                for component in 0..4 {
                    bits.refill();
                    let value = apply_difference(
                        predictions[component],
                        tables[component].decode_difference_fast(&mut bits),
                        mask,
                    );
                    chunk[component] = value;
                    predictions[component] = i32::from(value);
                }
            }
            sink(row, &scratch.current)?;
            std::mem::swap(&mut scratch.previous, &mut scratch.current);
        }
        if let Some(error) = bits.take_error() {
            return Err(error.to_owned());
        }
        return Ok(());
    }

    for row in 0..frame.height {
        let mut predictions = [initial; 4];
        if row != 0 {
            for (prediction, &previous) in predictions.iter_mut().zip(&scratch.previous) {
                *prediction = i32::from(previous);
            }
        }
        for (column, chunk) in scratch
            .current
            .as_chunks_mut::<4>()
            .0
            .iter_mut()
            .enumerate()
        {
            for component in 0..4 {
                bits.refill();
                let value = apply_difference(
                    predictions[component],
                    tables[component].decode_difference_fast(&mut bits),
                    mask,
                );
                chunk[component] = value;
                predictions[component] = i32::from(value);
                if point_transform != 0 {
                    scratch.shifted[column * 4 + component] =
                        value.wrapping_shl(u32::from(point_transform));
                }
            }
        }
        sink(row, &scratch.shifted)?;
        std::mem::swap(&mut scratch.previous, &mut scratch.current);
    }
    if let Some(error) = bits.take_error() {
        return Err(error.to_owned());
    }
    Ok(())
}

fn parse_jpeg(source: &[u8], expected_precision: u8) -> Result<ParsedJpeg<'_>, String> {
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
            MARKER_DRI => {
                return Err("lossless JPEG restart intervals are unsupported".to_owned());
            }
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
    Ok(ParsedJpeg {
        frame,
        tables,
        predictor,
        point_transform,
        entropy,
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
    reshape_scan_into(
        decoded.width,
        decoded.height,
        expected_width,
        expected_height,
        &decoded.samples,
        output,
    )
}

#[cfg(test)]
fn decode_into_rows(
    source: &[u8],
    expected_precision: u8,
    expected_width: usize,
    expected_height: usize,
    mut sink: impl FnMut(usize, &[u16]) -> Result<(), String>,
) -> Result<bool, String> {
    let mut scratch = FastRowScratch::default();
    decode_into_rows_with_scratch(
        source,
        expected_precision,
        expected_width,
        expected_height,
        &mut scratch,
        &mut sink,
    )
}

fn decode_into_rows_with_scratch(
    source: &[u8],
    expected_precision: u8,
    expected_width: usize,
    expected_height: usize,
    scratch: &mut FastRowScratch,
    mut sink: impl FnMut(usize, &[u16]) -> Result<(), String>,
) -> Result<bool, String> {
    let parsed = parse_jpeg(source, expected_precision)?;
    if parsed.predictor != 1 || parsed.frame.components.len() > 2 {
        return Ok(false);
    }
    let decoded_width = parsed
        .frame
        .width
        .checked_mul(parsed.frame.components.len())
        .ok_or_else(|| "lossless JPEG output width overflow".to_owned())?;
    let decoded_samples = decoded_width
        .checked_mul(parsed.frame.height)
        .ok_or_else(|| "lossless JPEG decoded sample count overflow".to_owned())?;
    let expected_samples = expected_width
        .checked_mul(expected_height)
        .ok_or_else(|| "lossless JPEG expected sample count overflow".to_owned())?;
    if decoded_samples != expected_samples {
        return Err(format!(
            "lossless JPEG frame is {}x{} with sample count {decoded_samples}, expected \
             {expected_width}x{expected_height} with sample count {expected_samples}",
            decoded_width, parsed.frame.height
        ));
    }
    if expected_width == 0 || !decoded_width.is_multiple_of(expected_width) {
        return Err(format!(
            "lossless JPEG frame width {decoded_width} is not an integer multiple of the \
             expected tile width {expected_width}"
        ));
    }
    let rows_per_decoded_row = decoded_width / expected_width;
    let decoded_tables = parsed
        .frame
        .components
        .iter()
        .map(|component| {
            parsed.tables[component.huffman_table]
                .as_ref()
                .ok_or_else(|| "lossless JPEG references a missing Huffman table".to_owned())
        })
        .collect::<Result<Vec<_>, _>>()?;
    decode_scan_rows_with_scratch(
        parsed.entropy,
        &parsed.frame,
        &decoded_tables,
        parsed.predictor,
        parsed.point_transform,
        decoded_width,
        scratch,
        |row, samples| {
            for (offset, expected_row) in (0..rows_per_decoded_row)
                .map(|offset| (offset, row * rows_per_decoded_row + offset))
            {
                sink(
                    expected_row,
                    &samples[offset * expected_width..][..expected_width],
                )?;
            }
            Ok(())
        },
    )?;
    Ok(true)
}

fn reshape_scan_into(
    decoded_width: usize,
    decoded_height: usize,
    expected_width: usize,
    expected_height: usize,
    decoded: &[u16],
    output: &mut [u16],
) -> Result<(), String> {
    let expected_samples = expected_width
        .checked_mul(expected_height)
        .ok_or_else(|| "lossless JPEG expected sample count overflow".to_owned())?;
    if output.len() != expected_samples {
        return Err(format!(
            "lossless JPEG output sample count is {}, expected {expected_samples}",
            output.len()
        ));
    }
    let decoded_samples = decoded_width
        .checked_mul(decoded_height)
        .ok_or_else(|| "lossless JPEG decoded sample count overflow".to_owned())?;
    if decoded.len() != decoded_samples {
        return Err(format!(
            "lossless JPEG decoded sample count is {}, expected {decoded_samples}",
            decoded.len()
        ));
    }
    if decoded_samples != expected_samples {
        return Err(format!(
            "lossless JPEG frame is {decoded_width}x{decoded_height} with sample count \
             {decoded_samples}, expected {expected_width}x{expected_height} with sample count \
             {expected_samples}"
        ));
    }
    if (decoded_width, decoded_height) != (expected_width, expected_height)
        && (expected_width == 0 || !decoded_width.is_multiple_of(expected_width))
    {
        return Err(format!(
            "lossless JPEG frame width {decoded_width} is not an integer multiple of the \
             expected tile width {expected_width}"
        ));
    }
    output.copy_from_slice(decoded);
    Ok(())
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
        tables[table_index] = Some(HuffmanTable::parse(&counts, symbols)?);
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
    fast: bool,
) -> Result<bool, String> {
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
    if fast && predictor == 1 && frame.components.len() <= 2 {
        decode_scan_prediction_one(
            entropy,
            frame,
            &components,
            point_transform,
            output_width,
            output,
        )?;
        return Ok(true);
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

    if let Some(error) = bits.take_error() {
        return Err(error.to_owned());
    }
    Ok(false)
}

fn decode_scan_prediction_one(
    entropy: &[u8],
    frame: &Frame,
    tables: &[&HuffmanTable],
    point_transform: u8,
    output_width: usize,
    output: &mut [u16],
) -> Result<(), String> {
    if point_transform == 0 && matches!(frame.components.len(), 1 | 2) {
        if entropy.len() >= UNSTUFF_THRESHOLD_BYTES {
            let unstuffed = unstuff_entropy_parallel(entropy)?;
            return decode_scan_direct_inner(
                UnstuffedBitReader::new(&unstuffed),
                frame,
                tables,
                output_width,
                output,
            );
        }
        return decode_scan_direct_inner(
            FastBitReader::new(entropy),
            frame,
            tables,
            output_width,
            output,
        );
    }
    decode_scan_rows(
        entropy,
        frame,
        tables,
        1,
        point_transform,
        output_width,
        |row, samples| {
            let target = &mut output[row * output_width..][..output_width];
            target.copy_from_slice(samples);
            Ok(())
        },
    )
}

#[inline(always)]
fn decode_one_comp_row<B: FastBits>(
    bits: &mut B,
    t0: &HuffmanTable,
    mask: u32,
    col0: &mut i32,
    row: &mut [u16],
) {
    let mut p0 = *col0;
    for slot in row.iter_mut() {
        bits.refill();
        let value = apply_difference(p0, t0.decode_difference_fast(bits), mask);
        p0 = i32::from(value);
        *slot = value;
    }
    if let Some(&first) = row.first() {
        *col0 = i32::from(first);
    }
}

#[inline(always)]
fn decode_two_comp_row<B: FastBits>(
    bits: &mut B,
    t0: &HuffmanTable,
    t1: &HuffmanTable,
    mask: u32,
    col0: &mut [i32; 2],
    row: &mut [u16],
) {
    let [mut p0, mut p1] = *col0;
    for pair in row.as_chunks_mut::<2>().0 {
        bits.refill();
        let v0 = apply_difference(p0, t0.decode_difference_fast(bits), mask);
        p0 = i32::from(v0);
        bits.refill();
        let v1 = apply_difference(p1, t1.decode_difference_fast(bits), mask);
        p1 = i32::from(v1);
        pair[0] = v0;
        pair[1] = v1;
    }
    if let [c0, c1, ..] = *row {
        *col0 = [i32::from(c0), i32::from(c1)];
    }
}

fn decode_scan_direct_inner<B: FastBits>(
    mut bits: B,
    frame: &Frame,
    tables: &[&HuffmanTable],
    output_width: usize,
    output: &mut [u16],
) -> Result<(), String> {
    let components = frame.components.len();
    let expected_width = frame
        .width
        .checked_mul(components)
        .ok_or_else(|| "lossless JPEG output width overflow".to_owned())?;
    if output_width != expected_width {
        return Err("lossless JPEG row width does not match the frame".to_owned());
    }
    let initial = 1_i32 << (frame.precision - 1);
    let mask = (1_u32 << frame.precision) - 1;
    match components {
        1 => {
            let t0 = tables[0];
            let mut col0 = initial;
            for row in output.chunks_exact_mut(output_width).take(frame.height) {
                decode_one_comp_row(&mut bits, t0, mask, &mut col0, row);
            }
        }
        2 => {
            let (t0, t1) = (tables[0], tables[1]);
            let mut col0 = [initial; 2];
            for row in output.chunks_exact_mut(output_width).take(frame.height) {
                decode_two_comp_row(&mut bits, t0, t1, mask, &mut col0, row);
            }
        }
        _ => return Err("fast lossless JPEG row decode does not support this scan".to_owned()),
    }
    if let Some(error) = bits.take_error() {
        return Err(error.to_owned());
    }
    Ok(())
}

fn decode_scan_rows(
    entropy: &[u8],
    frame: &Frame,
    tables: &[&HuffmanTable],
    predictor: u8,
    point_transform: u8,
    output_width: usize,
    sink: impl FnMut(usize, &[u16]) -> Result<(), String>,
) -> Result<(), String> {
    let mut scratch = FastRowScratch::default();
    decode_scan_rows_with_scratch(
        entropy,
        frame,
        tables,
        predictor,
        point_transform,
        output_width,
        &mut scratch,
        sink,
    )
}

#[derive(Default)]
struct FastRowScratch {
    previous: Vec<u16>,
    current: Vec<u16>,
    shifted: Vec<u16>,
}

impl FastRowScratch {
    fn resize(&mut self, width: usize, shift: bool) {
        self.previous.resize(width, 0);
        self.current.resize(width, 0);
        self.previous.fill(0);
        self.current.fill(0);
        if shift {
            self.shifted.resize(width, 0);
        }
    }
}

fn decode_scan_rows_with_scratch(
    entropy: &[u8],
    frame: &Frame,
    tables: &[&HuffmanTable],
    predictor: u8,
    point_transform: u8,
    output_width: usize,
    scratch: &mut FastRowScratch,
    sink: impl FnMut(usize, &[u16]) -> Result<(), String>,
) -> Result<(), String> {
    if entropy.len() >= UNSTUFF_THRESHOLD_BYTES {
        let unstuffed = unstuff_entropy_parallel(entropy)?;
        let bits = UnstuffedBitReader::new(&unstuffed);
        decode_scan_rows_inner(
            bits,
            frame,
            tables,
            predictor,
            point_transform,
            output_width,
            scratch,
            sink,
        )
    } else {
        let bits = FastBitReader::new(entropy);
        decode_scan_rows_inner(
            bits,
            frame,
            tables,
            predictor,
            point_transform,
            output_width,
            scratch,
            sink,
        )
    }
}

fn decode_scan_rows_inner<B: FastBits>(
    mut bits: B,
    frame: &Frame,
    tables: &[&HuffmanTable],
    predictor: u8,
    point_transform: u8,
    output_width: usize,
    scratch: &mut FastRowScratch,
    mut sink: impl FnMut(usize, &[u16]) -> Result<(), String>,
) -> Result<(), String> {
    if predictor != 1 || !matches!(frame.components.len(), 1 | 2) {
        return Err("fast lossless JPEG row decode does not support this scan".to_owned());
    }
    let components = frame.components.len();
    let expected_width = frame
        .width
        .checked_mul(components)
        .ok_or_else(|| "lossless JPEG output width overflow".to_owned())?;
    if output_width != expected_width {
        return Err("lossless JPEG row width does not match the frame".to_owned());
    }
    let initial = 1_i32 << (frame.precision - point_transform - 1);
    let mask = (1_u32 << frame.precision) - 1;
    scratch.resize(output_width, point_transform != 0);

    if point_transform == 0 && components == 2 {
        let (t0, t1) = (tables[0], tables[1]);
        let mut col0 = [initial; 2];
        for row in 0..frame.height {
            decode_two_comp_row(&mut bits, t0, t1, mask, &mut col0, &mut scratch.current);
            sink(row, &scratch.current)?;
        }
    } else if point_transform == 0 && components == 1 {
        let t0 = tables[0];
        let mut col0 = initial;
        for row in 0..frame.height {
            decode_one_comp_row(&mut bits, t0, mask, &mut col0, &mut scratch.current);
            sink(row, &scratch.current)?;
        }
    } else {
        for row in 0..frame.height {
            let mut predictions = [initial; 2];
            if row != 0 {
                for (prediction, &previous) in predictions.iter_mut().zip(&scratch.previous) {
                    *prediction = i32::from(previous);
                }
            }
            for (column, chunk) in scratch.current.chunks_exact_mut(components).enumerate() {
                for (component, output) in chunk.iter_mut().enumerate() {
                    let prediction = predictions[component];
                    bits.refill();
                    let value = apply_difference(
                        prediction,
                        tables[component].decode_difference_fast(&mut bits),
                        mask,
                    );
                    *output = value;
                    predictions[component] = i32::from(value);
                    scratch.shifted[column * components + component] =
                        value.wrapping_shl(u32::from(point_transform));
                }
            }
            sink(row, &scratch.shifted)?;
            std::mem::swap(&mut scratch.previous, &mut scratch.current);
        }
    }
    if let Some(error) = bits.take_error() {
        return Err(error.to_owned());
    }
    Ok(())
}

#[cfg(test)]
fn decode_scan_rows_generic_oracle<B: FastBits>(
    mut bits: B,
    frame: &Frame,
    tables: &[&HuffmanTable],
    predictor: u8,
    point_transform: u8,
    output_width: usize,
    scratch: &mut FastRowScratch,
    mut sink: impl FnMut(usize, &[u16]) -> Result<(), String>,
) -> Result<(), String> {
    if predictor != 1 || !matches!(frame.components.len(), 1 | 2) {
        return Err("fast lossless JPEG row decode does not support this scan".to_owned());
    }
    let components = frame.components.len();
    let expected_width = frame
        .width
        .checked_mul(components)
        .ok_or_else(|| "lossless JPEG output width overflow".to_owned())?;
    if output_width != expected_width {
        return Err("lossless JPEG row width does not match the frame".to_owned());
    }
    let initial = 1_i32 << (frame.precision - point_transform - 1);
    let mask = (1_u32 << frame.precision) - 1;
    scratch.resize(output_width, point_transform != 0);

    for row in 0..frame.height {
        let mut predictions = [initial; 2];
        if row != 0 {
            for (prediction, &previous) in predictions.iter_mut().zip(&scratch.previous) {
                *prediction = i32::from(previous);
            }
        }
        for (column, chunk) in scratch.current.chunks_exact_mut(components).enumerate() {
            for (component, output) in chunk.iter_mut().enumerate() {
                let prediction = predictions[component];
                bits.refill();
                let value = apply_difference(
                    prediction,
                    tables[component].decode_difference_fast(&mut bits),
                    mask,
                );
                *output = value;
                predictions[component] = i32::from(value);
                if point_transform != 0 {
                    scratch.shifted[column * components + component] =
                        value.wrapping_shl(u32::from(point_transform));
                }
            }
        }
        if point_transform == 0 {
            sink(row, &scratch.current)?;
        } else {
            sink(row, &scratch.shifted)?;
        }
        std::mem::swap(&mut scratch.previous, &mut scratch.current);
    }
    if let Some(error) = bits.take_error() {
        return Err(error.to_owned());
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
    Ok(apply_difference(
        prediction,
        decode_difference(symbol, bits)?,
        (1_u32 << precision) - 1,
    ))
}

fn decode_difference(symbol: u8, bits: &mut BitReader<'_>) -> Result<i32, String> {
    match symbol {
        0 => Ok(0),
        16 => Ok(-32_768),
        1..=15 => {
            let bit_count = symbol;
            let raw = i32::from(bits.peek(bit_count));
            bits.skip(usize::from(bit_count));
            let threshold = 1_i32 << (bit_count - 1);
            Ok(if raw < threshold {
                raw - ((1_i32 << bit_count) - 1)
            } else {
                raw
            })
        }
        _ => Err(format!(
            "lossless JPEG difference class {symbol} is unsupported"
        )),
    }
}

fn decode_difference_fast<B: FastBits>(symbol: u8, bits: &mut B) -> i32 {
    match symbol {
        0 => 0,
        16 => -32_768,
        1..=15 => {
            let raw = i32::from(bits.peek_no_fill(usize::from(symbol)));
            bits.skip_no_fill(usize::from(symbol));
            let threshold = 1_i32 << (symbol - 1);
            if raw < threshold {
                raw - ((1_i32 << symbol) - 1)
            } else {
                raw
            }
        }
        _ => {
            bits.record_error("lossless JPEG difference class is unsupported");
            0
        }
    }
}

const fn apply_difference(prediction: i32, difference: i32, mask: u32) -> u16 {
    let difference = u32::from_le_bytes(prediction.wrapping_add(difference).to_le_bytes());
    low_u16(difference & mask)
}

const fn low_u16(value: u32) -> u16 {
    let bytes = value.to_le_bytes();
    u16::from_le_bytes([bytes[0], bytes[1]])
}

const fn low_u16_from_u64(value: u64) -> u16 {
    let bytes = value.to_le_bytes();
    u16::from_le_bytes([bytes[0], bytes[1]])
}

const fn low_u32_from_u64(value: u64) -> u32 {
    let bytes = value.to_le_bytes();
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

const HUFFMAN_BITS: u8 = 5;

fn encode_tile(samples: &[u16], width: u16, height: u16, bits: u8) -> Result<Vec<u8>, String> {
    encode_interleaved(samples, width, height, 1, bits)
}

/// Encodes a synthetic lossless-JPEG tile for downstream codec tests.
#[doc(hidden)]
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
    use std::hint::black_box;
    use std::time::{Duration, Instant};

    use super::{
        BitReader, BitWriter, FastBitReader, FastRowScratch, Geometry, MARKER_EOI, MARKER_PREFIX,
        add_prediction, apply_difference, copy_tile_into, decode_difference,
        decode_interleaved_impl, decode_interleaved_rows, decode_into_rows,
        decode_into_rows_with_scratch, low_u16, predict,
    };
    use crate::{ByteOrder, Chunk, DecodeRequest, Predictor, SampleBuffer, SampleFormat};

    #[test]
    fn fourteen_bit_difference_class_consumes_amplitude_bits() {
        let mut bits = BitReader::new(&[0x8c, 0xa0]);
        assert_eq!(add_prediction(100, 14, &mut bits, 14).unwrap(), 9_100);
    }

    #[test]
    fn truncation_helpers_preserve_wrapped_and_masked_sample_bits() {
        assert_eq!(low_u16(0x1234_abcd), 0xabcd);
        assert_eq!(apply_difference(0x7fff, 0x7fff, 0x3fff), 0x3ffe);
        assert_eq!(apply_difference(-1, 0, 0x3fff), 0x3fff);
        assert_eq!(apply_difference(i32::MAX, 1, 0xffff), 0);
        assert_eq!(apply_difference(0, i32::MIN, 0xffff), 0);
    }

    #[test]
    fn fast_peek_truncates_to_the_requested_high_bits() {
        let mut bits = FastBitReader::new(&[0b1010_1100, 0b0011_0101, 0xff, 0]);
        bits.refill();
        assert_eq!(bits.peek_no_fill(1), 1);
        bits.skip_no_fill(1);
        bits.refill();
        assert_eq!(bits.peek_no_fill(7), 0b010_1100);
        bits.skip_no_fill(7);
        bits.refill();
        assert_eq!(bits.peek_no_fill(8), 0b0011_0101);
        bits.skip_no_fill(8);
        bits.refill();
        assert_eq!(bits.peek_no_fill(16), 0xff00);
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
    fn fused_prediction_one_matches_the_general_scanner() {
        for components in [1_u8, 2] {
            let width = 17_u16;
            let height = 9_u16;
            let samples = (0..usize::from(width) * usize::from(height) * usize::from(components))
                .map(|index| ((index * 977 + 12_345) & 0x3fff) as u16)
                .collect::<Vec<_>>();
            let encoded =
                super::encode_interleaved_for_test(&samples, width, height, components, 14);
            let fast = decode_interleaved_impl(&encoded, 14, true).unwrap();
            let general = decode_interleaved_impl(&encoded, 14, false).unwrap();

            assert_eq!(
                (fast.width, fast.height, fast.components),
                (
                    usize::from(width) * usize::from(components),
                    usize::from(height),
                    usize::from(components)
                )
            );
            assert_eq!(fast.samples, general.samples);
        }
    }

    #[test]
    fn component_local_rows_cover_degenerate_dimensions() {
        for components in [1_u8, 2] {
            for (width, height) in [(1_u16, 1_u16), (1, 5), (5, 1), (2, 2)] {
                let samples =
                    (0..usize::from(width) * usize::from(height) * usize::from(components))
                        .map(|index| ((index * 613 + 11) & 0x3fff) as u16)
                        .collect::<Vec<_>>();
                let encoded =
                    super::encode_interleaved_for_test(&samples, width, height, components, 14);
                let fast = decode_interleaved_impl(&encoded, 14, true).unwrap();
                let general = decode_interleaved_impl(&encoded, 14, false).unwrap();
                assert_eq!(fast.samples, general.samples);
            }
        }
    }

    #[test]
    fn four_component_row_decoder_matches_the_interleaved_scan() {
        let samples = (0_u16..48)
            .map(|index| 8_000 + (index * 73) % 1_000)
            .collect::<Vec<_>>();
        let encoded = super::encode_interleaved_for_test(&samples, 4, 3, 4, 14);
        let mut rows = Vec::new();
        let handled = decode_interleaved_rows(&encoded, 14, 8, 6, |row, samples| {
            rows.push((row, samples.to_vec()));
            Ok(())
        })
        .unwrap();

        assert!(handled);
        assert_eq!(rows.len(), 3);
        for (row, decoded) in rows {
            assert_eq!(decoded, samples[row * 16..(row + 1) * 16]);
        }
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

    #[test]
    fn left_aligned_fast_reader_matches_reference_across_stuffing() {
        let source = [0x12, 0x34, 0xff, 0x00, 0x7f, 0x80, 0x55];
        let mut reference = BitReader::new(&source);
        let mut fast = FastBitReader::new(&source);

        for count in [1_u8, 5, 9, 13, 16, 3, 11, 2] {
            fast.refill();
            assert_eq!(fast.peek_no_fill(usize::from(count)), reference.peek(count));
            fast.skip_no_fill(usize::from(count));
            reference.skip(usize::from(count));
        }
    }

    #[test]
    fn fast_difference_decode_matches_reference_for_every_class() {
        let mut counts = [0_u8; 16];
        counts[0] = 1;
        for symbol in 0_u8..=16 {
            let table = super::HuffmanTable::parse(&counts, &[symbol]).unwrap();
            let raw_values: &[u32] = match symbol {
                0 | 16 => &[0],
                _ => &[0, (1_u32 << symbol) - 1],
            };
            for &raw in raw_values {
                let mut encoded = Vec::new();
                let mut writer = BitWriter::new(&mut encoded);
                writer.write(0, 1);
                if !matches!(symbol, 0 | 16) {
                    writer.write(raw, symbol);
                }
                writer.finish();

                let mut reference = BitReader::new(&encoded);
                reference.peek(1);
                reference.skip(1);
                if !matches!(symbol, 0 | 16) {
                    assert_eq!(
                        reference.peek(symbol),
                        u16::try_from(raw).unwrap(),
                        "encoded bits for symbol {symbol}, raw {raw}"
                    );
                }
                let expected = decode_difference(symbol, &mut reference).unwrap();
                let mut fast = FastBitReader::new(&encoded);
                fast.refill();
                let actual = table.decode_difference_fast(&mut fast);
                assert_eq!(actual, expected, "symbol {symbol}, raw {raw}");
                assert!(!fast.has_error());
            }
        }
    }

    #[test]
    fn ten_bit_fast_table_covers_every_encoder_symbol() {
        assert_eq!(super::FastHuffmanCode::new(5, 0).length(), 5);
        let mut one_count = [0_u8; 16];
        one_count[4] = 1;
        let one = super::HuffmanTable::parse(&one_count, &[0]).unwrap();
        assert_eq!(one.fast_lookup[0].length(), 5);

        let mut counts = [0_u8; 16];
        counts[4] = 17;
        let symbols = (0_u8..=16).collect::<Vec<_>>();
        let table = super::HuffmanTable::parse(&counts, &symbols).unwrap();

        for symbol in symbols {
            let entry = table.fast_lookup[usize::from(symbol) << 5];
            if symbol == 16 {
                assert_eq!(entry.length(), 31);
            } else {
                assert_eq!(entry.length(), 5);
                assert_eq!(entry.symbol(), symbol);
            }
        }

        let source = [0x80, 0, 0, 0];
        let mut bits = FastBitReader::new(&source);
        bits.refill();
        assert_eq!(table.decode_difference_fast(&mut bits), -32_768);
        assert!(!bits.has_error());
    }

    #[test]
    fn fast_difference_reader_matches_reference_across_encoded_scan() {
        let samples = [8_100_u16, 8_110, 8_090, 8_120];
        let encoded = super::encode_interleaved_for_test(&samples, 2, 2, 1, 14);
        let parsed = super::parse_jpeg(&encoded, 14).unwrap();
        let table = parsed.tables[0].as_ref().unwrap();
        let mut reference = BitReader::new(parsed.entropy);
        let mut fast = FastBitReader::new(parsed.entropy);

        for sample in 0..samples.len() {
            let symbol = table.decode(&mut reference).unwrap();
            let expected = decode_difference(symbol, &mut reference).unwrap();
            fast.refill();
            let entry = table.fast_lookup
                [usize::from(fast.peek_no_fill(super::FAST_HUFFMAN_LOOKAHEAD_BITS))];
            assert_eq!(entry.symbol(), symbol, "sample {sample}");
            let actual = table.decode_difference_fast(&mut fast);
            assert_eq!(actual, expected, "sample {sample}, symbol {symbol}");
            assert!(!fast.has_error(), "sample {sample}, symbol {symbol}");
        }
    }

    #[test]
    fn refill_matches_reference_across_stuffing_markers_and_truncation() {
        for period in [1_usize, 2, 5, 8, 13] {
            let mut source = Vec::new();
            for value in 0_u8..96 {
                source.push(value.wrapping_mul(37) & 0x7f);
                if value != 0 && usize::from(value).is_multiple_of(period) {
                    source.extend_from_slice(&[MARKER_PREFIX, 0]);
                }
            }
            source.extend_from_slice(&[MARKER_PREFIX, MARKER_EOI]);
            let mut reference = BitReader::new(&source);
            let mut fast = FastBitReader::new(&source);
            for count in [13_usize, 16, 1, 11, 9, 14, 7, 16] {
                fast.refill();
                let count_u8 = u8::try_from(count).unwrap();
                assert_eq!(fast.peek_no_fill(count), reference.peek(count_u8));
                fast.skip_no_fill(count);
                reference.skip(count);
            }
        }

        let truncated = [0xaa, 0xbb];
        let mut reference = BitReader::new(&truncated);
        let mut fast = FastBitReader::new(&truncated);
        fast.refill();
        assert_eq!(fast.peek_no_fill(16), reference.peek(16));
        fast.skip_no_fill(16);
        reference.skip(16);
        fast.refill();
        assert_eq!(fast.peek_no_fill(16), reference.peek(16));
    }

    #[test]
    fn buffered_fast_decode_records_invalid_huffman_codes() {
        let mut counts = [0_u8; 16];
        counts[0] = 1;
        let table = super::HuffmanTable::parse(&counts, &[1]).unwrap();
        let mut bits = FastBitReader::new(&[0x80, 0, 0, 0]);

        bits.refill();
        let _ = table.decode_difference_fast(&mut bits);
        assert!(bits.has_error());
    }

    #[test]
    fn fast_decode_resolves_a_sixteen_bit_huffman_code() {
        let mut counts = [0_u8; 16];
        counts[15] = 1;
        let table = super::HuffmanTable::parse(&counts, &[1]).unwrap();
        let mut bits = FastBitReader::new(&[0, 0, 0]);

        bits.refill();
        assert_eq!(table.decode_difference_fast(&mut bits), -1);
        assert!(!bits.has_error());
    }

    #[test]
    fn malformed_huffman_code_returns_scan_error() {
        let encoded = super::encode_interleaved_for_test(&[8_192; 16], 4, 4, 1, 14);
        let dht = encoded
            .windows(2)
            .position(|marker| marker == [0xff, 0xc4])
            .unwrap();
        let sos = encoded[dht + 2..]
            .windows(2)
            .position(|marker| marker == [0xff, 0xda])
            .map(|offset| dht + 2 + offset)
            .unwrap();

        let mut malformed = encoded[..dht].to_vec();
        malformed.extend_from_slice(&[0xff, 0xc4, 0x00, 20, 0]);
        malformed.extend_from_slice(&[0, 0, 0, 0, 1, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0, 0]);
        malformed.push(0);
        let new_sos = malformed.len();
        malformed.extend_from_slice(&encoded[sos..]);
        let entropy_start = new_sos + 10;
        malformed[entropy_start] |= 0x80;

        let Err(error) = decode_interleaved_impl(&malformed, 14, true) else {
            panic!("malformed stream decoded successfully");
        };
        assert!(error.contains("invalid Huffman code"), "{error}");
    }

    #[test]
    fn fast_row_sink_matches_general_decode_and_maps_packed_rows() {
        for components in [1_u8, 2] {
            let width = 17_u16;
            let height = 9_u16;
            let samples = (0..usize::from(width) * usize::from(height) * usize::from(components))
                .map(|index| ((index * 977 + 12_345) & 0x3fff) as u16)
                .collect::<Vec<_>>();
            let encoded =
                super::encode_interleaved_for_test(&samples, width, height, components, 14);
            let expected_width = usize::from(width) * usize::from(components);
            let mut rows = Vec::new();
            assert!(
                decode_into_rows(
                    &encoded,
                    14,
                    expected_width,
                    usize::from(height),
                    |row, samples| {
                        rows.push((row, samples.to_vec()));
                        Ok(())
                    },
                )
                .unwrap()
            );
            assert_eq!(
                rows.into_iter()
                    .flat_map(|(_, samples)| samples)
                    .collect::<Vec<_>>(),
                decode_interleaved_impl(&encoded, 14, false)
                    .unwrap()
                    .samples
            );
        }

        let packed = super::encode_interleaved_for_test(&[1, 2, 3, 4], 4, 1, 1, 16);
        let mut rows = Vec::new();
        assert!(
            decode_into_rows(&packed, 16, 2, 2, |row, samples| {
                rows.push((row, samples.to_vec()));
                Ok(())
            })
            .unwrap()
        );
        assert_eq!(rows, vec![(0, vec![1, 2]), (1, vec![3, 4])]);
    }

    #[test]
    #[ignore = "manual allocation benchmark"]
    fn bench_reused_output_buffer() {
        fn median(mut values: Vec<Duration>) -> Duration {
            values.sort_unstable();
            values[values.len() / 2]
        }

        const WIDTH: u16 = 256;
        const HEIGHT: u16 = 256;
        for components in [1_u8, 2] {
            let output_width = usize::from(WIDTH) * usize::from(components);
            let samples = (0..output_width * usize::from(HEIGHT))
                .map(|index| ((index * 9_973 + 17) & 0x3fff) as u16)
                .collect::<Vec<_>>();
            let encoded =
                super::encode_interleaved_for_test(&samples, WIDTH, HEIGHT, components, 14);
            for tiles in [16_usize, 256] {
                let fresh = median(
                    (0..7)
                        .map(|_| {
                            let start = Instant::now();
                            for _ in 0..tiles {
                                let decoded =
                                    decode_interleaved_impl(black_box(&encoded), 14, true).unwrap();
                                black_box(decoded.samples);
                            }
                            start.elapsed()
                        })
                        .collect(),
                );

                let mut reused = vec![0_u16; output_width * usize::from(HEIGHT)];
                reused.fill(0xaaaa);
                black_box(&reused);
                let reused_output = median(
                    (0..7)
                        .map(|_| {
                            let start = Instant::now();
                            for _ in 0..tiles {
                                let mut scratch = FastRowScratch::default();
                                assert!(
                                    decode_into_rows_with_scratch(
                                        &encoded,
                                        14,
                                        output_width,
                                        usize::from(HEIGHT),
                                        &mut scratch,
                                        |row, samples| {
                                            let target =
                                                &mut reused[row * output_width..][..output_width];
                                            target.copy_from_slice(samples);
                                            Ok(())
                                        },
                                    )
                                    .unwrap()
                                );
                            }
                            black_box(&reused);
                            start.elapsed()
                        })
                        .collect(),
                );

                let mut scratch = FastRowScratch::default();
                let reused_all = median(
                    (0..7)
                        .map(|_| {
                            let start = Instant::now();
                            for _ in 0..tiles {
                                assert!(
                                    decode_into_rows_with_scratch(
                                        &encoded,
                                        14,
                                        output_width,
                                        usize::from(HEIGHT),
                                        &mut scratch,
                                        |row, samples| {
                                            let target =
                                                &mut reused[row * output_width..][..output_width];
                                            target.copy_from_slice(samples);
                                            Ok(())
                                        },
                                    )
                                    .unwrap()
                                );
                            }
                            black_box(&reused);
                            start.elapsed()
                        })
                        .collect(),
                );

                println!(
                    "benchmark,components={components},tiles={tiles},fresh_owned_ms={:.6},\
                     reused_output_ms={:.6},reused_output_scratch_ms={:.6}",
                    fresh.as_secs_f64() * 1_000.0,
                    reused_output.as_secs_f64() * 1_000.0,
                    reused_all.as_secs_f64() * 1_000.0,
                );
            }
        }
    }

    #[test]
    #[ignore = "manual full-output benchmark"]
    fn bench_reused_full_output() {
        const TILE: usize = 256;
        const WIDTH: usize = 5_920;
        const HEIGHT: usize = 4_096;

        let samples = (0..TILE * TILE)
            .map(|index| ((index * 9_973 + 17) & 0x3fff) as u16)
            .collect::<Vec<_>>();
        let encoded = super::encode_interleaved_for_test(&samples, TILE as u16, TILE as u16, 1, 14);
        let across = WIDTH.div_ceil(TILE);
        let down = HEIGHT.div_ceil(TILE);
        let chunk_count = across * down;
        let mut source = Vec::with_capacity(encoded.len() * chunk_count);
        let mut chunks = Vec::with_capacity(chunk_count);
        for _ in 0..chunk_count {
            chunks.push(Chunk {
                offset: source.len(),
                byte_count: encoded.len(),
            });
            source.extend_from_slice(&encoded);
        }
        let request = DecodeRequest {
            source: &source,
            width: WIDTH,
            height: HEIGHT,
            chunk_width: TILE,
            chunk_height: TILE,
            chunks: &chunks,
            bits_per_sample: 14,
            sample_format: SampleFormat::Integer,
            byte_order: ByteOrder::Little,
            predictor: Predictor::None,
        };

        let mut fresh = Vec::new();
        for _ in 0..7 {
            let start = Instant::now();
            black_box(super::decode(black_box(&request)).unwrap());
            fresh.push(start.elapsed());
        }
        fresh.sort_unstable();

        let geometry = Geometry::new(WIDTH, HEIGHT, TILE, TILE, chunk_count).unwrap();
        let mut output = vec![0_u16; WIDTH * HEIGHT];
        output.fill(0xaaaa);
        black_box(&output);
        let mut reused = Vec::new();
        for _ in 0..7 {
            let start = Instant::now();
            for tile_y_index in 0..geometry.down {
                let mut scratch = FastRowScratch::default();
                let tile_y = tile_y_index * TILE;
                let copy_height = TILE.min(HEIGHT - tile_y);
                for tile_x_index in 0..geometry.across {
                    let index = tile_y_index * geometry.across + tile_x_index;
                    let chunk = request.chunks[index];
                    let encoded = &request.source[chunk.offset..chunk.offset + chunk.byte_count];
                    let tile_x = tile_x_index * TILE;
                    let copy_width = TILE.min(WIDTH - tile_x);
                    assert!(
                        decode_into_rows_with_scratch(
                            encoded,
                            14,
                            TILE,
                            TILE,
                            &mut scratch,
                            |row, samples| {
                                if row < copy_height {
                                    let target_start = row * WIDTH + tile_x;
                                    output[target_start..target_start + copy_width]
                                        .copy_from_slice(&samples[..copy_width]);
                                }
                                Ok(())
                            },
                        )
                        .unwrap()
                    );
                }
            }
            black_box(&output);
            reused.push(start.elapsed());
        }
        reused.sort_unstable();

        println!(
            "benchmark,full_output=tiles_{chunk_count},fresh_owned_ms={:.6},reused_ms={:.6}",
            fresh[fresh.len() / 2].as_secs_f64() * 1_000.0,
            reused[reused.len() / 2].as_secs_f64() * 1_000.0,
        );
    }

    #[test]
    fn unstuffed_reader_matches_fast_bit_reader_across_boundaries_and_errors() {
        use super::{
            FastBits, FastRowScratch, UnstuffedBitReader, decode_scan_rows_inner, parse_jpeg,
            unstuff_entropy_with_chunk_size,
        };

        // Case 1: byte-for-byte / bit-for-bit boundary checks against
        // FastBitReader.
        let streams: [&[u8]; 7] = [
            &[],
            &[0xff],
            &[0xff, 0x00, 0xab, 0xff, 0x00, 0xcd],
            &[0x12, 0x34, 0xff, 0x00, 0x56, 0xff, 0xd9, 0xaa, 0xbb],
            &[0x12, 0xff, 0xff, 0x00, 0x34, 0x56],
            &[0xaa, 0xbb, 0xcc, 0xdd, 0xee, 0xff, 0x00, 0x11, 0xff],
            &[
                1, 2, 3, 4, 5, 6, 7, 0xff, 0x00, 8, 9, 10, 11, 12, 13, 14, 15, 0xff, 0x00, 0xff,
                0x00, 0x42, 0xff, 0xc4, 0x99,
            ],
        ];
        for stream in streams {
            for chunk_size in [1, 2, 7, 8, 9, 16, 64] {
                let chunks = unstuff_entropy_with_chunk_size(stream, chunk_size).unwrap();
                let mut oracle = FastBitReader::new(stream);
                let mut fast = UnstuffedBitReader::new(&chunks);
                for _ in 0..(stream.len() * 2 + 8) {
                    oracle.refill();
                    fast.refill();
                    assert_eq!(
                        fast.peek_no_fill(16),
                        oracle.peek_no_fill(16),
                        "chunk_size={chunk_size} stream={stream:02x?}"
                    );
                    oracle.skip_no_fill(5);
                    fast.skip_no_fill(5);
                    assert_eq!(fast.take_error(), oracle.take_error());
                }
            }
        }

        // Case 2: full 2-component scan decode equivalence (valid, truncated,
        // invalid).
        let width = 64_u16;
        let height = 32_u16;
        let samples: Vec<u16> = (0..usize::from(width) * usize::from(height) * 2)
            .map(|i| ((i * 977) ^ 0x2a5a) as u16 & 0x3fff)
            .collect();
        let encoded = super::encode_interleaved_for_test(&samples, width, height, 2, 14);
        let parsed = parse_jpeg(&encoded, 14).unwrap();
        let tables: Vec<_> = parsed
            .frame
            .components
            .iter()
            .map(|c| parsed.tables[c.huffman_table].as_ref().unwrap())
            .collect();

        let decode_oracle = |entropy: &[u8]| {
            let mut scratch = FastRowScratch::default();
            let mut out = vec![0_u16; samples.len()];
            let row_w = usize::from(width) * 2;
            let res = decode_scan_rows_inner(
                FastBitReader::new(entropy),
                &parsed.frame,
                &tables,
                parsed.predictor,
                parsed.point_transform,
                row_w,
                &mut scratch,
                |r, s| {
                    out[r * row_w..][..row_w].copy_from_slice(s);
                    Ok(())
                },
            );
            res.map(|()| out)
        };
        let oracle_ok = decode_oracle(parsed.entropy).unwrap();
        assert_eq!(oracle_ok, samples);

        for chunk_size in [1, 7, 8, 9, 17, 64] {
            let chunks = unstuff_entropy_with_chunk_size(parsed.entropy, chunk_size).unwrap();
            let mut scratch = FastRowScratch::default();
            let mut out = vec![0_u16; samples.len()];
            let row_w = usize::from(width) * 2;
            decode_scan_rows_inner(
                UnstuffedBitReader::new(&chunks),
                &parsed.frame,
                &tables,
                parsed.predictor,
                parsed.point_transform,
                row_w,
                &mut scratch,
                |r, s| {
                    out[r * row_w..][..row_w].copy_from_slice(s);
                    Ok(())
                },
            )
            .unwrap();
            assert_eq!(out, oracle_ok, "chunk_size={chunk_size}");
        }

        let trunc_len = parsed.entropy.len() / 2;
        let truncated = &parsed.entropy[..trunc_len];
        let oracle_trunc = decode_oracle(truncated).unwrap_err();
        let chunks = unstuff_entropy_with_chunk_size(truncated, 7).unwrap();
        let mut scratch = FastRowScratch::default();
        let err = decode_scan_rows_inner(
            UnstuffedBitReader::new(&chunks),
            &parsed.frame,
            &tables,
            parsed.predictor,
            parsed.point_transform,
            usize::from(width) * 2,
            &mut scratch,
            |_, _| Ok(()),
        )
        .unwrap_err();
        assert_eq!(err, oracle_trunc);
    }

    #[test]
    fn direct_one_and_two_component_scan_matches_oracle_across_widths_and_truncation() {
        use super::{
            FastRowScratch, decode_scan_prediction_one, decode_scan_rows,
            decode_scan_rows_generic_oracle, parse_jpeg,
        };

        for components in [1_u8, 2_u8] {
            for (width, height) in [(1_u16, 1_u16), (3, 4), (17, 9)] {
                let count = usize::from(width) * usize::from(height) * usize::from(components);
                let samples: Vec<u16> = (0..count)
                    .map(|i| ((i * 613) ^ 0x1357) as u16 & 0x3fff)
                    .collect();
                let encoded =
                    super::encode_interleaved_for_test(&samples, width, height, components, 14);
                let parsed = parse_jpeg(&encoded, 14).unwrap();
                let tables: Vec<_> = parsed
                    .frame
                    .components
                    .iter()
                    .map(|c| parsed.tables[c.huffman_table].as_ref().unwrap())
                    .collect();
                let row_w = usize::from(width) * usize::from(components);

                let mut oracle_out = vec![0_u16; count];
                let mut scratch = FastRowScratch::default();
                decode_scan_rows_generic_oracle(
                    FastBitReader::new(parsed.entropy),
                    &parsed.frame,
                    &tables,
                    parsed.predictor,
                    parsed.point_transform,
                    row_w,
                    &mut scratch,
                    |r, s| {
                        oracle_out[r * row_w..][..row_w].copy_from_slice(s);
                        Ok(())
                    },
                )
                .unwrap();

                let mut direct_out = vec![0_u16; count];
                decode_scan_prediction_one(
                    parsed.entropy,
                    &parsed.frame,
                    &tables,
                    parsed.point_transform,
                    row_w,
                    &mut direct_out,
                )
                .unwrap();
                assert_eq!(direct_out, oracle_out);

                let mut sink_out = vec![0_u16; count];
                decode_scan_rows(
                    parsed.entropy,
                    &parsed.frame,
                    &tables,
                    parsed.predictor,
                    parsed.point_transform,
                    row_w,
                    |r, s| {
                        sink_out[r * row_w..][..row_w].copy_from_slice(s);
                        Ok(())
                    },
                )
                .unwrap();
                assert_eq!(sink_out, oracle_out);

                let truncated = &parsed.entropy[..parsed.entropy.len() / 2];
                let mut trunc_out = vec![0_u16; count];
                assert!(
                    decode_scan_prediction_one(
                        truncated,
                        &parsed.frame,
                        &tables,
                        parsed.point_transform,
                        row_w,
                        &mut trunc_out,
                    )
                    .is_err()
                );
            }
        }
    }
}
