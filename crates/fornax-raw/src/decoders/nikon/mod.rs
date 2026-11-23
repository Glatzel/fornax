//! Nikon NEF container, metadata, and CFA sample decoding.

use std::path::Path;

use fornax_tiff::{Directory, ParsedTiff, TiffSource};

use crate::dng::metadata::IDecoderDng;
use crate::error::DecodeError;
use crate::metadata::{EncodingMetadata, EncodingSampleFormat, ProcessMetadata};
use crate::protocol::{IDecoder, IDecoderEncoding};
use crate::{CroppedMosaic, RawFloat, RawMosaic, crop_and_cast};

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
        Self::parse_source(TiffSource::from_vec(source.to_vec()))
    }

    /// Parses a Nikon NEF container from owned or mapped source storage.
    pub fn parse_source(source: TiffSource) -> Result<Self, DecodeError> {
        let parsed = fornax_tiff::parse_source(source)
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
        T: RawFloat,
    {
        let pixels = self.raw_mosaic()?;
        let active = self.process_metadata()?.active_area;
        match pixels {
            RawMosaic::U8(image) => crop_and_cast(image, active),
            RawMosaic::U16(image) => crop_and_cast(image, active),
            RawMosaic::U32(image) => crop_and_cast(image, active),
            RawMosaic::F32(image) => crop_and_cast(image, active),
        }
    }
}

impl IDecoder for NefImage {
    fn decode(input: &Path) -> Result<Self, DecodeError> {
        let source = TiffSource::from_path(input).map_err(|error| {
            DecodeError::Other(format!("opening {} failed: {error}", input.display()).into())
        })?;
        Self::parse_source(source)
    }

    fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> { Self::raw_mosaic(self) }

    fn cropped_mosaic<T>(&self) -> Result<CroppedMosaic<T>, DecodeError>
    where
        T: RawFloat,
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

#[cfg(test)]
mod tests;
