use fornax_codec::{
    ByteOrder, Chunk, CodecError, DecodeRequest, ICodecDecode, Predictor, SampleBuffer,
    SampleFormat,
};

use crate::codec::PanasonicCodec;
use crate::{PanasonicV8Params, PanasonicV8Strip};

fn decode(
    source: &[u8],
    codec: PanasonicCodec,
    width: usize,
    height: usize,
) -> Result<SampleBuffer, CodecError> {
    decode_bits(source, codec, width, height, 12)
}

fn decode_bits(
    source: &[u8],
    codec: PanasonicCodec,
    width: usize,
    height: usize,
    bits: u16,
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
        chunk_height: height,
        chunks: &chunks,
        bits_per_sample: bits,
        sample_format: SampleFormat::Integer,
        byte_order: ByteOrder::Little,
        predictor: Predictor::None,
    })
}

fn raw_u16(buffer: SampleBuffer) -> Vec<u16> {
    buffer.into_raw_u16().expect("RW2 output must be u16")
}

#[test]
fn panasonic_v4_decodes_zero_groups_with_and_without_split_blocks() {
    let source = vec![0_u8; 0x4000];

    for split_blocks in [false, true] {
        let decoded = decode(&source, PanasonicCodec::V4 { split_blocks }, 14, 1).unwrap();
        assert_eq!(raw_u16(decoded), vec![0; 14], "split={split_blocks}");
    }
}

#[test]
fn panasonic_v4_applies_a_difference_to_the_second_phase_predictor() {
    let mut source = vec![0_u8; 64];
    write_v4_bits(&mut source, 0, 8, 1);
    write_v4_bits(&mut source, 20, 2, 0);
    write_v4_bits(&mut source, 22, 8, 1);
    assert_eq!(read_v4_bits(&source, 0, 8), 1);
    assert_eq!(read_v4_bits(&source, 20, 2), 0);
    assert_eq!(read_v4_bits(&source, 22, 8), 1);
    let mut expected = vec![0_u16; 14];
    expected[0] = 16;
    for index in (2..14).step_by(2) {
        expected[index] = 1;
    }

    let decoded = decode(
        &source,
        PanasonicCodec::V4 {
            split_blocks: false,
        },
        14,
        1,
    )
    .unwrap();

    assert_eq!(raw_u16(decoded), expected);
}

#[test]
fn panasonic_v5_decodes_ten_little_endian_packets() {
    let expected = [1_u16, 2, 3, 4, 5, 6, 7, 8, 9, 10];
    let packet = pack_lsb(&expected, 12);
    let mut source = vec![0_u8; 0x4000];
    source[0x1ff8..0x1ff8 + packet.len()].copy_from_slice(&packet);

    let decoded = decode(&source, PanasonicCodec::V5, 10, 1).unwrap();

    assert_eq!(raw_u16(decoded), expected);
}

#[test]
fn panasonic_v6_12_and_14_bit_codes_reconstruct_zero_pixels() {
    for (bits, width) in [(12_u16, 14_usize), (14, 11)] {
        let source = v6_zero_block(bits, width);
        let decoded = decode_bits(&source, PanasonicCodec::V6, width, 1, bits).unwrap();

        assert_eq!(raw_u16(decoded), vec![0; width], "bits={bits}");
    }
}

#[test]
fn panasonic_v8_uses_initial_predictors_and_places_a_strip() {
    let source = [0_u8; 1];
    let left = PanasonicV8Strip::new(0, 4, 0, 0, 2, 2).unwrap();
    let right = PanasonicV8Strip::new(0, 4, 2, 0, 2, 2).unwrap();
    let params = identity_v8_params([10, 20, 30, 40], vec![left, right]).unwrap();

    let decoded = decode_bits(&source, PanasonicCodec::V8 { params }, 4, 2, 14).unwrap();

    assert_eq!(raw_u16(decoded), vec![10, 20, 10, 20, 30, 40, 30, 40]);
}

#[test]
fn panasonic_v8_applies_nonlinear_gamma() {
    let source = [0_u8; 1];
    let strip = PanasonicV8Strip::new(0, 8, 0, 0, 4, 2).unwrap();
    let params = PanasonicV8Params::new(
        vec![0x11; 6],
        vec![
            0x0005_0000,
            0x0000_ffff,
            0x0000_ffff,
            0x0000_ffff,
            0x0000_ffff,
            0,
        ],
        0xffff,
        [10, 20, 30, 40],
        vec![(1, 0)],
        vec![0],
        vec![strip],
    )
    .unwrap();

    let decoded = decode_bits(&source, PanasonicCodec::V8 { params }, 4, 2, 14).unwrap();

    assert_eq!(raw_u16(decoded), vec![25, 45, 25, 45, 65, 85, 65, 85]);
}

#[test]
fn panasonic_unpacked_12_shifts_left_aligned_words() {
    let source = [0x30, 0x12, 0xc0, 0xab];

    let decoded = decode(&source, PanasonicCodec::Unpacked12, 2, 1).unwrap();

    assert_eq!(raw_u16(decoded), vec![0x123, 0xabc]);
}

