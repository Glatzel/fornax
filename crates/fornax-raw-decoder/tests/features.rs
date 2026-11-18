use std::path::Path;

use fornax_raw_core::error::DecodeError;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_decoder::Decoder;

#[test]
fn every_known_vendor_format_routes_without_a_format_feature() {
    let cases = [
        "photo.ARW",
        "photo.CR3",
        "photo.NEF",
        "photo.RAF",
        "photo.3FR",
        "photo.RW2",
        "photo.IIQ",
    ];

    for path in cases {
        let error = Decoder::decode(Path::new(path)).unwrap_err();
        assert!(
            matches!(error, DecodeError::Other(_)),
            "{path} did not reach its native reader: {error:?}"
        );
    }
}
