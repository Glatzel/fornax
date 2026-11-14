use std::fs;
#[cfg(feature = "mmap")]
use std::fs::File;
use std::path::Path;
#[cfg(any(target_os = "windows", target_os = "macos"))]
mod dnc;

use fornax_dng::Directory;
use fornax_dng::dng_decoder::DngImage;
use fornax_dng::metadata::IDecoderDng;
#[cfg(feature = "canon")]
use fornax_raw_canon::Cr3Image;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::{EncodingMetadata, ProcessMetadata};
use fornax_raw_core::protocol::{IDecoder, IDecoderEncoding};
use fornax_raw_core::{CroppedMosaic, Primitive, RawFormat, RawMosaic};
#[cfg(feature = "fujifilm")]
use fornax_raw_fujifilm::RafImage;
#[cfg(feature = "hasselblad")]
use fornax_raw_hasselblad::HasselbladImage;
#[cfg(feature = "nikon")]
use fornax_raw_nikon::NefImage;
#[cfg(feature = "panasonic")]
use fornax_raw_panasonic::Rw2Image;
#[cfg(feature = "phase-one")]
use fornax_raw_phase_one::IiqImage;
#[cfg(feature = "sony")]
use fornax_raw_sony::ArwImage;
use fornax_tiff::TiffSource;

#[derive(Clone, Debug)]
pub enum Decoder {
    Dng(DngImage),
    #[cfg(feature = "canon")]
    Cr3(Cr3Image),
    #[cfg(feature = "sony")]
    Arw(ArwImage),
    #[cfg(feature = "hasselblad")]
    Hasselblad(HasselbladImage),
    #[cfg(feature = "phase-one")]
    Iiq(IiqImage),
    #[cfg(feature = "nikon")]
    Nef(NefImage),
    #[cfg(feature = "fujifilm")]
    Raf(RafImage),
    #[cfg(feature = "panasonic")]
    Rw2(Rw2Image),
}

impl Decoder {
    /// Return the source encoding selected by the parsed decoder.
    pub fn encoding_metadata(&self) -> EncodingMetadata {
        match self {
            Self::Dng(parsed) => parsed.encoding_metadata(),
            #[cfg(feature = "canon")]
            Self::Cr3(parsed) => parsed.encoding_metadata(),
            #[cfg(feature = "sony")]
            Self::Arw(parsed) => parsed.encoding_metadata(),
            #[cfg(feature = "hasselblad")]
            Self::Hasselblad(parsed) => parsed.encoding_metadata(),
            #[cfg(feature = "phase-one")]
            Self::Iiq(parsed) => parsed.encoding_metadata(),
            #[cfg(feature = "nikon")]
            Self::Nef(parsed) => parsed.encoding_metadata(),
            #[cfg(feature = "fujifilm")]
            Self::Raf(parsed) => parsed.encoding_metadata(),
            #[cfg(feature = "panasonic")]
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
                #[cfg(feature = "canon")]
                {
                    clerk::debug!("Reading Canon CR3/CRM: {}", input.display());
                    let source = fs::read(input).map_err(|error| {
                        DecodeError::Other(
                            format!("reading {} failed: {error}", input.display()).into(),
                        )
                    })?;
                    Ok(Self::Cr3(Cr3Image::parse(&source)?))
                }
                #[cfg(not(feature = "canon"))]
                {
                    Err(disabled("canon"))
                }
            }
            Ok(RawFormat::Arw | RawFormat::Nrw) => {
                #[cfg(feature = "sony")]
                {
                    clerk::debug!("Reading Sony RAW: {}", input.display());
                    Ok(Self::Arw(ArwImage::decode(input)?))
                }
                #[cfg(not(feature = "sony"))]
                {
                    Err(disabled("sony"))
                }
            }
            Ok(RawFormat::Nef) => {
                #[cfg(feature = "nikon")]
                {
                    clerk::debug!("Reading Nikon NEF: {}", input.display());
                    Ok(Self::Nef(NefImage::decode(input)?))
                }
                #[cfg(not(feature = "nikon"))]
                {
                    Err(disabled("nikon"))
                }
            }
            Ok(RawFormat::Hasselblad3fr) => {
                #[cfg(feature = "hasselblad")]
                {
                    clerk::debug!("Reading Hasselblad 3FR: {}", input.display());
                    Ok(Self::Hasselblad(HasselbladImage::decode(input)?))
                }
                #[cfg(not(feature = "hasselblad"))]
                {
                    Err(disabled("hasselblad"))
                }
            }
            Ok(RawFormat::Iiq) => {
                #[cfg(feature = "phase-one")]
                {
                    clerk::debug!("Reading Phase One IIQ: {}", input.display());
                    Ok(Self::Iiq(IiqImage::decode(input)?))
                }
                #[cfg(not(feature = "phase-one"))]
                {
                    Err(disabled("phase-one"))
                }
            }
            Ok(RawFormat::Raf) => {
                #[cfg(feature = "fujifilm")]
                {
                    clerk::debug!("Reading Fujifilm RAF: {}", input.display());
                    Ok(Self::Raf(RafImage::decode(input)?))
                }
                #[cfg(not(feature = "fujifilm"))]
                {
                    Err(disabled("fujifilm"))
                }
            }
            Ok(RawFormat::Rw2) => {
                #[cfg(feature = "panasonic")]
                {
                    clerk::debug!("Reading Panasonic RW2: {}", input.display());
                    Ok(Self::Rw2(Rw2Image::decode(input)?))
                }
                #[cfg(not(feature = "panasonic"))]
                {
                    Err(disabled("panasonic"))
                }
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
            #[cfg(feature = "canon")]
            Self::Cr3(parsed) => parsed.raw_mosaic(),
            #[cfg(feature = "sony")]
            Self::Arw(parsed) => parsed.raw_mosaic(),
            #[cfg(feature = "hasselblad")]
            Self::Hasselblad(parsed) => parsed.raw_mosaic(),
            #[cfg(feature = "phase-one")]
            Self::Iiq(parsed) => parsed.raw_mosaic(),
            #[cfg(feature = "nikon")]
            Self::Nef(parsed) => parsed.raw_mosaic(),
            #[cfg(feature = "fujifilm")]
            Self::Raf(parsed) => parsed.raw_mosaic(),
            #[cfg(feature = "panasonic")]
            Self::Rw2(parsed) => parsed.raw_mosaic(),
        }
    }

