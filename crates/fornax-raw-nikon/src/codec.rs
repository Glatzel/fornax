use std::sync::Arc;

use fornax_codec::ICodecDecode;

use crate::error::{invalid, unsupported};
use crate::huffman::{BitReader, HuffmanTable};
use crate::request::{ByteOrder, DecodeRequest};
use crate::samples::{buffer_u16, chunk};
use crate::{CodecError, NikonTreeId, SampleBuffer, SampleFormat};

// Numeric format data only. Provenance, licensing, source agreement, and the
// lossless-only scope are documented in docs/reference/decoders/nikon-nef.md.
const LOSSLESS_12_COUNTS: [u8; 16] = [0, 1, 4, 2, 3, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0, 0];
const LOSSLESS_12_SYMBOLS: [u8; 13] = [5, 4, 6, 3, 7, 2, 8, 1, 9, 0, 10, 11, 12];
const LOSSLESS_14_COUNTS: [u8; 16] = [0, 1, 4, 2, 2, 3, 1, 2, 0, 0, 0, 0, 0, 0, 0, 0];
const LOSSLESS_14_SYMBOLS: [u8; 15] = [7, 6, 8, 5, 9, 4, 10, 3, 11, 12, 2, 0, 1, 13, 14];

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
    let mut reader = BitReader::new(payload);
    let mut output = vec![0_u16; sample_count];
    let mut vertical = init.map(|values| values.map(i32::from));

    for row in 0..request.height {
        let parity = row & 1;
        let mut horizontal = vertical[parity];
        for column in 0..request.width {
            let difference = decode_difference(&table, &mut reader, bits)?;
            let phase = column & 1;
            let predictor = {
                let base = if column < 2 {
                    &mut vertical[parity][column]
                } else {
                    &mut horizontal[phase]
                };
                *base = base
                    .checked_add(difference)
                    .ok_or_else(|| invalid("Nikon Huffman predictor overflow"))?;
                *base
            };
            horizontal[phase] = predictor;
            let index = usize::try_from(predictor)
                .map_err(|_| invalid("Nikon Huffman curve index is negative"))?;
            let mapped = curve.get(index).ok_or_else(|| {
                invalid(format!(
                    "Nikon Huffman curve index {index} exceeds {} entries",
                    curve.len()
                ))
            })?;
            output[row * request.width + column] = *mapped;
        }
    }

    buffer_u16(request.width, request.height, output)
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

    for (tile, strip) in request.chunks.iter().copied().enumerate() {
        let start_row = tile
            .checked_mul(request.chunk_height)
            .ok_or_else(|| invalid("Nikon packed row offset overflow"))?;
        let rows = rows[tile];
        let payload = chunk(request.source, strip)?;
        for row in 0..rows {
            let source = &payload[row * row_stride..][..row_bytes];
            let target = &mut output[(start_row + row) * request.width..][..request.width];
            unpack_row(source, target, bits, byte_order)?;
        }
    }
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
    let mut output = vec![0_u16; sample_count];
    let mask = (1_u16 << bits) - 1;

    for (tile, strip) in request.chunks.iter().copied().enumerate() {
        let start_row = tile
            .checked_mul(request.chunk_height)
            .ok_or_else(|| invalid("Nikon unpacked row offset overflow"))?;
        let payload = chunk(request.source, strip)?;
        for row in 0..rows[tile] {
            let source = &payload[row * request.width * 2..][..request.width * 2];
            let target = &mut output[(start_row + row) * request.width..][..request.width];
            for (target, bytes) in target.iter_mut().zip(source.as_chunks::<2>().0) {
                *target = match byte_order {
                    ByteOrder::Little => u16::from_le_bytes(*bytes),
                    ByteOrder::Big => u16::from_be_bytes(*bytes),
                } & mask;
            }
        }
    }
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

    use super::tree_tables;
    use crate::NikonTreeId;
    use crate::codec::NikonCodec;
    use crate::huffman::HuffmanTable;

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
        let SampleBuffer::U16(image) = codec.decode(&request)? else {
            panic!("Nikon Huffman returned non-u16 samples");
        };
        Ok(image.into_raw())
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
