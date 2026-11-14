#![cfg(feature = "raster")]

use fornax_codec::{
    ByteOrder, Chunk, DecodeCompression, ICodecDecode, Predictor, SampleBuffer, SampleFormat,
};
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{
    Directory, DirectoryLookup, RasterError, RasterLayout, RasterRequest, RasterStorage, Tag,
    TiffCompressionError, Value, resolve_tiff_compression,
};

fn directory(
    width: u32,
    height: u32,
    entries: impl IntoIterator<Item = (TiffTag, Value)>,
) -> Directory {
    let mut directory = Directory::default();
    directory.insert(Tag::from(TiffTag::ImageWidth), Value::Unsigned(width));
    directory.insert(Tag::from(TiffTag::ImageLength), Value::Unsigned(height));
    for (tag, value) in entries {
        directory.insert(Tag::from(tag), value);
    }
    directory
}

fn unsigned(values: impl IntoIterator<Item = u32>) -> Value {
    Value::List(values.into_iter().map(Value::Unsigned).collect())
}

struct Overlay<'a> {
    root: &'a Directory,
    raw: &'a Directory,
}

impl DirectoryLookup for Overlay<'_> {
    fn get(&self, tag: &Tag) -> Option<&Value> { self.raw.get(tag).or_else(|| self.root.get(tag)) }

    fn contains_key(&self, tag: &Tag) -> bool {
        self.raw.contains_key(tag) || self.root.contains_key(tag)
    }
}

#[test]
fn directory_lookup_prefers_raw_geometry_and_falls_back_to_root() {
    let root = directory(
        10,
        4,
        [
            (TiffTag::RowsPerStrip, Value::Unsigned(4)),
            (TiffTag::StripOffsets, Value::Unsigned(0)),
            (TiffTag::StripByteCounts, Value::Unsigned(80)),
        ],
    );
    let raw = directory(20, 4, []);
    let overlay = Overlay {
        root: &root,
        raw: &raw,
    };

    let layout = RasterLayout::from_lookup(&overlay).unwrap();

    assert_eq!(layout.width(), 20);
    assert_eq!(
        overlay.get(&TiffTag::RowsPerStrip.into()),
        Some(&Value::Unsigned(4))
    );
}

#[test]
fn strips_preserve_rows_per_strip_and_chunk_order() {
    let directory = directory(
        5,
        5,
        [
            (TiffTag::RowsPerStrip, Value::Unsigned(2)),
            (TiffTag::StripOffsets, unsigned([100, 20, 70])),
            (TiffTag::StripByteCounts, unsigned([10, 11, 12])),
        ],
    );

    let layout = RasterLayout::from_directory(&directory).unwrap();

    assert_eq!(layout.storage(), RasterStorage::Strips);
    assert_eq!((layout.width(), layout.height()), (5, 5));
    assert_eq!((layout.chunk_width(), layout.chunk_height()), (5, 2));
    assert_eq!(
        layout.chunks(),
        &[
            Chunk {
                offset: 100,
                byte_count: 10,
            },
            Chunk {
                offset: 20,
                byte_count: 11,
            },
            Chunk {
                offset: 70,
                byte_count: 12,
            },
        ]
    );
}

#[test]
fn tiles_preserve_geometry_and_row_major_chunk_order() {
    let directory = directory(
        5,
        5,
        [
            (TiffTag::TileWidth, Value::Unsigned(3)),
            (TiffTag::TileLength, Value::Unsigned(2)),
            (TiffTag::TileOffsets, unsigned([90, 10, 60, 30, 50, 40])),
            (TiffTag::TileByteCounts, unsigned([1, 2, 3, 4, 5, 6])),
        ],
    );

    let layout = RasterLayout::from_directory(&directory).unwrap();

    assert_eq!(layout.storage(), RasterStorage::Tiles);
    assert_eq!((layout.width(), layout.height()), (5, 5));
    assert_eq!((layout.chunk_width(), layout.chunk_height()), (3, 2));
    assert_eq!(
        layout.chunks(),
        &[
            Chunk {
                offset: 90,
                byte_count: 1,
            },
            Chunk {
                offset: 10,
                byte_count: 2,
            },
            Chunk {
                offset: 60,
                byte_count: 3,
            },
            Chunk {
                offset: 30,
                byte_count: 4,
            },
            Chunk {
                offset: 50,
                byte_count: 5,
            },
            Chunk {
                offset: 40,
                byte_count: 6,
            },
        ]
    );
}

