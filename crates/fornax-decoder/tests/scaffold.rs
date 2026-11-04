use std::collections::BTreeMap;
use std::io::Cursor;
use std::path::Path;

use fornax_core::protocol::IDecoder;
use fornax_core::{BayerPattern, MonoImage};
use fornax_decoder::{
    CfaPatternError, DecodeError, DecodedRaw, Decoder, DngCompression, DngMetadata, DngTag,
    DngValue, DngVersion, DngWriteOptions, HandoffError, MosaicData, SourceFormat,
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
fn source_format_uses_strum_conversions() {
    assert_eq!(SourceFormat::Dng.to_string(), "DNG");
    assert_eq!("CR3".parse(), Ok(SourceFormat::Cr3));

    let name: &'static str = SourceFormat::Hasselblad3fr.into();
    assert_eq!(name, "3FR");
}

#[test]
fn dng_tag_preserves_known_and_unknown_numbers() {
    assert_eq!(DngTag::from(274), DngTag::Orientation);
    assert_eq!(u16::from(DngTag::Orientation), 274);
    assert_eq!(DngTag::from(0x1234), DngTag::Unknown(0x1234));
    assert_eq!(u16::from(DngTag::Unknown(0x1234)), 0x1234);

    let tiff_tag: tiff::tags::Tag = DngTag::Orientation.into();
    assert_eq!(tiff_tag, tiff::tags::Tag::Orientation);
}

#[test]
fn dng_tags_are_ordered_by_tiff_number() {
    let mut metadata = DngMetadata::new();
    metadata.insert(DngTag::Unknown(100), DngValue::Short(1));
    metadata.insert(DngTag::Orientation, DngValue::Short(1));
    metadata.insert(DngTag::CfaPattern, DngValue::Byte(0));

    assert_eq!(
        metadata.keys().copied().collect::<Vec<_>>(),
        [
            DngTag::Unknown(100),
            DngTag::Orientation,
            DngTag::CfaPattern
        ]
    );
}

#[test]
fn metadata_is_a_btreemap_of_tiff_values() {
    let mut metadata = DngMetadata::new();
    let value: DngValue = tiff::decoder::ifd::Value::Short(512);
    metadata.insert(DngTag::BlackLevel, value.clone());

    let typed: &BTreeMap<DngTag, DngValue> = &metadata;
    assert_eq!(typed.get(&DngTag::BlackLevel), Some(&value));
}

#[test]
fn metadata_can_be_accessed_directly_by_tag() {
    let decoded = DecodedRaw::new(
        MosaicData::U16(MonoImage::from_raw(1, 1, vec![0]).expect("valid mosaic")),
        image_metadata(1, 1),
    )
    .expect("valid image");

    assert_eq!(
        decoded.metadata_by_tag(DngTag::ImageWidth),
        Some(&DngValue::Unsigned(1))
    );
    assert_eq!(decoded.metadata_by_tag(DngTag::BlackLevel), None);
}

#[test]
fn black_level_has_a_tag_specific_getter() {
    let mut metadata = image_metadata(1, 1);
    metadata.insert(DngTag::BlackLevel, DngValue::Short(512));
    let decoded = DecodedRaw::new(
        MosaicData::U16(MonoImage::from_raw(1, 1, vec![0]).expect("valid mosaic")),
        metadata,
    )
    .expect("valid image");

    assert_eq!(decoded.black_level(), Some(&DngValue::Short(512)));
}

#[test]
fn mosaic_data_uses_image_buffers_for_geometry() {
    let pixels = MosaicData::U16(MonoImage::from_raw(3, 2, vec![0; 6]).expect("valid mosaic"));
    let decoded = DecodedRaw::new(pixels, image_metadata(3, 2)).expect("matching dimensions");

    assert_eq!(decoded.dimensions(), (3, 2));
    assert_eq!(
        decoded.metadata().get(&DngTag::ImageWidth),
        Some(&DngValue::Unsigned(3))
    );
}

#[test]
fn decoded_raw_rejects_image_and_tag_dimension_disagreement() {
    let pixels = MosaicData::U16(MonoImage::from_raw(2, 2, vec![0; 4]).expect("valid mosaic"));
    let error = DecodedRaw::new(pixels, image_metadata(3, 2))
        .expect_err("tag dimensions must match image dimensions");

    assert!(matches!(error, DecodeError::InvalidInput { .. }));
}

#[test]
fn cfa_is_read_from_dng_tags() {
    let decoded = decoded_with_cfa(vec![0, 1, 1, 2], vec![0, 1, 2]);

    assert_eq!(
        decoded.cfa_pattern().expect("valid CFA"),
        Some(vec![0, 1, 1, 2])
    );
    assert_eq!(
        decoded.preprocessor_pattern().expect("RGB Bayer"),
        BayerPattern::RGGB
    );
}

#[test]
fn non_bayer_cfa_is_rejected_at_handoff() {
    let decoded = decoded_with_cfa(vec![1; 36], vec![0, 1, 2]);

    assert!(matches!(
        decoded.preprocessor_pattern(),
        Err(HandoffError::UnsupportedCfa {
            width: 6,
            height: 6
        })
    ));
}

#[test]
fn non_rgb_plane_colors_are_rejected_at_handoff() {
    let decoded = decoded_with_cfa(vec![0, 1, 1, 2], vec![0, 1, 3]);

    assert!(matches!(
        decoded.preprocessor_pattern(),
        Err(HandoffError::UnsupportedCfa {
            width: 2,
            height: 2
        })
    ));
}

#[test]
fn non_rectangular_cfa_layout_is_rejected_at_handoff() {
    let decoded = decoded_with_cfa(vec![0, 1, 1, 2], vec![0, 1, 2]);
    let mut metadata = decoded.metadata().clone();
    metadata.insert(DngTag::CfaLayout, DngValue::Short(2));
    let decoded = DecodedRaw::new(decoded.pixels().clone(), metadata).expect("valid image");

    assert!(matches!(
        decoded.preprocessor_pattern(),
        Err(HandoffError::UnsupportedCfa {
            width: 2,
            height: 2
        })
    ));
}

#[test]
fn malformed_cfa_returns_typed_error() {
    let mut metadata = image_metadata(2, 2);
    metadata.insert(
        DngTag::CfaRepeatPatternDim,
        DngValue::List(vec![DngValue::Short(2), DngValue::Short(2)]),
    );
    metadata.insert(
        DngTag::CfaPattern,
        DngValue::List(vec![
            DngValue::Byte(0),
            DngValue::Byte(1),
            DngValue::Byte(2),
        ]),
    );
    let decoded = DecodedRaw::new(
        MosaicData::U16(MonoImage::from_raw(2, 2, vec![0; 4]).expect("valid mosaic")),
        metadata,
    )
    .expect("valid image");

    assert!(matches!(
        decoded.cfa_pattern(),
        Err(CfaPatternError::LengthMismatch {
            expected: 4,
            actual: 3
        })
    ));
}

#[test]
fn writer_methods_live_on_decoded_raw_and_are_unsupported() {
    let decoded = decoded_with_cfa(vec![0, 1, 1, 2], vec![0, 1, 2]);
    let options = DngWriteOptions::default();
    let mut output = Cursor::new(Vec::new());

    assert!(matches!(
        decoded.write_dng("output.dng", options),
        Err(fornax_decoder::DngWriteError::Unsupported { .. })
    ));
    assert!(matches!(
        decoded.write_dng_with(&mut output, options),
        Err(fornax_decoder::DngWriteError::Unsupported { .. })
    ));
}

#[test]
fn dng_writer_defaults_to_latest_lossless() {
    assert_eq!(DngVersion::latest().major, 1);
    assert_eq!(DngCompression::default(), DngCompression::LosslessJpeg);

    let options = DngWriteOptions::default();
    assert_eq!(options.version(), DngVersion::latest());
    assert_eq!(options.compression(), DngCompression::LosslessJpeg);
}

fn image_metadata(width: u32, height: u32) -> DngMetadata {
    let mut metadata = DngMetadata::new();
    metadata.insert(DngTag::ImageWidth, DngValue::Unsigned(width));
    metadata.insert(DngTag::ImageLength, DngValue::Unsigned(height));
    metadata
}

fn decoded_with_cfa(pattern: Vec<u8>, plane_colors: Vec<u8>) -> DecodedRaw {
    let side = if pattern.len() == 4 { 2 } else { 6 };
    let mut metadata = image_metadata(side, side);
    metadata.insert(
        DngTag::CfaRepeatPatternDim,
        DngValue::List(vec![
            DngValue::Short(side as u16),
            DngValue::Short(side as u16),
        ]),
    );
    metadata.insert(
        DngTag::CfaPattern,
        DngValue::List(pattern.into_iter().map(DngValue::Byte).collect()),
    );
    metadata.insert(
        DngTag::CfaPlaneColor,
        DngValue::List(plane_colors.into_iter().map(DngValue::Byte).collect()),
    );
    let pixels = MosaicData::U16(
        MonoImage::from_raw(side, side, vec![0; (side * side) as usize]).expect("valid mosaic"),
    );
    DecodedRaw::new(pixels, metadata).expect("valid image")
}
