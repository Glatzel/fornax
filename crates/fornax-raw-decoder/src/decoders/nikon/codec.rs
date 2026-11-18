use std::sync::Arc;

use fornax_codec::{ICodecDecode, unpack_packed_u16_rows, unpack_u16_word_rows};

use crate::decoders::nikon::error::{invalid, unsupported};
use crate::decoders::nikon::huffman::{BitReader, HuffmanTable};
use crate::decoders::nikon::request::{ByteOrder, DecodeRequest};
use crate::decoders::nikon::samples::{buffer_u16, chunk};
use crate::decoders::nikon::{CodecError, NikonTreeId, SampleBuffer, SampleFormat};

// Numeric format data only. Provenance, licensing, source agreement, and the
// lossless-only scope are documented in
// docs/reference/decoders/reference/nikon-nef.md.
const LOSSLESS_12_COUNTS: [u8; 16] = [0, 1, 4, 2, 3, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const LOSSLESS_12_SYMBOLS: [u8; 13] = [5, 4, 6, 3, 7, 2, 8, 1, 9, 0, 10, 11, 12];
const LOSSLESS_14_COUNTS: [u8; 16] = [0, 1, 4, 2, 2, 3, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0];
const LOSSLESS_14_SYMBOLS: [u8; 15] = [7, 6, 8, 5, 9, 4, 10, 3, 11, 12, 2, 0, 1, 13, 14];
const FUSED_BITS: u8 = 13;
const FUSED_SIZE: usize = 1 << FUSED_BITS;
const FAST_GROUP: usize = 4;

#[derive(Clone, Copy, Debug)]
struct FusedEntry {
    consumed: u8,
    difference: i16,
}

impl FusedEntry {
    const ESCAPE: Self = Self {
        consumed: 0,
        difference: 0,
    };
}

struct FusedTable {
    entries: Box<[FusedEntry; FUSED_SIZE]>,
}

impl FusedTable {
    fn build(counts: &[u8; 16], symbols: &[u8], bits: u8) -> Result<Self, CodecError> {
        let mut entries = Box::new([FusedEntry::ESCAPE; FUSED_SIZE]);
        let mut code = 0_u32;
        let mut offset = 0_usize;
        for (index, count) in counts.iter().copied().enumerate() {
            let code_length = u8::try_from(index + 1).expect("Huffman lengths fit u8");
            code <<= 1;
            for symbol_index in 0..usize::from(count) {
                let difference_bits = symbols[offset + symbol_index];
                if difference_bits > bits {
                    return Err(invalid(format!(
                        "Nikon Huffman difference length {difference_bits} exceeds {bits} bits"
                    )));
                }
                let consumed = code_length + difference_bits;
                if code_length <= FUSED_BITS && consumed <= FUSED_BITS {
                    let suffix_bits = FUSED_BITS - code_length;
                    let first = (code
                        + u32::try_from(symbol_index).expect("symbol index fits u32"))
                        << suffix_bits;
                    let suffix_count = 1_usize << suffix_bits;
                    let difference_shift = FUSED_BITS - consumed;
                    for suffix in 0..suffix_count {
                        let difference_bits = (suffix >> difference_shift) as u32;
                        let difference = decode_difference_value(
                            difference_bits,
                            symbols[offset + symbol_index],
                        )? as i16;
                        entries[first as usize + suffix] = FusedEntry {
                            consumed,
                            difference,
                        };
                    }
                }
            }
            code += u32::from(count);
            offset += usize::from(count);
        }
        Ok(Self { entries })
    }
}

#[allow(dead_code, reason = "the reference kernel remains the test oracle")]
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum HuffmanKernel {
    Fused,
    Reference,
}

struct WindowReader<'a> {
    source: &'a [u8],
    bit_position: usize,
    total_bits: usize,
}

impl<'a> WindowReader<'a> {
    fn new(source: &'a [u8]) -> Result<Self, CodecError> {
        let total_bits = source
            .len()
            .checked_mul(8)
            .ok_or_else(|| invalid("Nikon Huffman bit count overflow"))?;
        Ok(Self {
            source,
            bit_position: 0,
            total_bits,
        })
    }

    fn window(&self) -> Option<u64> {
        let byte = self.bit_position / 8;
        let offset = self.bit_position % 8;
        self.source.get(byte..byte + 8).map(|bytes| {
            u64::from_be_bytes(bytes.try_into().expect("a byte window has eight bytes")) << offset
        })
    }

    fn advance(&mut self, count: u8) -> Result<(), CodecError> {
        let count = usize::from(count);
        let remaining = self
            .total_bits
            .checked_sub(self.bit_position)
            .ok_or_else(|| invalid("Nikon Huffman bit position overflow"))?;
        if count > remaining {
            return Err(invalid("Huffman bitstream is exhausted"));
        }
        self.bit_position += count;
        Ok(())
    }

    fn decode_reference_difference(
        &mut self,
        table: &HuffmanTable,
        bits: u8,
    ) -> Result<i32, CodecError> {
        let byte = self.bit_position / 8;
        let offset = u8::try_from(self.bit_position % 8).expect("bit offset fits u8");
        let mut reader = BitReader::new(&self.source[byte..]);
        if offset != 0 {
            reader.read_bits(offset)?;
        }
        let before = reader.remaining_bits();
        let difference = decode_difference(table, &mut reader, bits)?;
        let consumed = before - reader.remaining_bits();
        self.bit_position = self
            .bit_position
            .checked_add(consumed)
            .ok_or_else(|| invalid("Nikon Huffman bit position overflow"))?;
        Ok(difference)
    }
}

#[derive(Clone, Debug)]
pub enum NikonCodec {
    Packed {
        row_stride: usize,
        bits: u8,
        byte_order: ByteOrder,
    },
    UnpackedWords {
        bits: u8,
        byte_order: ByteOrder,
    },
    StripedPacked12,
    Huffman {
        bits: u8,
        tree: NikonTreeId,
        init: [[u16; 2]; 2],
        curve: Arc<[u16]>,
    },
}

