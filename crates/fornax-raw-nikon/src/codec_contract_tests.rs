use fornax_codec::{
    ByteOrder, Chunk, CodecError, DecodeRequest, ICodecDecode, Predictor, SampleBuffer,
    SampleFormat, unpack_packed_u16_rows,
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
fn unpacked_words_reject_truncated_strips() {
    let error = decode(
        &[0x23, 0x01, 0x56],
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
    .unwrap_err();
    assert!(matches!(error, CodecError::InvalidInput(_)), "{error}");
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

#[test]
fn shared_packed_row_helper_handles_padded_strides_and_partial_strips() {
    let width = 11_usize;
    let height = 7_usize;
    let chunk_height = 3_usize;
    let row_stride = 24_usize;
    let expected = (0..width * height)
        .map(|index| ((index * 977 + 31) & 0x3fff) as u16)
        .collect::<Vec<_>>();
    let payloads = (0..height.div_ceil(chunk_height))
        .map(|strip| {
            let first_row = strip * chunk_height;
            let rows = chunk_height.min(height - first_row);
            let mut payload = Vec::with_capacity(rows * row_stride);
            for row in first_row..first_row + rows {
                let packed = pack(&expected[row * width..][..width], 14, ByteOrder::Little);
                payload.extend_from_slice(&packed);
                payload.resize((row - first_row + 1) * row_stride, 0xaa);
            }
            payload
        })
        .collect::<Vec<_>>();
    let payload_refs = payloads.iter().map(Vec::as_slice).collect::<Vec<_>>();
    let mut actual = vec![0_u16; width * height];

    unpack_packed_u16_rows(
        &payload_refs,
        &mut actual,
        width,
        height,
        chunk_height,
        row_stride,
        14,
        |source, target| {
            unpack_little_endian_row(source, target, 14);
            Ok(())
        },
    )
    .unwrap();

    assert_eq!(actual, expected);
}

#[test]
fn shared_packed_row_helper_rejects_truncated_padded_strips() {
    let width = 11;
    let height = 1;
    let row_stride = 24;
    let payload = vec![0_u8; row_stride - 1];
    let mut output = vec![0_u16; width * height];

    let error = unpack_packed_u16_rows(
        &[&payload],
        &mut output,
        width,
        height,
        height,
        row_stride,
        14,
        |source, target| {
            unpack_little_endian_row(source, target, 14);
            Ok(())
        },
    )
    .unwrap_err();

    assert!(matches!(error, CodecError::InvalidInput(_)));
}

#[test]
fn packed_14_decodes_unaligned_rows_across_partial_strips() {
    let width = 11;
    let height = 7;
    let chunk_height = 3;
    let row_stride = 24;
    let expected = (0..width * height)
        .map(|index| ((index * 613 + 17) & 0x3fff) as u16)
        .collect::<Vec<_>>();
    let (source, chunks) = packed_strips(&expected, width, height, chunk_height, row_stride, 14);
    let decoded = decode_with_chunks(
        &source,
        NikonCodec::Packed {
            row_stride,
            bits: 14,
            byte_order: ByteOrder::Little,
        },
        width,
        height,
        chunk_height,
        &chunks,
        14,
        ByteOrder::Little,
    )
    .unwrap();

    assert_eq!(decoded.into_raw_u16(), Some(expected));
}

#[test]
fn packed_14_one_row_and_one_column_handle_partial_groups() {
    for (width, height, expected) in [
        (1, 1, vec![0x1234]),
        (5, 1, vec![0x0001, 0x1234, 0x2abc, 0x3fff, 0x0102]),
        (1, 3, vec![0x0123, 0x2345, 0x3fff]),
    ] {
        let row_stride = (width * 14_usize).div_ceil(8);
        let (source, chunks) = packed_strips(&expected, width, height, height, row_stride, 14);
        let decoded = decode_with_chunks(
            &source,
            NikonCodec::Packed {
                row_stride,
                bits: 14,
                byte_order: ByteOrder::Little,
            },
            width,
            height,
            height,
            &chunks,
            14,
            ByteOrder::Little,
        )
        .unwrap();

        assert_eq!(decoded.into_raw_u16(), Some(expected));
    }
}

#[test]
fn parallel_packed_rows_match_the_serial_reference() {
    let width = 4001;
    let height = 1049;
    let chunk_height = 263;
    let row_stride = (width * 14_usize).div_ceil(8).next_multiple_of(16);
    let row = (0..width)
        .map(|column| ((column * 37 + 11) & 0x3fff) as u16)
        .collect::<Vec<_>>();
    let expected = row.repeat(height);
    let (source, chunks) = packed_strips(&expected, width, height, chunk_height, row_stride, 14);
    let codec = NikonCodec::Packed {
        row_stride,
        bits: 14,
        byte_order: ByteOrder::Little,
    };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(4)
        .build()
        .unwrap();
    let parallel = pool
        .install(|| {
            decode_with_chunks(
                &source,
                codec.clone(),
                width,
                height,
                chunk_height,
                &chunks,
                14,
                ByteOrder::Little,
            )
        })
        .unwrap()
        .into_raw_u16()
        .unwrap();
    let serial = decode_packed_serial(
        &source,
        &chunks,
        width,
        height,
        chunk_height,
        row_stride,
        14,
        ByteOrder::Little,
    )
    .unwrap();

    assert_eq!(parallel, expected);
    assert_eq!(parallel, serial);
}

fn packed_strips(
    samples: &[u16],
    width: usize,
    height: usize,
    chunk_height: usize,
    row_stride: usize,
    bits: u16,
) -> (Vec<u8>, Vec<Chunk>) {
    let mut source = Vec::new();
    let mut chunks = Vec::new();
    for strip_start in (0..height).step_by(chunk_height) {
        let rows = chunk_height.min(height - strip_start);
        let offset = source.len();
        for row in strip_start..strip_start + rows {
            let packed = pack(&samples[row * width..][..width], bits, ByteOrder::Little);
            source.extend_from_slice(&packed);
            source.resize(source.len() + row_stride - packed.len(), 0xaa);
        }
        chunks.push(Chunk {
            offset,
            byte_count: source.len() - offset,
        });
    }
    (source, chunks)
}

fn decode_with_chunks(
    source: &[u8],
    codec: NikonCodec,
    width: usize,
    height: usize,
    chunk_height: usize,
    chunks: &[Chunk],
    bits: u16,
    byte_order: ByteOrder,
) -> Result<SampleBuffer, CodecError> {
    codec.decode(&DecodeRequest {
        source,
        width,
        height,
        chunk_width: width,
        chunk_height,
        chunks,
        bits_per_sample: bits,
        sample_format: SampleFormat::Integer,
        byte_order,
        predictor: Predictor::None,
    })
}

fn decode_packed_serial(
    source: &[u8],
    chunks: &[Chunk],
    width: usize,
    height: usize,
    chunk_height: usize,
    row_stride: usize,
    bits: u8,
    byte_order: ByteOrder,
) -> Result<Vec<u16>, CodecError> {
    let row_bytes = (width * usize::from(bits)).div_ceil(8);
    let mut output = vec![0_u16; width * height];
    for (strip, chunk) in chunks.iter().copied().enumerate() {
        let first_row = strip * chunk_height;
        let rows = chunk_height.min(height - first_row);
        let payload = &source[chunk.offset..chunk.offset + chunk.byte_count];
        for row in 0..rows {
            let source_row = &payload[row * row_stride..][..row_bytes];
            let target = &mut output[(first_row + row) * width..][..width];
            let mut buffer = 0_u64;
            let mut buffered = 0_u32;
            let mut bytes = source_row.iter().copied();
            for value in target {
                while buffered < u32::from(bits) {
                    buffer |= u64::from(bytes.next().unwrap()) << buffered;
                    buffered += 8;
                }
                *value = (buffer & 0x3fff) as u16;
                buffer >>= bits;
                buffered -= u32::from(bits);
            }
        }
    }
    match byte_order {
        ByteOrder::Little => Ok(output),
        ByteOrder::Big => Err(CodecError::InvalidInput(
            "test serial reference is little-endian only".into(),
        )),
    }
}

fn unpack_little_endian_row(source: &[u8], target: &mut [u16], bits: u32) {
    let mut bit = 0_usize;
    for value in target {
        let mut sample = 0_u16;
        for offset in 0..bits as usize {
            let source_bit = bit + offset;
            if source[source_bit / 8] & (1 << (source_bit % 8)) != 0 {
                sample |= 1 << offset;
            }
        }
        *value = sample;
        bit += bits as usize;
    }
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
