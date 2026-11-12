#[cfg(feature = "deflate")]
use std::io::Write;

#[cfg(feature = "deflate")]
use flate2::Compression as FlateCompression;
#[cfg(feature = "deflate")]
use flate2::write::ZlibEncoder;
use fornax_codec::{
    ByteOrder, Chunk, DecodeCompression, DecodeRequest, Predictor, SampleBuffer, SampleFormat,
    decode,
};

const fn request<'a>(
    source: &'a [u8],
    chunks: &'a [Chunk],
    width: usize,
    height: usize,
    chunk_width: usize,
    chunk_height: usize,
    bits_per_sample: u16,
    sample_format: SampleFormat,
    byte_order: ByteOrder,
    compression: DecodeCompression,
) -> DecodeRequest<'a> {
    DecodeRequest {
        source,
        compression,
        width,
        height,
        chunk_width,
        chunk_height,
        chunks,
        bits_per_sample,
        sample_format,
        byte_order,
        predictor: Predictor::None,
    }
}

const fn request_with_predictor<'a>(
    source: &'a [u8],
    chunks: &'a [Chunk],
    width: usize,
    height: usize,
    chunk_width: usize,
    chunk_height: usize,
    bits_per_sample: u16,
    sample_format: SampleFormat,
    byte_order: ByteOrder,
    predictor: Predictor,
    compression: DecodeCompression,
) -> DecodeRequest<'a> {
    DecodeRequest {
        source,
        compression,
        width,
        height,
        chunk_width,
        chunk_height,
        chunks,
        bits_per_sample,
        sample_format,
        byte_order,
        predictor,
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
        DecodeCompression::Uncompressed,
    );

    let SampleBuffer::U16(image) = decode(&request).unwrap() else {
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
        DecodeCompression::Uncompressed,
    );

    let SampleBuffer::U16(image) = decode(&request).unwrap() else {
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
        DecodeCompression::Uncompressed,
    );

    let SampleBuffer::F32(image) = decode(&request).unwrap() else {
        panic!("float16 samples must widen to f32");
    };

    assert_eq!(image.as_raw().as_slice(), &[1.0, 2.0, 0.5, -1.0]);
}

#[test]
fn uncompressed_float64_preserves_exact_bits() {
    let expected = [1.0_f64, -0.0, f64::from_bits(0x7ff8_0000_0000_1234)];
    let source = expected
        .iter()
        .flat_map(|value| value.to_le_bytes())
        .collect::<Vec<_>>();
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request(
        &source,
        &chunks,
        3,
        1,
        3,
        1,
        64,
        SampleFormat::Float,
        ByteOrder::Little,
        DecodeCompression::Uncompressed,
    );

    let SampleBuffer::F64(image) = decode(&request).unwrap() else {
        panic!("64-bit float samples must decode as f64");
    };

    assert_eq!(
        image
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
        DecodeCompression::Uncompressed,
    );

    let error = decode(&request).unwrap_err();

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
        DecodeCompression::Uncompressed,
    );

    let error = decode(&request).unwrap_err();

    assert!(matches!(error, fornax_codec::CodecError::InvalidInput(_)));
}

#[test]
fn uncompressed_rejects_overflowing_chunk_geometry_without_panicking() {
    let source = [];
    let chunks = [Chunk {
        offset: 0,
        byte_count: 0,
    }];
    let request = request(
        &source,
        &chunks,
        1,
        1,
        usize::MAX,
        2,
        10,
        SampleFormat::Integer,
        ByteOrder::Little,
        DecodeCompression::Uncompressed,
    );

    let error = decode(&request).unwrap_err();

    assert!(matches!(error, fornax_codec::CodecError::InvalidInput(_)));
}

#[test]
fn uncompressed_rejects_predictors_it_cannot_reverse() {
    let source = [1_u8, 2, 4, 8];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request_with_predictor(
        &source,
        &chunks,
        4,
        1,
        4,
        1,
        8,
        SampleFormat::Integer,
        ByteOrder::Little,
        Predictor::HorizontalInteger,
        DecodeCompression::Uncompressed,
    );

    let error = decode(&request).unwrap_err();

    assert!(matches!(error, fornax_codec::CodecError::InvalidInput(_)));
}

