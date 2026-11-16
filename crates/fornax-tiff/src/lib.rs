//! Generic TIFF container support used by Fornax RAW decoders.

use std::collections::BTreeMap;
use std::fmt;
#[cfg(not(feature = "mmap"))]
use std::fs;
#[cfg(feature = "mmap")]
use std::fs::File;
use std::ops::Deref;
use std::path::Path;
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
    LARGE_BYTE_VALUE_THRESHOLD, Tag, Value, ValueAccessError, ValueConversionError,
    unsigned_scalar, unsigned_vec, unsigned_vec_lookup,
};
pub use writer::{IntoValue, TiffDirectory, TiffWriter};

/// A TIFF image file directory.
pub type Directory = BTreeMap<Tag, Value>;

/// Read-only lookup over TIFF directory metadata.
pub trait DirectoryLookup {
    /// Returns the value associated with `tag`.
    fn get(&self, tag: &Tag) -> Option<&Value>;

    /// Returns whether `tag` is present.
    fn contains_key(&self, tag: &Tag) -> bool;
}

impl DirectoryLookup for Directory {
    fn get(&self, tag: &Tag) -> Option<&Value> { Self::get(self, tag) }

    fn contains_key(&self, tag: &Tag) -> bool { Self::contains_key(self, tag) }
}

/// Owned backing storage for parsed TIFF bytes.
#[derive(Clone)]
pub enum TiffSource {
    Owned(Arc<Vec<u8>>),
    #[cfg(feature = "mmap")]
    Mapped(Arc<memmap2::Mmap>),
}

impl TiffSource {
    /// Opens a TIFF source, using a read-only memory map when available.
    ///
    /// # Errors
    ///
    /// Returns the file-open, mapping, or fallback read error.
    pub fn from_path(path: &Path) -> std::io::Result<Self> {
        #[cfg(feature = "mmap")]
        {
            let file = File::open(path)?;
            // The mapping is read-only. Truncating the file while it is in use
            // can make later accesses fault, as with any memory map.
            let mmap = unsafe { memmap2::Mmap::map(&file)? };
            Ok(Self::from_mmap(mmap))
        }
        #[cfg(not(feature = "mmap"))]
        {
            fs::read(path).map(Self::from_vec)
        }
    }

    /// Creates source storage from an owned vector without copying it.
    pub fn from_vec(bytes: Vec<u8>) -> Self { Self::Owned(Arc::new(bytes)) }

    /// Creates source storage from a read-only memory mapping.
    #[cfg(feature = "mmap")]
    pub fn from_mmap(mmap: memmap2::Mmap) -> Self { Self::Mapped(Arc::new(mmap)) }
}

impl AsRef<[u8]> for TiffSource {
    fn as_ref(&self) -> &[u8] {
        match self {
            Self::Owned(bytes) => bytes,
            #[cfg(feature = "mmap")]
            Self::Mapped(mmap) => mmap,
        }
    }
}

impl Deref for TiffSource {
    type Target = [u8];

    fn deref(&self) -> &Self::Target { self.as_ref() }
}

impl fmt::Debug for TiffSource {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter
            .debug_struct("TiffSource")
            .field("kind", &if self.is_mapped() { "mapped" } else { "owned" })
            .field("len", &self.len())
            .finish()
    }
}

impl TiffSource {
    const fn is_mapped(&self) -> bool {
        match self {
            Self::Owned(_) => false,
            #[cfg(feature = "mmap")]
            Self::Mapped(_) => true,
        }
    }
}

/// Parsed TIFF directories and their source bytes.
#[derive(Clone, Debug)]
pub struct ParsedTiff {
    pub directories: Vec<Directory>,
    pub source: TiffSource,
}

impl ParsedTiff {
    /// Returns the first TIFF image file directory.
    pub fn root(&self) -> &Directory { &self.directories[0] }
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

#[cfg(all(test, feature = "mmap", unix))]
mod source_tests {
    use std::time::{SystemTime, UNIX_EPOCH};

    use super::TiffSource;

    #[test]
    fn mapped_source_remains_readable_after_the_path_is_removed() {
        let unique = SystemTime::now()
            .duration_since(UNIX_EPOCH)
            .unwrap()
            .as_nanos();
        let path =
            std::env::temp_dir().join(format!("fornax-tiff-map-{}-{unique}", std::process::id()));
        std::fs::write(&path, b"mapped source").unwrap();

        let source = TiffSource::from_path(&path).unwrap();
        std::fs::remove_file(&path).unwrap();

        assert_eq!(&*source, b"mapped source");
    }
}

/// Parses classic TIFF or BigTIFF directories.
pub fn parse(source: &[u8]) -> Result<ParsedTiff, TiffError> {
    parse_source(TiffSource::from_vec(source.to_vec()))
}

/// Parses classic TIFF or BigTIFF directories from owned source storage.
pub fn parse_source(source: TiffSource) -> Result<ParsedTiff, TiffError> {
    parser::parse_source(source)
}

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
