use std::fs;
use std::path::Path;
#[cfg(any(target_os = "windows", target_os = "macos"))]
mod dnc;

use fornax_arw::ArwImage;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::ProcessMetadata;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_core::{CroppedMosaic, Primitive, RawFormat, RawMosaic};
use fornax_dng::Directory;
use fornax_dng::dng_decoder::DngImage;
use fornax_dng::metadata::IDecoderDng;

#[derive(Clone, Debug)]
pub enum Decoder {
    Dng(DngImage),
    Arw(ArwImage),
}
impl IDecoder for Decoder {
    fn decode(input: &Path) -> Result<Self, DecodeError>
    where
        Self: Sized,
    {
        let extension = input
            .extension()
            .ok_or_else(|| DecodeError::Unsupported("DNG input has no file extension".into()))?;
        let extension = extension
            .to_str()
            .ok_or_else(|| DecodeError::InvalidInput("file extension is not UTF-8".into()))?;

        match RawFormat::try_from(extension) {
            Ok(RawFormat::Dng) => {
                clerk::debug!("Reading DNG: {}", input.display());
                let source = fs::read(input).map_err(|error| {
                    DecodeError::Other(
                        format!("reading {} failed: {error}", input.display()).into(),
                    )
                })?;
                Ok(Self::Dng(DngImage::parse(&source)?))
            }
            Ok(RawFormat::Arw | RawFormat::Nrw) => {
                clerk::debug!("Reading Sony RAW: {}", input.display());
                Ok(Self::Arw(ArwImage::decode(input)?))
            }
            _ => {
                cfg_select! {
                    any(target_os = "windows", target_os = "macos") => {
                        clerk::info!(
                            "Fallback to use `Adobe DNG Converter` to convert RAW image to DNG: {}",
                            input.display()
                        );
                        let dng_file = dnc::Dnc::convert(input)?;
                        let source = fs::read(&dng_file).map_err(|error| {
                            DecodeError::Other(
                                format!("reading {} failed: {error}", dng_file.display()).into(),
                            )
                        })?;
                        Ok(Self::Dng(DngImage::parse(&source)?))
                    }
                    _ => {
                        clerk::error!("Unsupported RAW format: {}", extension);
                        Err(DecodeError::Unsupported(extension.into()))
                    }
                }
            }
        }
    }

    fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> {
        match self {
            Self::Dng(parsed) => parsed.raw_mosaic(),
            Self::Arw(parsed) => parsed.raw_mosaic(),
        }
    }

    fn cropped_mosaic<T>(&self) -> Result<CroppedMosaic<T>, DecodeError>
    where
        T: Primitive,
    {
        match self {
            Self::Dng(parsed) => parsed.cropped_mosaic(),
            Self::Arw(parsed) => parsed.cropped_mosaic(),
        }
    }

    fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        match self {
            Self::Dng(parsed) => parsed.process_metadata(),
            Self::Arw(parsed) => parsed.process_metadata(),
        }
    }
}
impl IDecoderDng for Decoder {
    fn dng_metadata(&self) -> Result<Directory, DecodeError> {
        match self {
            Self::Dng(parsed) => parsed.dng_metadata(),
            Self::Arw(parsed) => parsed.dng_metadata(),
        }
    }
}
