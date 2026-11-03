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

mod cfa;
mod decoded;
mod error;
mod metadata;
mod writer;

use std::io::Write;
use std::path::Path;

pub use cfa::{CfaPattern, CfaPatternError};
pub use decoded::{DecodedRaw, MosaicData};
pub use error::{DecodeError, DngWriteError, HandoffError, SourceFormat};
use fornax_core::error::DecodeError as CoreDecodeError;
use fornax_core::metadata::Metadata;
use fornax_core::protocol::IDecoder;
use fornax_core::{DecodedRawActive, DecodedRawFull, Primitive};
pub use metadata::{DngMetadata, DngTag, DngValue, Rational, SignedRational};
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

    /// Writes a decoded RAW image as DNG.
    ///
    /// # Arguments
    ///
    /// * `_decoded` - DNG-shaped image to serialize.
    /// * `_writer` - Destination for the DNG byte stream.
    /// * `_options` - DNG version and compression selection.
    ///
    /// # Errors
    ///
    /// Returns [`DngWriteError::Unsupported`] until DNG writing is
    /// implemented.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_decoder::{
    ///     CfaPattern, DecodedRaw, Decoder, DngCompression, DngMetadata, DngWriteOptions, MosaicData,
    /// };
    ///
    /// let decoded = DecodedRaw::new(
    ///     2,
    ///     2,
    ///     MosaicData::U16(vec![0; 4]),
    ///     CfaPattern::new(2, 2, vec![0, 1, 1, 2])?,
    ///     DngMetadata::new(),
    /// )?;
    /// let mut output = Vec::new();
    /// let result = Decoder.write_dng(
    ///     &decoded,
    ///     &mut output,
    ///     DngWriteOptions::new(DngCompression::Uncompressed),
    /// );
    /// assert!(result.is_err());
    /// # Ok::<(), Box<dyn std::error::Error>>(())
    /// ```
    pub fn write_dng<W: Write>(
        &self,
        _decoded: &DecodedRaw,
        _writer: &mut W,
        _options: DngWriteOptions,
    ) -> Result<(), DngWriteError> {
        Err(DngWriteError::Unsupported {
            operation: "DNG writing".into(),
        })
    }
}

impl<T> IDecoder<Path, T> for Decoder
where
    T: Primitive,
{
    fn decode_full(
        &self,
        _input: &Path,
    ) -> Result<(DecodedRawFull<T>, Metadata<T>), CoreDecodeError> {
        Err(unsupported_core_decode())
    }

    fn decode_active(
        &self,
        _input: &Path,
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), CoreDecodeError> {
        Err(unsupported_core_decode())
    }
}

fn unsupported_decode() -> DecodeError {
    DecodeError::UnsupportedFormat {
        detail: "fornax-decoder scaffolding: RAW decoding is unsupported".into(),
    }
}

fn unsupported_core_decode() -> CoreDecodeError {
    CoreDecodeError::Unsupported("fornax-decoder scaffolding: RAW decoding is unsupported".into())
}
