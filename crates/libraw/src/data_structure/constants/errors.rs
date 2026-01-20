use std::ffi::NulError;
use std::fmt::Display;
use std::str::Utf8Error;

use num_enum::{FromPrimitive, TryFromPrimitive, TryFromPrimitiveError};
use thiserror::Error;

///All functions returning integer numbers must return either errno or one of
/// the following error codes.
///
/// # References
///
/// * <https://www.libraw.org/docs/API-datastruct-eng.html#LibRaw_errors>
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, FromPrimitive)]
#[repr(i32)]
pub enum LibrawErrorCode {
    #[num_enum(default)]
    UnknownError = 1,
    OtherError = 2,

    //Non-Fatal Errors
    Success = 0,
    UnspecifiedError = -1,
    FileUnsupported = -2,
    RequestForNonexistentImage = -3,
    OutOfOrderCall = -4,
    NoThumbnail = -5,
    UnsupportedThumbnail = -6,
    InputClosed = -7,
    NotImplemented = -8,
    RequestForNonexistentThumbnail = -9,

    //Fatal Errors
    UnsufficientMemory = -100007,
    DataError = -100008,
    IoError = -100009,
    CancelledByCallback = -100010,
    BadCrop = -100011,
    TooBig = -100012,
    MempoolOverflow = -100013,
}

#[derive(Debug, Error)]
#[error("LibrawError {code:?} [{}]: {message}", *.code as i32)]
pub struct LibrawError {
    pub code: LibrawErrorCode,
    pub message: String,
}
impl LibrawError {
    pub fn from<T>(err: T) -> Self
    where
        T: Display,
    {
        Self {
            code: LibrawErrorCode::OtherError,
            message: format!("{}", err),
        }
    }
}
impl From<Utf8Error> for LibrawError {
    fn from(value: Utf8Error) -> Self { LibrawError::from(value) }
}
impl<T> From<TryFromPrimitiveError<T>> for LibrawError
where
    T: TryFromPrimitive,
{
    fn from(value: TryFromPrimitiveError<T>) -> Self { LibrawError::from(value) }
}
impl From<NulError> for LibrawError {
    fn from(value: NulError) -> Self { LibrawError::from(value) }
}
