//! Panasonic RW2 container, metadata, and Bayer sample decoding.

mod codec;
mod container;
mod error;
mod metadata;
mod request;
mod samples;

#[cfg(test)]
mod tests;

use std::fs;
use std::path::Path;

use fornax_codec::{ByteOrder, Chunk, ICodecDecode, Predictor, SampleFormat};
pub(crate) use fornax_codec::{CodecError, DecodeRequest, SampleBuffer};
use fornax_dng::Directory;
use fornax_dng::metadata::IDecoderDng;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::{EncodingMetadata, EncodingSampleFormat, ProcessMetadata};
use fornax_raw_core::protocol::{IDecoder, IDecoderEncoding};
use fornax_raw_core::{CroppedMosaic, MonoImage, Primitive, RawMosaic};
use generic_num::num;
use image::math::Rect;
use rayon::prelude::*;
pub use request::{PanasonicV8Params, PanasonicV8Strip};

#[cfg(test)]
#[path = "codec_contract_tests.rs"]
mod codec_contract_tests;

/// A parsed Panasonic RW2 image.
///
/// The decoder retains the original container and validates raw-format,
/// compression, metadata, and strip tables during construction.
///
/// # Examples
///
/// ```no_run
/// use fornax_raw_panasonic::Rw2Image;
///
/// let bytes = std::fs::read("photo.RW2")?;
/// let decoder = Rw2Image::parse(&bytes)?;
/// assert!(decoder.layout_dimensions().0 > 0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct Rw2Image {
    container: container::Container,
    metadata: Directory,
    black_pattern: [u16; 4],
}

impl Rw2Image {
    /// Parses a Panasonic RW2 container from memory.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::InvalidInput`] for malformed TIFF structures,
    /// geometry, payload ranges, or metadata, and
    /// [`DecodeError::Unsupported`] for multi-shot, V7, unknown-version,
    /// unknown-CFA, or non-conventional layouts.
    pub fn parse(source: &[u8]) -> Result<Self, DecodeError> {
        let container = container::parse(source)?;
        let mapped = metadata::map(&container)?;
        metadata::process(&container, &mapped.metadata)?;
        Ok(Self {
            container,
            metadata: mapped.metadata,
            black_pattern: mapped.black_pattern,
        })
    }

    /// Returns the full sensor storage dimensions.
    pub const fn layout_dimensions(&self) -> (u32, u32) {
        (
            self.container.layout.width as u32,
            self.container.layout.height as u32,
        )
    }

    /// Returns the active-area Bayer black pattern in row-major order.
    pub const fn black_pattern(&self) -> Option<[u16; 4]> { Some(self.black_pattern) }

    /// Decodes the full-resolution storage-native Bayer mosaic.
    ///
    /// # Errors
    ///
    /// Returns an error when the selected codec stream is malformed or
    /// inconsistent with the declared geometry.
    pub fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> {
        let layout = &self.container.layout;
        let chunks = [Chunk {
            offset: layout.chunk.offset,
            byte_count: layout.chunk.byte_count,
        }];
        let decoded = layout
            .family
            .codec()
            .decode(&DecodeRequest {
                source: self.container.parsed.source.as_ref(),
                width: layout.width,
                height: layout.height,
                chunk_width: layout.width,
                chunk_height: layout.height,
                chunks: &chunks,
                bits_per_sample: layout.bits,
                sample_format: SampleFormat::Integer,
                byte_order: ByteOrder::Little,
                predictor: Predictor::None,
            })
            .map_err(codec_error)?;
        let SampleBuffer::U16(image) = decoded else {
            return Err(DecodeError::InvalidInput(
                "RW2 codec returned non-u16 samples".into(),
            ));
        };
        MonoImage::from_raw(
            u32::try_from(layout.width).map_err(|_| container::invalid("RW2 width exceeds u32"))?,
            u32::try_from(layout.height)
                .map_err(|_| container::invalid("RW2 height exceeds u32"))?,
            image.into_raw(),
        )
        .map(RawMosaic::U16)
        .ok_or_else(|| container::invalid("decoded RW2 mosaic dimensions are invalid"))
    }

    /// Crops the active Bayer area and converts samples to `T`.
    ///
    /// # Errors
    ///
    /// Returns an error when decoding fails or the active area is invalid.
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

    /// Returns DNG-compatible metadata mapped from the RW2 container.
    pub fn dng_metadata(&self) -> Result<Directory, DecodeError> { Ok(self.metadata.clone()) }

    /// Return the selected Panasonic source compression and storage layout.
    pub fn encoding_metadata(&self) -> EncodingMetadata {
        encoding_metadata_for_layout(&self.container.layout)
    }

    /// Returns normalized process metadata for the active Bayer area.
    ///
    /// # Errors
    ///
    /// Returns an error when geometry, CFA, levels, crop, or neutral metadata
    /// is invalid.
    pub fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        metadata::process(&self.container, &self.metadata)
    }
}

impl IDecoder for Rw2Image {
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

impl IDecoderDng for Rw2Image {
    fn dng_metadata(&self) -> Result<Directory, DecodeError> { Self::dng_metadata(self) }
}

impl IDecoderEncoding for Rw2Image {
    fn encoding_metadata(&self) -> EncodingMetadata { Self::encoding_metadata(self) }
}

fn encoding_metadata_for_layout(layout: &container::RawLayout) -> EncodingMetadata {
    EncodingMetadata {
        compression_id: Some(34_316),
        compression_name: match layout.family {
            container::Family::V4 { .. } => "panasonic-v4",
            container::Family::V5 => "panasonic-v5",
            container::Family::V6 => "panasonic-v6",
            container::Family::V8(_) => "panasonic-v8",
            container::Family::Unpacked12 => "panasonic-unpacked12",
            container::Family::Packed12Control => "panasonic-packed12-control",
        },
        lossless: true,
        storage: "single-chunk",
        bits_per_sample: Some(layout.bits),
        sample_format: Some(EncodingSampleFormat::Integer),
        predictor: None,
        rows_per_strip: None,
        tile_width: None,
        tile_height: None,
    }
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
        .ok_or_else(|| container::invalid("cropped RW2 dimensions are invalid"))
}

fn codec_error(error: fornax_codec::CodecError) -> DecodeError {
    match error {
        fornax_codec::CodecError::InvalidInput(detail) => DecodeError::InvalidInput(detail),
        fornax_codec::CodecError::Unsupported(detail) => DecodeError::Unsupported(detail),
    }
}
