use fornax_tiff::Tag;
use fornax_tiff::constants::{CompressionMethod, PhotometricInterpretation, TiffTag};

#[test]
fn tiff_tags_round_trip_through_numeric_and_dynamic_tags() {
    let values = [
        (TiffTag::NewSubfileType, 254),
        (TiffTag::Compression, 259),
        (TiffTag::PhotometricInterpretation, 262),
        (TiffTag::Predictor, 317),
        (TiffTag::CfaPattern, 33_422),
    ];

    for (tag, value) in values {
        assert_eq!(u16::from(tag), value);
        assert_eq!(TiffTag::from(value), tag);
        assert_eq!(TiffTag::from(Tag(value)), tag);
        assert_eq!(Tag::from(tag), Tag(value));
    }

    assert_eq!(TiffTag::from(60_001), TiffTag::Unknown(60_001));
}

#[test]
fn compression_methods_preserve_known_and_unknown_identifiers() {
    let values = [
        (CompressionMethod::Uncompressed, 1),
        (CompressionMethod::LosslessJpeg, 7),
        (CompressionMethod::Deflate, 8),
        (CompressionMethod::LossyJpeg, 34_892),
        (CompressionMethod::Zstd, 50_000),
        (CompressionMethod::JpegXl, 52_546),
    ];

    for (compression, value) in values {
        assert_eq!(u16::from(compression), value);
        assert_eq!(CompressionMethod::from(value), compression);
    }

    assert_eq!(
        CompressionMethod::from(12_345),
        CompressionMethod::Unknown(12_345)
    );
}

#[test]
fn photometric_values_preserve_cfa_linear_raw_and_unknown_identifiers() {
    let values = [
        (PhotometricInterpretation::Cfa, 32_803),
        (PhotometricInterpretation::LinearRaw, 34_892),
    ];

    for (photometric, value) in values {
        assert_eq!(u16::from(photometric), value);
        assert_eq!(PhotometricInterpretation::from(value), photometric);
    }

    assert_eq!(
        PhotometricInterpretation::from(12_345),
        PhotometricInterpretation::Unknown(12_345)
    );
}
