use fornax_codec::{
    ByteOrder, Chunk, CodecError, DecodeRequest, ICodecDecode, Predictor, SampleBuffer,
    SampleFormat,
};

use crate::codec::NikonCodec;

fn decode(
    source: &[u8],
    codec: NikonCodec,
    width: usize,
    height: usize,
    chunk_height: usize,
    bits: u16,
    byte_order: ByteOrder,
) -> Result<SampleBuffer, CodecError> {
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    codec.decode(&DecodeRequest {
        source,
        width,
        height,
        chunk_width: width,
        chunk_height,
        chunks: &chunks,
        bits_per_sample: bits,
        sample_format: SampleFormat::Integer,
        byte_order,
        predictor: Predictor::None,
    })
}

#[test]
fn packed_12_little_and_big_endian_decode_two_samples() {
    let little = decode(
        &[0x21, 0x43, 0x65],
        NikonCodec::Packed {
            row_stride: 3,
            bits: 12,
            byte_order: ByteOrder::Little,
        },
        2,
        1,
        1,
        12,
        ByteOrder::Little,
    )
    .unwrap();
    assert_eq!(little.into_raw_u16(), Some(vec![0x321, 0x654]));

    let big = decode(
        &[0x32, 0x16, 0x54],
        NikonCodec::Packed {
            row_stride: 3,
            bits: 12,
            byte_order: ByteOrder::Big,
        },
        2,
        1,
        1,
        12,
        ByteOrder::Big,
    )
    .unwrap();
    assert_eq!(big.into_raw_u16(), Some(vec![0x321, 0x654]));
}

#[test]
fn packed_14_little_endian_preserves_padded_row_stride() {
    let expected = [
        0x0123, 0x0456, 0x0789, 0x0abc, 0x0def, 0x0234, 0x0567, 0x089a,
    ];
    let packed = pack(&expected, 14, ByteOrder::Little);
    let mut source = Vec::new();
    for row in packed.as_chunks::<7>().0 {
        source.extend_from_slice(row);
        source.extend_from_slice(&[0xaa, 0xbb]);
    }
    let decoded = decode(
        &source,
        NikonCodec::Packed {
            row_stride: 9,
            bits: 14,
            byte_order: ByteOrder::Little,
        },
        4,
        2,
        2,
        14,
        ByteOrder::Little,
    )
    .unwrap();

    assert_eq!(decoded.into_raw_u16(), Some(expected.to_vec()));
}

#[test]
fn unpacked_words_mask_to_sample_depth() {
    let source = [0x23, 0x01, 0x56, 0x04];
    let decoded = decode(
        &source,
        NikonCodec::UnpackedWords {
            bits: 12,
            byte_order: ByteOrder::Little,
        },
        2,
        1,
        1,
        12,
        ByteOrder::Little,
    )
    .unwrap();
    assert_eq!(decoded.into_raw_u16(), Some(vec![0x123, 0x456]));

    let source = [0x7f, 0x23, 0x3f, 0x56];
    let decoded = decode(
        &source,
        NikonCodec::UnpackedWords {
            bits: 14,
            byte_order: ByteOrder::Big,
        },
        2,
        1,
        1,
        14,
        ByteOrder::Big,
    )
    .unwrap();
    assert_eq!(decoded.into_raw_u16(), Some(vec![0x3f23, 0x3f56]));
}

#[test]
fn striped_mode_five_extracts_msb_first_from_little_endian_words() {
    let source = [
        0x21, 0x43, 0x65, 0x87, 0xba, 0xdc, 0xfe, 0x10, 0x32, 0x54, 0x76, 0x98,
    ];
    let decoded = decode(
        &source,
        NikonCodec::StripedPacked12,
        8,
        1,
        1,
        12,
        ByteOrder::Little,
    )
    .unwrap();

    assert_eq!(
        decoded.into_raw_u16(),
        Some(vec![
            0x0876, 0x0543, 0x0211, 0x00fe, 0x0dcb, 0x0a98, 0x0765, 0x0432,
        ])
    );
}