#[test]
fn decode_tiles_with_partial_final_geometry_preserves_row_major_copy() {
    let source = (0_u8..32).collect::<Vec<_>>();
    let directory = directory(
        5,
        3,
        [
            (TiffTag::TileWidth, Value::Unsigned(4)),
            (TiffTag::TileLength, Value::Unsigned(2)),
            (TiffTag::TileOffsets, unsigned([0, 8, 16, 24])),
            (TiffTag::TileByteCounts, unsigned([8, 8, 8, 8])),
        ],
    );
    let request = RasterRequest::new(
        &source,
        &directory,
        DecodeCompression::Uncompressed,
        8,
        SampleFormat::Integer,
        ByteOrder::Little,
        Predictor::None,
    )
    .unwrap();

    let decoded = request
        .compression()
        .decode(&request.codec_request())
        .unwrap();
    let SampleBuffer::U8(image) = decoded else {
        panic!("uncompressed 8-bit samples must decode as u8");
    };
    assert_eq!(image.dimensions(), (5, 3));
    assert_eq!(
        image.into_raw(),
        vec![0, 1, 2, 3, 8, 4, 5, 6, 7, 12, 16, 17, 18, 19, 24]
    );
}

#[test]
fn decode_strips_with_partial_final_strip_preserves_row_major_copy() {
    let source = (0_u8..15).collect::<Vec<_>>();
    let directory = directory(
        3,
        5,
        [
            (TiffTag::RowsPerStrip, Value::Unsigned(2)),
            (TiffTag::StripOffsets, unsigned([0, 6, 12])),
            (TiffTag::StripByteCounts, unsigned([6, 6, 3])),
        ],
    );
    let request = RasterRequest::new(
        &source,
        &directory,
        DecodeCompression::Uncompressed,
        8,
        SampleFormat::Integer,
        ByteOrder::Little,
        Predictor::None,
    )
    .unwrap();

    let decoded = request
        .compression()
        .decode(&request.codec_request())
        .unwrap();
    let SampleBuffer::U8(image) = decoded else {
        panic!("uncompressed 8-bit strips must decode as u8");
    };

    assert_eq!(image.dimensions(), (3, 5));
    assert_eq!(image.into_raw(), source);
}

#[test]
fn missing_byte_counts_reports_the_layout_tag() {
    let directory = directory(
        4,
        1,
        [
            (TiffTag::TileWidth, Value::Unsigned(4)),
            (TiffTag::TileLength, Value::Unsigned(1)),
            (TiffTag::TileOffsets, Value::Unsigned(0)),
        ],
    );

    let error = RasterLayout::from_directory(&directory).unwrap_err();

    assert_eq!(
        error,
        RasterError::MissingTag(Tag::from(TiffTag::TileByteCounts))
    );
}

#[test]
fn mismatched_offsets_and_byte_counts_are_rejected() {
    let directory = directory(
        4,
        2,
        [
            (TiffTag::TileWidth, Value::Unsigned(4)),
            (TiffTag::TileLength, Value::Unsigned(1)),
            (TiffTag::TileOffsets, unsigned([0, 4])),
            (TiffTag::TileByteCounts, Value::Unsigned(4)),
        ],
    );

    let error = RasterLayout::from_directory(&directory).unwrap_err();

    assert_eq!(
        error,
        RasterError::ChunkListLengthMismatch {
            offsets: 2,
            byte_counts: 1,
        }
    );
}

#[test]
fn chunk_count_that_disagrees_with_geometry_is_rejected() {
    let directory = directory(
        4,
        2,
        [
            (TiffTag::TileWidth, Value::Unsigned(4)),
            (TiffTag::TileLength, Value::Unsigned(1)),
            (TiffTag::TileOffsets, Value::Unsigned(0)),
            (TiffTag::TileByteCounts, Value::Unsigned(4)),
        ],
    );

    let error = RasterLayout::from_directory(&directory).unwrap_err();

    assert_eq!(
        error,
        RasterError::ChunkCountMismatch {
            expected: 2,
            found: 1,
        }
    );
}

#[test]
fn chunk_range_overflow_is_rejected() {
    let source = [0_u8; 8];
    let directory = directory(
        1,
        1,
        [
            (TiffTag::TileWidth, Value::Unsigned(1)),
            (TiffTag::TileLength, Value::Unsigned(1)),
            (
                TiffTag::TileOffsets,
                Value::UnsignedBig(u64::try_from(usize::MAX).unwrap()),
            ),
            (TiffTag::TileByteCounts, Value::Unsigned(1)),
        ],
    );

    let error = RasterRequest::new(
        &source,
        &directory,
        DecodeCompression::Uncompressed,
        8,
        SampleFormat::Integer,
        ByteOrder::Little,
        Predictor::None,
    )
    .unwrap_err();

    assert_eq!(error, RasterError::ChunkRangeOverflow { index: 0 });
}