#[test]
#[cfg(feature = "deflate")]
fn deflate_reverses_horizontal_integer_prediction() {
    let differences = [1_u8, 1, 2, 4];
    let mut encoder = ZlibEncoder::new(Vec::new(), FlateCompression::fast());
    encoder.write_all(&differences).unwrap();
    let encoded = encoder.finish().unwrap();
    let chunks = [Chunk {
        offset: 0,
        byte_count: encoded.len(),
    }];
    let request = request_with_predictor(
        &encoded,
        &chunks,
        4,
        1,
        4,
        1,
        8,
        SampleFormat::Integer,
        ByteOrder::Little,
        Predictor::HorizontalInteger,
        DecodeCompression::Deflate,
    );

    let SampleBuffer::U8(image) = decode(&request).unwrap() else {
        panic!("8-bit Deflate samples must decode as u8");
    };

    assert_eq!(image.as_raw().as_slice(), &[1, 2, 4, 8]);
}

#[test]
#[cfg(feature = "lossless-jpeg")]
fn lossless_jpeg_decodes_twelve_bit_tiles() {
    let tile = [
        0xff, 0xd8, 0xff, 0xc3, 0x00, 0x0b, 0x0c, 0x00, 0x02, 0x00, 0x02, 0x01, 0x01, 0x11, 0x00,
        0xff, 0xc4, 0x00, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xda, 0x00, 0x08, 0x01, 0x01, 0x00, 0x01,
        0x00, 0x00, 0x00, 0xff, 0xd9,
    ];
    let chunks = [Chunk {
        offset: 0,
        byte_count: tile.len(),
    }];
    let request = request(
        &tile,
        &chunks,
        2,
        2,
        2,
        2,
        12,
        SampleFormat::Integer,
        ByteOrder::Little,
        DecodeCompression::LosslessJpeg,
    );

    let SampleBuffer::U16(image) = decode(&request).unwrap() else {
        panic!("lossless JPEG samples must decode as u16");
    };

    assert_eq!(image.as_raw().as_slice(), &[2_048; 4]);
}

#[test]
#[cfg(feature = "lossless-jpeg")]
fn lossless_jpeg_applies_point_transform() {
    let mut tile = [
        0xff, 0xd8, 0xff, 0xc3, 0x00, 0x0b, 0x10, 0x00, 0x02, 0x00, 0x02, 0x01, 0x01, 0x11, 0x00,
        0xff, 0xc4, 0x00, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xda, 0x00, 0x08, 0x01, 0x01, 0x00, 0x01,
        0x00, 0x00, 0x00, 0xff, 0xd9,
    ];
    tile[46] = 1;
    let chunks = [Chunk {
        offset: 0,
        byte_count: tile.len(),
    }];
    let request = request(
        &tile,
        &chunks,
        2,
        2,
        2,
        2,
        16,
        SampleFormat::Integer,
        ByteOrder::Little,
        DecodeCompression::LosslessJpeg,
    );

    let SampleBuffer::U16(image) = decode(&request).unwrap() else {
        panic!("lossless JPEG samples must decode as u16");
    };

    assert_eq!(image.as_raw().as_slice(), &[32_768; 4]);
}

#[test]
#[cfg(feature = "hasselblad")]
fn hasselblad_lossless_jpeg_reads_pair_lengths_before_differences() {
    let tile = hasselblad_tile(&[0x00, 0x00, 0x94, 0x94]);
    let chunks = [Chunk {
        offset: 0,
        byte_count: tile.len(),
    }];
    let request = request(
        &tile,
        &chunks,
        4,
        2,
        4,
        2,
        16,
        SampleFormat::Integer,
        ByteOrder::Little,
        DecodeCompression::HasselbladLosslessJpeg,
    );

    let SampleBuffer::U16(image) = decode(&request).unwrap() else {
        panic!("Hasselblad lossless JPEG samples must decode as u16");
    };

    assert_eq!(
        image.as_raw().as_slice(),
        &[
            32_769, 32_768, 32_769, 32_767, 32_769, 32_768, 32_769, 32_767
        ]
    );
}

#[test]
#[cfg(feature = "hasselblad")]
fn hasselblad_lossless_jpeg_rejects_restart_intervals() {
    let tile = hasselblad_tile_with(1, 8, 0, Some(1), &[0x94, 0x94]);

    assert_hasselblad_rejected(&tile, "restart intervals");
}

