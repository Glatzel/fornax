#[path = "../benches/reference_preflight.rs"]
mod reference_preflight;

use std::path::{Path, PathBuf};

use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::ProcessMetadata;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_core::{CroppedMosaic, Primitive, RawMosaic};
use reference_preflight::{ReferencePreflight, preflight_reference};

struct FakeDecoder {
    raw: Result<RawMosaic, DecodeError>,
}

impl IDecoder for FakeDecoder {
    fn decode(input: &Path) -> Result<Self, DecodeError> {
        match input.file_name().and_then(|name| name.to_str()) {
            Some("decode-unsupported.raw") => Err(DecodeError::Unsupported("no decode".into())),
            Some("decode-backend.raw") => Err(DecodeError::Backend("backend".into())),
            Some("raw-unsupported.raw") => Ok(Self {
                raw: Err(DecodeError::Unsupported("no raw mosaic".into())),
            }),
            Some("raw-invalid.raw") => Ok(Self {
                raw: Err(DecodeError::InvalidInput("malformed output".into())),
            }),
            _ => Ok(Self {
                raw: Ok(RawMosaic::U16(
                    fornax_raw_core::MonoImage::from_raw(1, 1, vec![1]).unwrap(),
                )),
            }),
        }
    }

    fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> { self.raw.clone() }

    fn cropped_mosaic<T>(&self) -> Result<CroppedMosaic<T>, DecodeError>
    where
        T: Primitive,
    {
        panic!("reference preflight must not run cropped_mosaic")
    }

    fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        Err(DecodeError::Unsupported("not implemented".into()))
    }
}

fn preflight(name: &str) -> Result<ReferencePreflight<FakeDecoder>, DecodeError> {
    preflight_reference::<FakeDecoder>(&PathBuf::from(name))
}

#[test]
fn ready_requires_both_decode_and_raw_mosaic_support() {
    assert!(matches!(
        preflight("ready.raw").unwrap(),
        ReferencePreflight::Ready(_)
    ));
}

#[test]
fn typed_unsupported_decode_or_raw_mosaic_is_skippable() {
    assert!(matches!(
        preflight("decode-unsupported.raw").unwrap(),
        ReferencePreflight::Unsupported(detail) if detail.as_ref() == "no decode"
    ));
    assert!(matches!(
        preflight("raw-unsupported.raw").unwrap(),
        ReferencePreflight::Unsupported(detail) if detail.as_ref() == "no raw mosaic"
    ));
}

#[test]
fn backend_and_malformed_output_fail_the_preflight() {
    assert!(matches!(
        preflight("decode-backend.raw"),
        Err(DecodeError::Backend(_))
    ));
    assert!(matches!(
        preflight("raw-invalid.raw"),
        Err(DecodeError::InvalidInput(_))
    ));
}
