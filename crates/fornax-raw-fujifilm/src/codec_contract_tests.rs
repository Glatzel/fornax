use fornax_codec::{
    ByteOrder, Chunk, DecodeRequest, ICodecDecode, Predictor, SampleBuffer, SampleFormat,
};

use crate::FujiCfa;
use crate::codec::FujiCodec;

#[test]
fn packed_12_little_endian_reconstructs_two_pixels_per_three_bytes() {
    let source = [0x34, 0x12, 0x0a];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = DecodeRequest {
        source: &source,
        width: 2,
        height: 1,
        chunk_width: 2,
        chunk_height: 1,
        chunks: &chunks,
        bits_per_sample: 12,
        sample_format: SampleFormat::Integer,
        byte_order: ByteOrder::Little,
        predictor: Predictor::None,
    };

    let SampleBuffer::U16(image) = FujiCodec::Packed {
        bits: 12,
        byte_order: ByteOrder::Little,
    }
    .decode(&request)
    .unwrap() else {
        panic!("RAF packed samples must decode as u16");
    };
    assert_eq!(image.as_raw(), &[0x234, 0x0a1]);
}

#[test]
fn malformed_fuji_lossless_header_is_invalid_input() {
    let source = [0_u8; 32];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = DecodeRequest {
        source: &source,
        width: 768,
        height: 6,
        chunk_width: 768,
        chunk_height: 6,
        chunks: &chunks,
        bits_per_sample: 16,
        sample_format: SampleFormat::Integer,
        byte_order: ByteOrder::Big,
        predictor: Predictor::None,
    };

    let error = FujiCodec::Lossless {
        cfa: FujiCfa::Bayer([0, 1, 1, 2]),
    }
    .decode(&request)
    .unwrap_err();
    assert!(error.to_string().contains("Fujifilm"));
}

#[test]
fn fuji_lossy_compression_is_typed_unsupported() {
    let source = header(0, 0, 14);
    let error = decode_lossless(&source, FujiCfa::Bayer([0, 1, 1, 2])).unwrap_err();

    assert!(matches!(
        error,
        fornax_codec::CodecError::Unsupported(message) if message.contains("lossy")
    ));
}

#[test]
fn lossless_raw_type_must_match_the_cfa_dimensions() {
    let source = header(1, 16, 14);
    let error = decode_lossless(&source, FujiCfa::Bayer([0, 1, 1, 2])).unwrap_err();

    assert!(matches!(
        error,
        fornax_codec::CodecError::InvalidInput(message) if message.contains("CFA")
    ));
}

fn header(lossless: u8, raw_type: u8, bits: u8) -> [u8; 16] {
    let mut source = [0_u8; 16];
    source[0..2].copy_from_slice(&0x4953_u16.to_be_bytes());
    source[2] = lossless;
    source[3] = raw_type;
    source[4] = bits;
    source[5..7].copy_from_slice(&6_u16.to_be_bytes());
    source[7..9].copy_from_slice(&768_u16.to_be_bytes());
    source[9..11].copy_from_slice(&768_u16.to_be_bytes());
    source[11..13].copy_from_slice(&768_u16.to_be_bytes());
    source[13] = 1;
    source[14..16].copy_from_slice(&1_u16.to_be_bytes());
    source
}

fn decode_lossless(source: &[u8], cfa: FujiCfa) -> Result<SampleBuffer, fornax_codec::CodecError> {
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = DecodeRequest {
        source,
        width: 768,
        height: 6,
        chunk_width: 768,
        chunk_height: 6,
        chunks: &chunks,
        bits_per_sample: 16,
        sample_format: SampleFormat::Integer,
        byte_order: ByteOrder::Big,
        predictor: Predictor::None,
    };
    FujiCodec::Lossless { cfa }.decode(&request)
}
