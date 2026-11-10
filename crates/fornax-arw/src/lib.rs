//! Sony ARW/NRW containers, metadata, and CFA sample decoding.

mod container;
mod decode;
mod metadata;
mod sony;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::Path;

use fornax_core::error::DecodeError;
use fornax_core::metadata::ProcessMetadata;
use fornax_core::protocol::IDecoder;
use fornax_core::{CroppedMosaic, MonoImage, Primitive, RawMosaic};
use fornax_dng::metadata::IDecoderDng;
use fornax_tiff::{Directory, ParsedTiff};
use generic_num::num;
use image::math::Rect;
use rayon::prelude::*;

/// A parsed Sony ARW or NRW image.
///
/// The decoder keeps the source container and selected raw-IFD state private.
/// The public methods expose storage-native samples, cropped active-area
/// samples, DNG-compatible metadata, and normalized process metadata.
///
/// # Examples
///
/// ```no_run
/// use fornax_arw::ArwImage;
/// use fornax_core::protocol::IDecoder;
///
/// let decoder = ArwImage::decode(std::path::Path::new("photo.ARW"))?;
/// let mosaic = decoder.raw_mosaic()?;
/// # Ok::<(), fornax_core::error::DecodeError>(())
/// ```
#[derive(Clone, Debug)]
pub struct ArwImage {
    parsed: ParsedTiff,
    raw_ifd: usize,
    layout: container::RawLayout,
    metadata: Directory,
}

impl ArwImage {
    /// Parses and validates a Sony TIFF container from memory.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::InvalidInput`] for malformed offsets or metadata
    /// and [`DecodeError::Unsupported`] for a recognized but unsupported Sony
    /// layout such as ARQ or reduced-resolution YCbCr.
    pub fn parse(source: &[u8]) -> Result<Self, DecodeError> {
        let parsed = fornax_tiff::parse(source)
            .map_err(|error| container::invalid(format!("parsing ARW TIFF failed: {error}")))?;
        let (raw_ifd, layout) = container::RawLayout::parse(&parsed)?;
        let metadata = metadata::map(&parsed, raw_ifd, &layout)?;
        Ok(Self {
            parsed,
            raw_ifd,
            layout,
            metadata,
        })
    }

    /// Decodes the full-resolution storage-native CFA mosaic.
    ///
    /// # Errors
    ///
    /// Returns an error when the selected compression stream is truncated or
    /// inconsistent with its TIFF metadata.
    pub fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> {
        let raw = self
            .parsed
            .directories
            .get(self.raw_ifd)
            .ok_or_else(|| container::invalid("selected Sony raw IFD is missing"))?;
        let samples = decode::samples(&self.parsed.source, &self.layout, raw)?;
        let width = u32::try_from(self.layout.width)
            .map_err(|_| container::invalid("ARW width exceeds u32"))?;
        let height = u32::try_from(self.layout.stored_height)
            .map_err(|_| container::invalid("ARW height exceeds u32"))?;
        MonoImage::from_raw(width, height, samples)
            .map(RawMosaic::U16)
            .ok_or_else(|| container::invalid("decoded ARW mosaic dimensions are invalid"))
    }

    /// Crops the active CFA area and converts samples to `T`.
    ///
    /// # Errors
    ///
    /// Returns an error when the mosaic cannot be decoded, the active area is
    /// invalid, or the output sample type cannot represent the samples.
    pub fn cropped_mosaic<T>(&self) -> Result<CroppedMosaic<T>, DecodeError>
    where
        T: Primitive,
    {
        let pixels = self.raw_mosaic()?;
        let metadata = self.process_metadata()?;
        let image = match pixels {
            RawMosaic::U8(image) => crop_and_cast(image, metadata.active_area)?,
            RawMosaic::U16(image) => crop_and_cast(image, metadata.active_area)?,
            RawMosaic::U32(image) => crop_and_cast(image, metadata.active_area)?,
            RawMosaic::F32(image) => crop_and_cast(image, metadata.active_area)?,
        };
        Ok(CroppedMosaic::new(image))
    }

    /// Returns the DNG-compatible metadata mapped from the Sony container.
    pub fn dng_metadata(&self) -> Result<Directory, DecodeError> { Ok(self.metadata.clone()) }

    /// Returns normalized process metadata for the cropped Bayer mosaic.
    ///
    /// # Errors
    ///
    /// Returns an error when CFA, levels, geometry, or crop tags are invalid or
    /// describe a non-conventional Bayer layout.
    pub fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        fornax_dng::dng_decoder::process_metadata_from_dng_metadata(
            u32::try_from(self.layout.width)
                .map_err(|_| container::invalid("ARW width exceeds u32"))?,
            u32::try_from(self.layout.stored_height)
                .map_err(|_| container::invalid("ARW height exceeds u32"))?,
            &self.metadata,
        )
    }
}

impl IDecoder for ArwImage {
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

impl IDecoderDng for ArwImage {
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
        .ok_or_else(|| container::invalid("cropped ARW dimensions are invalid"))
}
