//! Phase One and Leaf IIQ container and sample decoding.
//!
//! # Examples
//!
//! ```no_run
//! use fornax_iiq::IiqImage;
//!
//! let bytes = std::fs::read("photo.IIQ")?;
//! let decoder = IiqImage::parse(&bytes)?;
//! assert!(decoder.raw_mosaic()?.dimensions().0 > 0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod container;
mod metadata;
mod profiles;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::Path;

use container::{MakerNote, RawLayout};
use fornax_codec::{ByteOrder, DecodeRequest, Predictor, SampleBuffer, SampleFormat};
use fornax_dng::metadata::IDecoderDng;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::ProcessMetadata;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_core::{CroppedMosaic, MonoImage, Primitive, RawMosaic};
use fornax_tiff::{Directory, ParsedTiff};
use generic_num::num;
use image::math::Rect;
use rayon::prelude::*;

/// A parsed Phase One or Leaf IIQ image.
///
/// The decoder exposes storage-native corrected samples, DNG-compatible
/// metadata, and normalized process metadata for conventional 2x2 Bayer input.
///
/// # Examples
///
/// ```no_run
/// use fornax_iiq::IiqImage;
///
/// let bytes = std::fs::read("photo.IIQ")?;
/// let decoder = IiqImage::parse(&bytes)?;
/// assert!(decoder.dng_metadata()?.len() > 0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct IiqImage {
    parsed: ParsedTiff,
    maker: MakerNote,
    layout: RawLayout,
    metadata: Directory,
}

impl IiqImage {
    /// Parses a Phase One or Leaf IIQ container from memory.
    ///
    /// # Errors
    ///
    /// Returns an error when the TIFF header, private maker note, raw layout,
    /// camera profile, or required metadata is malformed or unsupported.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fornax_iiq::IiqImage;
    ///
    /// let bytes = std::fs::read("photo.IIQ")?;
    /// let decoder = IiqImage::parse(&bytes)?;
    /// assert!(decoder.raw_mosaic()?.dimensions().0 > 0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn parse(source: &[u8]) -> Result<Self, DecodeError> {
        let parsed = fornax_tiff::parse(source)
            .map_err(|error| invalid(format!("parsing IIQ TIFF failed: {error}")))?;
        let profile = profiles::resolve(&parsed.root)?;
        let maker = MakerNote::parse(source)?;
        let layout = RawLayout::parse(source, &maker)?;
        let metadata = metadata::map(&parsed.root, &maker, &layout, &profile)?;
        Ok(Self {
            parsed,
            maker,
            layout,
            metadata,
        })
    }

    /// Decodes the full-resolution storage-native IIQ mosaic.
    ///
    /// Sensor corrections are applied before samples are returned.
    ///
    /// # Errors
    ///
    /// Returns an error when the selected mode, row table, sample stream, or
    /// correction payload is malformed or unsupported.
    pub fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> {
        let chunks = self.layout.chunks(&self.parsed.source)?;
        let request = DecodeRequest {
            source: &self.parsed.source,
            compression: self.layout.compression.codec(),
            width: self.layout.width,
            height: self.layout.height,
            chunk_width: self.layout.width,
            chunk_height: self.layout.chunk_height(),
            chunks: &chunks,
            bits_per_sample: self.layout.compression.bits(),
            sample_format: SampleFormat::Integer,
            byte_order: ByteOrder::Little,
            predictor: Predictor::None,
        };
        let SampleBuffer::U16(image) = fornax_codec::decode(&request).map_err(codec_error)? else {
            return Err(invalid("IIQ codec returned non-U16 samples"));
        };
        MonoImage::from_raw(
            u32::try_from(self.layout.width).map_err(|_| invalid("IIQ width exceeds u32"))?,
            u32::try_from(self.layout.height).map_err(|_| invalid("IIQ height exceeds u32"))?,
            image.into_raw(),
        )
        .map(RawMosaic::U16)
        .ok_or_else(|| invalid("decoded IIQ mosaic dimensions are invalid"))
    }

    /// Returns the DNG-compatible metadata mapped from the IIQ container.
    pub fn dng_metadata(&self) -> Result<Directory, DecodeError> { Ok(self.metadata.clone()) }

    /// Returns normalized process metadata for the active Bayer mosaic.
    ///
    /// # Errors
    ///
    /// Returns an error when geometry, levels, white balance, or CFA metadata
    /// cannot be represented as conventional 2x2 RGB Bayer input.
    pub fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        metadata::process(
            &self.metadata,
            u32::try_from(self.layout.width).map_err(|_| invalid("IIQ width exceeds u32"))?,
            u32::try_from(self.layout.height).map_err(|_| invalid("IIQ height exceeds u32"))?,
        )
    }

    /// Crops the active Bayer area and converts samples to `T`.
    pub fn cropped_mosaic<T>(&self) -> Result<CroppedMosaic<T>, DecodeError>
    where
        T: Primitive,
    {
        let pixels = self.raw_mosaic()?;
        let active = self.process_metadata()?.active_area;
        let image = match pixels {
            RawMosaic::U8(image) => crop_and_cast(image, active)?,
            RawMosaic::U16(image) => crop_and_cast(image, active)?,
            RawMosaic::U32(image) => crop_and_cast(image, active)?,
            RawMosaic::F32(image) => crop_and_cast(image, active)?,
        };
        Ok(CroppedMosaic::new(image))
    }
}

impl IDecoder for IiqImage {
    fn decode(input: &Path) -> Result<Self, DecodeError> {
        let source = fs::read(input).map_err(|error| {
            DecodeError::Other(format!("reading {} failed: {error}", input.display()).into())
        })?;
        Self::parse(&source)
    }

    fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> { Self::raw_mosaic(self) }

    fn cropped_mosaic<T>(&self) -> Result<CroppedMosaic<T>, DecodeError>
    where
        T: Primitive,
    {
        Self::cropped_mosaic(self)
    }

    fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        Self::process_metadata(self)
    }
}

impl IDecoderDng for IiqImage {
    fn dng_metadata(&self) -> Result<Directory, DecodeError> { Self::dng_metadata(self) }
}

fn crop_and_cast<S, T>(mut image: MonoImage<S>, area: Rect) -> Result<MonoImage<T>, DecodeError>
where
    S: image::Primitive + Send + Sync,
    T: Primitive,
{
    image.crop_in_place(area);
    let (width, height) = image.dimensions();
    let samples = image
        .into_raw()
        .into_par_iter()
        .map(|value| num!(value))
        .collect();
    MonoImage::from_raw(width, height, samples)
        .ok_or_else(|| invalid("cropped IIQ dimensions are invalid"))
}

fn codec_error(error: impl std::fmt::Display) -> DecodeError { invalid(error.to_string()) }

pub(crate) fn invalid(detail: impl Into<Box<str>>) -> DecodeError {
    DecodeError::InvalidInput(detail.into())
}
