#![cfg(feature = "canon")]

use std::fs;

use fornax_dng::metadata::IDecoderDng;
use fornax_raw_canon::CanonCrxCfa;
use fornax_raw_core::RawMosaic;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_decoder::Decoder;

#[test]
fn non_2x2_layouts_are_rejected_before_pipeline_handoff() {
    let error = CanonCrxCfa::from_cmp1_layout(4).unwrap_err();
    assert!(matches!(error, fornax_codec::CodecError::Unsupported(_)));
}

#[test]
fn lossless_cr3_dispatches_through_the_public_decoder() {
    let Ok(path) = std::env::var("FORNAX_CR3_LOSSLESS_FIXTURE") else {
        return;
    };
    let temporary = fixture_copy(&path, "lossless", "cr3");
    let decoder = Decoder::decode(&temporary).unwrap();
    let _ = fs::remove_file(&temporary);
    assert!(matches!(decoder.raw_mosaic().unwrap(), RawMosaic::U16(_)));
    assert!(decoder.cropped_mosaic::<f32>().unwrap().image().width() > 1);
    assert!(!decoder.dng_metadata().unwrap().is_empty());
    assert!(decoder.process_metadata().unwrap().active_area.width > 1);
}

#[test]
fn craw_returns_typed_lossy_error_through_the_public_decoder() {
    let Ok(path) = std::env::var("FORNAX_CR3_CRAW_FIXTURE") else {
        return;
    };
    let temporary = fixture_copy(&path, "craw", "cr3");
    let error = Decoder::decode(&temporary).unwrap_err();
    let _ = fs::remove_file(&temporary);
    assert!(matches!(error, DecodeError::Unsupported(_)));
}

#[test]
fn crm_extension_dispatches_to_the_cr3_decoder() {
    let Ok(path) = std::env::var("FORNAX_CR3_LOSSLESS_FIXTURE") else {
        return;
    };
    let temporary = fixture_copy(&path, "lossless", "crm");
    let result = Decoder::decode(&temporary);
    let _ = fs::remove_file(&temporary);
    assert!(result.is_ok());
}

fn fixture_copy(path: &str, label: &str, extension: &str) -> std::path::PathBuf {
    let temporary = std::env::temp_dir().join(format!(
        "fornax-{label}-{}-{}.{extension}",
        std::process::id(),
        std::thread::current()
            .name()
            .unwrap_or("test")
            .replace("::", "-")
    ));
    fs::copy(path, &temporary).unwrap();
    temporary
}
