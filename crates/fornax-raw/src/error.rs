use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RawValidationError {
    #[error("invalid decoded RAW image")]
    InvalidImage,
    #[error("decoded RAW contains a non-finite sample")]
    NonFiniteSample,
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum MetadataValidationError {
    #[error("invalid RAW image geometry: {0}")]
    InvalidGeometry(Box<str>),
    #[error("invalid active area: {0}")]
    InvalidActiveArea(Box<str>),
    #[error("invalid default crop: {0}")]
    InvalidDefaultCrop(Box<str>),
    #[error("invalid RAW levels: {0}")]
    InvalidLevels(Box<str>),
}

/// A failure while processing RAW image data.
///
/// # Examples
///
/// ```
/// use fornax_raw::{PreprocessError, RawError};
///
/// let error: RawError = PreprocessError::EmptyImage;
/// assert_eq!(error.to_string(), "image dimensions must not be zero");
/// ```
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RawError {
    #[error("invalid RAW input: {0}")]
    InvalidInput(Box<str>),
    #[error("unsupported Bayer pattern: {0}")]
    UnsupportedPattern(Box<str>),
    #[error("unsupported RAW format or layout: {0}")]
    Unsupported(Box<str>),
    #[error("decoder backend failure: {0}")]
    Backend(Box<str>),
    #[error(transparent)]
    InvalidMetadata(#[from] MetadataValidationError),
    #[error(transparent)]
    InvalidOutput(#[from] RawValidationError),
    #[error("{0}")]
    Other(Box<str>),
    #[error("image dimensions must not be zero")]
    EmptyImage,
    #[error("image dimensions {width}x{height} exceed the platform address space")]
    ImageTooLarge { width: usize, height: usize },
    #[error("image has {actual} samples but dimensions require at least {expected}")]
    InvalidSampleCount { expected: usize, actual: usize },
    #[error("black-level pattern must not be empty")]
    InvalidBlackPattern,
    #[error("black_delta_h length {actual} does not match image width {expected}")]
    BlackDeltaWidthMismatch { expected: usize, actual: usize },
    #[error("black_delta_v length {actual} does not match image height {expected}")]
    BlackDeltaHeightMismatch { expected: usize, actual: usize },
    #[error("invalid color transform: {0}")]
    InvalidColorTransform(Box<str>),
    #[error("unsupported decoder operation: {operation}")]
    WriteUnsupported { operation: Box<str> },
    #[error("{operation} failed: {detail}")]
    WriteIo {
        operation: Box<str>,
        detail: Box<str>,
    },
    #[error("invalid DNG metadata: {detail}")]
    WriteInvalidMetadata { detail: Box<str> },
}

/// Decode-stage failure.
pub type DecodeError = RawError;
/// Preprocess-stage failure.
pub type PreprocessError = RawError;
/// Demosaic-stage failure.
pub type DemosaicError = RawError;
/// Post-process-stage failure.
pub type PostProcessError = RawError;