impl ICodecDecode for NikonCodec {
    fn decode(&self, request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
        match self {
            Self::Packed {
                row_stride,
                bits,
                byte_order,
            } => decode_packed(request, *row_stride, *bits, *byte_order),
            Self::UnpackedWords { bits, byte_order } => {
                decode_unpacked_words(request, *bits, *byte_order)
            }
            Self::StripedPacked12 => decode_striped_packed_12(request),
            Self::Huffman {
                bits,
                tree,
                init,
                curve,
            } => decode_huffman(request, *bits, *tree, *init, curve),
        }
    }
}

pub fn decode_huffman(
    request: &DecodeRequest<'_>,
    bits: u8,
    tree: NikonTreeId,
    init: [[u16; 2]; 2],
    curve: &[u16],
) -> Result<SampleBuffer, CodecError> {
    decode_huffman_with_kernel(request, bits, tree, init, curve, HuffmanKernel::Fused)
}

fn decode_huffman_with_kernel(
    request: &DecodeRequest<'_>,
    bits: u8,
    tree: NikonTreeId,
    init: [[u16; 2]; 2],
    curve: &[u16],
    kernel: HuffmanKernel,
) -> Result<SampleBuffer, CodecError> {
    if request.sample_format != SampleFormat::Integer {
        return Err(invalid("Nikon Huffman decoding requires integer samples"));
    }
    if tree.bits() != bits {
        return Err(invalid(format!(
            "Nikon Huffman {bits}-bit data uses a {}-bit tree",
            tree.bits()
        )));
    }
    if request.bits_per_sample != u16::from(bits) {
        return Err(invalid(format!(
            "Nikon Huffman bit depth {} disagrees with request depth {}",
            bits, request.bits_per_sample
        )));
    }
    if curve.is_empty() {
        return Err(invalid("Nikon Huffman curve must not be empty"));
    }
    if request.width == 0 || request.height == 0 {
        return Err(invalid("Nikon Huffman dimensions must not be zero"));
    }
    if request.chunks.len() != 1 {
        return Err(unsupported(
            "multi-strip Nikon mode-3 Huffman data is unsupported",
        ));
    }
    if request.chunk_width != request.width || request.chunk_height < request.height {
        return Err(invalid(
            "Nikon Huffman requires one strip spanning the full image",
        ));
    }
    let sample_count = request
        .width
        .checked_mul(request.height)
        .ok_or_else(|| invalid("Nikon Huffman sample count overflow"))?;
    let payload = chunk(request.source, request.chunks[0])?;
    let (counts, symbols) = tree_tables(tree);
    let table = HuffmanTable::from_counts(counts, symbols)?;
    let mut output = vec![0_u16; sample_count];
    match kernel {
        HuffmanKernel::Fused if payload.len() >= 8 => {
            let fused = FusedTable::build(counts, symbols, bits)?;
            decode_huffman_fused(
                payload,
                request.width,
                request.height,
                bits,
                &table,
                &fused,
                init,
                curve,
                &mut output,
            )?;
        }
        HuffmanKernel::Fused | HuffmanKernel::Reference => decode_huffman_reference(
            payload,
            request.width,
            request.height,
            bits,
            &table,
            init,
            curve,
            &mut output,
        )?,
    }

    buffer_u16(request.width, request.height, output)
}

fn decode_huffman_reference(
    payload: &[u8],
    width: usize,
    height: usize,
    bits: u8,
    table: &HuffmanTable,
    init: [[u16; 2]; 2],
    curve: &[u16],
    output: &mut [u16],
) -> Result<(), CodecError> {
    let mut reader = BitReader::new(payload);
    let mut vertical = init.map(|values| values.map(i32::from));

    for row in 0..height {
        let parity = row & 1;
        let mut horizontal = vertical[parity];
        for column in 0..width {
            let difference = decode_difference(table, &mut reader, bits)?;
            let predictor =
                update_predictor(column, difference, &mut vertical[parity], &mut horizontal)?;
            output[row * width + column] = curve_value(curve, predictor)?;
        }
    }
    Ok(())
}

fn decode_huffman_fused(
    payload: &[u8],
    width: usize,
    height: usize,
    bits: u8,
    table: &HuffmanTable,
    fused: &FusedTable,
    init: [[u16; 2]; 2],
    curve: &[u16],
    output: &mut [u16],
) -> Result<(), CodecError> {
    let mut reader = WindowReader::new(payload)?;
    let mut vertical = init.map(|values| values.map(i32::from));

    for row in 0..height {
        let parity = row & 1;
        let mut vertical_row = vertical[parity];
        let mut left = vertical_row;
        let mut column = 0_usize;
        let row_output = &mut output[row * width..][..width];

        while column + FAST_GROUP <= width {
            let Some(mut window) = reader.window() else {
                break;
            };
            let mut consumed = 0_u8;
            let mut decoded = 0_usize;
            while decoded < FAST_GROUP {
                let entry = fused.entries[(window >> (64 - FUSED_BITS)) as usize];
                if entry.consumed == 0 {
                    break;
                }
                let predictor = update_predictor(
                    column + decoded,
                    i32::from(entry.difference),
                    &mut vertical_row,
                    &mut left,
                )?;
                row_output[column + decoded] = curve_value(curve, predictor)?;
                window <<= entry.consumed;
                consumed = consumed
                    .checked_add(entry.consumed)
                    .ok_or_else(|| invalid("Nikon Huffman fused bit count overflow"))?;
                decoded += 1;
            }
            reader.advance(consumed)?;
            column += decoded;
            if decoded != FAST_GROUP {
                let difference = reader.decode_reference_difference(table, bits)?;
                let predictor = update_predictor(column, difference, &mut vertical_row, &mut left)?;
                row_output[column] = curve_value(curve, predictor)?;
                column += 1;
            }
        }
        while column < width {
            let difference = reader.decode_reference_difference(table, bits)?;
            let predictor = update_predictor(column, difference, &mut vertical_row, &mut left)?;
            row_output[column] = curve_value(curve, predictor)?;
            column += 1;
        }
        vertical[parity] = vertical_row;
    }
    Ok(())
}

#[inline(always)]
fn update_predictor(
    column: usize,
    difference: i32,
    vertical: &mut [i32; 2],
    horizontal: &mut [i32; 2],
) -> Result<i32, CodecError> {
    let phase = column & 1;
    let base = if column < 2 {
        &mut vertical[column]
    } else {
        &mut horizontal[phase]
    };
    *base = base
        .checked_add(difference)
        .ok_or_else(|| invalid("Nikon Huffman predictor overflow"))?;
    let predictor = *base;
    horizontal[phase] = predictor;
    Ok(predictor)
}

