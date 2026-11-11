use thiserror::Error;

use crate::SampleBufferError;

/// Failure while decoding or encoding raw samples.
///
/// # Examples
///
/// ```
/// let error = fornax_codec::CodecError::InvalidInput("bad payload".into());
/// assert!(matches!(error, fornax_codec::CodecError::InvalidInput(_)));
/// ```
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum CodecError {
    /// The encoded payload or requested sample layout is malformed.
    #[error("{0}")]
    InvalidInput(Box<str>),
    /// The codec does not support the requested operation or layout.
    #[error("{0}")]
    Unsupported(Box<str>),
}

impl From<SampleBufferError> for CodecError {
    fn from(error: SampleBufferError) -> Self { invalid(error.to_string()) }
}

pub fn invalid<T>(detail: T) -> CodecError
where
    T: Into<Box<str>>,
{
    CodecError::InvalidInput(detail.into())
}

#[cfg(any(feature = "arw", feature = "nef"))]
pub fn unsupported<T>(detail: T) -> CodecError
where
    T: Into<Box<str>>,
{
    CodecError::Unsupported(detail.into())
}
