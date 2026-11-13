use thiserror::Error;

/// A bounded BMFF parsing failure.
#[derive(Debug, Error)]
pub enum BmffError {
    /// The byte stream or box layout is malformed.
    #[error("invalid BMFF input: {0}")]
    InvalidInput(Box<str>),
    /// A configured parser limit was exceeded.
    #[error("BMFF limit exceeded: {0}")]
    LimitExceeded(Box<str>),
}