#[inline(always)]
fn curve_value(curve: &[u16], predictor: i32) -> Result<u16, CodecError> {
    let index =
        usize::try_from(predictor).map_err(|_| invalid("Nikon Huffman curve index is negative"))?;
    curve.get(index).copied().ok_or_else(|| {
        invalid(format!(
            "Nikon Huffman curve index {index} exceeds {} entries",
            curve.len()
        ))
    })
}

fn decode_difference(
    table: &HuffmanTable,
    reader: &mut BitReader<'_>,
    bits: u8,
) -> Result<i32, CodecError> {
    let length = table.decode_symbol(reader)?;
    if length == 0 {
        return Ok(0);
    }
    if length > bits {
        return Err(invalid(format!(
            "Nikon Huffman difference length {length} exceeds {bits} bits"
        )));
    }
    let value = reader.read_bits(length)? as i32;
    decode_difference_value(value as u32, length)
}

fn decode_difference_value(value: u32, length: u8) -> Result<i32, CodecError> {
    if length == 0 {
        return Ok(0);
    }
    if length > 14 {
        return Err(invalid(format!(
            "Nikon Huffman difference length {length} is invalid"
        )));
    }
    let value = i32::try_from(value).expect("a Nikon difference bit field fits i32");
    if value & (1 << (length - 1)) == 0 {
        Ok(value - ((1 << length) - 1))
    } else {
        Ok(value)
    }
}

const fn tree_tables(tree: NikonTreeId) -> (&'static [u8; 16], &'static [u8]) {
    match tree {
        NikonTreeId::Lossless12 => (&LOSSLESS_12_COUNTS, &LOSSLESS_12_SYMBOLS),
        NikonTreeId::Lossless14 => (&LOSSLESS_14_COUNTS, &LOSSLESS_14_SYMBOLS),
    }
}

pub fn decode_packed(
    request: &DecodeRequest<'_>,
    row_stride: usize,
    bits: u8,
    byte_order: ByteOrder,
) -> Result<SampleBuffer, CodecError> {
    validate_bits(bits)?;
    if bits == 14 && byte_order == ByteOrder::Big {
        return Err(invalid(
            "big-endian packed 14-bit Nikon data is unsupported",
        ));
    }
    let rows = validate_geometry(request)?;
    let row_bytes = packed_row_bytes(request.width, bits)?;
    if row_stride < row_bytes {
        return Err(invalid(format!(
            "Nikon packed row stride {row_stride} is shorter than required {row_bytes}"
        )));
    }
    validate_chunk_lengths(request, &rows, |count| {
        count
            .checked_mul(row_stride)
            .ok_or_else(|| invalid("Nikon packed strip size overflow"))
    })?;
    let sample_count = checked_sample_count(request.width, request.height)?;
    let mut output = vec![0_u16; sample_count];
    let payloads = request
        .chunks
        .iter()
        .copied()
        .map(|strip| chunk(request.source, strip))
        .collect::<Result<Vec<_>, _>>()?;
    unpack_packed_u16_rows(
        &payloads,
        &mut output,
        request.width,
        request.height,
        request.chunk_height,
        row_stride,
        u16::from(bits),
        |source, target| unpack_row(source, target, bits, byte_order),
    )?;
    buffer_u16(request.width, request.height, output)
}

pub fn decode_unpacked_words(
    request: &DecodeRequest<'_>,
    bits: u8,
    byte_order: ByteOrder,
) -> Result<SampleBuffer, CodecError> {
    validate_bits(bits)?;
    let rows = validate_geometry(request)?;
    validate_chunk_lengths(request, &rows, |count| {
        count
            .checked_mul(request.width)
            .and_then(|samples| samples.checked_mul(2))
            .ok_or_else(|| invalid("Nikon unpacked-word strip size overflow"))
    })?;
    let sample_count = checked_sample_count(request.width, request.height)?;
    let mut output = if rayon::current_num_threads() == 1 {
        fornax_large_alloc::zeroed_samples(sample_count)
            .map_err(|error| invalid(format!("Nikon unpacked-word allocation failed: {error}")))?
    } else {
        vec![0_u16; sample_count]
    };
    let mask = (1_u16 << bits) - 1;
    let payloads = request
        .chunks
        .iter()
        .copied()
        .map(|strip| chunk(request.source, strip))
        .collect::<Result<Vec<_>, _>>()?;
    unpack_u16_word_rows(
        &payloads,
        &mut output,
        request.width,
        request.height,
        request.chunk_height,
        byte_order,
        mask,
    )?;
    buffer_u16(request.width, request.height, output)
}

pub fn decode_striped_packed_12(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    if request.width == 0 || !request.width.is_multiple_of(8) {
        return Err(invalid(
            "Nikon mode-5 rows require a non-zero width divisible by eight",
        ));
    }
    let rows = validate_geometry(request)?;
    let row_bytes = request
        .width
        .checked_mul(12)
        .map(|bits| bits / 8)
        .ok_or_else(|| invalid("Nikon mode-5 row size overflow"))?;
    validate_chunk_lengths(request, &rows, |count| {
        count
            .checked_mul(row_bytes)
            .ok_or_else(|| invalid("Nikon mode-5 strip size overflow"))
    })?;
    let sample_count = checked_sample_count(request.width, request.height)?;
    let mut output = vec![0_u16; sample_count];

    for (tile, strip) in request.chunks.iter().copied().enumerate() {
        let start_row = tile
            .checked_mul(request.chunk_height)
            .ok_or_else(|| invalid("Nikon mode-5 row offset overflow"))?;
        let payload = chunk(request.source, strip)?;
        for row in 0..rows[tile] {
            let source = &payload[row * row_bytes..][..row_bytes];
            unpack_word_stream(
                source,
                &mut output[(start_row + row) * request.width..][..request.width],
            )?;
        }
    }
    buffer_u16(request.width, request.height, output)
}

fn unpack_row(
    source: &[u8],
    target: &mut [u16],
    bits: u8,
    byte_order: ByteOrder,
) -> Result<(), CodecError> {
    match byte_order {
        ByteOrder::Little => unpack_row_grouped(source, target, bits),
        ByteOrder::Big => unpack_row_grouped_big_endian(source, target, bits),
    }
}

