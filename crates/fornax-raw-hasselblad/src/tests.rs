use fornax_raw_core::EncodingSampleFormat;

use super::profiles::profile;

#[test]
fn accepted_models_have_explicit_rgb_profiles() {
    for model in [
        "Hasselblad H3D",
        "Hasselblad H4D",
        "Hasselblad CFV",
        "Hasselblad96",
        "Hasselblad ixpressCF132",
        "Hasselblad X2D 100C",
        "Hasselblad CFV 100C/Electronic Shutter",
    ] {
        let profile =
            profile(model, model).unwrap_or_else(|| panic!("missing profile for {model}"));
        assert_eq!(profile.cfa, [0, 1, 1, 2]);
        assert_eq!(profile.borders, [0, 0, 0, 0]);
    }
}

#[test]
fn x1d_profiles_remove_the_inactive_sensor_borders() {
    for model in ["Hasselblad X1D", "Hasselblad X1D II 50C"] {
        let profile =
            profile(model, model).unwrap_or_else(|| panic!("missing profile for {model}"));
        assert_eq!(profile.cfa, [0, 1, 1, 2]);
        assert_eq!(profile.borders, [46, 96, 58, 0]);
    }
}

#[test]
fn cfv_fallback_model_has_a_profile() {
    let profile = profile("Hasselblad96", "Hasselblad 500 mech.").unwrap();

    assert_eq!(profile.cfa, [0, 1, 1, 2]);
    assert_eq!(profile.borders, [0, 0, 0, 0]);
}

#[test]
fn encoding_metadata_distinguishes_hasselblad_compression() {
    for (compression, expected_name) in [
        (super::container::Compression::Uncompressed, "uncompressed"),
        (
            super::container::Compression::LosslessJpeg,
            "hasselblad-lossless-jpeg",
        ),
    ] {
        let layout = super::container::RawLayout {
            compression,
            width: 64,
            height: 32,
            offset: 0,
            byte_count: 128,
        };

        let encoding = super::encoding_metadata_for_layout(&layout);

        assert_eq!(
            encoding.compression_id,
            Some(u32::from(match compression {
                super::container::Compression::Uncompressed => 1_u16,
                super::container::Compression::LosslessJpeg => 7_u16,
            }))
        );
        assert_eq!(encoding.compression_name, expected_name);
        assert!(encoding.lossless);
        assert_eq!(encoding.storage, "strips");
        assert_eq!(encoding.bits_per_sample, Some(16));
        assert_eq!(encoding.sample_format, Some(EncodingSampleFormat::Integer));
    }
}
