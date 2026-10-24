use thiserror::Error;

use crate::SampleFormat;

/// A structured validation failure in the universal RAW model.
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
    /// A sample encoding is incomplete or invalid.
    #[error("invalid sample encoding")]
    InvalidSampleEncoding,
    /// The sample encoding is outside the initial supported model.
    #[error("unsupported sample encoding: {bits_per_sample} bits, {format:?}")]
    UnsupportedSampleEncoding {
        /// Declared semantic precision in bits.
        bits_per_sample: u16,
        /// Declared sample format.
        format: SampleFormat,
    },
    /// A stored plane's container does not match the declared encoding.
    #[error("unsupported plane/encoding combination")]
    UnsupportedPlaneEncoding,
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
    /// A custom operation left metadata that standard preprocessing cannot use.
    #[error("custom processing metadata basis is incompatible with standard preprocessing")]
    InvalidCustomMetadataBasis,
    /// A stage transition received an invariant-breaking plane.
    #[error("invalid stage transition")]
    InvalidStageTransition,
}

/// A structured metadata validation or conflict failure.
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
    MissingRequiredField(crate::TypedMetadataField),
    /// Two equally authoritative candidates disagree.
    #[error("metadata candidates conflict")]
    Conflict(crate::MetadataField),
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