fn unpack_row_grouped(source: &[u8], target: &mut [u16], bits: u8) -> Result<(), CodecError> {
    let group_bytes = usize::from(bits);
    let groups = target.len() / 8;
    let grouped_bytes = groups
        .checked_mul(group_bytes)
        .ok_or_else(|| invalid("Nikon packed-row group size overflow"))?;
    if source.len() < grouped_bytes {
        return Err(invalid("Nikon packed row is truncated"));
    }
    let mask = (1_u128 << bits) - 1;
    for (group, values) in target.as_chunks_mut::<8>().0.iter_mut().enumerate() {
        let start = group * group_bytes;
        let mut bytes = [0_u8; 16];
        bytes[..group_bytes].copy_from_slice(&source[start..start + group_bytes]);
        let word = u128::from_le_bytes(bytes);
        for (index, value) in values.iter_mut().enumerate() {
            *value = ((word >> (index * usize::from(bits))) & mask) as u16;
        }
    }
    unpack_row_little_endian_tail(&source[grouped_bytes..], &mut target[groups * 8..], bits)
}

fn unpack_row_grouped_big_endian(
    source: &[u8],
    target: &mut [u16],
    bits: u8,
) -> Result<(), CodecError> {
    let group_bytes = usize::from(bits);
    let groups = target.len() / 8;
    let grouped_bytes = groups
        .checked_mul(group_bytes)
        .ok_or_else(|| invalid("Nikon packed-row group size overflow"))?;
    if source.len() < grouped_bytes {
        return Err(invalid("Nikon packed row is truncated"));
    }
    let mask = (1_u128 << bits) - 1;
    for (group, values) in target.as_chunks_mut::<8>().0.iter_mut().enumerate() {
        let start = group * group_bytes;
        let mut bytes = [0_u8; 16];
        bytes[16 - group_bytes..].copy_from_slice(&source[start..start + group_bytes]);
        let word = u128::from_be_bytes(bytes);
        for (index, value) in values.iter_mut().enumerate() {
            let shift = group_bytes * 8 - (index + 1) * usize::from(bits);
            *value = ((word >> shift) & mask) as u16;
        }
    }
    unpack_row_big_endian_tail(&source[grouped_bytes..], &mut target[groups * 8..], bits)
}

fn unpack_row_little_endian_tail(
    source: &[u8],
    target: &mut [u16],
    bits: u8,
) -> Result<(), CodecError> {
    let bits = u32::from(bits);
    let mask = (1_u64 << bits) - 1;
    let mut buffer = 0_u64;
    let mut buffered = 0_u32;
    let mut bytes = source.iter().copied();
    for value in target {
        while buffered < bits {
            buffer |= u64::from(
                bytes
                    .next()
                    .ok_or_else(|| invalid("Nikon packed row is truncated"))?,
            ) << buffered;
            buffered += 8;
        }
        *value = (buffer & mask) as u16;
        buffer >>= bits;
        buffered -= bits;
    }
    Ok(())
}

fn unpack_row_big_endian_tail(
    source: &[u8],
    target: &mut [u16],
    bits: u8,
) -> Result<(), CodecError> {
    let bits = u32::from(bits);
    let mask = (1_u64 << bits) - 1;
    let mut buffer = 0_u64;
    let mut buffered = 0_u32;
    let mut bytes = source.iter().copied();
    for value in target {
        while buffered < bits {
            buffer = (buffer << 8)
                | u64::from(
                    bytes
                        .next()
                        .ok_or_else(|| invalid("Nikon packed row is truncated"))?,
                );
            buffered += 8;
        }
        buffered -= bits;
        *value = ((buffer >> buffered) & mask) as u16;
    }
    Ok(())
}

#[cfg(test)]
fn unpack_row_reference(
    source: &[u8],
    target: &mut [u16],
    bits: u8,
    byte_order: ByteOrder,
) -> Result<(), CodecError> {
    let bits = u32::from(bits);
    let mask = (1_u64 << bits) - 1;
    let mut buffer = 0_u64;
    let mut buffered = 0_u32;
    let mut bytes = source.iter().copied();
    match byte_order {
        ByteOrder::Little => {
            for value in target {
                while buffered < bits {
                    buffer |= u64::from(
                        bytes
                            .next()
                            .ok_or_else(|| invalid("Nikon packed row is truncated"))?,
                    ) << buffered;
                    buffered += 8;
                }
                *value = (buffer & mask) as u16;
                buffer >>= bits;
                buffered -= bits;
            }
        }
        ByteOrder::Big => {
            for value in target {
                while buffered < bits {
                    buffer = (buffer << 8)
                        | u64::from(
                            bytes
                                .next()
                                .ok_or_else(|| invalid("Nikon packed row is truncated"))?,
                        );
                    buffered += 8;
                }
                buffered -= bits;
                *value = ((buffer >> buffered) & mask) as u16;
            }
        }
    }
    Ok(())
}

fn unpack_word_stream(source: &[u8], target: &mut [u16]) -> Result<(), CodecError> {
    let mut words = source.as_chunks::<4>().0.iter();
    let mut buffer = 0_u64;
    let mut buffered = 0_u32;
    for value in target {
        while buffered < 12 {
            let bytes = words
                .next()
                .ok_or_else(|| invalid("Nikon mode-5 row is truncated"))?;
            let word = u32::from_le_bytes(*bytes);
            buffer = (buffer << 32) | u64::from(word);
            buffered += 32;
        }
        buffered -= 12;
        *value = ((buffer >> buffered) & 0x0fff) as u16;
    }
    Ok(())
}

fn validate_geometry(request: &DecodeRequest<'_>) -> Result<Vec<usize>, CodecError> {
    if request.width == 0 || request.height == 0 {
        return Err(invalid("Nikon image dimensions must not be zero"));
    }
    if request.chunk_width != request.width || request.chunk_height == 0 {
        return Err(invalid(
            "Nikon strips must span the full image width and have non-zero height",
        ));
    }
    let count = request.height.div_ceil(request.chunk_height);
    if request.chunks.len() != count {
        return Err(invalid(format!(
            "Nikon image requires {count} strips, found {}",
            request.chunks.len()
        )));
    }
    Ok((0..count)
        .map(|index| {
            let start = index * request.chunk_height;
            request.chunk_height.min(request.height - start)
        })
        .collect())
}

