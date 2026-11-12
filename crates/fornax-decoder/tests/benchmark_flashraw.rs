#[path = "../benches/flashraw_wrapper.rs"]
mod flashraw_wrapper;

use flashraw::{CfaPattern, RawPixelData, Rect};
use flashraw_wrapper::FlashRawDecoder;
use fornax_raw_core::RawMosaic;
use fornax_raw_core::protocol::IDecoder;

#[test]
fn preserves_u16_flashraw_samples() {
    let decoder = FlashRawDecoder::from_parts(
        2,
        2,
        1,
        Some(CfaPattern::RGGB),
        RawPixelData::U16(vec![1, 2, 3, 4]),
        Rect::new(0, 0, 2, 2),
    )
    .unwrap();
    let RawMosaic::U16(raw) = decoder.raw_mosaic().unwrap() else {
        panic!("expected u16 samples");
    };
    assert_eq!(raw.as_raw(), &[1, 2, 3, 4]);
}

#[test]
fn preserves_f32_flashraw_samples() {
    let decoder = FlashRawDecoder::from_parts(
        2,
        1,
        1,
        Some(CfaPattern::RGGB),
        RawPixelData::F32(vec![0.25, 0.75]),
        Rect::new(0, 0, 2, 1),
    )
    .unwrap();
    let RawMosaic::F32(raw) = decoder.raw_mosaic().unwrap() else {
        panic!("expected f32 samples");
    };
    assert_eq!(raw.as_raw(), &[0.25, 0.75]);
}

#[test]
fn crops_to_the_flashraw_active_area() {
    let decoder = FlashRawDecoder::from_parts(
        4,
        4,
        1,
        Some(CfaPattern::RGGB),
        RawPixelData::U16((0..16).collect()),
        Rect::new(1, 1, 2, 2),
    )
    .unwrap();
    let cropped = decoder.cropped_mosaic::<f32>().unwrap();
    assert_eq!(cropped.image().dimensions(), (2, 2));
    assert_eq!(cropped.image().as_raw(), &[5.0, 6.0, 9.0, 10.0]);
}

#[test]
fn rejects_non_one_sample_data() {
    let error = FlashRawDecoder::from_parts(
        2,
        1,
        2,
        Some(CfaPattern::RGGB),
        RawPixelData::U16(vec![1, 2, 3, 4]),
        Rect::new(0, 0, 2, 1),
    )
    .unwrap_err();
    assert!(error.to_string().contains("one sample per pixel"), "{error}");
}

#[test]
fn rejects_missing_or_non_bayer_cfa_layouts() {
    for cfa in [None, CfaPattern::parse("RGBE")] {
        let error = FlashRawDecoder::from_parts(
            2,
            2,
            1,
            cfa,
            RawPixelData::U16(vec![1, 2, 3, 4]),
            Rect::new(0, 0, 2, 2),
        )
        .unwrap_err();
        assert!(error.to_string().contains("Bayer"), "{error}");
    }
}
