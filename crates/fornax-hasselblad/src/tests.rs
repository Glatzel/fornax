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
