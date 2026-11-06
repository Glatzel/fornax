//! Generic TIFF container support used by Fornax RAW decoders.

use std::collections::BTreeMap;
use std::fmt;
use std::sync::Arc;

mod parser;
mod value;
mod writer;

pub use value::{Tag, Value, ValueConversionError};
pub use writer::TiffWriter;

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
