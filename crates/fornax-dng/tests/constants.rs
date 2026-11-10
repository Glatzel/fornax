use fornax_dng::constants::{DngCompressionMethod, DngPhotometricInterpretation};
use fornax_dng::metadata::DngTag;
use fornax_tiff::Tag;

#[test]
fn dng_compression_values_round_trip_through_numeric_values() {
    let values = [
        (DngCompressionMethod::LossyJpeg, 34_892),
        (DngCompressionMethod::JpegXl, 52_546),
    ];

    for (compression, value) in values {
        assert_eq!(u16::from(compression), value);
        assert_eq!(DngCompressionMethod::from(value), compression);
    }

    assert_eq!(
        DngCompressionMethod::from(12_345),
        DngCompressionMethod::Unknown(12_345)
    );
}

#[test]
fn dng_compression_values_reject_non_dng_ids() {
    for value in [1, 5, 7, 8, 32_773, 32_946, 34_925, 50_000, 50_001, 50_002] {
        assert_eq!(
            DngCompressionMethod::from(value),
            DngCompressionMethod::Unknown(value),
            "compression {value}"
        );
    }
}

#[test]
fn dng_photometric_values_round_trip_through_numeric_values() {
    let values = [
        (DngPhotometricInterpretation::LinearRaw, 34_892),
        (DngPhotometricInterpretation::Depth, 51_177),
        (DngPhotometricInterpretation::PhotometricMask, 52_527),
    ];

    for (photometric, value) in values {
        assert_eq!(u16::from(photometric), value);
        assert_eq!(DngPhotometricInterpretation::from(value), photometric);
    }

    assert_eq!(
        DngPhotometricInterpretation::from(12_345),
        DngPhotometricInterpretation::Unknown(12_345)
    );
    assert_eq!(
        u16::from(DngPhotometricInterpretation::from(12_345)),
        12_345
    );
}

#[test]
fn dng_private_data_round_trips_through_dng_and_dynamic_tags() {
    assert_eq!(DngTag::from(50_740), DngTag::DngPrivateData);
    assert_eq!(u16::from(DngTag::DngPrivateData), 50_740);
    assert_eq!(Tag::from(DngTag::DngPrivateData), Tag(50_740));
    assert_eq!(DngTag::from(Tag(50_740)), DngTag::DngPrivateData);
}
