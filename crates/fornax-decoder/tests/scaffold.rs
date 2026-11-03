use std::path::Path;

use fornax_core::BayerPattern;
use fornax_core::protocol::IDecoder;
use fornax_decoder::{
    CfaPattern, DecodeError, DecodedRaw, Decoder, DngCompression, DngMetadata, DngTag, DngValue,
    DngVersion, DngWriteOptions, HandoffError, MosaicData,
};

#[test]
fn decode_stub_returns_unsupported() {
    let error = Decoder
        .decode(Path::new("input.dng"))
        .expect_err("the scaffold decoder must not decode yet");

    assert!(matches!(error, DecodeError::UnsupportedFormat { .. }));
    assert!(error.to_string().contains("unsupported"));
}

#[test]
fn current_protocol_adapter_returns_unsupported() {
    let result = <Decoder as IDecoder<Path, f32>>::decode_active(&Decoder, Path::new("input.dng"));

    assert!(matches!(
        result,
        Err(fornax_core::error::DecodeError::Unsupported(_))
    ));
}

#[test]
fn metadata_storage_preserves_dng_tag_and_value_type() {
    let mut metadata = DngMetadata::new();
    metadata.insert(
        DngTag::BLACK_LEVEL,
        DngValue::Short(vec![512, 513, 512, 513]),
    );
    metadata.insert(DngTag::CFA_PATTERN, DngValue::Byte(vec![0, 1, 1, 2]));

    assert_eq!(
        metadata.black_level(),
        Some(&DngValue::Short(vec![512, 513, 512, 513]))
    );
    assert_eq!(
        metadata.cfa_pattern(),
        Some(&DngValue::Byte(vec![0, 1, 1, 2]))
    );
}

#[test]
fn bayer_cfa_is_accepted_at_handoff() {
    let cfa = CfaPattern::new(2, 2, vec![0, 1, 1, 2]).expect("valid RGGB CFA");

    assert_eq!(cfa.bayer_pattern(), Some(BayerPattern::RGGB));
}

#[test]
fn non_bayer_cfa_is_rejected_at_handoff() {
    let cfa = CfaPattern::new(6, 6, vec![1; 36]).expect("valid X-Trans-shaped CFA");

    assert_eq!(
        cfa.bayer_pattern(),
        None,
        "X-Trans must not be represented as a 2x2 Bayer pattern"
    );

    let image = DecodedRaw::new(6, 6, MosaicData::U16(vec![0; 36]), cfa, DngMetadata::new())
        .expect("valid X-Trans-shaped image");
    assert!(matches!(
        image.preprocessor_pattern(),
        Err(HandoffError::UnsupportedCfa(_))
    ));
}

#[test]
fn non_rgb_plane_colors_are_rejected_at_handoff() {
    let mut metadata = DngMetadata::new();
    metadata.insert(DngTag::CFA_PLANE_COLOR, DngValue::Byte(vec![0, 1, 3]));
    let image = DecodedRaw::new(
        2,
        2,
        MosaicData::U16(vec![0; 4]),
        CfaPattern::new(2, 2, vec![0, 1, 1, 2]).expect("valid 2x2 CFA shape"),
        metadata,
    )
    .expect("valid four-color-shaped image");

    assert!(matches!(
        image.preprocessor_pattern(),
        Err(HandoffError::UnsupportedCfa(_))
    ));
}

#[test]
fn dng_writer_stub_returns_unsupported() {
    let decoded = DecodedRaw::new(
        2,
        2,
        MosaicData::U16(vec![0; 4]),
        CfaPattern::new(2, 2, vec![0, 1, 1, 2]).expect("valid RGGB CFA"),
        DngMetadata::new(),
    )
    .expect("valid decoded RAW scaffold");
    let mut output = Vec::new();
    let error = Decoder
        .write_dng(
            &decoded,
            &mut output,
            DngWriteOptions::new(DngCompression::Uncompressed),
        )
        .expect_err("the scaffold writer must not write DNG files yet");

    assert!(matches!(
        error,
        fornax_decoder::DngWriteError::Unsupported { .. }
    ));
}

#[test]
fn dng_writer_defaults_to_current_specification() {
    let options = DngWriteOptions::new(DngCompression::Uncompressed);

    assert_eq!(options.version(), DngVersion::V1_7_1_0);
    assert_eq!(DngWriteOptions::default().version(), DngVersion::V1_7_1_0);
}

#[test]
fn handoff_error_identifies_non_bayer_layout() {
    let cfa = CfaPattern::new(6, 6, vec![1; 36]).expect("valid X-Trans-shaped CFA");
    let error = HandoffError::UnsupportedCfa(cfa);

    assert!(error.to_string().contains("6x6"));
}
