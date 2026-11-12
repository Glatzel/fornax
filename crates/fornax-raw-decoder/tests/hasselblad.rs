#![cfg(feature = "hasselblad")]

use std::path::PathBuf;

use fornax_dng::metadata::IDecoderDng;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_decoder::Decoder;

#[test]
fn tfr_extension_routes_to_the_native_reader() {
    let path = PathBuf::from(format!(
        "{}/fornax-missing-{}.3FR",
        std::env::temp_dir().display(),
        std::process::id()
    ));
    let error = Decoder::decode(&path).unwrap_err();

    assert!(
        matches!(error, DecodeError::Other(_)),
        "expected native 3FR read error, got {error:?}"
    );
}

#[test]
fn native_tfr_dispatch_forwards_samples_metadata_and_handoff() {
    let Some(root) = std::env::var_os("FORNAX_RAW_SAMPLES").map(PathBuf::from) else {
        return;
    };
    for relative in ["Hasselblad/H3D/A9391120.3FR", "Hasselblad/X1D/B9999910.3FR"] {
        let path = root.join(relative);
        let Ok(source) = std::fs::read(&path) else {
            continue;
        };
        if source.starts_with(b"version https://git-lfs.github.com/spec/v1") {
            continue;
        }

        let decoder = Decoder::decode(&path).unwrap();
        let raw = decoder.raw_mosaic().unwrap();
        let process = decoder.process_metadata().unwrap();
        let cropped = decoder.cropped_mosaic::<f32>().unwrap();

        assert!(raw.dimensions().0 >= process.active_area.width);
        assert_eq!(
            cropped.image().dimensions(),
            (process.active_area.width, process.active_area.height)
        );
        assert!(decoder.dng_metadata().unwrap().len() > 20);
    }
}
