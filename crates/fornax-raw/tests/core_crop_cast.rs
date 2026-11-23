use fornax_core::MonoImage;
use fornax_raw::crop_and_cast;
use image::math::Rect;

#[test]
fn crop_and_cast_uses_half_open_bounds_and_converts_samples() {
    let image = MonoImage::from_raw(4, 3, (0_u16..12).collect()).unwrap();

    let cropped = crop_and_cast::<u16, f32>(image, Rect::from_xy_ranges(1..3, 1..3)).unwrap();

    assert_eq!(cropped.image().dimensions(), (2, 2));
    assert_eq!(cropped.image().as_raw(), &[5.0, 6.0, 9.0, 10.0]);
}

#[test]
fn every_decoder_adapter_uses_the_shared_crop_and_cast() {
    let adapters = [
        include_str!("../src/dng/dng_decoder/mod.rs"),
        include_str!("../src/decoders/canon/mod.rs"),
        include_str!("../src/decoders/sony/mod.rs"),
        include_str!("../src/decoders/hasselblad/mod.rs"),
        include_str!("../src/decoders/nikon/mod.rs"),
        include_str!("../src/decoders/panasonic/mod.rs"),
        include_str!("../src/decoders/fujifilm/mod.rs"),
    ];

    for source in adapters {
        assert!(source.contains("crop_and_cast(image,"));
        assert!(!source.contains("fn crop_and_cast"));
    }
    assert!(include_str!("../src/crop_cast.rs").contains("zeroed_samples"));
}
