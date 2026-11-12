use fornax_codec::{
    ByteOrder, Chunk, DecodeRequest, ICodecDecode, Predictor, SampleBuffer, SampleFormat,
};

use crate::codec::HasselbladCodec;

fn request<'a>(
    source: &'a [u8],
    chunks: &'a [Chunk],
    width: usize,
    height: usize,
) -> DecodeRequest<'a> {
    DecodeRequest {
        source,
        width,
        height,
        chunk_width: width,
        chunk_height: height,
        chunks,
        bits_per_sample: 16,
        sample_format: SampleFormat::Integer,
        byte_order: ByteOrder::Big,
        predictor: Predictor::None,
    }
}

#[test]
fn lossless_jpeg_reads_pair_lengths_before_differences() {
    let tile = hasselblad_tile(&[0x00, 0x00, 0x94, 0x94]);
    let chunks = [Chunk {
        offset: 0,
        byte_count: tile.len(),
    }];
    let request = request(&tile, &chunks, 4, 2);

    let SampleBuffer::U16(image) = HasselbladCodec::LosslessJpeg.decode(&request).unwrap() else {
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
fn rejects_restart_intervals() {
    let tile = hasselblad_tile_with(1, 8, 0, Some(1), &[0x94, 0x94]);
    assert_rejected(&tile, "restart intervals");
}

#[test]
fn rejects_point_transform() {
    let tile = hasselblad_tile_with(1, 8, 1, None, &[0x94, 0x94]);
    assert_rejected(&tile, "point transform");
}

#[test]
fn rejects_other_predictors() {
    let tile = hasselblad_tile_with(1, 7, 0, None, &[0x94, 0x94]);
    assert_rejected(&tile, "predictor 8");
}

#[test]
fn rejects_multiple_components() {
    let tile = hasselblad_tile_with(2, 8, 0, None, &[0x94, 0x94]);
    assert_rejected(&tile, "one-component");
}

#[test]
fn rejects_missing_huffman_table() {
    let tile = hasselblad_tile_without_dht(&[0x94, 0x94]);
    assert_rejected(&tile, "Huffman table");
}

#[test]
fn rejects_truncated_entropy() {
    let tile = hasselblad_tile(&[]);
    assert_rejected(&tile, "entropy");
}

#[test]
fn rejects_entropy_truncated_after_a_row() {
    let tile = hasselblad_tile(&[0x00, 0x00, 0x94]);
    assert_rejected(&tile, "lossless JPEG");
}

#[test]
fn consumes_the_dng_symbol_sixteen_payload() {
    let tile = hasselblad_symbol_sixteen_tile();
    let chunks = [Chunk {
        offset: 0,
        byte_count: tile.len(),
    }];
    let request = request(&tile, &chunks, 2, 2);

    let SampleBuffer::U16(image) = HasselbladCodec::LosslessJpeg.decode(&request).unwrap() else {
        panic!("Hasselblad lossless JPEG samples must decode as u16");
    };

    assert_eq!(image.as_raw().as_slice(), &[0, 32_768, 0, 32_768]);
}

fn hasselblad_tile(entropy: &[u8]) -> Vec<u8> { hasselblad_tile_with(1, 8, 0, None, entropy) }

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

fn hasselblad_tile_without_dht(entropy: &[u8]) -> Vec<u8> {
    let mut tile = hasselblad_tile(entropy);
    let dht_start = 2 + 2 + ushort_segment_length(&tile[4..6]) as usize;
    let dht_length = ushort_segment_length(&tile[dht_start + 2..dht_start + 4]) as usize;
    tile.drain(dht_start..dht_start + 2 + dht_length);
    tile
}

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

fn ushort_segment_length(bytes: &[u8]) -> u16 {
    u16::from_be_bytes(
        bytes
            .try_into()
            .expect("a JPEG segment length is two bytes"),
    )
}

fn assert_rejected(tile: &[u8], expected: &str) {
    let chunks = [Chunk {
        offset: 0,
        byte_count: tile.len(),
    }];
    let request = request(tile, &chunks, 4, 2);
    let error = HasselbladCodec::LosslessJpeg
        .decode(&request)
        .unwrap_err()
        .to_string();
    assert!(error.contains(expected), "unexpected error: {error}");
}
