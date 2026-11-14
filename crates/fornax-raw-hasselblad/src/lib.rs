//! Hasselblad 3FR container, metadata, and CFA sample decoding.
//!
//! # Examples
//!
//! ```no_run
//! use fornax_raw_hasselblad::HasselbladImage;
//!
//! let bytes = std::fs::read("photo.3FR")?;
//! let decoder = HasselbladImage::parse(&bytes)?;
//! assert!(decoder.raw_mosaic()?.dimensions().0 > 0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod codec;
mod container;
mod metadata;
mod profiles;
mod samples;

#[cfg(test)]
#[path = "codec_contract_tests.rs"]
mod codec_contract_tests;
#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::Arc;

use codec::HasselbladCodec;
use fornax_codec::{
    ByteOrder, Chunk, DecodeRequest, ICodecDecode, Predictor, SampleBuffer, SampleFormat,
};
use fornax_dng::metadata::IDecoderDng;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::{EncodingMetadata, EncodingSampleFormat, ProcessMetadata};
use fornax_raw_core::protocol::{IDecoder, IDecoderEncoding};
use fornax_raw_core::{CroppedMosaic, MonoImage, Primitive, RawMosaic};
use fornax_tiff::Directory;
use generic_num::num;

/// A parsed Hasselblad 3FR image.
///
/// The decoder retains the full-resolution CFA mosaic and DNG-compatible
/// metadata. Only conventional 2x2 RGB Bayer layouts can enter preprocessing.
///
/// # Examples
///
/// ```no_run
/// use fornax_raw_hasselblad::HasselbladImage;
///
/// let bytes = std::fs::read("photo.3FR")?;
/// let decoder = HasselbladImage::parse(&bytes)?;
/// assert!(decoder.dng_metadata()?.len() > 0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct HasselbladImage {
    source: Arc<[u8]>,
    layout: container::RawLayout,
    metadata: Directory,
}

impl HasselbladImage {
    /// Parses a Hasselblad 3FR TIFF container from memory.
    ///
    /// # Errors
    ///
    /// Returns an error when the TIFF directories, raw SubIFD, profile, CFA,
    /// crop, levels, or compression layout are invalid or unsupported.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fornax_raw_hasselblad::HasselbladImage;
    ///
    /// let bytes = std::fs::read("photo.3FR")?;
    /// let decoder = HasselbladImage::parse(&bytes)?;
    /// assert!(decoder.dng_metadata()?.len() > 0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn parse(source: &[u8]) -> Result<Self, DecodeError> {
        let container = container::parse_container(source)?;
        let metadata = metadata::map(&container)?;
        Ok(Self {
            source: container.source,
            layout: container.layout,
            metadata,
        })
    }

    /// Decodes the full-resolution storage-native CFA mosaic.
    ///
    /// # Errors
    ///
    /// Returns an error when the selected uncompressed or lossless-JPEG payload
    /// is malformed or unsupported.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fornax_raw_hasselblad::HasselbladImage;
    ///
    /// let bytes = std::fs::read("photo.3FR")?;
    /// let decoder = HasselbladImage::parse(&bytes)?;
    /// assert!(decoder.raw_mosaic()?.dimensions().0 > 0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> {
        let chunks = [Chunk {
            offset: self.layout.offset,
            byte_count: self.layout.byte_count,
        }];
        let (codec, byte_order) = match self.layout.compression {
            container::Compression::Uncompressed => {
                (HasselbladCodec::Uncompressed, ByteOrder::Little)
            }
            container::Compression::LosslessJpeg => (HasselbladCodec::LosslessJpeg, ByteOrder::Big),
        };
        let request = DecodeRequest {
            source: &self.source,
            width: self.layout.width,
            height: self.layout.height,
            chunk_width: self.layout.width,
            chunk_height: self.layout.height,
            chunks: &chunks,
            bits_per_sample: 16,
            sample_format: SampleFormat::Integer,
            byte_order,
            predictor: Predictor::None,
        };
        let SampleBuffer::U16(image) = codec.decode(&request).map_err(codec_error)? else {
            return Err(invalid("3FR codec returned non-u16 samples"));
        };
        MonoImage::from_raw(
            u32::try_from(self.layout.width).map_err(|_| invalid("3FR width exceeds u32"))?,
            u32::try_from(self.layout.height).map_err(|_| invalid("3FR height exceeds u32"))?,
            image.into_raw(),
        )
        .map(RawMosaic::U16)
        .ok_or_else(|| invalid("decoded 3FR mosaic dimensions are invalid"))
    }

    /// Returns the DNG-compatible metadata mapped from the 3FR container.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fornax_raw_hasselblad::HasselbladImage;
    ///
    /// let bytes = std::fs::read("photo.3FR")?;
    /// let decoder = HasselbladImage::parse(&bytes)?;
    /// assert!(decoder.dng_metadata()?.len() > 0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn dng_metadata(&self) -> Result<Directory, DecodeError> { Ok(self.metadata.clone()) }

    /// Return the selected Hasselblad source compression and storage layout.
    pub fn encoding_metadata(&self) -> EncodingMetadata {
        encoding_metadata_for_layout(&self.layout)
    }

    /// Returns normalized process metadata for a conventional Bayer 3FR.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::Unsupported`] for non-2x2 or non-RGB CFA layouts.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fornax_raw_hasselblad::HasselbladImage;
    ///
    /// let bytes = std::fs::read("photo.3FR")?;
    /// let decoder = HasselbladImage::parse(&bytes)?;
    /// assert!(decoder.process_metadata()?.active_area.width > 0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        metadata::process(
            &self.metadata,
            u32::try_from(self.layout.width).map_err(|_| invalid("3FR width exceeds u32"))?,
            u32::try_from(self.layout.height).map_err(|_| invalid("3FR height exceeds u32"))?,
        )
    }

    /// Crops the active Bayer area and converts samples to `T`.
    ///
    /// # Errors
    ///
    /// Returns an error when decoding fails, the CFA is not conventional RGB
    /// Bayer, or the active area is invalid.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fornax_raw_hasselblad::HasselbladImage;
    ///
    /// let bytes = std::fs::read("photo.3FR")?;
    /// let decoder = HasselbladImage::parse(&bytes)?;
    /// assert!(decoder.cropped_mosaic::<f32>()?.image().width() > 0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
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

