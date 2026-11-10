#![cfg(feature = "arw")]

use fornax_codec::{
    ByteOrder, Chunk, DecodeCompression, DecodeRequest, Predictor, SampleFormat, decode,
};

fn request<'a>(
    source: &'a [u8],
    chunks: &'a [Chunk],
    compression: DecodeCompression,
) -> DecodeRequest<'a> {
    DecodeRequest {
        source,
        compression,
        width: 2,
        height: 2,
        chunk_width: 2,
        chunk_height: 2,
        chunks,
        bits_per_sample: 12,
        sample_format: SampleFormat::Integer,
        byte_order: ByteOrder::Little,
        predictor: Predictor::None,
    }
}

#[test]
fn sony_compressions_are_feature_gated_decode_variants() {
    let chunks = [];
    for compression in [
        DecodeCompression::Arw1,
        DecodeCompression::Arw2 { curve: None },
        DecodeCompression::Arw6,
        DecodeCompression::SonyLosslessJpeg,
    ] {
        assert!(decode(&request(&[], &chunks, compression)).is_err());
    }
}