#[test]
fn packed_decoder_handles_a_short_final_strip() {
    let source = [0x21, 0x43, 0x65, 0x87, 0xa9, 0xcb, 0xed, 0x0f, 0x21];
    let chunks = [
        Chunk {
            offset: 0,
            byte_count: 6,
        },
        Chunk {
            offset: 6,
            byte_count: 3,
        },
    ];
    let decoded = NikonCodec::Packed {
        row_stride: 3,
        bits: 12,
        byte_order: ByteOrder::Little,
    }
    .decode(&DecodeRequest {
        source: &source,
        width: 2,
        height: 3,
        chunk_width: 2,
        chunk_height: 2,
        chunks: &chunks,
        bits_per_sample: 12,
        sample_format: SampleFormat::Integer,
        byte_order: ByteOrder::Little,
        predictor: Predictor::None,
    })
    .unwrap();

    assert_eq!(
        decoded.into_raw_u16(),
        Some(vec![0x321, 0x654, 0x987, 0xcba, 0xfed, 0x210])
    );
}

#[test]
fn packed_decoders_match_a_naive_bit_reader_on_random_rows() {
    for bits in [12_u16, 14] {
        for byte_order in [ByteOrder::Little, ByteOrder::Big] {
            if bits == 14 && byte_order == ByteOrder::Big {
                continue;
            }
            for width in 1..=17 {
                let expected = (0..width)
                    .map(|index| {
                        ((index as u16)
                            .wrapping_mul(977)
                            .wrapping_add((bits << 5) + width as u16))
                            & ((1_u16 << bits) - 1)
                    })
                    .collect::<Vec<_>>();
                let source = pack(&expected, bits, byte_order);
                let decoded = decode(
                    &source,
                    NikonCodec::Packed {
                        row_stride: source.len(),
                        bits: bits as u8,
                        byte_order,
                    },
                    width,
                    1,
                    1,
                    bits,
                    byte_order,
                )
                .unwrap();
                assert_eq!(
                    decoded.into_raw_u16(),
                    Some(expected),
                    "bits={bits}, byte_order={byte_order:?}, width={width}"
                );
            }
        }
    }
}

#[test]
fn nef_codecs_reject_truncated_or_impossible_requests() {
    let truncated = decode(
        &[0x21],
        NikonCodec::Packed {
            row_stride: 3,
            bits: 12,
            byte_order: ByteOrder::Little,
        },
        2,
        1,
        1,
        12,
        ByteOrder::Little,
    );
    assert!(matches!(truncated, Err(CodecError::InvalidInput(_))));

    let short_stride = decode(
        &[0; 2],
        NikonCodec::Packed {
            row_stride: 2,
            bits: 12,
            byte_order: ByteOrder::Little,
        },
        2,
        1,
        1,
        12,
        ByteOrder::Little,
    );
    assert!(matches!(short_stride, Err(CodecError::InvalidInput(_))));

    let big_14 = decode(
        &[0; 4],
        NikonCodec::Packed {
            row_stride: 4,
            bits: 14,
            byte_order: ByteOrder::Big,
        },
        2,
        1,
        1,
        14,
        ByteOrder::Big,
    );
    assert!(matches!(big_14, Err(CodecError::InvalidInput(_))));

    let overflow = decode(
        &[],
        NikonCodec::Packed {
            row_stride: 1,
            bits: 12,
            byte_order: ByteOrder::Little,
        },
        usize::MAX / 2,
        4,
        1,
        12,
        ByteOrder::Little,
    );
    assert!(matches!(overflow, Err(CodecError::InvalidInput(_))));
}

fn pack(samples: &[u16], bits: u16, byte_order: ByteOrder) -> Vec<u8> {
    let mut output = vec![0_u8; (samples.len() * usize::from(bits)).div_ceil(8)];
    match byte_order {
        ByteOrder::Little => {
            for (sample_index, sample) in samples.iter().copied().enumerate() {
                for bit in 0..bits {
                    if sample & (1 << bit) != 0 {
                        let index = sample_index * usize::from(bits) + usize::from(bit);
                        output[index / 8] |= 1 << (index % 8);
                    }
                }
            }
        }
        ByteOrder::Big => {
            let mut bit_offset = 0;
            for sample in samples {
                for bit in (0..bits).rev() {
                    if sample & (1 << bit) != 0 {
                        output[bit_offset / 8] |= 1 << (7 - bit_offset % 8);
                    }
                    bit_offset += 1;
                }
            }
        }
    }
    output
}
