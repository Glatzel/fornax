//! Structured model, metadata, and pipeline errors.
//!
//! # Examples
//!
//! ```
//! use fornax::error::RawValidationError;
//!
//! let error = RawValidationError::MissingMetadata("stored_geometry");
//! assert_eq!(
//!     error.to_string(),
//!     "missing required metadata: stored_geometry"
//! );
//! ```

use thiserror::Error;

use crate::decoded_raw::plane::SampleFormat;
use crate::metadata;

/// A structured validation failure in the universal RAW model.
///
/// # Examples
///
/// ```
/// use fornax::error::RawValidationError;
///
/// assert_eq!(
///     RawValidationError::InvalidGeometry.to_string(),
///     "invalid RAW geometry"
/// );
/// ```
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RawValidationError {
    /// A required typed metadata field is absent.
    #[error("missing required metadata: {0}")]
    MissingMetadata(&'static str),
    /// Pixel geometry is empty, overflows, or is internally inconsistent.
    #[error("invalid RAW geometry")]
    InvalidGeometry,
    /// An ActiveArea is empty or lies outside the stored plane.
    #[error("invalid ActiveArea")]
    InvalidActiveArea,
    /// A sample specification is incomplete or invalid.
    #[error("invalid sample specification")]
    InvalidSampleSpec,
    /// The sample specification is outside the initial supported model.
    #[error("unsupported sample specification: {bits_per_sample} bits, {format:?}")]
    UnsupportedSampleSpec {
        /// Declared semantic precision in bits.
        bits_per_sample: u16,
        /// Declared sample format.
        format: SampleFormat,
    },
    /// A stored plane's container does not match the declared sample
    /// specification.
    #[error("unsupported plane/sample-specification combination")]
    UnsupportedPlaneSpec,
    /// A CFA shape has zero dimensions.
    #[error("invalid CFA shape")]
    InvalidCfaShape,
    /// A CFA color repeat, coordinate, or slot is malformed.
    #[error("invalid CFA layout")]
    InvalidCfa,
    /// A coordinate lies outside the ActiveArea.
    #[error("coordinate is outside the ActiveArea")]
    CoordOutsideActiveArea,
    /// A CFA repeat other than a conventional 2x2 RGB Bayer tile was observed.
    #[error("unsupported CFA repeat: {rows}x{columns}")]
    UnsupportedCfa {
        /// Observed repeat row count.
        rows: u32,
        /// Observed repeat column count.
        columns: u32,
    },
    /// A monochrome or otherwise non-mosaic payload was observed.
    #[error("non-mosaic RAW payloads are unsupported")]
    UnsupportedNonMosaic,
    /// A mosaic contains a color outside the supported RGB plane set.
    #[error("non-RGB CFA color code {color_code} is unsupported")]
    UnsupportedNonRgb {
        /// Source CFA plane or color code.
        color_code: u8,
    },
    /// Metadata failed structural or semantic validation.
    #[error("invalid metadata: {0}")]
    Metadata(#[from] MetadataError),
    /// A floating-point plane contains NaN or infinity.
    #[error("RAW plane contains a non-finite sample")]
    NonFiniteSample,
    /// A demosaic-ready plane contains a sample outside `[0, 1]`.
    #[error("demosaic-ready sample is outside [0, 1]")]
    InvalidReadinessRange,
    /// A decoded image received an operation that is invalid for its sample
    /// storage.
    #[error("invalid decoded RAW sample storage")]
    InvalidStorageState,
}

/// A structured metadata validation or conflict failure.
///
/// # Examples
///
/// ```
/// use fornax::error::MetadataError;
///
/// assert_eq!(
///     MetadataError::InvalidValue.to_string(),
///     "invalid metadata value"
/// );
/// ```
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum MetadataError {
    /// An extensible metadata key does not use the required grammar.
    #[error("invalid metadata key: {0}")]
    InvalidKey(Box<str>),
    /// An extensible key attempts to redefine a typed field.
    #[error("metadata key duplicates a typed field")]
    DuplicateTypedField(Box<str>),
    /// A required typed field is absent.
    #[error("missing required typed metadata field")]
    MissingRequiredField(metadata::TypedMetadataField),
    /// Two equally authoritative candidates disagree.
    #[error("metadata candidates conflict")]
    Conflict(metadata::MetadataField),
    /// A level value, level length, or level shape is invalid.
    #[error("invalid level metadata")]
    InvalidLevels,
    /// A metadata value is not valid for its declared role.
    #[error("invalid metadata value")]
    InvalidValue,
    /// Preserving metadata would exceed the implementation allocation bound.
    #[error("metadata allocation limit exceeded")]
    AllocationLimit,
}

/// Structured decoder backend failure.
///
/// # Examples
///
/// ```
/// use fornax::error::DecodeError;
///
/// let error = DecodeError::Unsupported("compact RAW".into());
/// assert_eq!(
///     error.to_string(),
///     "unsupported RAW format or layout: compact RAW"
/// );
/// ```
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum DecodeError {
    /// The input container or file cannot be parsed.
    #[error("invalid RAW input: {0}")]
    InvalidInput(Box<str>),
    /// The input format or layout is unsupported by this backend.
    #[error("unsupported RAW format or layout: {0}")]
    Unsupported(Box<str>),
    /// The backend failed after recognizing the input.
    #[error("decoder backend failure: {0}")]
    Backend(Box<str>),
    /// The backend produced metadata or samples that fail core validation.
    #[error(transparent)]
    InvalidOutput(#[from] RawValidationError),
}

/// Structured preprocessing failure.
///
/// # Examples
///
/// ```
/// use fornax::error::PreprocessError;
///
/// assert_eq!(
///     PreprocessError::Numeric.to_string(),
///     "invalid preprocessing numerical result"
/// );
/// ```
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PreprocessError {
    /// The decoded image cannot enter standard preprocessing.
    #[error("invalid preprocessing input: {0}")]
    InvalidInput(Box<str>),
    /// The configured preprocessing operation is unsupported.
    #[error("unsupported preprocessing operation: {0}")]
    Unsupported(Box<str>),
    /// Floating-point arithmetic produced an invalid value.
    #[error("invalid preprocessing numerical result")]
    Numeric,
    /// A required preprocessing allocation failed.
    #[error("preprocessing allocation failed")]
    Allocation,
}

/// Structured demosaicing failure.
///
/// # Examples
///
/// ```
/// use fornax::error::DemosaicError;
///
/// assert_eq!(
///     DemosaicError::Allocation.to_string(),
///     "demosaicing allocation failed"
/// );
/// ```
#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum DemosaicError {
    /// The preprocessed image or requested output dimensions are invalid.
    #[error("invalid demosaicing input: {0}")]
    InvalidInput(Box<str>),
    /// The selected algorithm does not support the image or configuration.
    #[error("unsupported demosaicing operation: {0}")]
    Unsupported(Box<str>),
    /// The algorithm failed after accepting the input.
    #[error("demosaicing algorithm failure: {0}")]
    Algorithm(Box<str>),
    /// A required demosaicing allocation failed.
    #[error("demosaicing allocation failed")]
    Allocation,
}
