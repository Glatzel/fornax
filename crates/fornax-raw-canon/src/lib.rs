//! Canon CR3/CRM container parsing and lossless CRX decoding.

use std::fs;
use std::path::Path;
use std::sync::Arc;

use fornax_dng::Directory;
use fornax_dng::metadata::IDecoderDng;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::{EncodingMetadata, MetadataSource, ProcessMetadata};
use fornax_raw_core::protocol::{IDecoder, IDecoderEncoding};
use fornax_raw_core::{CroppedMosaic, MonoImage, Primitive, RawMosaic};
use generic_num::num;
use image::math::Rect;
use rayon::prelude::*;

mod codec;
mod color_data;
mod container;
mod error;
mod huffman;
mod metadata;
mod request;

use codec::CanonCodec;
pub use codec::CanonCrxCfa;
use color_data::ColorDataStatus;
use fornax_codec::ICodecDecode;
pub(crate) use fornax_codec::{DecodeRequest, SampleBuffer};
pub use request::CanonCrxHeader;

#[cfg(test)]
#[path = "codec_contract_tests.rs"]
mod codec_contract_tests;

/// A parsed Canon CR3 or CRM image.
#[derive(Clone, Debug)]
pub struct Cr3Image {
    source: Arc<[u8]>,
    sample: container::SampleRange,
    header: container::CodecHeader,
    metadata: Directory,
    color_data: ColorDataStatus,
    quality_mode: Option<u16>,
}

impl Cr3Image {
    /// Parses the default raw sample, index zero, from CR3 or CRM bytes.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::InvalidInput`] for malformed boxes and
    /// [`DecodeError::Unsupported`] for C-RAW, preview, ambiguous tracks, or
    /// non-2x2 CFA layouts.
    pub fn parse(source: &[u8]) -> Result<Self, DecodeError> { Self::parse_sample(source, 0) }

    /// Parses one zero-based raw sample from CR3 or CRM bytes.
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError::InvalidInput`] when the sample index is out of
    /// range and rejects every unsupported container or compression layout.
    pub fn parse_sample(source: &[u8], sample_index: usize) -> Result<Self, DecodeError> {
        let parsed = container::parse(source, sample_index)?;
        Ok(Self {
            source: Arc::from(source),
            sample: parsed.sample,
            header: parsed.header,
            metadata: parsed.metadata,
            color_data: parsed.color_data,
            quality_mode: parsed.quality_mode,
        })
    }

    /// Decodes the full-resolution storage-native CFA mosaic.
    ///
    /// # Errors
    ///
    /// Returns an error when the CRX payload is malformed or unsupported.
    pub fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> {
        let cfa = self.header.cfa;
        let chunks = [fornax_codec::Chunk {
            offset: self.sample.offset,
            byte_count: self.sample.size,
        }];
        let request = fornax_codec::DecodeRequest {
            source: &self.source,
            width: self.header.width,
            height: self.header.height,
            chunk_width: self.header.width,
            chunk_height: self.header.height,
            chunks: &chunks,
            bits_per_sample: u16::from(self.header.bit_depth),
            sample_format: fornax_codec::SampleFormat::Integer,
            byte_order: fornax_codec::ByteOrder::Big,
            predictor: fornax_codec::Predictor::None,
        };
        let codec = CanonCodec::Crx {
            header: self.header.header.clone(),
            cfa,
        };
        let fornax_codec::SampleBuffer::U16(image) = codec
            .decode(&request)
            .map_err(|error| DecodeError::InvalidInput(error.to_string().into()))?
        else {
            return Err(DecodeError::InvalidInput(
                "Canon CRX returned non-u16 samples".into(),
            ));
        };
        MonoImage::from_raw(
            u32::try_from(self.header.width).map_err(|_| invalid("Canon CR3 width exceeds u32"))?,
            u32::try_from(self.header.height)
                .map_err(|_| invalid("Canon CR3 height exceeds u32"))?,
            image.into_raw(),
        )
        .map(RawMosaic::U16)
        .ok_or_else(|| invalid("Canon CR3 decoded dimensions are invalid"))
    }

    /// Crops the active CFA area and converts samples to `T`.
    ///
    /// # Errors
    ///
    /// Returns an error when decoding, metadata validation, cropping, or the
    /// sample conversion fails.
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

    /// Returns the DNG-compatible metadata directory.
    pub fn dng_metadata(&self) -> Result<Directory, DecodeError> { Ok(self.metadata.clone()) }

    /// Return source encoding metadata for the selected CRX raw sample.
    pub fn encoding_metadata(&self) -> EncodingMetadata { self.header.header.encoding_metadata() }

    /// Returns normalized process metadata.
    ///
    /// # Errors
    ///
    /// Returns a typed error for missing or unknown ColorData, invalid
    /// geometry, non-2x2 CFA, or malformed levels.
    pub fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        let color_data = match &self.color_data {
            ColorDataStatus::Parsed(color_data) => color_data,
            ColorDataStatus::Missing => {
                return Err(invalid("Canon CR3 ColorData is missing"));
            }
            ColorDataStatus::Unknown(version) => {
                return Err(DecodeError::Unsupported(
                    format!("Canon CR3 ColorData version {version} is unsupported").into(),
                ));
            }
        };
        let mut metadata = self.metadata.clone();
        if self.quality_mode == Some(130) {
            metadata.insert(
                fornax_tiff::Tag::from(fornax_dng::DngTag::AsShotNeutral),
                fornax_tiff::Value::List(vec![
                    fornax_tiff::Value::Rational(1, 1),
                    fornax_tiff::Value::Rational(1, 1),
                    fornax_tiff::Value::Rational(1, 1),
                ]),
            );
        }
        let mut process = fornax_dng::dng_decoder::process_metadata_from_dng_metadata(
            u32::try_from(self.header.width).map_err(|_| invalid("Canon CR3 width exceeds u32"))?,
            u32::try_from(self.header.height)
                .map_err(|_| invalid("Canon CR3 height exceeds u32"))?,
            &metadata,
        )?;
        process.wb_source = MetadataSource::Tag {
            tag: 0x4001,
            version: Some(color_data.version as u16),
        };
        Ok(process)
    }
}

impl IDecoder for Cr3Image {
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

impl IDecoderDng for Cr3Image {
    fn dng_metadata(&self) -> Result<Directory, DecodeError> { Self::dng_metadata(self) }
}

impl IDecoderEncoding for Cr3Image {
    fn encoding_metadata(&self) -> EncodingMetadata { Self::encoding_metadata(self) }
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
        .ok_or_else(|| invalid("cropped Canon CR3 dimensions are invalid"))
}

fn invalid(message: impl Into<Box<str>>) -> DecodeError {
    DecodeError::InvalidInput(message.into())
}

#[cfg(test)]
mod tests;
