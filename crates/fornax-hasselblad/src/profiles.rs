#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) struct ModelProfile {
    pub cfa: [u8; 4],
    /// Left, top, right, and bottom inactive borders relative to the full
    /// frame.
    pub borders: [u32; 4],
}

pub(crate) fn profile(unique_model: &str, model: &str) -> Option<ModelProfile> {
    let key = unique_model.trim();
    let borders = match key {
        "Hasselblad X1D" | "Hasselblad X1D II 50C" => [46, 96, 58, 0],
        "Hasselblad H3D"
        | "Hasselblad H4D"
        | "Hasselblad CFV"
        | "Hasselblad96"
        | "Hasselblad ixpressCF132"
        | "Hasselblad X2D 100C"
        | "Hasselblad CFV 100C/Electronic Shutter" => [0, 0, 0, 0],
        _ if model.trim().eq_ignore_ascii_case("Hasselblad 500 mech.") => [0, 0, 0, 0],
        _ => return None,
    };
    Some(ModelProfile {
        cfa: [0, 1, 1, 2],
        borders,
    })
}
