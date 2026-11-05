mod dng_decoder;

use std::fs;
use std::path::Path;

use fornax_core::error::DecodeError;
use fornax_core::metadata::dng::DngMetadata;
use fornax_core::metadata::process::ProcessMetadata;
use fornax_core::protocol::IDecoder;
use fornax_core::{CroppedMosaic, Primitive, RawFormat, RawMosaic};

#[derive(Clone, Debug)]
pub enum Decoder {
    Dng(dng_decoder::ParsedTiff),
}
impl<P, T> IDecoder<P, T> for Decoder
where
    T: Primitive,
    P: AsRef<Path>,
{
    fn decode(input: P) -> Result<Self, DecodeError> {
        if let Some(extension) = input.as_ref().extension() {
            let decoder = match RawFormat::try_from(extension.to_str().unwrap()) {
                Ok(RawFormat::Dng) => Self::Dng(dng_decoder::decode(&fs::read(input).unwrap())?),
                _ => {
                    return Err(DecodeError::Unsupported(
                        format!("{}", extension.display()).into(),
                    ));
                }
            };
            Ok(decoder)
        } else {
            Err(DecodeError::Unsupported("Unknown extension.".into()))
        }
    }

    fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> {
        match self {
            Decoder::Dng(parsed) => dng_decoder::raw_mosaic(parsed),
        }
    }

    fn cropped_mosaic(&self) -> Result<CroppedMosaic<T>, DecodeError> {
        match self {
            Decoder::Dng(parsed) => dng_decoder::cropped_mosaic(parsed),
        }
    }

    fn dng_metadata(&self) -> Result<DngMetadata, DecodeError> {
        match self {
            Decoder::Dng(parsed) => dng_decoder::dng_metadata(parsed),
        }
    }

    fn process_metadata(&self) -> Result<ProcessMetadata<T>, DecodeError> {
        match self {
            Decoder::Dng(parsed) => dng_decoder::process_metadata(parsed),
        }
    }
}