    fn cropped_mosaic<T>(&self) -> Result<CroppedMosaic<T>, DecodeError>
    where
        T: Primitive,
    {
        match self {
            Self::Dng(parsed) => parsed.cropped_mosaic(),
            #[cfg(feature = "canon")]
            Self::Cr3(parsed) => parsed.cropped_mosaic(),
            #[cfg(feature = "sony")]
            Self::Arw(parsed) => parsed.cropped_mosaic(),
            #[cfg(feature = "hasselblad")]
            Self::Hasselblad(parsed) => parsed.cropped_mosaic(),
            #[cfg(feature = "phase-one")]
            Self::Iiq(parsed) => parsed.cropped_mosaic(),
            #[cfg(feature = "nikon")]
            Self::Nef(parsed) => parsed.cropped_mosaic(),
            #[cfg(feature = "fujifilm")]
            Self::Raf(parsed) => parsed.cropped_mosaic(),
            #[cfg(feature = "panasonic")]
            Self::Rw2(parsed) => parsed.cropped_mosaic(),
        }
    }

    fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        match self {
            Self::Dng(parsed) => parsed.process_metadata(),
            #[cfg(feature = "canon")]
            Self::Cr3(parsed) => parsed.process_metadata(),
            #[cfg(feature = "sony")]
            Self::Arw(parsed) => parsed.process_metadata(),
            #[cfg(feature = "hasselblad")]
            Self::Hasselblad(parsed) => parsed.process_metadata(),
            #[cfg(feature = "phase-one")]
            Self::Iiq(parsed) => parsed.process_metadata(),
            #[cfg(feature = "nikon")]
            Self::Nef(parsed) => parsed.process_metadata(),
            #[cfg(feature = "fujifilm")]
            Self::Raf(parsed) => parsed.process_metadata(),
            #[cfg(feature = "panasonic")]
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
            #[cfg(feature = "canon")]
            Self::Cr3(parsed) => parsed.dng_metadata(),
            #[cfg(feature = "sony")]
            Self::Arw(parsed) => parsed.dng_metadata(),
            #[cfg(feature = "hasselblad")]
            Self::Hasselblad(parsed) => parsed.dng_metadata(),
            #[cfg(feature = "phase-one")]
            Self::Iiq(parsed) => parsed.dng_metadata(),
            #[cfg(feature = "nikon")]
            Self::Nef(parsed) => parsed.dng_metadata(),
            #[cfg(feature = "fujifilm")]
            Self::Raf(parsed) => parsed.dng_metadata(),
            #[cfg(feature = "panasonic")]
            Self::Rw2(parsed) => parsed.dng_metadata(),
        }
    }
}

#[cfg(feature = "mmap")]
fn open_dng_source(path: &Path) -> Result<TiffSource, DecodeError> {
    let file = File::open(path).map_err(|error| {
        DecodeError::Other(format!("opening {} failed: {error}", path.display()).into())
    })?;
    // The mapping is read-only. As with any mmap, truncating the underlying
    // file while the decoder is alive can make accesses fault with SIGBUS.
    let mmap = unsafe { memmap2::Mmap::map(&file) }.map_err(|error| {
        DecodeError::Other(format!("mapping {} failed: {error}", path.display()).into())
    })?;
    Ok(TiffSource::from_mmap(mmap))
}

#[cfg(not(feature = "mmap"))]
fn open_dng_source(path: &Path) -> Result<TiffSource, DecodeError> {
    fs::read(path).map(TiffSource::from_vec).map_err(|error| {
        DecodeError::Other(format!("reading {} failed: {error}", path.display()).into())
    })
}

#[allow(
    dead_code,
    reason = "used only by dispatch arms whose optional format feature is disabled"
)]
fn disabled(feature: &'static str) -> DecodeError {
    DecodeError::Unsupported(
        format!("this RAW format requires the `{feature}` decoder feature").into(),
    )
}
