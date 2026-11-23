use std::fs;
use std::path::Path;
mod decoders;
#[cfg(any(target_os = "windows", target_os = "macos"))]
mod dnc;

use decoders::canon::Cr3Image;
use decoders::fujifilm::RafImage;
use decoders::hasselblad::HasselbladImage;
use decoders::nikon::NefImage;
use decoders::panasonic::Rw2Image;
use decoders::phase_one::IiqImage;
use decoders::sony::ArwImage;
pub use decoders::{canon, fujifilm, hasselblad, nikon, panasonic, phase_one, sony};
use fornax_dng::Directory;
use fornax_dng::dng_decoder::DngImage;
use fornax_dng::metadata::IDecoderDng;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::{EncodingMetadata, ProcessMetadata};
use fornax_raw_core::protocol::{IDecoder, IDecoderEncoding};
use fornax_raw_core::{CroppedMosaic, RawFloat, RawFormat, RawMosaic};
use fornax_tiff::TiffSource;

#[derive(Clone, Debug)]
pub enum Decoder {
    Dng(DngImage),
    Cr3(Cr3Image),
    Arw(ArwImage),
    Hasselblad(HasselbladImage),
    Iiq(IiqImage),
    Nef(NefImage),
    Raf(RafImage),
    Rw2(Rw2Image),
}

impl Decoder {
    /// Return the source encoding selected by the parsed decoder.
    pub fn encoding_metadata(&self) -> EncodingMetadata {
        match self {
            Self::Dng(parsed) => parsed.encoding_metadata(),
            Self::Cr3(parsed) => parsed.encoding_metadata(),
            Self::Arw(parsed) => parsed.encoding_metadata(),
            Self::Hasselblad(parsed) => parsed.encoding_metadata(),
            Self::Iiq(parsed) => parsed.encoding_metadata(),
            Self::Nef(parsed) => parsed.encoding_metadata(),
            Self::Raf(parsed) => parsed.encoding_metadata(),
            Self::Rw2(parsed) => parsed.encoding_metadata(),
        }
    }
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
                Ok(Self::Dng(DngImage::parse_source(open_dng_source(input)?)?))
            }
            Ok(RawFormat::Cr3 | RawFormat::Crm) => {
                clerk::debug!("Reading Canon CR3/CRM: {}", input.display());
                let source = fs::read(input).map_err(|error| {
                    DecodeError::Other(
                        format!("reading {} failed: {error}", input.display()).into(),
                    )
                })?;
                Ok(Self::Cr3(Cr3Image::parse(&source)?))
            }
            Ok(RawFormat::Arw | RawFormat::Nrw) => {
                clerk::debug!("Reading Sony RAW: {}", input.display());
                Ok(Self::Arw(ArwImage::decode(input)?))
            }
            Ok(RawFormat::Nef) => {
                clerk::debug!("Reading Nikon NEF: {}", input.display());
                Ok(Self::Nef(NefImage::decode(input)?))
            }
            Ok(RawFormat::Hasselblad3fr) => {
                clerk::debug!("Reading Hasselblad 3FR: {}", input.display());
                Ok(Self::Hasselblad(HasselbladImage::decode(input)?))
            }
            Ok(RawFormat::Iiq) => {
                clerk::debug!("Reading Phase One IIQ: {}", input.display());
                Ok(Self::Iiq(IiqImage::decode(input)?))
            }
            Ok(RawFormat::Raf) => {
                clerk::debug!("Reading Fujifilm RAF: {}", input.display());
                Ok(Self::Raf(RafImage::decode(input)?))
            }
            Ok(RawFormat::Rw2) => {
                clerk::debug!("Reading Panasonic RW2: {}", input.display());
                Ok(Self::Rw2(Rw2Image::decode(input)?))
            }
            _ => {
                cfg_select! {
                    any(target_os = "windows", target_os = "macos") => {
                        clerk::info!(
                            "Fallback to use `Adobe DNG Converter` to convert RAW image to DNG: {}",
                            input.display()
                        );
                        let dng_file = dnc::Dnc::convert(input)?;
                        Ok(Self::Dng(DngImage::parse_source(open_dng_source(
                            &dng_file,
                        )?)?))
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
            Self::Cr3(parsed) => parsed.raw_mosaic(),
            Self::Arw(parsed) => parsed.raw_mosaic(),
            Self::Hasselblad(parsed) => parsed.raw_mosaic(),
            Self::Iiq(parsed) => parsed.raw_mosaic(),
            Self::Nef(parsed) => parsed.raw_mosaic(),
            Self::Raf(parsed) => parsed.raw_mosaic(),
            Self::Rw2(parsed) => parsed.raw_mosaic(),
        }
    }

    fn cropped_mosaic<T>(&self) -> Result<CroppedMosaic<T>, DecodeError>
    where
        T: RawFloat,
    {
        match self {
            Self::Dng(parsed) => parsed.cropped_mosaic(),
            Self::Cr3(parsed) => parsed.cropped_mosaic(),
            Self::Arw(parsed) => parsed.cropped_mosaic(),
            Self::Hasselblad(parsed) => parsed.cropped_mosaic(),
            Self::Iiq(parsed) => parsed.cropped_mosaic(),
            Self::Nef(parsed) => parsed.cropped_mosaic(),
            Self::Raf(parsed) => parsed.cropped_mosaic(),
            Self::Rw2(parsed) => parsed.cropped_mosaic(),
        }
    }

    fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        match self {
            Self::Dng(parsed) => parsed.process_metadata(),
            Self::Cr3(parsed) => parsed.process_metadata(),
            Self::Arw(parsed) => parsed.process_metadata(),
            Self::Hasselblad(parsed) => parsed.process_metadata(),
            Self::Iiq(parsed) => parsed.process_metadata(),
            Self::Nef(parsed) => parsed.process_metadata(),
            Self::Raf(parsed) => parsed.process_metadata(),
            Self::Rw2(parsed) => parsed.process_metadata(),
        }
    }
}

impl IDecoderEncoding for Decoder {
    fn encoding_metadata(&self) -> EncodingMetadata { Self::encoding_metadata(self) }
}

impl IDecoderDng for Decoder {
    fn dng_metadata(&self) -> Result<Directory, DecodeError> {
        match self {
            Self::Dng(parsed) => parsed.dng_metadata(),
            Self::Cr3(parsed) => parsed.dng_metadata(),
            Self::Arw(parsed) => parsed.dng_metadata(),
            Self::Hasselblad(parsed) => parsed.dng_metadata(),
            Self::Iiq(parsed) => parsed.dng_metadata(),
            Self::Nef(parsed) => parsed.dng_metadata(),
            Self::Raf(parsed) => parsed.dng_metadata(),
            Self::Rw2(parsed) => parsed.dng_metadata(),
        }
    }
}

fn open_dng_source(path: &Path) -> Result<TiffSource, DecodeError> {
    TiffSource::from_path(path).map_err(|error| {
        DecodeError::Other(format!("opening {} failed: {error}", path.display()).into())
    })
}
