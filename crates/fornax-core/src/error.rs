use thiserror::Error;

#[derive(Clone, Debug, Error, PartialEq, Eq)]
pub enum RawValidationError {
    #[error("invalid decoded RAW image")]
    InvalidImage,
    #[error("decoded RAW contains a non-finite sample")]
    NonFiniteSample,
}