fn validate_chunk_lengths(
    request: &DecodeRequest<'_>,
    rows: &[usize],
    required: impl Fn(usize) -> Result<usize, CodecError>,
) -> Result<(), CodecError> {
    for (strip, rows) in request.chunks.iter().zip(rows) {
        let required = required(*rows)?;
        if strip.byte_count < required {
            return Err(invalid(format!(
                "Nikon strip is truncated: {} bytes available, {required} required",
                strip.byte_count
            )));
        }
        chunk(request.source, *strip)?;
    }
    Ok(())
}

fn checked_sample_count(width: usize, height: usize) -> Result<usize, CodecError> {
    width
        .checked_mul(height)
        .ok_or_else(|| invalid("Nikon sample count overflow"))
}

fn packed_row_bytes(width: usize, bits: u8) -> Result<usize, CodecError> {
    width
        .checked_mul(usize::from(bits))
        .map(|bits| bits.div_ceil(8))
        .ok_or_else(|| invalid("Nikon packed row size overflow"))
}

fn validate_bits(bits: u8) -> Result<(), CodecError> {
    if matches!(bits, 12 | 14) {
        Ok(())
    } else {
        Err(invalid(format!(
            "Nikon packed sample depth {bits} is unsupported"
        )))
    }
}

#[cfg(test)]
mod lossless_tests {
    use std::sync::Arc;

    use fornax_codec::{
        ByteOrder, Chunk, CodecError, DecodeRequest, ICodecDecode, Predictor, SampleBuffer,
        SampleFormat,
    };

    use super::{HuffmanKernel, tree_tables};
    use crate::decoders::nikon::NikonTreeId;
    use crate::decoders::nikon::codec::NikonCodec;
    use crate::decoders::nikon::huffman::HuffmanTable;

    #[test]
    fn both_lossless_tables_are_complete_canonical_prefix_codes() {
        for tree in [NikonTreeId::Lossless12, NikonTreeId::Lossless14] {
            let (counts, symbols) = tree_tables(tree);
            let table = HuffmanTable::from_counts(counts, symbols).unwrap();
            assert_eq!(table.symbol_count(), symbols.len());
        }
    }

    #[test]
    fn hand_written_stream_decodes_sign_and_horizontal_prediction() {
        let decoded = decode_identity(
            &[0xc9, 0x56, 0x69, 0x40],
            NikonTreeId::Lossless12,
            [[10, 20], [300, 400]],
            4,
            1,
        )
        .unwrap();

        assert_eq!(decoded, [12, 30, 15, 20]);
    }

    #[test]
    fn rows_use_their_initial_vertical_predictors_and_propagate_down() {
        let decoded = decode_identity(
            &[0xe7, 0x2e, 0x31],
            NikonTreeId::Lossless12,
            [[100, 200], [300, 400]],
            2,
            2,
        )
        .unwrap();

        assert_eq!(decoded, [101, 202, 299, 398]);
    }

    #[test]
    fn odd_width_decodes_only_the_needed_final_sample() {
        let decoded = decode_identity(
            &[0xe7, 0x2c, 0xc0],
            NikonTreeId::Lossless12,
            [[10, 20], [0, 0]],
            3,
            1,
        )
        .unwrap();

        assert_eq!(decoded, [11, 22, 14]);
    }

    #[test]
    fn non_trivial_curve_maps_predictor_indices() {
        let curve: Arc<[u16]> = Arc::from([100, 200, 300, 400]);
        let decoded = decode_with_curve(
            &[0xe7, 0x90],
            NikonTreeId::Lossless12,
            [[0, 1], [0, 0]],
            2,
            1,
            curve,
        )
        .unwrap();

        assert_eq!(decoded, [200, 300]);
    }

    #[test]
    fn curve_index_beyond_the_table_is_invalid() {
        let error = decode_with_curve(
            &[0xe7, 0x90],
            NikonTreeId::Lossless12,
            [[0, 1], [0, 0]],
            2,
            1,
            Arc::from([100, 200]),
        )
        .unwrap_err();

        assert!(matches!(error, CodecError::InvalidInput(_)));
    }

    #[test]
    fn truncated_stream_and_invalid_shape_are_invalid() {
        let truncated =
            decode_identity(&[0], NikonTreeId::Lossless12, [[0, 0], [0, 0]], 2, 1).unwrap_err();
        assert!(matches!(truncated, CodecError::InvalidInput(_)));

        let zero_width =
            decode_identity(&[0], NikonTreeId::Lossless12, [[0, 0], [0, 0]], 0, 1).unwrap_err();
        assert!(matches!(zero_width, CodecError::InvalidInput(_)));
    }

    #[test]
    fn trailing_bytes_are_ignored() {
        let decoded = decode_identity(
            &[0xe7, 0x2e, 0x31, 0xff],
            NikonTreeId::Lossless12,
            [[100, 200], [300, 400]],
            2,
            2,
        )
        .unwrap();

        assert_eq!(decoded, [101, 202, 299, 398]);
    }

    #[test]
    fn single_strip_may_declare_more_rows_than_the_image() {
        let curve = Arc::from(identity_curve(12));
        let chunks = [Chunk {
            offset: 0,
            byte_count: 2,
        }];
        let codec = NikonCodec::Huffman {
            bits: 12,
            tree: NikonTreeId::Lossless12,
            init: [[0, 1], [0, 0]],
            curve,
        };
        let request = DecodeRequest {
            source: &[0xe7, 0x90],
            width: 2,
            height: 1,
            chunk_width: 2,
            chunk_height: 3,
            chunks: &chunks,
            bits_per_sample: 12,
            sample_format: SampleFormat::Integer,
            byte_order: ByteOrder::Little,
            predictor: Predictor::None,
        };
        let SampleBuffer::U16(image) = codec.decode(&request).unwrap() else {
            panic!("Nikon Huffman returned non-u16 samples");
        };

        assert_eq!(image.into_raw(), [1, 2]);
    }

