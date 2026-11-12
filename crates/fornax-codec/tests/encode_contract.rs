use fornax_codec::{
    ByteOrder, Chunk, DecodeCompression, DecodeRequest, EncodeCompression, EncodeRequest,
    EncodedChunks, ICodecDecode, Predictor, SampleBuffer, SampleFormat, encode,
};

const fn request(
    image: &SampleBuffer,
    chunk_width: usize,
    chunk_height: usize,
    predictor: Predictor,
    compression: EncodeCompression,
) -> EncodeRequest<'_> {
    EncodeRequest {
        image,
        compression,
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

fn assert_sample_buffer_eq(actual: &SampleBuffer, expected: &SampleBuffer) {
    match (actual, expected) {
        (SampleBuffer::F32(actual), SampleBuffer::F32(expected)) => assert_eq!(
            actual
                .as_raw()
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            expected
                .as_raw()
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>()
        ),
        (SampleBuffer::F64(actual), SampleBuffer::F64(expected)) => assert_eq!(
            actual
                .as_raw()
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            expected
                .as_raw()
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>()
        ),
        _ => assert_eq!(actual, expected),
    }
}

struct TestRequest<'a> {
    compression: DecodeCompression,
    request: DecodeRequest<'a>,
}

impl TestRequest<'_> {
    fn decode(&self) -> Result<SampleBuffer, fornax_codec::CodecError> {
        self.compression.decode(&self.request)
    }
}

const fn decode_request<'a>(
    source: &'a [u8],
    chunks: &'a [Chunk],
    width: usize,
    height: usize,
    chunk_width: usize,
    chunk_height: usize,
    bits_per_sample: u16,
    sample_format: SampleFormat,
    predictor: Predictor,
    compression: DecodeCompression,
) -> TestRequest<'a> {
    TestRequest {
        compression,
        request: DecodeRequest {
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
        },
    }
}

#[test]
fn uncompressed_u16_round_trips_partial_chunks() {
    let expected = vec![1_u16, 2, 3, 4, 5, 6];
    let image = SampleBuffer::from_u16(3, 2, expected.clone()).unwrap();
    let encoded = encode(&request(
        &image,
        2,
        1,
        Predictor::None,
        EncodeCompression::Uncompressed,
    ))
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
        DecodeCompression::Uncompressed,
    );
    let SampleBuffer::U16(decoded) = request.decode().unwrap() else {
        panic!("u16 input must decode as u16");
    };

    assert_eq!(decoded.as_raw().as_slice(), expected.as_slice());
}

#[test]
fn uncompressed_full_image_returns_one_exact_chunk() {
    let image = SampleBuffer::from_u16(2, 2, vec![0x0102, 0x0304, 0xabcd, 0xff00]).unwrap();
    let mut request = request(
        &image,
        2,
        2,
        Predictor::None,
        EncodeCompression::Uncompressed,
    );
    request.byte_order = ByteOrder::Big;

    let encoded = encode(&request).unwrap();

    assert_eq!(
        encoded.chunks,
        vec![vec![0x01, 0x02, 0x03, 0x04, 0xab, 0xcd, 0xff, 0x00]]
    );
}

