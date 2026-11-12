use std::path::Path;

use fornax_raw_core::error::DecodeError;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_decoder::Decoder;

#[test]
fn disabled_known_formats_name_their_required_feature_before_io() {
    let cases: Vec<(&str, &str)> = vec![
        #[cfg(not(feature = "sony"))]
        ("photo.ARW", "sony"),
        #[cfg(not(feature = "canon"))]
        ("photo.CR3", "canon"),
        #[cfg(not(feature = "nikon"))]
        ("photo.NEF", "nikon"),
        #[cfg(not(feature = "fujifilm"))]
        ("photo.RAF", "fujifilm"),
        #[cfg(not(feature = "hasselblad"))]
        ("photo.3FR", "hasselblad"),
        #[cfg(not(feature = "panasonic"))]
        ("photo.RW2", "panasonic"),
        #[cfg(not(feature = "phase-one"))]
        ("photo.IIQ", "phase-one"),
    ];

    for (path, feature) in cases {
        let error = Decoder::decode(Path::new(path)).unwrap_err();
        assert!(
            matches!(
                error,
                DecodeError::Unsupported(detail) if detail.contains(feature)
            ),
            "{path} did not report the disabled {feature} feature"
        );
    }
}
