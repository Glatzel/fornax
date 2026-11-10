//! Generic TIFF container support used by Fornax RAW decoders.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

pub mod constants;
mod parser;
#[cfg(feature = "raster")]
mod raster;
mod value;
mod writer;

#[cfg(feature = "raster")]
pub use raster::{
    RasterError, RasterLayout, RasterRequest, RasterStorage, TiffCompressionError,
    resolve_tiff_compression,
};
pub use value::{
    Tag, Value, ValueAccessError, ValueConversionError, unsigned_scalar, unsigned_vec,
};
pub use writer::{IntoValue, TiffDirectory, TiffWriter};

/// A TIFF image file directory.
pub type Directory = BTreeMap<Tag, Value>;

/// Parsed TIFF directories and their source bytes.
#[derive(Clone, Debug)]
pub struct ParsedTiff {
    pub root: Directory,
    pub directories: Vec<Directory>,
    pub source: Arc<[u8]>,
}

/// TIFF header byte order.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ByteOrder {
    Little,
    Big,
}

/// TIFF parsing or writing failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TiffError(Box<str>);

impl TiffError {
    pub(crate) fn new(detail: impl Into<Box<str>>) -> Self { Self(detail.into()) }
}

impl fmt::Display for TiffError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for TiffError {}

/// Parses classic TIFF or BigTIFF directories.
pub fn parse(source: &[u8]) -> Result<ParsedTiff, TiffError> { parser::parse(source) }

/// Parses one classic-TIFF image file directory at an absolute offset.
///
/// `offset_correction` is added to every out-of-line value and child-IFD
/// offset. Use zero when offsets are absolute file offsets, or the negative
/// source offset when `source` is an extracted sub-buffer whose values still
/// point into the original container.
pub fn parse_ifd_at(
    source: &[u8],
    offset: u64,
    byte_order: ByteOrder,
    offset_correction: i64,
) -> Result<Directory, TiffError> {
    parser::parse_ifd_at(source, offset, byte_order, offset_correction)
}
