#![cfg(feature = "nikon")]

use std::io::Cursor;
use std::path::PathBuf;

use fornax_raw_core::error::DecodeError;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_decoder::Decoder;
use fornax_tiff::TiffWriter;

#[test]
fn nef_extension_routes_to_the_native_reader() {
    let path = PathBuf::from(format!(
        "{}/fornax-missing-{}.NEF",
        std::env::temp_dir().display(),
        std::process::id()
    ));
    let error = Decoder::decode(&path).unwrap_err();

    assert!(
        matches!(error, DecodeError::Other(_)),
        "expected native NEF read error, got {error:?}"
    );
}

#[test]
fn nikon_nrw_stays_on_the_sony_route() {
    let path = PathBuf::from(format!(
        "{}/fornax-nikon-{}.NRW",
        std::env::temp_dir().display(),
        std::process::id()
    ));
    let mut output = Cursor::new(Vec::new());
    write_nikon_tiff(&mut output);
    std::fs::write(&path, output.into_inner()).unwrap();
    let error = Decoder::decode(&path).unwrap_err();
    std::fs::remove_file(path).unwrap();

    assert!(
        error.to_string().contains("not a Sony image"),
        "expected Sony route rejection, got {error:?}"
    );
}

fn write_nikon_tiff(output: &mut Cursor<Vec<u8>>) {
    let mut writer = TiffWriter::new(output).unwrap();
    writer
        .finish([(
            fornax_tiff::Tag::from(fornax_tiff::constants::TiffTag::Make),
            fornax_tiff::Value::Ascii("NIKON CORPORATION".into()),
        )])
        .unwrap();
}
