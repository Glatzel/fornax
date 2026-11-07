use fornax_codec::{
    ByteOrder, Chunk, Compression, DecodeRequest, EncodeRequest, EncodedChunks, ICodecDecode,
    ICodecEncode, Predictor, SampleFormat, decode, encode,
};
use fornax_core::{MonoImage, RawMosaic};

fn request<'a>(
    image: &'a RawMosaic,
    chunk_width: usize,
    chunk_height: usize,
    predictor: Predictor,
) -> EncodeRequest<'a> {
    EncodeRequest {
        image,
        chunk_width,
        chunk_height,
        byte_order: ByteOrder::Little,
        predictor,
    }
}

fn source_and_chunks(encoded: EncodedChunks) -> (Vec<u8>, Vec<Chunk>) {
    let mut source = Vec::new();
    let mut chunks = Vec::with_capacity(encoded.chunks.len());
    for bytes in encoded.chunks {
        let offset = source.len();
        source.extend_from_slice(&bytes);
        chunks.push(Chunk {
            offset,
            byte_count: bytes.len(),
        });
    }
    (source, chunks)
}

fn decode_request<'a>(
    source: &'a [u8],
    chunks: &'a [Chunk],
    width: usize,
    height: usize,
    chunk_width: usize,
    chunk_height: usize,
    bits_per_sample: u16,
    sample_format: SampleFormat,
    predictor: Predictor,
) -> DecodeRequest<'a> {
    DecodeRequest {
        source,
        width,
        height,
        chunk_width,
        chunk_height,
        chunks,
        bits_per_sample,
        sample_format,
        byte_order: ByteOrder::Little,
        predictor,
    }
}

#[test]
fn uncompressed_u16_round_trips_partial_chunks() {
    let expected = vec![1_u16, 2, 3, 4, 5, 6];
    let image = RawMosaic::U16(MonoImage::from_raw(3, 2, expected.clone()).unwrap());
    let encoded = encode(
        Compression::Uncompressed,
        &request(&image, 2, 1, Predictor::None),
    )
    .unwrap();
    assert_eq!(encoded.chunks.len(), 4);

    let (source, chunks) = source_and_chunks(encoded);
    let request = decode_request(
        &source,
        &chunks,
        3,
        2,
        2,
        1,
        16,
        SampleFormat::Integer,
        Predictor::None,
    );
    let RawMosaic::U16(decoded) = decode(Compression::Uncompressed, &request).unwrap() else {
        panic!("u16 input must decode as u16");
    };

    assert_eq!(decoded.as_raw().as_slice(), expected.as_slice());
}

#[test]
fn uncompressed_float_round_trips_exact_bits() {
    let expected = vec![-1.5_f32, 0.25, 100.0, f32::from_bits(0x7fc0_1234)];
    let image = RawMosaic::F32(MonoImage::from_raw(2, 2, expected.clone()).unwrap());
    let encoded = encode(
        Compression::Uncompressed,
        &request(&image, 2, 2, Predictor::None),
    )
    .unwrap();
    let (source, chunks) = source_and_chunks(encoded);
    let request = decode_request(
        &source,
        &chunks,
        2,
        2,
        2,
        2,
        32,
        SampleFormat::Float,
        Predictor::None,
    );
    let RawMosaic::F32(decoded) = decode(Compression::Uncompressed, &request).unwrap() else {
        panic!("f32 input must decode as f32");
    };

    assert_eq!(
        decoded
            .as_raw()
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        expected
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>()
    );
}

#[test]
fn deflate_horizontal_predictor_round_trips() {
    let expected = vec![1_u8, 2, 4, 8, 16, 32];
    let image = RawMosaic::U8(MonoImage::from_raw(3, 2, expected.clone()).unwrap());
    let encoded = encode(
        Compression::Deflate,
        &request(&image, 3, 1, Predictor::HorizontalInteger),
    )
    .unwrap();
    let (source, chunks) = source_and_chunks(encoded);
    let request = decode_request(
        &source,
        &chunks,
        3,
        2,
        3,
        1,
        8,
        SampleFormat::Integer,
        Predictor::HorizontalInteger,
    );
    let RawMosaic::U8(decoded) = decode(Compression::Deflate, &request).unwrap() else {
        panic!("u8 input must decode as u8");
    };

    assert_eq!(decoded.as_raw().as_slice(), expected.as_slice());
}

