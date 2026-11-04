//! Pure-Rust, DNG-centred RAW decoding.
//!
//! This crate currently contains only the public data model and unsupported
//! stubs for the first implementation stage. Decoding and DNG writing are
//! intentionally absent.
//!
//! # Examples
//!
//! Decode attempts report an unsupported operation:
//!
//! ```
//! use std::path::Path;
//!
//! use fornax_decoder::{DecodeError, Decoder};
//!
//! let error = Decoder.decode(Path::new("input.dng")).unwrap_err();
//! assert!(matches!(error, DecodeError::UnsupportedFormat { .. }));
//! ```

mod decoded;
mod error;
mod format;
mod metadata;
mod protocol;
mod writer;

use std::path::Path;

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
/// ```
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
    /// Returns [`DecodeError::UnsupportedFormat`] until format decoding is
    /// implemented.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::path::Path;
    ///
    /// use fornax_decoder::{DecodeError, Decoder};
    ///
    /// let error = Decoder.decode(Path::new("input.dng")).unwrap_err();
    /// assert!(matches!(error, DecodeError::UnsupportedFormat { .. }));
    /// ```
    pub fn decode(&self, _input: &Path) -> Result<DecodedRaw, DecodeError> {
        Err(unsupported_decode())
    }
}

fn unsupported_decode() -> DecodeError {
    DecodeError::UnsupportedFormat {
        detail: "fornax-decoder scaffolding: RAW decoding is unsupported".into(),
    }
}