#[test]
fn panasonic_packed_12_control_interleaves_ten_pixels_per_sixteen_bytes() {
    let expected = [
        0x123_u16, 0x456, 0x789, 0xabc, 0xdef, 0x234, 0x567, 0x89a, 0x0bc, 0xdef,
    ];
    let source = pack_control_12(&expected);

    let decoded = decode(&source, PanasonicCodec::Packed12Control, 10, 1).unwrap();

    assert_eq!(raw_u16(decoded), expected);
}

#[test]
fn panasonic_v8_constructor_rejects_mismatched_huffman_tables() {
    let strip = PanasonicV8Strip::new(0, 8, 0, 0, 2, 2).unwrap();
    let error = PanasonicV8Params::new(
        vec![65536; 6],
        vec![0; 6],
        0xffff,
        [0; 4],
        vec![(1, 0)],
        Vec::new(),
        vec![strip],
    )
    .unwrap_err();

    assert!(matches!(error, CodecError::InvalidInput(_)));
}

#[test]
fn panasonic_codecs_reject_impossible_geometry_and_malformed_tables() {
    let overflow = decode(&[0; 32], PanasonicCodec::Unpacked12, usize::MAX, 2);
    assert!(matches!(overflow, Err(CodecError::InvalidInput(_))));

    let v4_width = decode(
        &[0; 64],
        PanasonicCodec::V4 {
            split_blocks: false,
        },
        15,
        1,
    );
    assert!(matches!(v4_width, Err(CodecError::InvalidInput(_))));

    let strip = PanasonicV8Strip::new(0, 8, 0, 0, 4, 2).unwrap();
    let params = PanasonicV8Params::new(
        vec![65536; 6],
        vec![0; 6],
        0xffff,
        [0; 4],
        vec![(1, 1)],
        vec![0],
        vec![strip],
    )
    .unwrap();
    let malformed = decode_bits(&[0; 2], PanasonicCodec::V8 { params }, 4, 2, 14);
    assert!(matches!(malformed, Err(CodecError::InvalidInput(_))));
}

#[test]
fn rw2_codecs_survive_every_truncated_prefix() {
    let v4 = vec![0x5a; 64];
    sweep_prefixes(&v4, |source| {
        decode(
            source,
            PanasonicCodec::V4 {
                split_blocks: false,
            },
            14,
            1,
        )
    });

    let v5 = vec![0x5a; 0x4000];
    sweep_prefixes(&v5, |source| decode(source, PanasonicCodec::V5, 10, 1));

    let v6 = [0x5a; 16];
    sweep_prefixes(&v6, |source| decode(source, PanasonicCodec::V6, 14, 1));

    let v8_source = [0x5a; 8];
    let strip = PanasonicV8Strip::new(0, 64, 0, 0, 4, 2).unwrap();
    let params = identity_v8_params([1, 2, 3, 4], vec![strip]).unwrap();
    sweep_prefixes(&v8_source, |source| {
        decode_bits(
            source,
            PanasonicCodec::V8 {
                params: params.clone(),
            },
            4,
            2,
            14,
        )
    });

    let packed = pack_control_12(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    sweep_prefixes(&packed, |source| {
        decode(source, PanasonicCodec::Packed12Control, 10, 1)
    });

    let unpacked = [0x10, 0x20, 0x30, 0x40];
    sweep_prefixes(&unpacked, |source| {
        decode(source, PanasonicCodec::Unpacked12, 2, 1)
    });
}

#[test]
fn rw2_codecs_survive_representative_byte_flips() {
    let mut v4 = vec![0x5a; 64];
    sweep_flips(&mut v4, |source| {
        decode(
            source,
            PanasonicCodec::V4 {
                split_blocks: false,
            },
            14,
            1,
        )
    });

    let mut v5 = vec![0; 0x4000];
    v5[0x1ff8..0x2008].fill(0x5a);
    sweep_flips(&mut v5[0x1ff8..0x2008], |source| {
        let mut packet = vec![0_u8; 0x4000];
        packet[0x1ff8..0x1ff8 + source.len()].copy_from_slice(source);
        decode(&packet, PanasonicCodec::V5, 10, 1)
    });

    let mut v6 = [0x5a; 16];
    sweep_flips(&mut v6, |source| decode(source, PanasonicCodec::V6, 14, 1));

    let strip = PanasonicV8Strip::new(0, 64, 0, 0, 4, 2).unwrap();
    let params = identity_v8_params([1, 2, 3, 4], vec![strip]).unwrap();
    let mut v8 = [0x5a; 8];
    sweep_flips(&mut v8, |source| {
        decode_bits(
            source,
            PanasonicCodec::V8 {
                params: params.clone(),
            },
            4,
            2,
            14,
        )
    });

    let mut packed = pack_control_12(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]);
    sweep_flips(&mut packed, |source| {
        decode(source, PanasonicCodec::Packed12Control, 10, 1)
    });

    let mut unpacked = [0x10, 0x20, 0x30, 0x40];
    sweep_flips(&mut unpacked, |source| {
        decode(source, PanasonicCodec::Unpacked12, 2, 1)
    });
}