    #[test]
    fn multiple_strips_are_unsupported() {
        let curve = Arc::from(identity_curve(12));
        let chunks = [
            Chunk {
                offset: 0,
                byte_count: 1,
            },
            Chunk {
                offset: 1,
                byte_count: 1,
            },
        ];
        let codec = NikonCodec::Huffman {
            bits: 12,
            tree: NikonTreeId::Lossless12,
            init: [[0, 0], [0, 0]],
            curve,
        };
        let request = DecodeRequest {
            source: &[0, 0],
            width: 2,
            height: 2,
            chunk_width: 2,
            chunk_height: 1,
            chunks: &chunks,
            bits_per_sample: 12,
            sample_format: SampleFormat::Integer,
            byte_order: ByteOrder::Little,
            predictor: Predictor::None,
        };
        let error = codec.decode(&request).unwrap_err();

        assert!(matches!(error, CodecError::Unsupported(_)));
    }

    #[test]
    fn oversized_dimensions_fail_before_output_allocation() {
        let curve = Arc::from(identity_curve(12));
        let chunks = [Chunk {
            offset: 0,
            byte_count: 1,
        }];
        let codec = NikonCodec::Huffman {
            bits: 12,
            tree: NikonTreeId::Lossless12,
            init: [[0, 0], [0, 0]],
            curve,
        };
        let request = DecodeRequest {
            source: &[0],
            width: usize::MAX / 2,
            height: 4,
            chunk_width: usize::MAX / 2,
            chunk_height: 4,
            chunks: &chunks,
            bits_per_sample: 12,
            sample_format: SampleFormat::Integer,
            byte_order: ByteOrder::Little,
            predictor: Predictor::None,
        };
        let error = codec.decode(&request).unwrap_err();

        assert!(matches!(error, CodecError::InvalidInput(_)));
    }

    #[test]
    fn randomized_images_round_trip_at_twelve_and_fourteen_bits() {
        for (tree, bits) in [
            (NikonTreeId::Lossless12, 12_u8),
            (NikonTreeId::Lossless14, 14),
        ] {
            let mask = (1_u16 << bits) - 1;
            let init = [[mask / 5, mask / 7], [mask / 3, mask / 2]];
            let mut state = 0x1234_5678_u32 ^ u32::from(bits);
            let samples = (0..17 * 5)
                .map(|_| {
                    state ^= state << 13;
                    state ^= state >> 17;
                    state ^= state << 5;
                    (state as u16) & mask
                })
                .collect::<Vec<_>>();
            let source = encode_image(&samples, tree, init, 17, 5);
            let decoded = decode_identity(&source, tree, init, 17, 5).unwrap();

            assert_eq!(decoded, samples, "bits={bits}");
        }
    }

    #[test]
    fn fused_kernel_matches_reference_across_table_types_and_geometry() {
        for (tree, bits) in [
            (NikonTreeId::Lossless12, 12_u8),
            (NikonTreeId::Lossless14, 14),
        ] {
            let mask = (1_u16 << bits) - 1;
            for (width, height) in [(1, 1), (1, 4), (2, 1), (3, 2), (5, 3), (17, 2)] {
                let init = [[mask / 5, mask / 7], [mask / 3, mask / 2]];
                let mut state = 0x9e37_79b9_u32 ^ ((width as u32) << 8) ^ u32::from(bits);
                let samples = (0..width * height)
                    .map(|_| {
                        state ^= state << 13;
                        state ^= state >> 17;
                        state ^= state << 5;
                        (state as u16) & mask
                    })
                    .collect::<Vec<_>>();
                let source = encode_image(&samples, tree, init, width, height);

                let fast = decode_identity_with_kernel(
                    &source,
                    tree,
                    init,
                    width,
                    height,
                    HuffmanKernel::Fused,
                )
                .unwrap();
                let reference = decode_identity_with_kernel(
                    &source,
                    tree,
                    init,
                    width,
                    height,
                    HuffmanKernel::Reference,
                )
                .unwrap();

                assert_eq!(
                    fast, reference,
                    "bits={bits}, width={width}, height={height}"
                );
            }
        }
    }

    #[test]
    fn fused_kernel_preserves_large_difference_fallbacks() {
        for (tree, bits, init) in [
            (NikonTreeId::Lossless12, 12_u8, [[0, 4095], [4095, 0]]),
            (NikonTreeId::Lossless14, 14_u8, [[0, 16383], [16383, 0]]),
        ] {
            let mask = (1_u16 << bits) - 1;
            let samples = [mask, 0, mask, mask - 1, 0, mask, 1, 2, 3, 4, 5];
            let source = encode_image(&samples, tree, init, 11, 1);
            let fast =
                decode_identity_with_kernel(&source, tree, init, 11, 1, HuffmanKernel::Fused)
                    .unwrap();
            let reference =
                decode_identity_with_kernel(&source, tree, init, 11, 1, HuffmanKernel::Reference)
                    .unwrap();

            assert_eq!(fast, samples, "bits={bits}");
            assert_eq!(fast, reference, "bits={bits}");
        }
    }

    #[test]
    fn fused_and_reference_kernels_reject_exact_end_edges_identically() {
        for (tree, bits) in [
            (NikonTreeId::Lossless12, 12_u8),
            (NikonTreeId::Lossless14, 14),
        ] {
            let mask = (1_u16 << bits) - 1;
            let init = [[mask / 11, mask / 13], [mask / 17, mask / 19]];
            let mut samples = (0..40)
                .map(|index| ((index * 977) as u16).wrapping_mul(37) & mask)
                .collect::<Vec<_>>();
            samples[39] = mask;
            samples[37] = 0;
            let encoded_bits = encode_image_bits(&samples, tree, init, 8, 5);
            let exact = pack_bits(encoded_bits.clone());
            let before = pack_bits(encoded_bits[..encoded_bits.len() - 16].to_vec());
            let mid_code = pack_bits(encoded_bits[..encoded_bits.len() - 8].to_vec());

            for kernel in [HuffmanKernel::Fused, HuffmanKernel::Reference] {
                assert!(
                    decode_identity_with_kernel(&exact, tree, init, 8, 5, kernel).is_ok(),
                    "exact end failed for {kernel:?}"
                );
                assert!(
                    decode_identity_with_kernel(&before, tree, init, 8, 5, kernel).is_err(),
                    "before-end truncation passed for {kernel:?}"
                );
                assert!(
                    decode_identity_with_kernel(&mid_code, tree, init, 8, 5, kernel).is_err(),
                    "mid-code truncation passed for {kernel:?}"
                );
            }

            for cut in 0..exact.len() {
                let truncated = &exact[..cut];
                let fast =
                    decode_identity_with_kernel(truncated, tree, init, 8, 5, HuffmanKernel::Fused);
                let reference = decode_identity_with_kernel(
                    truncated,
                    tree,
                    init,
                    8,
                    5,
                    HuffmanKernel::Reference,
                );
                match (fast, reference) {
                    (Ok(fast), Ok(reference)) => assert_eq!(fast, reference),
                    (Err(_), Err(_)) => {}
                    (fast, reference) => panic!(
                        "kernels disagreed at {cut} retained bytes: fast={fast:?}, reference={reference:?}"
                    ),
                }
            }
        }
    }