#[test]
#[cfg(feature = "hasselblad")]
fn hasselblad_lossless_jpeg_rejects_point_transform() {
    let tile = hasselblad_tile_with(1, 8, 1, None, &[0x94, 0x94]);

    assert_hasselblad_rejected(&tile, "point transform");
}

#[test]
#[cfg(feature = "hasselblad")]
fn hasselblad_lossless_jpeg_rejects_other_predictors() {
    let tile = hasselblad_tile_with(1, 7, 0, None, &[0x94, 0x94]);

    assert_hasselblad_rejected(&tile, "predictor 8");
}

#[test]
#[cfg(feature = "hasselblad")]
fn hasselblad_lossless_jpeg_rejects_multiple_components() {
    let tile = hasselblad_tile_with(2, 8, 0, None, &[0x94, 0x94]);

    assert_hasselblad_rejected(&tile, "one-component");
}

#[test]
#[cfg(feature = "hasselblad")]
fn hasselblad_lossless_jpeg_rejects_missing_huffman_table() {
    let tile = hasselblad_tile_without_dht(&[0x94, 0x94]);

    assert_hasselblad_rejected(&tile, "Huffman table");
}

#[test]
#[cfg(feature = "hasselblad")]
fn hasselblad_lossless_jpeg_rejects_truncated_entropy() {
    let tile = hasselblad_tile(&[]);

    assert_hasselblad_rejected(&tile, "entropy");
}

#[test]
#[cfg(feature = "hasselblad")]
fn hasselblad_lossless_jpeg_rejects_entropy_truncated_after_a_row() {
    let tile = hasselblad_tile(&[0x00, 0x00, 0x94]);

    assert_hasselblad_rejected(&tile, "lossless JPEG");
}

#[test]
#[cfg(feature = "hasselblad")]
fn hasselblad_lossless_jpeg_consumes_the_dng_symbol_sixteen_payload() {
    let tile = hasselblad_symbol_sixteen_tile();
    let chunks = [Chunk {
        offset: 0,
        byte_count: tile.len(),
    }];
    let request = request(
        &tile,
        &chunks,
        2,
        2,
        2,
        2,
        16,
        SampleFormat::Integer,
        ByteOrder::Little,
        DecodeCompression::HasselbladLosslessJpeg,
    );

    let SampleBuffer::U16(image) = decode(&request).unwrap() else {
        panic!("Hasselblad lossless JPEG samples must decode as u16");
    };

    assert_eq!(image.as_raw().as_slice(), &[0, 32_768, 0, 32_768]);
}

#[cfg(feature = "hasselblad")]
fn hasselblad_tile(entropy: &[u8]) -> Vec<u8> { hasselblad_tile_with(1, 8, 0, None, entropy) }

#[cfg(feature = "hasselblad")]
fn hasselblad_tile_with(
    component_count: u8,
    predictor: u8,
    point_transform: u8,
    restart_interval: Option<u16>,
    entropy: &[u8],
) -> Vec<u8> {
    let mut tile = vec![
        0xff,
        0xd8,
        0xff,
        0xc3,
        0x00,
        0x08 + component_count * 3,
        0x10,
        0x00,
        0x02,
        0x00,
        0x04,
        component_count,
    ];
    for component in 1..=component_count {
        tile.extend_from_slice(&[component, 0x11, 0x00]);
    }
    tile.extend_from_slice(&[0xff, 0xc4, 0x00, 0x15, 0x00, 0x01, 0x01]);
    tile.extend_from_slice(&[0; 14]);
    tile.extend_from_slice(&[0x00, 0x01]);
    if let Some(interval) = restart_interval {
        tile.extend_from_slice(&[0xff, 0xdd, 0x00, 0x04]);
        tile.extend_from_slice(&interval.to_be_bytes());
    }
    tile.extend_from_slice(&[0xff, 0xda, 0x00, 6 + component_count * 2, component_count]);
    for component in 1..=component_count {
        tile.extend_from_slice(&[component, 0x00]);
    }
    tile.extend_from_slice(&[predictor, 0x00, point_transform]);
    tile.extend_from_slice(entropy);
    tile.extend_from_slice(&[0xff, 0xd9]);
    tile
}

