use std::path::Path;

use fornax_raw::RawMosaic;
use fornax_raw::decoder::Decoder;
use fornax_raw::dng::metadata::IDecoderDng;
use fornax_raw::protocol::IDecoder;

#[test]
fn rw2_routes_to_the_native_panasonic_decoder() {
    let Some(root) = std::env::var_os("FORNAX_RAW_CORPUS") else {
        return;
    };
    let path = Path::new(&root).join("Panasonic/DMC-G85/_1030982.RW2");
    let decoder = Decoder::decode(&path).unwrap();

    assert!(matches!(decoder, Decoder::Rw2(_)));
    assert!(matches!(decoder.raw_mosaic().unwrap(), RawMosaic::U16(_)));
    assert!(decoder.cropped_mosaic::<f32>().unwrap().image().width() > 0);
    assert!(decoder.process_metadata().unwrap().active_area.width > 0);
    assert!(!decoder.dng_metadata().unwrap().is_empty());
}

#[test]
fn rw2_facade_propagates_typed_unsupported_layouts() {
    let Some(root) = std::env::var_os("FORNAX_RAW_CORPUS") else {
        return;
    };
    let path = Path::new(&root).join("Panasonic/DC-S5/dc-s5_hr_96MP.RW2");

    let error = Decoder::decode(&path).unwrap_err();

    assert!(matches!(
        error,
        fornax_raw::error::DecodeError::Unsupported(_)
    ));
}
