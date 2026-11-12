use fornax_codec::{ByteOrder, Chunk, DecodeRequest, ICodecDecode, Predictor, SampleFormat};

use crate::codec::SonyCodec;

fn request<'a>(source: &'a [u8], chunks: &'a [Chunk]) -> DecodeRequest<'a> {
    DecodeRequest {
        source,
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
fn sony_compressions_are_distinct_vendor_codecs() {
    let chunks = [];
    for codec in [
        SonyCodec::Arw1,
        SonyCodec::Arw2 { curve: None },
        SonyCodec::Arw6,
        SonyCodec::LosslessJpeg,
    ] {
        assert!(codec.decode(&request(&[], &chunks)).is_err());
    }
}
