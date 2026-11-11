use fornax_codec::{CanonCrxCfa, CanonCrxHeader};
use image::math::Rect;

use super::*;
use crate::metadata::shifted_pattern;

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
