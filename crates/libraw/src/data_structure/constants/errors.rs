use envoy::EnvoyError;
use num_enum::{FromPrimitive, TryFromPrimitive};
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
pub enum LibrawError {
    #[error("NumEnum TryFromPrimitive Error: {0}")]
    NumEnumTryFromPrimitiveError(String),
    #[error(transparent)]
    EnvoyError(#[from] EnvoyError),
    #[error("LibrawError {code:?} [{}]: {message}", *.code as i32)]
    LibrawError {
        code: LibrawErrorCode,
        message: String,
    },
}

impl<T: TryFromPrimitive> From<num_enum::TryFromPrimitiveError<T>> for LibrawError {
    fn from(value: num_enum::TryFromPrimitiveError<T>) -> Self {
        Self::NumEnumTryFromPrimitiveError(value.to_string())
    }
}
