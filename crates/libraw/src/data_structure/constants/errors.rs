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
    OtherError = 2,
    #[num_enum(default)]
    UnknownError = 1,

    //Non-Fatal Errors
    Success = libraw_sys::LibRaw_errors_LIBRAW_SUCCESS,
    UnspecifiedError = libraw_sys::LibRaw_errors_LIBRAW_UNSPECIFIED_ERROR,
    FileUnsupported = libraw_sys::LibRaw_errors_LIBRAW_FILE_UNSUPPORTED,
    RequestForNonexistentImage = libraw_sys::LibRaw_errors_LIBRAW_REQUEST_FOR_NONEXISTENT_IMAGE,
    OutOfOrderCall = libraw_sys::LibRaw_errors_LIBRAW_OUT_OF_ORDER_CALL,
    NoThumbnail = libraw_sys::LibRaw_errors_LIBRAW_NO_THUMBNAIL,
    UnsupportedThumbnail = libraw_sys::LibRaw_errors_LIBRAW_UNSUPPORTED_THUMBNAIL,
    InputClosed = libraw_sys::LibRaw_errors_LIBRAW_INPUT_CLOSED,
    NotImplemented = libraw_sys::LibRaw_errors_LIBRAW_NOT_IMPLEMENTED,
    RequestForNonexistentThumbnail =
        libraw_sys::LibRaw_errors_LIBRAW_REQUEST_FOR_NONEXISTENT_THUMBNAIL,

    //Fatal Errors
    UnsufficientMemory = libraw_sys::LibRaw_errors_LIBRAW_UNSUFFICIENT_MEMORY,
    DataError = libraw_sys::LibRaw_errors_LIBRAW_DATA_ERROR,
    IoError = libraw_sys::LibRaw_errors_LIBRAW_IO_ERROR,
    CancelledByCallback = libraw_sys::LibRaw_errors_LIBRAW_CANCELLED_BY_CALLBACK,
    BadCrop = libraw_sys::LibRaw_errors_LIBRAW_BAD_CROP,
    TooBig = libraw_sys::LibRaw_errors_LIBRAW_TOO_BIG,
    MempoolOverflow = libraw_sys::LibRaw_errors_LIBRAW_MEMPOOL_OVERFLOW,
}

#[derive(Debug, Error)]
pub enum LibrawError {
    #[error("NumEnum TryFromPrimitive Error: {0}")]
    NumEnumTryFromPrimitive(String),
    #[error(transparent)]
    Envoy(#[from] EnvoyError),
    #[error("LibrawError {code:?} [{}]: {message}", *.code as i32)]
    Libraw {
        code: LibrawErrorCode,
        message: String,
    },
}

impl<T: TryFromPrimitive> From<num_enum::TryFromPrimitiveError<T>> for LibrawError {
    fn from(value: num_enum::TryFromPrimitiveError<T>) -> Self {
        Self::NumEnumTryFromPrimitive(value.to_string())
    }
}
