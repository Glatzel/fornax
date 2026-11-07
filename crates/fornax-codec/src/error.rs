use thiserror::Error;

/// Failure while decoding or encoding raw samples.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CodecError {
    /// The encoded payload or requested sample layout is malformed.
    #[error("{0}")]
    InvalidInput(Box<str>),
    /// The codec does not support the requested operation or layout.
    #[error("{0}")]
    Unsupported(Box<str>),
}

pub(crate) fn invalid(detail: impl Into<Box<str>>) -> CodecError {
    CodecError::InvalidInput(detail.into())
}

pub(crate) fn unsupported(detail: impl Into<Box<str>>) -> CodecError {
    CodecError::Unsupported(detail.into())
}
