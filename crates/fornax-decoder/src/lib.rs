use std::fs;
use std::marker::PhantomData;
use std::path::Path;

use fornax_core::error::DecodeError;
use fornax_core::metadata::ProcessMetadata;
use fornax_core::protocol::IDecoder;
use fornax_core::{CroppedMosaic, Primitive, RawFormat, RawMosaic};
use fornax_dng::metadata::IDecoderDng;
use fornax_dng::{Directory, DngImage};

#[derive(Clone, Debug)]
pub enum Decoder<T> {
    Dng(DngImage),
    _Mark(PhantomData<T>),
}
impl<T> IDecoder<&Path, T> for Decoder<T>
where
    T: Primitive,
{
    fn decode(input: &Path) -> Result<Self, DecodeError> {
        let extension = input
            .extension()
            .ok_or_else(|| DecodeError::Unsupported("DNG input has no file extension".into()))?;
        let extension = extension
            .to_str()
            .ok_or_else(|| DecodeError::InvalidInput("file extension is not UTF-8".into()))?;
        match RawFormat::try_from(extension) {
            Ok(RawFormat::Dng) => {
                let source = fs::read(input).map_err(|error| {
                    DecodeError::Other(
                        format!("reading {} failed: {error}", input.display()).into(),
                    )
                })?;
                Ok(Self::Dng(DngImage::parse(&source)?))
            }
            _ => Err(DecodeError::Unsupported(extension.into())),
        }
    }

    fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> {
        match self {
            Self::Dng(parsed) => fornax_dng::raw_mosaic(parsed),
            Self::_Mark(_) => unreachable!(),
        }
    }

    fn cropped_mosaic(&self) -> Result<CroppedMosaic<T>, DecodeError> {
        match self {
            Self::Dng(parsed) => fornax_dng::cropped_mosaic(parsed),
            Self::_Mark(_) => unreachable!(),
        }
    }

    fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        match self {
            Self::Dng(parsed) => fornax_dng::process_metadata(parsed),
            Self::_Mark(_) => unreachable!(),
        }
    }
}
impl<T> IDecoderDng<&Path, T> for Decoder<T>
where
    T: Primitive,
{
    fn dng_metadata(&self) -> Result<Directory, DecodeError> {
        match self {
            Self::Dng(parsed) => fornax_dng::dng_metadata(parsed),
            Self::_Mark(_) => unreachable!(),
        }
    }
}