#[test]
fn deflate_floating_point_predictor_round_trips() {
    let expected = vec![-1.5_f32, 0.25, 100.0, f32::from_bits(0x7fc0_1234)];
    let image = RawMosaic::F32(MonoImage::from_raw(2, 2, expected.clone()).unwrap());
    let encoded = encode(
        Compression::Deflate,
        &request(
            &image,
            2,
            2,
            Predictor::FloatingPoint { byte_stride: 1 },
        ),
    )
    .unwrap();
    let (source, chunks) = source_and_chunks(encoded);
    let request = decode_request(
        &source,
        &chunks,
        2,
        2,
        2,
        2,
        32,
        SampleFormat::Float,
        Predictor::FloatingPoint { byte_stride: 1 },
    );
    let RawMosaic::F32(decoded) = decode(Compression::Deflate, &request).unwrap() else {
        panic!("f32 input must decode as f32");
    };

    assert_eq!(
        decoded
            .as_raw()
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>(),
        expected
            .iter()
            .map(|value| value.to_bits())
            .collect::<Vec<_>>()
    );
}

#[test]
fn lossless_jpeg_round_trips_u8_and_u16() {
    for (image, bits, format) in [
        (
            RawMosaic::U8(MonoImage::from_raw(4, 2, vec![0, 255, 128, 1, 254, 64, 200, 7]).unwrap()),
            8,
            SampleFormat::Integer,
        ),
        (
            RawMosaic::U16(
                MonoImage::from_raw(4, 2, vec![0, 65_535, 30_000, 32_768, 1, 2, 40_000, 12_345])
                    .unwrap(),
            ),
            16,
            SampleFormat::Integer,
        ),
    ] {
        let encoded = encode(
            Compression::LosslessJpeg,
            &request(&image, 4, 2, Predictor::None),
        )
        .unwrap();
        let (source, chunks) = source_and_chunks(encoded);
        let request = decode_request(
            &source,
            &chunks,
            4,
            2,
            4,
            2,
            bits,
            format,
            Predictor::None,
        );
        assert_eq!(
            decode(Compression::LosslessJpeg, &request).unwrap(),
            image
        );
    }
}

#[cfg(feature = "zstd")]
#[test]
fn zstd_round_trips_integer_and_float_chunks() {
    for (image, bits, format) in [
        (
            RawMosaic::U16(MonoImage::from_raw(3, 1, vec![0, 30_000, 65_535]).unwrap()),
            16,
            SampleFormat::Integer,
        ),
        (
            RawMosaic::F32(MonoImage::from_raw(3, 1, vec![-1.5, 0.25, 100.0]).unwrap()),
            32,
            SampleFormat::Float,
        ),
    ] {
        let encoded = encode(
            Compression::Zstd,
            &request(&image, 3, 1, Predictor::None),
        )
        .unwrap();
        let (source, chunks) = source_and_chunks(encoded);
        let request = decode_request(
            &source,
            &chunks,
            3,
            1,
            3,
            1,
            bits,
            format,
            Predictor::None,
        );
        let decoded = decode(Compression::Zstd, &request).unwrap();
        match (&image, &decoded) {
            (RawMosaic::F32(expected), RawMosaic::F32(decoded)) => assert_eq!(
                expected
                    .as_raw()
                    .iter()
                    .map(|value| value.to_bits())
                    .collect::<Vec<_>>(),
                decoded
                    .as_raw()
                    .iter()
                    .map(|value| value.to_bits())
                    .collect::<Vec<_>>()
            ),
            _ => assert_eq!(image, decoded),
        }
    }
}

#[test]
fn decode_only_compression_rejects_encoding() {
    let image = RawMosaic::U8(MonoImage::from_raw(1, 1, vec![0]).unwrap());
    let error =
        encode(Compression::LossyJpeg, &request(&image, 1, 1, Predictor::None)).unwrap_err();

    assert!(matches!(error, fornax_codec::CodecError::Unsupported(_)));
}
