//! Fujifilm RAF container, metadata, and CFA sample decoding.
//!
//! # Examples
//!
//! ```no_run
//! use fornax_raw_fujifilm::RafImage;
//!
//! let bytes = std::fs::read("photo.RAF")?;
//! let decoder = RafImage::parse(&bytes)?;
//! let mosaic = decoder.raw_mosaic()?;
//! assert!(mosaic.dimensions().0 > 0);
//! # Ok::<(), Box<dyn std::error::Error>>(())
//! ```

mod codec;
mod container;
mod error;
mod metadata;
mod profiles;
mod request;

#[cfg(test)]
mod tests;

use std::path::Path;
use std::sync::Arc;

use codec::FujiCodec;
use fornax_codec::{ByteOrder, Chunk, ICodecDecode, Predictor, SampleFormat};
pub(crate) use fornax_codec::{CodecError, DecodeRequest, SampleBuffer};
use fornax_dng::metadata::IDecoderDng;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::{EncodingMetadata, EncodingSampleFormat, ProcessMetadata};
use fornax_raw_core::protocol::{IDecoder, IDecoderEncoding};
use fornax_raw_core::{CroppedMosaic, MonoImage, Primitive, RawMosaic};
use fornax_tiff::Directory;
use generic_num::num;
use image::math::Rect;
use rayon::prelude::*;
pub(crate) use request::FujiCfa;

#[cfg(test)]
#[path = "codec_contract_tests.rs"]
mod codec_contract_tests;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub(crate) enum Cfa {
    Bayer([u8; 4]),
    XTrans([u8; 36]),
}

impl Cfa {
    pub(crate) const fn is_bayer(self) -> bool { matches!(self, Self::Bayer(_)) }

    pub(crate) const fn codec(self) -> FujiCfa {
        match self {
            Self::Bayer(pattern) => FujiCfa::Bayer(pattern),
            Self::XTrans(pattern) => FujiCfa::XTrans(pattern),
        }
    }
}

/// A parsed Fujifilm RAF image.
///
/// The decoder retains the storage-native CFA mosaic, including X-Trans
/// layouts. Only conventional 2x2 Bayer images can enter the preprocessing
/// handoff.
///
/// # Examples
///
/// ```no_run
/// use fornax_raw_fujifilm::RafImage;
///
/// let bytes = std::fs::read("photo.RAF")?;
/// let decoder = RafImage::parse(&bytes)?;
/// assert!(decoder.dng_metadata()?.len() > 0);
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug)]
pub struct RafImage {
    source: Arc<[u8]>,
    layout: container::RawLayout,
    cfa: Cfa,
    metadata: Directory,
}

