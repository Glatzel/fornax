//! Nikon NEF container, metadata, and CFA sample decoding.

use fornax_raw_core::RawMosaic;
use fornax_raw_core::error::DecodeError;
use fornax_tiff::ParsedTiff;

pub(crate) mod container;
mod decode;
pub(crate) mod makernote;

use container::RawLayout;
use makernote::MakerNote;

/// A parsed Nikon NEF image.
#[derive(Clone, Debug)]
pub struct NefImage {
    parsed: ParsedTiff,
    raw_ifd: usize,
    layout: RawLayout,
    maker: MakerNote,
}

impl NefImage {
    /// Parses a Nikon NEF TIFF container from memory.
    pub fn parse(source: &[u8]) -> Result<Self, DecodeError> {
        let parsed = fornax_tiff::parse(source)
            .map_err(|error| DecodeError::InvalidInput(error.to_string().into()))?;
        container::validate_make(&parsed)?;
        let maker = MakerNote::parse(&parsed)?;
        let (raw_ifd, layout) = RawLayout::parse(&parsed, &maker)?;
        Ok(Self {
            parsed,
            raw_ifd,
            layout,
            maker,
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

    pub(crate) const fn maker(&self) -> &MakerNote { &self.maker }
}

#[cfg(test)]
#[allow(dead_code)]
mod tests;