    #[test]
    fn fused_kernel_reports_predictor_overflow_like_the_reference() {
        for (tree, bits) in [
            (NikonTreeId::Lossless12, 12_u8),
            (NikonTreeId::Lossless14, 14),
        ] {
            let source = encode_differences(&[1], tree);
            for kernel in [HuffmanKernel::Fused, HuffmanKernel::Reference] {
                let error = decode_identity_with_kernel(
                    &source,
                    tree,
                    [[u16::MAX, 0], [0, 0]],
                    1,
                    1,
                    kernel,
                )
                .unwrap_err();
                assert!(
                    matches!(error, CodecError::InvalidInput(_)),
                    "bits={bits}, kernel={kernel:?}: {error}"
                );
            }
        }
    }

    #[test]
    fn fused_and_reference_kernels_handle_bit_flips_without_diverging() {
        for (tree, bits) in [
            (NikonTreeId::Lossless12, 12_u8),
            (NikonTreeId::Lossless14, 14),
        ] {
            let mask = (1_u16 << bits) - 1;
            let init = [[mask / 7, mask / 9], [mask / 11, mask / 13]];
            let samples = (0..29)
                .map(|index| ((index * 313) as u16).wrapping_mul(19) & mask)
                .collect::<Vec<_>>();
            let source = encode_image(&samples, tree, init, 29, 1);
            for bit in (0..source.len() * 8).step_by(11) {
                let mut corrupt = source.clone();
                corrupt[bit / 8] ^= 1 << (7 - bit % 8);
                let fast =
                    decode_identity_with_kernel(&corrupt, tree, init, 29, 1, HuffmanKernel::Fused)
                        .map_err(|_| ());
                let reference = decode_identity_with_kernel(
                    &corrupt,
                    tree,
                    init,
                    29,
                    1,
                    HuffmanKernel::Reference,
                )
                .map_err(|_| ());
                assert_eq!(fast, reference, "bits={bits}, flipped bit={bit}");
            }
        }
    }

    fn decode_identity(
        source: &[u8],
        tree: NikonTreeId,
        init: [[u16; 2]; 2],
        width: usize,
        height: usize,
    ) -> Result<Vec<u16>, CodecError> {
        let bits = match tree {
            NikonTreeId::Lossless12 => 12,
            NikonTreeId::Lossless14 => 14,
        };
        decode_with_curve(
            source,
            tree,
            init,
            width,
            height,
            Arc::from(identity_curve(bits)),
        )
    }

    fn decode_with_curve(
        source: &[u8],
        tree: NikonTreeId,
        init: [[u16; 2]; 2],
        width: usize,
        height: usize,
        curve: Arc<[u16]>,
    ) -> Result<Vec<u16>, CodecError> {
        decode_with_curve_kernel(
            source,
            tree,
            init,
            width,
            height,
            curve,
            HuffmanKernel::Fused,
        )
    }

    fn decode_identity_with_kernel(
        source: &[u8],
        tree: NikonTreeId,
        init: [[u16; 2]; 2],
        width: usize,
        height: usize,
        kernel: HuffmanKernel,
    ) -> Result<Vec<u16>, CodecError> {
        let bits = match tree {
            NikonTreeId::Lossless12 => 12,
            NikonTreeId::Lossless14 => 14,
        };
        decode_with_curve_kernel(
            source,
            tree,
            init,
            width,
            height,
            Arc::from(identity_curve(bits)),
            kernel,
        )
    }

    fn decode_with_curve_kernel(
        source: &[u8],
        tree: NikonTreeId,
        init: [[u16; 2]; 2],
        width: usize,
        height: usize,
        curve: Arc<[u16]>,
        kernel: HuffmanKernel,
    ) -> Result<Vec<u16>, CodecError> {
        let bits = match tree {
            NikonTreeId::Lossless12 => 12,
            NikonTreeId::Lossless14 => 14,
        };
        let chunks = [Chunk {
            offset: 0,
            byte_count: source.len(),
        }];
        let codec = NikonCodec::Huffman {
            bits,
            tree,
            init,
            curve,
        };
        let request = DecodeRequest {
            source,
            width,
            height,
            chunk_width: width,
            chunk_height: height,
            chunks: &chunks,
            bits_per_sample: u16::from(bits),
            sample_format: SampleFormat::Integer,
            byte_order: ByteOrder::Little,
            predictor: Predictor::None,
        };
        let decoded = if matches!(&codec, NikonCodec::Huffman { .. }) {
            super::decode_huffman_with_kernel(
                &request,
                bits,
                tree,
                init,
                &request_curve(&codec),
                kernel,
            )?
        } else {
            codec.decode(&request)?
        };
        let SampleBuffer::U16(image) = decoded else {
            panic!("Nikon Huffman returned non-u16 samples");
        };
        Ok(image.into_raw())
    }

    fn request_curve(codec: &NikonCodec) -> Arc<[u16]> {
        match codec {
            NikonCodec::Huffman { curve, .. } => curve.clone(),
            _ => panic!("test helper requires a Huffman codec"),
        }
    }

    fn identity_curve(bits: u8) -> Vec<u16> {
        (0..1_u32 << bits).map(|value| value as u16).collect()
    }

    fn encode_image(
        samples: &[u16],
        tree: NikonTreeId,
        init: [[u16; 2]; 2],
        width: usize,
        height: usize,
    ) -> Vec<u8> {
        pack_bits(encode_image_bits(samples, tree, init, width, height))
    }

