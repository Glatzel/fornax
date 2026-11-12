use std::collections::BTreeMap;

use fornax_dng::DngTag;
use fornax_tiff::{Tag, Value};
use image::math::Rect;

use super::*;
use crate::color_data::{ColorData, ColorDataStatus};
use crate::container::Cmt;
use crate::metadata::shifted_pattern;
use crate::{CanonCrxCfa, CanonCrxHeader};

#[test]
fn odd_active_origin_shifts_the_cfa_pattern() {
    assert_eq!(shifted_pattern(CanonCrxCfa::Rggb, 0, 0), [0, 1, 1, 2]);
    assert_eq!(shifted_pattern(CanonCrxCfa::Rggb, 1, 0), [1, 0, 2, 1]);
    assert_eq!(shifted_pattern(CanonCrxCfa::Rggb, 1, 1), [2, 1, 1, 0]);
}

#[test]
fn typed_errors_are_used_for_lossy_and_missing_color_data() {
    let error = CanonCrxHeader::new(0x0200, 4, 2, 4, 2, 14, 0, 3, 0x70, 14).unwrap_err();
    assert!(matches!(error, fornax_codec::CodecError::Unsupported(_)));

    let status = ColorDataStatus::Missing;
    assert!(status.parsed().is_none());
}

#[test]
fn crop_rectangles_are_half_open_and_bounded() {
    let geometry =
        container::parse_geometry_for_test(0, 10, 12, 49, 59, 1, 2, 50, 60, 100, 100).unwrap();
    assert_eq!(
        geometry.crop,
        Rect {
            x: 10,
            y: 12,
            width: 40,
            height: 48
        }
    );
    assert_eq!(geometry.active, geometry.crop);
}

#[test]
fn valid_crop_outside_active_area_replaces_active_area() {
    let geometry =
        container::parse_geometry_for_test(2, 10, 12, 49, 59, 20, 22, 39, 49, 100, 100).unwrap();
    assert_eq!(geometry.active, geometry.crop);
}

#[test]
fn active_area_overshoot_is_clamped_to_cmp1() {
    let geometry =
        container::parse_geometry_for_test(2, 0, 0, 99, 99, 0, 0, 500, 500, 100, 100).unwrap();
    assert_eq!(
        geometry.active,
        Rect {
            x: 0,
            y: 0,
            width: 100,
            height: 100
        }
    );
}

#[test]
fn cmt_tiff_header_allows_a_bounded_payload_prefix() {
    let tiff = b"II\x2a\0\x08\0\0\0\0\0\0\0\0\0";
    let mut child_payload = vec![0_u8; 4];
    child_payload.extend_from_slice(tiff);
    let mut child = u32::try_from(child_payload.len() + 8)
        .unwrap()
        .to_be_bytes()
        .to_vec();
    child.extend_from_slice(b"CMT1");
    child.extend_from_slice(&child_payload);
    let mut parent = u32::try_from(child.len() + 8)
        .unwrap()
        .to_be_bytes()
        .to_vec();
    parent.extend_from_slice(b"moov");
    parent.extend_from_slice(&child);
    let bmff = fornax_bmff::Bmff::parse(&parent).unwrap();
    let cmt = container::optional_tiff_child(bmff.boxes().first().unwrap(), *b"CMT1")
        .unwrap()
        .unwrap();
    assert!(cmt.is_empty());
}

#[test]
fn unknown_color_data_errors_at_process_metadata() {
    let header = CanonCrxHeader::new(0x0100, 4, 2, 4, 2, 14, 0, 0, 0x70, 14).unwrap();
    let image = Cr3Image {
        source: Arc::from([]),
        sample: container::SampleRange { offset: 0, size: 0 },
        header: container::CodecHeader {
            header,
            cfa: CanonCrxCfa::Rggb,
            width: 4,
            height: 2,
            bit_depth: 14,
        },
        metadata: Directory::default(),
        color_data: ColorDataStatus::Unknown(999),
        quality_mode: None,
    };
    assert!(matches!(
        image.process_metadata(),
        Err(DecodeError::Unsupported(_))
    ));
}

#[test]
fn light_raw_mode_uses_unit_as_shot_neutral() {
    let cmt = Cmt {
        cmt1: Directory::default(),
        cmt2: Directory::default(),
        cmt3: BTreeMap::from([(
            Tag(0x0001),
            Value::List(vec![
                Value::Short(0),
                Value::Short(0),
                Value::Short(0),
                Value::Short(130),
            ]),
        )]),
        cmt4: Directory::default(),
    };
    let status = ColorDataStatus::Parsed(ColorData {
        version: 48,
        white_balance: [1000, 1024, 1024, 900],
        black_level: [512; 4],
        specular_white_level: 16_383,
    });
    let directory = metadata::map(
        &cmt,
        4,
        2,
        Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 2,
        },
        Rect {
            x: 0,
            y: 0,
            width: 4,
            height: 2,
        },
        CanonCrxCfa::Rggb,
        &status,
        14,
        Some(130),
    )
    .unwrap();
    assert_eq!(
        directory.get(&Tag::from(DngTag::AsShotNeutral)),
        Some(&Value::List(vec![
            Value::Rational(1, 1),
            Value::Rational(1, 1),
            Value::Rational(1, 1),
        ]))
    );
}
