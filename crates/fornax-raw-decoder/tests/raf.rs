#![cfg(feature = "fujifilm")]

use std::path::PathBuf;

use fornax_raw_core::error::DecodeError;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_decoder::Decoder;

#[test]
fn raf_extension_routes_to_the_native_reader() {
    let path = PathBuf::from(format!(
        "{}/fornax-missing-{}.RAF",
        std::env::temp_dir().display(),
        std::process::id()
    ));
    let error = Decoder::decode(&path).unwrap_err();

    assert!(
        matches!(error, DecodeError::Other(_)),
        "expected native RAF read error, got {error:?}"
    );
}
