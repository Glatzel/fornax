use tiff::decoder::ifd::Value;

use super::decode_samples;
use crate::{DngCompression, DngMetadata, DngTag, MosaicData};

fn u16_source(samples: &[u16]) -> Vec<u8> {
    let mut source = b"II".to_vec();
    source.extend(samples.iter().flat_map(|sample| sample.to_le_bytes()));
    source
}

fn storage_metadata(
    rows_per_strip: u32,
    strip_offsets: Vec<u32>,
    strip_byte_counts: Vec<u32>,
) -> DngMetadata {
    DngMetadata::from([
        (DngTag::ImageWidth, Value::Unsigned(4)),
        (DngTag::ImageLength, Value::Unsigned(3)),
        (DngTag::BitsPerSample, Value::Short(16)),
        (DngTag::Compression, Value::Short(1)),
        (DngTag::SamplesPerPixel, Value::Short(1)),
        (DngTag::RowsPerStrip, Value::Unsigned(rows_per_strip)),
        (
            DngTag::StripOffsets,
            Value::List(strip_offsets.into_iter().map(Value::Unsigned).collect()),
        ),
        (
            DngTag::StripByteCounts,
            Value::List(strip_byte_counts.into_iter().map(Value::Unsigned).collect()),
        ),
    ])
}

fn tile_metadata() -> DngMetadata {
    DngMetadata::from([
        (DngTag::ImageWidth, Value::Unsigned(4)),
        (DngTag::ImageLength, Value::Unsigned(3)),
        (DngTag::BitsPerSample, Value::Short(16)),
        (DngTag::Compression, Value::Short(1)),
        (DngTag::SamplesPerPixel, Value::Short(1)),
        (DngTag::TileWidth, Value::Unsigned(2)),
        (DngTag::TileLength, Value::Unsigned(2)),
        (
            DngTag::TileOffsets,
            Value::List(vec![
                Value::Unsigned(2),
                Value::Unsigned(10),
                Value::Unsigned(18),
                Value::Unsigned(26),
            ]),
        ),
        (
            DngTag::TileByteCounts,
            Value::List(vec![
                Value::Unsigned(8),
                Value::Unsigned(8),
                Value::Unsigned(8),
                Value::Unsigned(8),
            ]),
        ),
    ])
}

#[test]
fn uncompressed_multistrip_last_partial_strip_decodes_in_bounds() {
    let expected = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let source = u16_source(&expected);
    let metadata = storage_metadata(2, vec![2, 18], vec![16, 8]);

    let (decoded, compression) =
        decode_samples(&source, &metadata).expect("valid multi-strip DNG should decode");

    assert_eq!(compression, DngCompression::Uncompressed);
    let MosaicData::U16(decoded) = decoded else {
        panic!("16-bit samples should decode as U16");
    };
    assert_eq!(decoded.as_raw().as_slice(), expected.as_slice());
}

#[test]
fn uncompressed_multitile_image_decodes_each_tile() {
    let expected = vec![1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12];
    let tile_samples = [1, 2, 5, 6, 3, 4, 7, 8, 9, 10, 0, 0, 11, 12, 0, 0];
    let source = u16_source(&tile_samples);

    let (decoded, compression) =
        decode_samples(&source, &tile_metadata()).expect("valid multi-tile DNG should decode");

    assert_eq!(compression, DngCompression::Uncompressed);
    let MosaicData::U16(decoded) = decoded else {
        panic!("16-bit samples should decode as U16");
    };
    assert_eq!(decoded.as_raw().as_slice(), expected.as_slice());
}

#[test]
fn zero_sized_dimensions_return_an_error() {
    let mut metadata = storage_metadata(1, vec![2], vec![2]);
    metadata.insert(DngTag::ImageWidth, Value::Unsigned(0));

    let error = decode_samples(b"II", &metadata).expect_err("zero width must be rejected");
    assert!(error.to_string().contains("must not be zero"));
}

#[test]
fn zero_sized_tiles_return_an_error() {
    let mut metadata = tile_metadata();
    metadata.insert(DngTag::TileWidth, Value::Unsigned(0));

    let error = decode_samples(b"II", &metadata).expect_err("zero tile width must be rejected");
    assert!(
        error
            .to_string()
            .contains("tile dimensions must not be zero")
    );
}