impl RafImage {
    /// Parses a Fujifilm RAF container from memory.
    ///
    /// # Errors
    ///
    /// Returns an error when the RAF pointers, embedded TIFF/Fuji IFD,
    /// proprietary tag block, CFA description, or raw payload layout is
    /// malformed or unsupported.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fornax_raw_fujifilm::RafImage;
    ///
    /// let bytes = std::fs::read("photo.RAF")?;
    /// let decoder = RafImage::parse(&bytes)?;
    /// assert!(decoder.dng_metadata()?.len() > 0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn parse(source: &[u8]) -> Result<Self, DecodeError> {
        let container = container::parse_container(source)?;
        let metadata = metadata::map(&container)?;
        Ok(Self {
            source: container.source,
            layout: container.layout,
            cfa: container.cfa,
            metadata,
        })
    }

    /// Decodes the full-resolution storage-native CFA mosaic.
    ///
    /// # Errors
    ///
    /// Returns an error when the selected compression stream is malformed or
    /// unsupported.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fornax_raw_fujifilm::RafImage;
    ///
    /// let bytes = std::fs::read("photo.RAF")?;
    /// let decoder = RafImage::parse(&bytes)?;
    /// let mosaic = decoder.raw_mosaic()?;
    /// assert!(mosaic.dimensions().0 > 0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> {
        let chunks = [Chunk {
            offset: self.layout.offset,
            byte_count: self.layout.byte_count,
        }];
        let (codec, bits, byte_order) = match self.layout.compression {
            container::Compression::Packed12 => (
                FujiCodec::Packed {
                    bits: 12,
                    byte_order: ByteOrder::Little,
                },
                12,
                ByteOrder::Little,
            ),
            container::Compression::Words => (
                FujiCodec::Uncompressed,
                16,
                match self.layout.byte_order {
                    fornax_tiff::ByteOrder::Little => ByteOrder::Little,
                    fornax_tiff::ByteOrder::Big => ByteOrder::Big,
                },
            ),
            container::Compression::FujiLossless => (
                FujiCodec::Lossless {
                    cfa: self.cfa.codec(),
                },
                self.layout.bits,
                ByteOrder::Big,
            ),
        };
        let request = DecodeRequest {
            source: &self.source,
            width: self.layout.width,
            height: self.layout.height,
            chunk_width: self.layout.width,
            chunk_height: self.layout.height,
            chunks: &chunks,
            bits_per_sample: u16::from(bits),
            sample_format: SampleFormat::Integer,
            byte_order,
            predictor: Predictor::None,
        };
        let SampleBuffer::U16(image) = codec.decode(&request).map_err(codec_error)? else {
            return Err(DecodeError::InvalidInput(
                "RAF codec returned non-u16 samples".into(),
            ));
        };
        MonoImage::from_raw(
            u32::try_from(self.layout.width).map_err(|_| invalid("RAF width exceeds u32"))?,
            u32::try_from(self.layout.height).map_err(|_| invalid("RAF height exceeds u32"))?,
            image.into_raw(),
        )
        .map(RawMosaic::U16)
        .ok_or_else(|| invalid("decoded RAF mosaic dimensions are invalid"))
    }

    /// Returns the DNG-compatible metadata mapped from the RAF container.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fornax_raw_fujifilm::RafImage;
    ///
    /// let bytes = std::fs::read("photo.RAF")?;
    /// let decoder = RafImage::parse(&bytes)?;
    /// assert!(decoder.dng_metadata()?.len() > 0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn dng_metadata(&self) -> Result<Directory, DecodeError> { Ok(self.metadata.clone()) }

    /// Return the selected Fujifilm source compression and storage layout.
    pub fn encoding_metadata(&self) -> EncodingMetadata {
        encoding_metadata_for_layout(&self.layout)
    }

    /// Returns normalized process metadata for a conventional Bayer RAF.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::Unsupported`] for X-Trans and other non-2x2 CFA
    /// layouts. Malformed levels, geometry, crops, or Bayer tags also return
    /// an error.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fornax_raw_fujifilm::RafImage;
    ///
    /// let bytes = std::fs::read("gfx.raf")?;
    /// let decoder = RafImage::parse(&bytes)?;
    /// let metadata = decoder.process_metadata()?;
    /// assert!(metadata.active_area.width > 0);
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        if !self.cfa.is_bayer() {
            return Err(DecodeError::Unsupported(
                "RAF X-Trans CFA is not conventional 2x2 Bayer pipeline input".into(),
            ));
        }
        metadata::process(
            &self.metadata,
            u32::try_from(self.layout.width).map_err(|_| invalid("RAF width exceeds u32"))?,
            u32::try_from(self.layout.height).map_err(|_| invalid("RAF height exceeds u32"))?,
        )
    }

    /// Crops the active Bayer area and converts samples to `T`.
    ///
    /// # Errors
    ///
    /// Returns an error when decoding fails, the CFA is not conventional
    /// 2x2 Bayer, or the active area is invalid.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use fornax_raw_fujifilm::RafImage;
    ///
    /// let bytes = std::fs::read("gfx.raf")?;
    /// let decoder = RafImage::parse(&bytes)?;
    /// let cropped = decoder.cropped_mosaic::<f32>()?;
    /// assert!(cropped.image().width() > 0);
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

impl IDecoder for RafImage {
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

impl IDecoderDng for RafImage {
    fn dng_metadata(&self) -> Result<Directory, DecodeError> { Self::dng_metadata(self) }
}

impl IDecoderEncoding for RafImage {
    fn encoding_metadata(&self) -> EncodingMetadata { Self::encoding_metadata(self) }
}

fn encoding_metadata_for_layout(layout: &container::RawLayout) -> EncodingMetadata {
    EncodingMetadata {
        compression_id: None,
        compression_name: match layout.compression {
            container::Compression::Packed12 => "fuji-packed12",
            container::Compression::Words => "fuji-words",
            container::Compression::FujiLossless => "fuji-lossless",
        },
        lossless: true,
        storage: "raf-block",
        bits_per_sample: Some(u16::from(layout.bits)),
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
        .ok_or_else(|| invalid("cropped RAF dimensions are invalid"))
}

fn codec_error(error: fornax_codec::CodecError) -> DecodeError {
    match error {
        fornax_codec::CodecError::InvalidInput(detail) => DecodeError::InvalidInput(detail),
        fornax_codec::CodecError::Unsupported(detail) => DecodeError::Unsupported(detail),
    }
}

fn invalid(detail: impl Into<Box<str>>) -> DecodeError { DecodeError::InvalidInput(detail.into()) }
