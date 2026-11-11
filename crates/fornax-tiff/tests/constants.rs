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
        (CompressionMethod::Lzw, 5),
        (CompressionMethod::LosslessJpeg, 7),
        (CompressionMethod::Deflate, 8),
        (CompressionMethod::PackBits, 32_773),
        (CompressionMethod::DeflateLegacy, 32_946),
        (CompressionMethod::Lzma, 34_925),
        (CompressionMethod::Zstd, 50_000),
        (CompressionMethod::WebP, 50_001),
        (CompressionMethod::JpegXl, 50_002),
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
fn compression_methods_do_not_claim_dng_owned_identifiers() {
    for value in [34_892, 52_546] {
        assert_eq!(
            CompressionMethod::from(value),
            CompressionMethod::Unknown(value),
            "compression {value}"
        );
    }
}

#[test]
fn exif_and_maker_note_tags_have_typed_names() {
    assert_eq!(TiffTag::from(34_665), TiffTag::ExifIfdPointer);
    assert_eq!(TiffTag::from(37_500), TiffTag::MakerNote);
    assert_eq!(u16::from(TiffTag::ExifIfdPointer), 34_665);
    assert_eq!(u16::from(TiffTag::MakerNote), 37_500);
}

#[test]
fn photometric_values_preserve_tiff_ep_cfa_and_unknown_identifiers() {
    let values = [(PhotometricInterpretation::Cfa, 32_803)];

    for (photometric, value) in values {
        assert_eq!(u16::from(photometric), value);
        assert_eq!(PhotometricInterpretation::from(value), photometric);
    }

    assert_eq!(
        PhotometricInterpretation::from(12_345),
        PhotometricInterpretation::Unknown(12_345)
    );
}

#[test]
fn photometric_values_do_not_claim_dng_linear_raw() {
    assert_eq!(
        PhotometricInterpretation::from(34_892),
        PhotometricInterpretation::Unknown(34_892)
    );
}