#[test]
fn chunk_outside_source_is_rejected() {
    let source = [0_u8; 8];
    let directory = directory(
        1,
        1,
        [
            (TiffTag::TileWidth, Value::Unsigned(1)),
            (TiffTag::TileLength, Value::Unsigned(1)),
            (TiffTag::TileOffsets, Value::Unsigned(3)),
            (TiffTag::TileByteCounts, Value::Unsigned(6)),
        ],
    );

    let error = RasterRequest::new(
        &source,
        &directory,
        DecodeCompression::Uncompressed,
        8,
        SampleFormat::Integer,
        ByteOrder::Little,
        Predictor::None,
    )
    .unwrap_err();

    assert_eq!(
        error,
        RasterError::ChunkOutOfBounds {
            index: 0,
            offset: 3,
            byte_count: 6,
            source_len: 8,
        }
    );
}

#[test]
fn zero_dimensions_are_rejected_with_the_source_tag() {
    let directory = directory(
        1,
        1,
        [
            (TiffTag::TileWidth, Value::Unsigned(0)),
            (TiffTag::TileLength, Value::Unsigned(1)),
            (TiffTag::TileOffsets, Value::Unsigned(0)),
            (TiffTag::TileByteCounts, Value::Unsigned(1)),
        ],
    );

    let error = RasterLayout::from_directory(&directory).unwrap_err();

    assert_eq!(
        error,
        RasterError::ZeroDimension(Tag::from(TiffTag::TileWidth))
    );
}

#[test]
fn tiff_raster_compression_ids_resolve_to_their_codecs() {
    for (id, expected) in [
        (1, DecodeCompression::Uncompressed),
        (7, DecodeCompression::LosslessJpeg),
        (8, DecodeCompression::Deflate),
        (32_946, DecodeCompression::Deflate),
    ] {
        assert_eq!(
            resolve_tiff_compression(id),
            Ok(expected),
            "compression {id}"
        );
    }
}

#[test]
fn known_compression_ids_without_codecs_name_the_required_feature() {
    for (id, required_feature) in [(34_925, "lzma"), (50_001, "webp")] {
        assert_eq!(
            resolve_tiff_compression(id),
            Err(TiffCompressionError::UnsupportedCompression {
                id,
                required_feature,
            }),
            "compression {id}"
        );
    }
}

#[cfg(not(feature = "lzw"))]
#[test]
fn disabled_lzw_compression_names_the_feature() {
    assert_eq!(
        resolve_tiff_compression(5),
        Err(TiffCompressionError::FeatureDisabled {
            id: 5,
            required_feature: "lzw",
        })
    );
}

#[cfg(feature = "lzw")]
#[test]
fn enabled_lzw_compression_resolves() {
    assert_eq!(resolve_tiff_compression(5), Ok(DecodeCompression::Lzw));
}

#[cfg(not(feature = "packbits"))]
#[test]
fn disabled_packbits_compression_names_the_feature() {
    assert_eq!(
        resolve_tiff_compression(32_773),
        Err(TiffCompressionError::FeatureDisabled {
            id: 32_773,
            required_feature: "packbits",
        })
    );
}

#[cfg(feature = "packbits")]
#[test]
fn enabled_packbits_compression_resolves() {
    assert_eq!(
        resolve_tiff_compression(32_773),
        Ok(DecodeCompression::PackBits)
    );
}

#[cfg(all(feature = "lzw", feature = "packbits"))]
#[test]
fn optional_codecs_preserve_the_legacy_deflate_alias() {
    assert_eq!(
        resolve_tiff_compression(32_946),
        Ok(DecodeCompression::Deflate)
    );
}

#[cfg(not(feature = "jpegxl"))]
#[test]
fn disabled_jpegxl_compression_names_the_feature() {
    assert_eq!(
        resolve_tiff_compression(50_002),
        Err(TiffCompressionError::FeatureDisabled {
            id: 50_002,
            required_feature: "jpegxl",
        })
    );
}

#[cfg(feature = "jpegxl")]
#[test]
fn enabled_jpegxl_compression_resolves() {
    assert_eq!(
        resolve_tiff_compression(50_002),
        Ok(DecodeCompression::JpegXl)
    );
}

#[cfg(not(feature = "zstd"))]
#[test]
fn disabled_zstd_compression_names_the_feature() {
    assert_eq!(
        resolve_tiff_compression(50_000),
        Err(TiffCompressionError::FeatureDisabled {
            id: 50_000,
            required_feature: "zstd",
        })
    );
}

#[cfg(feature = "zstd")]
#[test]
fn enabled_zstd_compression_resolves() {
    assert_eq!(
        resolve_tiff_compression(50_000),
        Ok(DecodeCompression::Zstd)
    );
}

#[test]
fn unknown_and_non_tiff_compression_ids_remain_unknown() {
    for id in [12_345, 32_766, 32_767, 34_892, 52_546] {
        assert_eq!(
            resolve_tiff_compression(id),
            Err(TiffCompressionError::UnknownCompression(id)),
            "compression {id}"
        );
    }
}
