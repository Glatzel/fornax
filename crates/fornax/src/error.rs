use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RawValidationError {
    #[error("invalid decoded RAW image")]
    InvalidImage,
    #[error("decoded RAW contains a non-finite sample")]
    NonFiniteSample,
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
    InvalidOutput(#[from] RawValidationError),
}

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum PreprocessError {
    #[error("preprocessing ActiveArea is outside the decoded image")]
    InvalidActiveArea,
    #[error("black and white levels do not form a valid range")]
    InvalidLevelRange,
    #[error(transparent)]
    InvalidImage(RawValidationError),
}