#[test]
fn uncompressed_float_round_trips_exact_bits() {
    let expected = vec![-1.5_f32, 0.25, 100.0, f32::from_bits(0x7fc0_1234)];
    let image = SampleBuffer::from_f32(2, 2, expected.clone()).unwrap();
    let encoded = encode(&request(
        &image,
        2,
        2,
        Predictor::None,
        EncodeCompression::Uncompressed,
    ))
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
        DecodeCompression::Uncompressed,
    );
    let SampleBuffer::F32(decoded) = request.decode().unwrap() else {
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
fn uncompressed_f64_round_trips_exact_bits() {
    let expected = vec![-1.5_f64, 0.25, -0.0, f64::from_bits(0x7ff8_0000_0000_1234)];
    let image = SampleBuffer::from_f64(2, 2, expected).unwrap();
    let encoded = encode(&request(
        &image,
        2,
        2,
        Predictor::None,
        EncodeCompression::Uncompressed,
    ))
    .unwrap();
    let (source, chunks) = source_and_chunks(encoded);
    let request = decode_request(
        &source,
        &chunks,
        2,
        2,
        2,
        2,
        64,
        SampleFormat::Float,
        Predictor::None,
        DecodeCompression::Uncompressed,
    );

    assert_sample_buffer_eq(&request.decode().unwrap(), &image);
}

#[test]
#[cfg(feature = "packbits")]
fn packbits_float64_predictor_round_trips() {
    let expected = vec![-1.5_f64, 0.25, -0.0, f64::from_bits(0x7ff8_0000_0000_1234)];
    let image = SampleBuffer::from_f64(2, 2, expected).unwrap();
    let predictor = Predictor::FloatingPoint { byte_stride: 1 };
    let encoded = encode(&request(
        &image,
        2,
        2,
        predictor,
        EncodeCompression::PackBits,
    ))
    .unwrap();
    let (source, chunks) = source_and_chunks(encoded);
    let request = decode_request(
        &source,
        &chunks,
        2,
        2,
        2,
        2,
        64,
        SampleFormat::Float,
        predictor,
        DecodeCompression::PackBits,
    );

    assert_sample_buffer_eq(&request.decode().unwrap(), &image);
}

#[test]
#[cfg(feature = "deflate")]
fn deflate_horizontal_predictor_round_trips() {
    let expected = vec![1_u8, 2, 4, 8, 16, 32];
    let image = SampleBuffer::from_u8(3, 2, expected.clone()).unwrap();
    let encoded = encode(&request(
        &image,
        3,
        1,
        Predictor::HorizontalInteger,
        EncodeCompression::Deflate,
    ))
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
        DecodeCompression::Deflate,
    );
    let SampleBuffer::U8(decoded) = request.decode().unwrap() else {
        panic!("u8 input must decode as u8");
    };

    assert_eq!(decoded.as_raw().as_slice(), expected.as_slice());
}

#[test]
#[cfg(feature = "deflate")]
fn deflate_floating_point_predictor_round_trips() {
    let expected = vec![-1.5_f32, 0.25, 100.0, f32::from_bits(0x7fc0_1234)];
    let image = SampleBuffer::from_f32(2, 2, expected.clone()).unwrap();
    let encoded = encode(&request(
        &image,
        2,
        2,
        Predictor::FloatingPoint { byte_stride: 1 },
        EncodeCompression::Deflate,
    ))
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
        DecodeCompression::Deflate,
    );
    let SampleBuffer::F32(decoded) = request.decode().unwrap() else {
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
#[cfg(feature = "deflate")]
fn deflate_float64_predictor_round_trips() {
    let expected = vec![-1.5_f64, 0.25, -0.0, f64::from_bits(0x7ff8_0000_0000_1234)];
    let image = SampleBuffer::from_f64(2, 2, expected).unwrap();
    let predictor = Predictor::FloatingPoint { byte_stride: 1 };
    let encoded = encode(&request(
        &image,
        2,
        2,
        predictor,
        EncodeCompression::Deflate,
    ))
    .unwrap();
    let (source, chunks) = source_and_chunks(encoded);
    let request = decode_request(
        &source,
        &chunks,
        2,
        2,
        2,
        2,
        64,
        SampleFormat::Float,
        predictor,
        DecodeCompression::Deflate,
    );

    assert_sample_buffer_eq(&request.decode().unwrap(), &image);
}

#[test]
#[cfg(feature = "lossless-jpeg")]
fn lossless_jpeg_round_trips_u8_and_u16() {
    for (image, bits, format) in [
        (
            SampleBuffer::from_u8(4, 2, vec![0, 255, 128, 1, 254, 64, 200, 7]).unwrap(),
            8,
            SampleFormat::Integer,
        ),
        (
            SampleBuffer::from_u16(4, 2, vec![0, 65_535, 30_000, 32_768, 1, 2, 40_000, 12_345])
                .unwrap(),
            16,
            SampleFormat::Integer,
        ),
    ] {
        let encoded = encode(&request(
            &image,
            4,
            2,
            Predictor::None,
            EncodeCompression::LosslessJpeg,
        ))
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
            DecodeCompression::LosslessJpeg,
        );
        assert_sample_buffer_eq(&request.decode().unwrap(), &image);
    }
}

