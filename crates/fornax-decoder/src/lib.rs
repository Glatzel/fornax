mod dng_decoder;

use std::fs;
use std::marker::PhantomData;
use std::path::Path;

use fornax_core::error::DecodeError;
use fornax_core::metadata::dng::DngMetadata;
use fornax_core::metadata::process::ProcessMetadata;
use fornax_core::protocol::IDecoder;
use fornax_core::{CroppedMosaic, Primitive, RawFormat, RawMosaic};

#[derive(Clone, Debug)]
pub enum Decoder<T> {
    Dng(dng_decoder::ParsedTiff),
    _Mark(PhantomData<T>),
}
impl<T> IDecoder<&Path, T> for Decoder<T>
where
    T: Primitive,
{
    fn decode(input: &Path) -> Result<Self, DecodeError> {
        if let Some(extension) = input.extension() {
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
            Self::Dng(parsed) => dng_decoder::raw_mosaic(parsed),
            Self::_Mark(_) => unreachable!(),
        }
    }

    fn cropped_mosaic(&self) -> Result<CroppedMosaic<T>, DecodeError> {
        match self {
            Self::Dng(parsed) => dng_decoder::cropped_mosaic(parsed),
            Self::_Mark(_) => unreachable!(),
        }
    }

    fn dng_metadata(&self) -> Result<DngMetadata, DecodeError> {
        match self {
            Self::Dng(parsed) => dng_decoder::dng_metadata(parsed),
            Self::_Mark(_) => unreachable!(),
        }
    }

    fn process_metadata(&self) -> Result<ProcessMetadata<T>, DecodeError> {
        match self {
            Self::Dng(parsed) => dng_decoder::process_metadata(parsed),
            Self::_Mark(_) => unreachable!(),
        }
    }
}