fn v6_zero_block(bits: u16, width: usize) -> Vec<u8> {
    let blocks = width / if bits == 12 { 14 } else { 11 };
    let mut source = vec![0_u8; blocks * 16];
    for block in source.as_chunks_mut::<16>().0 {
        let mut fields = [15_u16; 18];
        for index in [2, 6, 10, 14] {
            fields[index] = 0;
        }
        write_v6_fields(block, &fields, bits);
    }
    source
}

fn write_v6_fields(block: &mut [u8], fields: &[u16; 18], bits: u16) {
    let mut bit = 0_usize;
    let mut put = |value: u16, count: u16| {
        for index in 0..count {
            if (value >> index) & 1 != 0 {
                block[(bit + usize::from(index)) / 8] |= 1 << ((bit + usize::from(index)) % 8);
            }
        }
        bit += usize::from(count);
    };
    if bits == 12 {
        put(fields[17], 8);
        put(fields[16], 8);
        put(fields[15], 8);
        put(fields[14], 2);
        put(fields[13], 8);
        put(fields[12], 8);
        put(fields[11], 8);
        put(fields[10], 2);
        put(fields[9], 8);
        put(fields[8], 8);
        put(fields[7], 8);
        put(fields[6], 2);
        put(fields[5], 8);
        put(fields[4], 8);
        put(fields[3], 8);
        put(fields[2], 2);
        put(fields[1], 12);
        put(fields[0], 12);
    } else {
        put(0, 4);
        put(fields[13], 10);
        put(fields[12], 10);
        put(fields[11], 10);
        put(fields[10], 2);
        put(fields[9], 10);
        put(fields[8], 10);
        put(fields[7], 10);
        put(fields[6], 2);
        put(fields[5], 10);
        put(fields[4], 10);
        put(fields[3], 10);
        put(fields[2], 2);
        put(fields[1], 14);
        put(fields[0], 14);
    }
}

fn identity_v8_params(
    initial: [u16; 4],
    strips: Vec<PanasonicV8Strip>,
) -> Result<PanasonicV8Params, CodecError> {
    PanasonicV8Params::new(
        vec![65536; 6],
        vec![0; 6],
        0xffff,
        initial,
        vec![(1, 0)],
        vec![0],
        strips,
    )
}

fn pack_lsb(values: &[u16], bits: u16) -> Vec<u8> {
    let mut bytes = vec![0_u8; values.len() * usize::from(bits) / 8];
    let mut bit = 0_usize;
    for value in values {
        for index in 0..bits {
            if (value >> index) & 1 != 0 {
                bytes[(bit + usize::from(index)) / 8] |= 1 << ((bit + usize::from(index)) % 8);
            }
        }
        bit += usize::from(bits);
    }
    bytes
}

fn pack_control_12(values: &[u16]) -> Vec<u8> {
    assert_eq!(values.len(), 10);
    let mut bytes = Vec::with_capacity(16);
    for pair in values.as_chunks::<2>().0 {
        bytes.push((pair[0] & 0xff) as u8);
        bytes.push((((pair[0] >> 8) & 0x0f) | ((pair[1] & 0x0f) << 4)) as u8);
        bytes.push((pair[1] >> 4) as u8);
    }
    bytes.push(0);
    bytes
}

fn write_v4_bits(bytes: &mut [u8], consumed: usize, count: usize, value: u32) {
    let block_bits = 0x4000 * 8;
    let bit = block_bits - consumed - count;
    let byte = (bit >> 3) ^ 0x3ff0;
    let shift = bit & 7;
    let mask = (0xffff_u32 >> (16 - count)) << shift;
    let current = u32::from(u16::from_le_bytes([bytes[byte], bytes[byte + 1]]));
    let word = (current & !mask) | ((value << shift) & mask);
    bytes[byte..byte + 2].copy_from_slice(&(word as u16).to_le_bytes());
}

fn read_v4_bits(bytes: &[u8], consumed: usize, count: usize) -> u32 {
    let block_bits = 0x4000 * 8;
    let bit = block_bits - consumed - count;
    let byte = (bit >> 3) ^ 0x3ff0;
    let current = u32::from(u16::from_le_bytes([bytes[byte], bytes[byte + 1]]));
    (current >> (bit & 7)) & (0xffff_u32 >> (16 - count))
}

fn sweep_prefixes(
    source: &[u8],
    mut decode: impl FnMut(&[u8]) -> Result<SampleBuffer, CodecError>,
) {
    for end in 0..=source.len() {
        let result =
            std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| decode(&source[..end])));
        assert!(result.is_ok(), "prefix length {end} panicked");
    }
}

fn sweep_flips(
    source: &mut [u8],
    mut decode: impl FnMut(&[u8]) -> Result<SampleBuffer, CodecError>,
) {
    for index in 0..source.len().min(64) {
        source[index] ^= 0xff;
        let result = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| decode(source)));
        assert!(result.is_ok(), "byte flip at {index} panicked");
        source[index] ^= 0xff;
    }
}
