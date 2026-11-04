use thiserror::Error;

use crate::{DngTag, SourceFormat};

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
    /// The CFA metadata itself is invalid.
    #[error(transparent)]
    InvalidCfa(#[from] CfaPatternError),
}

/// Invalid CFA metadata.
///
/// # Examples
///
/// ```
/// use fornax_decoder::CfaPatternError;
///
/// let error = CfaPatternError::LengthMismatch {
///     expected: 4,
///     actual: 3,
/// };
/// assert!(error.to_string().contains("3"));
/// ```
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CfaPatternError {
    /// One or both CFA repeat dimensions are zero.
    #[error("CFA repeat dimensions must not be zero")]
    Empty,
    /// `CFARepeatPatternDim` is absent.
    #[error("CFARepeatPatternDim is missing")]
    MissingRepeatPatternDim,
    /// `CFAPattern` is absent.
    #[error("CFAPattern is missing")]
    MissingPattern,
    /// A tag uses a value type that is not valid for that tag.
    #[error("{tag} has an invalid TIFF value type")]
    InvalidValueType {
        /// Tag with the invalid value type.
        tag: DngTag,
    },
    /// The product of the repeat dimensions exceeds the platform address space.
    #[error("CFA repeat dimensions exceed the platform address space")]
    DimensionsOverflow,
    /// The number of plane entries does not match the repeat dimensions.
    #[error("CFA pattern has {actual} entries, expected {expected}")]
    LengthMismatch {
        /// Required number of entries.
        expected: usize,
        /// Supplied number of entries.
        actual: usize,
    },
}

/// Failure while converting the DNG-shaped model for the preprocessor.
///
/// # Examples
///
/// ```
/// use fornax_decoder::HandoffError;
///
/// let error = HandoffError::UnsupportedCfa {
///     width: 6,
///     height: 6,
/// };
/// assert!(error.to_string().contains("6x6"));
/// ```
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum HandoffError {
    /// CFA metadata cannot be interpreted.
    #[error(transparent)]
    InvalidCfa(#[from] CfaPatternError),
    /// The decoded CFA cannot be represented by `fornax_core::BayerPattern`.
    #[error("CFA layout {width}x{height} is unsupported by the 2x2 Bayer preprocessor")]
    UnsupportedCfa {
        /// CFA repeat width.
        width: u16,
        /// CFA repeat height.
        height: u16,
    },
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
    /// The selected source and output combination is not implemented.
    #[error("unsupported decoder operation: {operation}")]
    Unsupported {
        /// Name of the unsupported operation.
        operation: Box<str>,
    },
    /// The output file or sink could not be written.
    #[error("{operation} failed: {detail}")]
    Io {
        /// Operation that failed.
        operation: Box<str>,
        /// Underlying I/O failure.
        detail: Box<str>,
    },
    /// Metadata required to write a valid DNG is missing or invalid.
    #[error("invalid DNG metadata: {detail}")]
    InvalidMetadata {
        /// Metadata validation failure.
        detail: Box<str>,
    },
}
