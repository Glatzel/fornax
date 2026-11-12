use fornax_codec::{
    ByteOrder, Chunk, CodecError, DecodeRequest, ICodecDecode, Predictor, SampleBuffer,
    SampleFormat,
};

use crate::codec::CanonCodec;
use crate::{CanonCrxCfa, CanonCrxHeader};

fn header(version: u16, levels: u8) -> CanonCrxHeader {
    CanonCrxHeader::new(version, 4, 2, 4, 2, 14, 0, levels, 0x70, 14).unwrap()
}

fn encode_top_line(values: [u32; 2]) -> u8 {
    let mut bits = Vec::new();
    bits.push(false);
    for value in [values[0], values[1] - values[0]] {
        let code = value << 1;
        bits.resize(bits.len() + usize::try_from(code).unwrap(), false);
        bits.push(true);
    }
    while bits.len() < 8 {
        bits.push(false);
    }
    bits.into_iter()
        .fold(0_u8, |byte, bit| (byte << 1) | u8::from(bit))
}

fn synthetic_sample() -> Vec<u8> {
    let mut header = Vec::new();
    header.extend_from_slice(&0xff01_u16.to_be_bytes());
    header.extend_from_slice(&8_u16.to_be_bytes());
    header.extend_from_slice(&32_u32.to_be_bytes());
    header.extend_from_slice(&0_u32.to_be_bytes());
    for plane in 0_u32..4 {
        header.extend_from_slice(&0xff02_u16.to_be_bytes());
        header.extend_from_slice(&8_u16.to_be_bytes());
        header.extend_from_slice(&8_u32.to_be_bytes());
        header.extend_from_slice(&(0x0800_0000_u32 | (plane << 28)).to_be_bytes());
        header.extend_from_slice(&0xff03_u16.to_be_bytes());
        header.extend_from_slice(&8_u16.to_be_bytes());
        header.extend_from_slice(&8_u32.to_be_bytes());
        header.extend_from_slice(&0x0020_0007_u32.to_be_bytes());
    }
    header.extend_from_slice(&0_u16.to_be_bytes());
    header.resize(0x70, 0);

    let mut sample = header;
    for _ in 0_u32..4 {
        sample.push(encode_top_line([1, 2]));
        sample.extend(std::iter::repeat_n(0, 7));
    }
    sample
}

fn try_decode_sample(header: CanonCrxHeader, sample: &[u8]) -> Result<SampleBuffer, CodecError> {
    let chunks = [Chunk {
        offset: 0,
        byte_count: sample.len(),
    }];
    let request = DecodeRequest {
        source: sample,
        width: 4,
        height: 2,
        chunk_width: 4,
        chunk_height: 2,
        chunks: &chunks,
        bits_per_sample: 14,
        sample_format: SampleFormat::Integer,
        byte_order: ByteOrder::Big,
        predictor: Predictor::None,
    };
    CanonCodec::Crx {
        header,
        cfa: CanonCrxCfa::Rggb,
    }
    .decode(&request)
}

fn decode_sample(header: CanonCrxHeader, sample: &[u8]) -> SampleBuffer {
    try_decode_sample(header, sample).unwrap()
}

#[test]
fn rejects_craw_header_before_decoding_payload() {
    let error =
        CanonCrxHeader::new(0x0200, 8352, 5586, 8352, 5586, 14, 0, 3, 0x70, 14).unwrap_err();
    assert!(matches!(error, fornax_codec::CodecError::Unsupported(_)));
}

#[test]
fn decodes_lossless_planes_into_one_mosaic() {
    let sample = synthetic_sample();
    let SampleBuffer::U16(image) = decode_sample(header(0x0100, 0), &sample) else {
        panic!("CRX must return u16 samples");
    };
    assert_eq!(image.dimensions(), (4, 2));
    assert_eq!(
        image.into_raw(),
        vec![8193, 8193, 8194, 8194, 8193, 8193, 8194, 8194]
    );
}

#[test]
fn all_bayer_placements_have_distinct_metadata_patterns() {
    assert_eq!(CanonCrxCfa::Rggb.pattern(), [0, 1, 1, 2]);
    assert_eq!(CanonCrxCfa::Grbg.pattern(), [1, 0, 2, 1]);
    assert_eq!(CanonCrxCfa::Gbrg.pattern(), [1, 2, 0, 1]);
    assert_eq!(CanonCrxCfa::Bggr.pattern(), [2, 1, 1, 0]);
}

#[test]
fn truncation_and_byte_flips_never_panic() {
    let sample = synthetic_sample();
    for length in 0..sample.len() {
        assert!(try_decode_sample(header(0x0100, 0), &sample[..length]).is_err());
    }
    for index in 0..sample.len() {
        let mut mutated = sample.clone();
        mutated[index] ^= 0x80;
        let result = std::panic::catch_unwind(|| {
            let _ = try_decode_sample(header(0x0100, 0), &mutated);
        });
        assert!(
            result.is_ok(),
            "decoder panicked after flipping byte {index}"
        );
    }
}