impl IDecoder for HasselbladImage {
    fn decode(input: &Path) -> Result<Self, DecodeError> {
        let source = std::fs::read(input).map_err(|error| {
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

impl IDecoderDng for HasselbladImage {
    fn dng_metadata(&self) -> Result<Directory, DecodeError> { Self::dng_metadata(self) }
}

impl IDecoderEncoding for HasselbladImage {
    fn encoding_metadata(&self) -> EncodingMetadata { Self::encoding_metadata(self) }
}

fn encoding_metadata_for_layout(layout: &container::RawLayout) -> EncodingMetadata {
    let (compression_id, compression_name) = match layout.compression {
        container::Compression::Uncompressed => (1, "uncompressed"),
        container::Compression::LosslessJpeg => (7, "hasselblad-lossless-jpeg"),
    };
    EncodingMetadata {
        compression_id: Some(compression_id),
        compression_name,
        lossless: true,
        storage: "strips",
        bits_per_sample: Some(16),
        sample_format: Some(EncodingSampleFormat::Integer),
        predictor: None,
        rows_per_strip: None,
        tile_width: None,
        tile_height: None,
    }
}

fn crop_and_cast<S, T>(
    mut image: MonoImage<S>,
    area: image::math::Rect,
) -> Result<MonoImage<T>, DecodeError>
where
    S: image::Primitive + Send + Sync,
    T: Primitive,
{
    image.crop_in_place(area);
    let (width, height) = image.dimensions();
    let samples = image
        .into_raw()
        .into_iter()
        .map(|value| num!(value))
        .collect();
    MonoImage::from_raw(width, height, samples)
        .ok_or_else(|| invalid("cropped 3FR dimensions are invalid"))
}

fn codec_error(error: fornax_codec::CodecError) -> DecodeError {
    match error {
        fornax_codec::CodecError::InvalidInput(detail) => DecodeError::InvalidInput(detail),
        fornax_codec::CodecError::Unsupported(detail) => DecodeError::Unsupported(detail),
    }
}

fn invalid(detail: impl Into<Box<str>>) -> DecodeError { DecodeError::InvalidInput(detail.into()) }