#[test]
#[cfg(feature = "lossless-jpeg")]
fn lossless_jpeg_round_trips_partial_edge_tiles() {
    for (image, bits, format) in [
        (
            SampleBuffer::from_u8(5, 3, (0_u8..15).collect()).unwrap(),
            8,
            SampleFormat::Integer,
        ),
        (
            SampleBuffer::from_u16(5, 3, (0_u16..15).map(|value| value * 1_000).collect()).unwrap(),
            16,
            SampleFormat::Integer,
        ),
    ] {
        let encoded = encode(&request(
            &image,
            4,
            2,
            Predictor::None,
            EncodeCompression::LosslessJpeg,
        ))
        .unwrap();
        assert_eq!(encoded.chunks.len(), 4);

        let (source, chunks) = source_and_chunks(encoded);
        let request = decode_request(
            &source,
            &chunks,
            5,
            3,
            4,
            2,
            bits,
            format,
            Predictor::None,
            DecodeCompression::LosslessJpeg,
        );
        assert_sample_buffer_eq(&request.decode().unwrap(), &image);
    }
}

#[test]
#[cfg(feature = "zstd")]
fn zstd_round_trips_integer_and_float_chunks() {
    for (image, bits, format) in [
        (
            SampleBuffer::from_u16(3, 1, vec![0, 30_000, 65_535]).unwrap(),
            16,
            SampleFormat::Integer,
        ),
        (
            SampleBuffer::from_f32(3, 1, vec![-1.5, 0.25, 100.0]).unwrap(),
            32,
            SampleFormat::Float,
        ),
        (
            SampleBuffer::from_f64(3, 1, vec![-1.5, 0.25, 100.0]).unwrap(),
            64,
            SampleFormat::Float,
        ),
    ] {
        let encoded = encode(&request(
            &image,
            3,
            1,
            Predictor::None,
            EncodeCompression::Zstd { level: 3 },
        ))
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
            DecodeCompression::Zstd,
        );
        assert_sample_buffer_eq(&request.decode().unwrap(), &image);
    }
}

#[test]
fn uncompressed_round_trips_every_sample_type() {
    let images = [
        SampleBuffer::from_u8(2, 1, vec![0, 255]).unwrap(),
        SampleBuffer::from_u16(2, 1, vec![0, 65_535]).unwrap(),
        SampleBuffer::from_u32(2, 1, vec![0, 4_000_000_000]).unwrap(),
        SampleBuffer::from_f32(2, 1, vec![-1.5, f32::from_bits(0x7fc0_1234)]).unwrap(),
        SampleBuffer::from_f64(2, 1, vec![-1.5, f64::from_bits(0x7ff8_0000_0000_1234)]).unwrap(),
    ];

    for image in images {
        let encoded = encode(&request(
            &image,
            2,
            1,
            Predictor::None,
            EncodeCompression::Uncompressed,
        ))
        .unwrap();
        let (source, chunks) = source_and_chunks(encoded);
        let (bits, sample_format) = match &image {
            SampleBuffer::U8(_) => (8, SampleFormat::Integer),
            SampleBuffer::U16(_) => (16, SampleFormat::Integer),
            SampleBuffer::U32(_) => (32, SampleFormat::Integer),
            SampleBuffer::F32(_) => (32, SampleFormat::Float),
            SampleBuffer::F64(_) => (64, SampleFormat::Float),
            _ => unreachable!(),
        };
        let request = decode_request(
            &source,
            &chunks,
            2,
            1,
            2,
            1,
            bits,
            sample_format,
            Predictor::None,
            DecodeCompression::Uncompressed,
        );
        assert_sample_buffer_eq(&request.decode().unwrap(), &image);
    }
}
