//! Nikon NEF container, metadata, and CFA sample decoding.

use std::fs;
use std::path::Path;

use fornax_dng::metadata::IDecoderDng;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::{EncodingMetadata, EncodingSampleFormat, ProcessMetadata};
use fornax_raw_core::protocol::{IDecoder, IDecoderEncoding};
use fornax_raw_core::{CroppedMosaic, MonoImage, Primitive, RawMosaic};
use fornax_tiff::{Directory, ParsedTiff};
use generic_num::num;
use image::math::Rect;
use rayon::prelude::*;

mod codec;
mod container;
mod curve;
mod decode;
mod error;
mod huffman;
mod makernote;
mod metadata;
mod request;
mod samples;
mod wb;

use container::RawLayout;
pub(crate) use fornax_codec::{CodecError, SampleBuffer, SampleFormat};
use makernote::MakerNote;
pub(crate) use request::NikonTreeId;

#[cfg(test)]
#[path = "codec_contract_tests.rs"]
mod codec_contract_tests;

/// A parsed Nikon NEF image.
#[derive(Clone, Debug)]
pub struct NefImage {
    parsed: ParsedTiff,
    raw_ifd: usize,
    layout: RawLayout,
    metadata: metadata::MappedMetadata,
}

impl NefImage {
    /// Parses a Nikon NEF TIFF container from memory.
    pub fn parse(source: &[u8]) -> Result<Self, DecodeError> {
        let parsed = fornax_tiff::parse(source)
            .map_err(|error| DecodeError::InvalidInput(error.to_string().into()))?;
        container::validate_make(&parsed)?;
        let maker = MakerNote::parse(&parsed)?;
        let (raw_ifd, layout) = RawLayout::parse(&parsed, &maker)?;
        let metadata = metadata::map(&parsed, raw_ifd, &layout, &maker)?;
        Ok(Self {
            parsed,
            raw_ifd,
            layout,
            metadata,
        })
    }

    /// Decodes the full-resolution storage-native CFA mosaic.
    pub fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> {
        let raw =
            self.parsed.directories.get(self.raw_ifd).ok_or_else(|| {
                DecodeError::InvalidInput("selected Nikon raw IFD is missing".into())
            })?;
        decode::samples(&self.parsed.source, &self.layout, raw)
    }

    pub fn dng_metadata(&self) -> Result<Directory, DecodeError> {
        Ok(self.metadata.directory.clone())
    }

    /// Return the selected Nikon source compression and storage layout.
    pub fn encoding_metadata(&self) -> EncodingMetadata {
        encoding_metadata_for_layout(&self.layout)
    }

    pub fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        metadata::process(
            &self.metadata,
            u32::try_from(self.layout.width)
                .map_err(|_| DecodeError::InvalidInput("Nikon width exceeds u32".into()))?,
            u32::try_from(self.layout.height)
                .map_err(|_| DecodeError::InvalidInput("Nikon height exceeds u32".into()))?,
        )
    }

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

impl IDecoder for NefImage {
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

impl IDecoderDng for NefImage {
    fn dng_metadata(&self) -> Result<Directory, DecodeError> { Self::dng_metadata(self) }
}

impl IDecoderEncoding for NefImage {
    fn encoding_metadata(&self) -> EncodingMetadata { Self::encoding_metadata(self) }
}

fn encoding_metadata_for_layout(layout: &RawLayout) -> EncodingMetadata {
    EncodingMetadata {
        compression_id: Some(u32::from(layout.compression_id)),
        compression_name: compression_name(layout),
        lossless: true,
        storage: "strips",
        bits_per_sample: Some(match layout.kind {
            container::LayoutKind::Packed { bits, .. }
            | container::LayoutKind::UnpackedWords { bits, .. }
            | container::LayoutKind::NikonHuffman { bits, .. } => u16::from(bits),
            container::LayoutKind::StripedPacked12 => 12,
        }),
        sample_format: Some(EncodingSampleFormat::Integer),
        predictor: None,
        rows_per_strip: u32::try_from(layout.rows_per_strip).ok(),
        tile_width: None,
        tile_height: None,
    }
}

const fn compression_name(layout: &RawLayout) -> &'static str {
    match layout.kind {
        container::LayoutKind::NikonHuffman { .. } => "nikon-huffman",
        container::LayoutKind::StripedPacked12 => "nikon-mode-5-packed12",
        container::LayoutKind::UnpackedWords { .. } if layout.compression_id == 1 => {
            "nikon-uncompressed-words"
        }
        container::LayoutKind::UnpackedWords { .. } => match layout.mode {
            Some(7) => "nikon-mode-7-words",
            Some(10) => "nikon-mode-10-words",
            _ => "nikon-uncompressed-words",
        },
        container::LayoutKind::Packed { .. } => match layout.mode {
            Some(2) => "nikon-mode-2-packed",
            Some(6) => "nikon-mode-6-packed",
            Some(9) => "nikon-mode-9-packed",
            Some(10) => "nikon-mode-10-packed",
            _ => "nikon-packed",
        },
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
        .ok_or_else(|| DecodeError::InvalidInput("cropped Nikon dimensions are invalid".into()))
}

#[cfg(test)]
mod tests;
