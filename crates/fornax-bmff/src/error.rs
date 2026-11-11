use std::fmt;

use fornax_raw_core::error::DecodeError;

/// A bounded BMFF parsing failure.
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum BmffError {
    /// The byte stream or box layout is malformed.
    InvalidInput(String),
    /// A configured parser limit was exceeded.
    LimitExceeded(String),
}

impl fmt::Display for BmffError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::InvalidInput(message) => write!(formatter, "invalid BMFF input: {message}"),
            Self::LimitExceeded(message) => write!(formatter, "BMFF limit exceeded: {message}"),
        }
    }
}

impl std::error::Error for BmffError {}

impl From<BmffError> for DecodeError {
    fn from(error: BmffError) -> Self {
        match error {
            BmffError::InvalidInput(message) => Self::InvalidInput(message.into()),
            BmffError::LimitExceeded(message) => Self::Unsupported(message.into()),
        }
    }
}

pub(crate) fn invalid(message: impl Into<String>) -> BmffError {
    BmffError::InvalidInput(message.into())
}

pub(crate) fn limited(message: impl Into<String>) -> BmffError {
    BmffError::LimitExceeded(message.into())
}