    fn encode_image_bits(
        samples: &[u16],
        tree: NikonTreeId,
        init: [[u16; 2]; 2],
        width: usize,
        height: usize,
    ) -> Vec<bool> {
        let codes = codes(tree);
        let mut output = Vec::new();
        let mut predictors = init;
        for row in 0..height {
            let mut left = predictors[row & 1];
            for column in 0..width {
                let expected = samples[row * width + column];
                let prediction = left[column & 1];
                let difference = i32::from(expected) - i32::from(prediction);
                write_difference(&mut output, difference, &codes);
                left[column & 1] = expected;
                if column < 2 {
                    predictors[row & 1][column] = expected;
                }
            }
        }
        output
    }

    fn encode_differences(differences: &[i32], tree: NikonTreeId) -> Vec<u8> {
        let codes = codes(tree);
        let mut output = Vec::new();
        for &difference in differences {
            write_difference(&mut output, difference, &codes);
        }
        pack_bits(output)
    }

    fn write_difference(output: &mut Vec<bool>, difference: i32, codes: &[(u8, u32, u8)]) {
        let length = if difference == 0 {
            0
        } else {
            (1_u8..=14)
                .find(|length| difference.unsigned_abs() < (1_u32 << length))
                .unwrap()
        };
        let (_, code, code_length) = codes
            .iter()
            .copied()
            .find(|(symbol, _, _)| *symbol == length)
            .unwrap();
        push_code(output, code, code_length);
        if length != 0 {
            let encoded = if difference < 0 {
                difference + ((1_i32 << length) - 1)
            } else {
                difference
            };
            push_code(output, encoded as u32, length);
        }
    }

    fn codes(id: NikonTreeId) -> Vec<(u8, u32, u8)> {
        let (counts, symbols) = tree_tables(id);
        let mut code = 0_u32;
        let mut offset = 0;
        let mut output = Vec::new();
        for (index, count) in counts.iter().copied().enumerate() {
            code <<= 1;
            for _ in 0..count {
                output.push((symbols[offset], code, index as u8 + 1));
                code += 1;
                offset += 1;
            }
        }
        output
    }

    fn push_code(output: &mut Vec<bool>, code: u32, length: u8) {
        output.extend((0..length).rev().map(|shift| code & (1 << shift) != 0));
    }

    fn pack_bits(bits: Vec<bool>) -> Vec<u8> {
        bits.chunks(8)
            .map(|chunk| {
                chunk.iter().enumerate().fold(0_u8, |byte, (index, bit)| {
                    byte | (u8::from(*bit) << (7 - index))
                })
            })
            .collect()
    }
}

#[cfg(test)]
mod packed_kernel_tests {
    use super::{unpack_row_grouped, unpack_row_grouped_big_endian, unpack_row_reference};
    use crate::decoders::nikon::request::ByteOrder;

    #[test]
    fn grouped_little_endian_matches_reference_for_partial_groups() {
        for bits in [12_u8, 14] {
            for width in [1_usize, 5, 8, 9, 17] {
                let expected = (0..width)
                    .map(|index| ((index * 613 + 17) as u16) & ((1 << bits) - 1))
                    .collect::<Vec<_>>();
                let source = pack_little_endian(&expected, bits);
                let row_bytes = (width * usize::from(bits)).div_ceil(8);
                let mut grouped = vec![0_u16; width];
                let mut reference = vec![0_u16; width];

                unpack_row_grouped(&source[..row_bytes], &mut grouped, bits).unwrap();
                unpack_row_reference(
                    &source[..row_bytes],
                    &mut reference,
                    bits,
                    ByteOrder::Little,
                )
                .unwrap();

                assert_eq!(grouped, expected, "bits={bits}, width={width}");
                assert_eq!(grouped, reference, "bits={bits}, width={width}");
            }
        }
    }

    #[test]
    fn grouped_kernel_handles_padded_non_byte_aligned_rows() {
        let expected = (0..9)
            .map(|index| ((index * 977 + 31) as u16) & 0x3fff)
            .collect::<Vec<_>>();
        let packed = pack_little_endian(&expected, 14);
        let mut source = packed.clone();
        source.resize(24, 0xaa);
        let mut actual = vec![0_u16; expected.len()];

        unpack_row_grouped(&source[..packed.len()], &mut actual, 14).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn grouped_big_endian_twelve_bit_matches_reference() {
        let expected = (0..17)
            .map(|index| ((index * 613 + 29) as u16) & 0x0fff)
            .collect::<Vec<_>>();
        let source = pack_big_endian(&expected, 12);
        let mut grouped = vec![0_u16; expected.len()];
        let mut reference = vec![0_u16; expected.len()];

        unpack_row_grouped_big_endian(&source, &mut grouped, 12).unwrap();
        unpack_row_reference(&source, &mut reference, 12, ByteOrder::Big).unwrap();

        assert_eq!(grouped, expected);
        assert_eq!(grouped, reference);
    }

    #[test]
    fn grouped_kernel_rejects_a_truncated_final_sample() {
        let mut output = vec![0_u16; 5];
        let error = unpack_row_grouped(&[0_u8; 8], &mut output, 14).unwrap_err();

        assert!(matches!(
            error,
            crate::decoders::nikon::CodecError::InvalidInput(_)
        ));
    }

    fn pack_little_endian(samples: &[u16], bits: u8) -> Vec<u8> {
        let mut output = vec![0_u8; (samples.len() * usize::from(bits)).div_ceil(8)];
        for (sample_index, sample) in samples.iter().copied().enumerate() {
            for bit in 0..bits {
                if sample & (1 << bit) != 0 {
                    let index = sample_index * usize::from(bits) + usize::from(bit);
                    output[index / 8] |= 1 << (index % 8);
                }
            }
        }
        output
    }

    fn pack_big_endian(samples: &[u16], bits: u8) -> Vec<u8> {
        let mut output = vec![0_u8; (samples.len() * usize::from(bits)).div_ceil(8)];
        let mut bit_offset = 0_usize;
        for sample in samples {
            for bit in (0..bits).rev() {
                if sample & (1 << bit) != 0 {
                    output[bit_offset / 8] |= 0x80 >> (bit_offset % 8);
                }
                bit_offset += 1;
            }
        }
        output
    }
}
