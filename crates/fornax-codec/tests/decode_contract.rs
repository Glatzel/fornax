use fornax_codec::{
    ByteOrder, Chunk, DecodeRequest, ICodecDecode, SampleFormat, UncompressedCodec,
};
use fornax_core::RawMosaic;

fn request<'a>(
    source: &'a [u8],
    chunks: &'a [Chunk],
    width: usize,
    height: usize,
    chunk_width: usize,
    chunk_height: usize,
    bits_per_sample: u16,
    sample_format: SampleFormat,
    byte_order: ByteOrder,
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
        byte_order,
        predictor: fornax_codec::Predictor::None,
    }
}

#[test]
fn uncompressed_ten_bit_rows_unpack_msb_first() {
    let source = [0_u8, 0, 0x10, 0x08, 0x03];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request(
        &source,
        &chunks,
        4,
        1,
        4,
        1,
        10,
        SampleFormat::Integer,
        ByteOrder::Little,
    );

    let RawMosaic::U16(image) = UncompressedCodec.decode(&request).unwrap() else {
        panic!("ten-bit samples must widen to u16");
    };

    assert_eq!(image.as_raw().as_slice(), &[0, 1, 2, 3]);
}

#[test]
fn uncompressed_big_endian_partial_strip_copies_the_last_row() {
    let source = [b'M', b'M', 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6];
    let chunks = [
        Chunk {
            offset: 2,
            byte_count: 8,
        },
        Chunk {
            offset: 10,
            byte_count: 4,
        },
    ];
    let request = request(
        &source,
        &chunks,
        2,
        3,
        2,
        2,
        16,
        SampleFormat::Integer,
        ByteOrder::Big,
    );

    let RawMosaic::U16(image) = UncompressedCodec.decode(&request).unwrap() else {
        panic!("sixteen-bit samples must decode as u16");
    };

    assert_eq!(image.as_raw().as_slice(), &[1, 2, 3, 4, 5, 6]);
}

#[test]
fn uncompressed_float16_widens_to_f32() {
    let source = [0x3c, 0x00, 0x40, 0x00, 0x38, 0x00, 0xbc, 0x00];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request(
        &source,
        &chunks,
        4,
        1,
        4,
        1,
        16,
        SampleFormat::Float,
        ByteOrder::Big,
    );

    let RawMosaic::F32(image) = UncompressedCodec.decode(&request).unwrap() else {
        panic!("float16 samples must widen to f32");
    };

    assert_eq!(image.as_raw().as_slice(), &[1.0, 2.0, 0.5, -1.0]);
}

#[test]
fn uncompressed_rejects_missing_chunks() {
    let source = [0_u8; 4];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request(
        &source,
        &chunks,
        2,
        2,
        2,
        1,
        8,
        SampleFormat::Integer,
        ByteOrder::Little,
    );

    let error = UncompressedCodec.decode(&request).unwrap_err();

    assert!(matches!(error, fornax_codec::CodecError::InvalidInput(_)));
}

#[test]
fn uncompressed_rejects_overflowing_chunk_ranges_without_panicking() {
    let source = [0_u8; 4];
    let chunks = [Chunk {
        offset: usize::MAX,
        byte_count: 1,
    }];
    let request = request(
        &source,
        &chunks,
        2,
        1,
        2,
        1,
        8,
        SampleFormat::Integer,
        ByteOrder::Little,
    );

    let error = UncompressedCodec.decode(&request).unwrap_err();

    assert!(matches!(error, fornax_codec::CodecError::InvalidInput(_)));
}
