//! Pure-Rust, DNG-centred RAW decoding.
//!
//! The current implementation reads classic TIFF DNG files with uncompressed
//! or lossless JPEG-92 strips and tiles, preserves typed metadata, and hands a
//! conventional 2x2 RGB Bayer mosaic to the shared preprocessing pipeline.
//! Default lossless-JPEG export preserves the source container byte for byte;
//! uncompressed export regenerates a DNG from the decoded samples.
//!
//! # Examples
//!
//! Decode a DNG and retain its typed metadata:
//!
//! ``` no_run
//! use std::path::Path;
//!
//! use fornax_decoder::Decoder;
//!
//! let decoded = Decoder.decode(Path::new("input.dng"))?;
//! assert!(decoded.dimensions().0 > 0);
//! # Ok::<(), fornax_decoder::DecodeError>(())
//! ```

mod decoded;
mod dng;
mod error;
mod format;
mod metadata;
mod protocol;
mod writer;

use std::path::Path;

pub(crate) use decoded::DngSource;
pub use decoded::{DecodedRaw, MosaicData};
pub use error::{CfaPatternError, DecodeError, DngWriteError, HandoffError};
pub use format::SourceFormat;
pub use metadata::{DngMetadata, DngTag, DngValue};
pub use writer::{DngCompression, DngVersion, DngWriteOptions};

/// DNG-centred decoder entry point.
///
/// The type is stateless so format detection can remain internal to each
/// decode call.
///
/// # Examples
///
/// ``` no_run
/// use fornax_decoder::Decoder;
///
/// let decoder = Decoder;
/// assert_eq!(std::mem::size_of_val(&decoder), 0);
/// ```
#[derive(Clone, Copy, Debug, Default)]
pub struct Decoder;

impl Decoder {
    /// Decodes a RAW file into the DNG-shaped model.
    ///
    /// # Arguments
    ///
    /// * `input` - Path to the RAW or DNG file.
    ///
    /// # Errors
    ///
    /// Returns a typed error when the input is not a supported DNG container,
    /// when its raw layout is unsupported, or when metadata is malformed.
    ///
    /// # Examples
    ///
    /// ```no_run
    /// use std::path::Path;
    ///
    /// use fornax_decoder::{DecodeError, Decoder};
    ///
    /// let decoded = Decoder.decode(Path::new("input.dng"))?;
    /// let _dimensions = decoded.dimensions();
    /// # Ok::<(), DecodeError>(())
    /// ```
    pub fn decode(&self, input: &Path) -> Result<DecodedRaw, DecodeError> {
        let source = std::fs::read(input).map_err(|error| DecodeError::InvalidInput {
            detail: format!("failed to read {}: {error}", input.display()).into(),
        })?;
        dng::decode(&source)
    }
}
