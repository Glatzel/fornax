use std::fmt::Display;

use num_enum::FromPrimitive;
use thiserror::Error;

///All functions returning integer numbers must return either errno or one of
/// the following error codes.
#[derive(Debug, Clone, Copy, FromPrimitive)]
#[repr(u8)]
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
