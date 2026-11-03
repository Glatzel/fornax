use std::fmt;

use thiserror::Error;

use crate::CfaPattern;

/// RAW container family recognized by the decoder.
///
/// # Examples
///
/// ```
/// use fornax_decoder::SourceFormat;
///
/// assert_eq!(SourceFormat::Dng.to_string(), "DNG");
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SourceFormat {
    /// Adobe Digital Negative.
    Dng,
    /// Canon CR3 or CRM.
    Cr3,
    /// Sony Alpha RAW.
    Arw,
    /// Nikon Electronic Format.
    Nef,
    /// Fujifilm RAW.
    Raf,
    /// Hasselblad 3FR.
    Hasselblad3fr,
    /// Panasonic RAW.
    Rw2,
    /// Phase One or Leaf IIQ.
    Iiq,
}

impl fmt::Display for SourceFormat {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(match self {
            Self::Dng => "DNG",
            Self::Cr3 => "CR3",
            Self::Arw => "ARW",
            Self::Nef => "NEF",
            Self::Raf => "RAF",
            Self::Hasselblad3fr => "3FR",
            Self::Rw2 => "RW2",
            Self::Iiq => "IIQ",
        })
    }
}

/// Decode failure with a stable category for corpus tracking.
///
/// # Examples
///
/// ```
/// use fornax_decoder::DecodeError;
///
/// let error = DecodeError::UnsupportedFormat {
///     detail: "unknown container".into(),
/// };
/// assert!(error.to_string().contains("unknown container"));
/// ```
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum DecodeError {
    /// The input is missing, unreadable, or otherwise invalid as an input.
    #[error("invalid RAW input: {detail}")]
    InvalidInput {
        /// Human-readable failure context.
        detail: Box<str>,
    },
    /// The container or compression scheme is unsupported.
    #[error("unsupported RAW format: {detail}")]
    UnsupportedFormat {
        /// Human-readable failure context.
        detail: Box<str>,
    },
    /// The container is supported but this camera is not.
    #[error("unsupported camera: make={make:?}, model={model:?}; {detail}")]
    UnsupportedCamera {
        /// Camera manufacturer reported by the file.
        make: Option<Box<str>>,
        /// Camera model reported by the file.
        model: Option<Box<str>>,
        /// Why the camera is unsupported.
        detail: Box<str>,
    },
    /// The format is supported but this file uses an unsupported variant.
    #[error("unsupported {format} variant: {detail}")]
    UnsupportedVariant {
        /// Container family that contains the variant.
        format: SourceFormat,
        /// Variant or compression details.
        detail: Box<str>,
    },
    /// Container data is malformed or internally inconsistent.
    #[error("corrupt {format} data: {detail}")]
    CorruptData {
        /// Container family being parsed.
        format: SourceFormat,
        /// Corrupt structure or invariant.
        detail: Box<str>,
    },
    /// The CFA descriptor itself is invalid.
    #[error(transparent)]
    InvalidCfa(#[from] crate::CfaPatternError),
}

/// Failure while converting the DNG-shaped model for the preprocessor.
///
/// # Examples
///
/// ```
/// use fornax_decoder::{CfaPattern, HandoffError};
///
/// let cfa = CfaPattern::new(6, 6, vec![1; 36])?;
/// let error = HandoffError::UnsupportedCfa(cfa);
/// assert!(error.to_string().contains("6x6"));
/// # Ok::<(), Box<dyn std::error::Error>>(())
/// ```
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum HandoffError {
    /// The decoded CFA cannot be represented by `fornax_core::BayerPattern`.
    #[error("CFA layout {0} is unsupported by the 2x2 Bayer preprocessor")]
    UnsupportedCfa(CfaPattern),
}

/// Failure while writing the DNG-shaped model.
///
/// # Examples
///
/// ```
/// use fornax_decoder::DngWriteError;
///
/// let error = DngWriteError::Unsupported {
///     operation: "DNG writing".into(),
/// };
/// assert!(error.to_string().contains("unsupported"));
/// ```
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum DngWriteError {
    /// The requested operation is not implemented by the scaffold.
    #[error("unsupported decoder operation: {operation}")]
    Unsupported {
        /// Name of the unsupported operation.
        operation: Box<str>,
    },
}