#[cfg(feature = "hasselblad")]
fn hasselblad_tile_without_dht(entropy: &[u8]) -> Vec<u8> {
    let mut tile = hasselblad_tile(entropy);
    let dht_start = 2 + 2 + ushort_segment_length(&tile[4..6]) as usize;
    let dht_length = ushort_segment_length(&tile[dht_start + 2..dht_start + 4]) as usize;
    tile.drain(dht_start..dht_start + 2 + dht_length);
    tile
}

#[cfg(feature = "hasselblad")]
fn hasselblad_symbol_sixteen_tile() -> Vec<u8> {
    let mut tile = vec![
        0xff, 0xd8, 0xff, 0xc3, 0x00, 0x0b, 0x10, 0x00, 0x02, 0x00, 0x02, 0x01, 0x01, 0x11, 0x00,
        0xff, 0xc4, 0x00, 0x15, 0x00, 0x02, 0x00,
    ];
    tile.extend_from_slice(&[0; 14]);
    tile.extend_from_slice(&[
        16, 0, 0xff, 0xda, 0x00, 0x08, 0x01, 0x01, 0x00, 0x08, 0x00, 0x00,
    ]);
    let mut bits = Vec::new();
    for _ in 0..2 {
        bits.push(false);
        bits.push(true);
        bits.extend(std::iter::repeat_n(false, 16));
    }
    while !bits.len().is_multiple_of(32) {
        bits.push(false);
    }
    let mut encoded = Vec::new();
    for byte in bits.chunks(8) {
        let mut value = 0_u8;
        for (index, bit) in byte.iter().enumerate() {
            value |= u8::from(*bit) << (7 - index);
        }
        encoded.push(value);
    }
    for chunk in encoded.chunks(4) {
        tile.extend(chunk.iter().rev());
    }
    tile.extend_from_slice(&[0xff, 0xd9]);
    tile
}

#[cfg(feature = "hasselblad")]
fn ushort_segment_length(bytes: &[u8]) -> u16 {
    u16::from_be_bytes(
        bytes
            .try_into()
            .expect("a JPEG segment length is two bytes"),
    )
}

#[cfg(feature = "hasselblad")]
fn assert_hasselblad_rejected(tile: &[u8], expected: &str) {
    let chunks = [Chunk {
        offset: 0,
        byte_count: tile.len(),
    }];
    let request = request(
        tile,
        &chunks,
        4,
        2,
        4,
        2,
        16,
        SampleFormat::Integer,
        ByteOrder::Little,
        DecodeCompression::HasselbladLosslessJpeg,
    );

    let error = decode(&request).unwrap_err().to_string();
    assert!(error.contains(expected), "unexpected error: {error}");
}

#[test]
#[cfg(feature = "lossy-jpeg")]
fn lossy_jpeg_decodes_a_single_component_tile() {
    use image::ImageEncoder;

    let pixels = [
        0_u8, 32, 64, 96, 96, 64, 32, 0, 16, 48, 80, 112, 112, 80, 48, 16,
    ];
    let mut encoded = Vec::new();
    image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 100)
        .write_image(&pixels, 4, 4, image::ExtendedColorType::L8)
        .unwrap();
    let chunks = [Chunk {
        offset: 0,
        byte_count: encoded.len(),
    }];
    let request = request(
        &encoded,
        &chunks,
        4,
        4,
        4,
        4,
        8,
        SampleFormat::Integer,
        ByteOrder::Little,
        DecodeCompression::LossyJpeg,
    );

    let SampleBuffer::U8(image) = decode(&request).unwrap() else {
        panic!("lossy JPEG samples must decode as u8");
    };

    assert_eq!(image.dimensions(), (4, 4));
}

#[test]
#[cfg(feature = "jpegxl")]
fn jpeg_xl_rejects_malformed_payloads_without_panicking() {
    let source = [0xff];
    let chunks = [Chunk {
        offset: 0,
        byte_count: source.len(),
    }];
    let request = request(
        &source,
        &chunks,
        1,
        1,
        1,
        1,
        16,
        SampleFormat::Integer,
        ByteOrder::Little,
        DecodeCompression::JpegXl,
    );

    let error = decode(&request).unwrap_err();

    assert!(matches!(error, fornax_codec::CodecError::InvalidInput(_)));
}
