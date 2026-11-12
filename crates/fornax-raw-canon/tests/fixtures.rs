use std::fs;
use std::io::Cursor;

use fornax_dng::writer::{DngCompression, DngWriter};
use fornax_raw_canon::Cr3Image;
use fornax_raw_core::RawMosaic;
use sha2::{Digest, Sha256};

#[test]
fn targeted_lossless_fixture_parses_and_decodes() {
    let Ok(path) = std::env::var("FORNAX_CR3_LOSSLESS_FIXTURE") else {
        return;
    };
    let bytes = fs::read(path).unwrap();
    let image = Cr3Image::parse(&bytes).unwrap();
    let RawMosaic::U16(mosaic) = image.raw_mosaic().unwrap() else {
        panic!("CR3 lossless decode must return U16 samples");
    };
    assert!(mosaic.width() > 1);
    assert!(mosaic.height() > 1);
    let digest = Sha256::digest(
        mosaic
            .as_raw()
            .iter()
            .flat_map(|sample| sample.to_le_bytes())
            .collect::<Vec<_>>(),
    );
    let digest = digest
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect::<String>();
    if let Ok(expected) = std::env::var("FORNAX_CR3_EXPECTED_SHA256") {
        assert_eq!(digest, expected);
    }
    let cropped = image.cropped_mosaic::<f32>().unwrap();
    assert_eq!(
        (cropped.image().width(), cropped.image().height()),
        (
            image.process_metadata().unwrap().active_area.width,
            image.process_metadata().unwrap().active_area.height
        )
    );
    assert!(image.dng_metadata().unwrap().len() > 4);
    assert!(image.process_metadata().unwrap().active_area.width > 1);
    let mosaic = image.raw_mosaic().unwrap();
    let mut dng = Cursor::new(Vec::new());
    DngWriter::new(DngCompression::Uncompressed)
        .write_dng_with(&mut dng, &mosaic, &image.dng_metadata().unwrap())
        .unwrap();
    let parsed = fornax_tiff::parse(dng.get_ref()).unwrap();
    assert!(!parsed.root.is_empty());
}

#[test]
fn targeted_craw_fixture_returns_typed_unsupported_error() {
    let Ok(path) = std::env::var("FORNAX_CR3_CRAW_FIXTURE") else {
        return;
    };
    let bytes = fs::read(path).unwrap();
    let error = Cr3Image::parse(&bytes).unwrap_err();
    assert!(matches!(
        error,
        fornax_raw_core::error::DecodeError::Unsupported(_)
    ));
}
