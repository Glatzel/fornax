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

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum DecodeError {
    #[error("invalid RAW input: {0}")]
    InvalidInput(Box<str>),
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
}
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PreprocessError {
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
}
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PostProcessError {
    #[error(transparent)]
    Demosaic(#[from] DemosaicError),
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum DemosaicError {
    #[error("invalid demosaic input: {0}")]
    InvalidInput(Box<str>),
    #[error("unsupported Bayer pattern: {0}")]
    UnsupportedPattern(Box<str>),
}
/// Failure while writing the DNG-shaped model.
///
/// # Examples
///
/// ```
/// use fornax_core::error::DngWriteError;
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
