use std::fmt::{self, Display, Formatter};
///All functions returning integer numbers must return either errno or one of the following error
/// codes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub enum LibRawErrors {
    //Fatal errors (return of such an error code implies that file processing has to be
    // terminated, since the state of data structures is unknown).
    UnspecifiedError = -1,
    FileUnsupported = -2,
    RequestForNonexistentImage = -3,
    OutOfOrderCall = -4,
    NoThumbnail = -5,
    UnsupportedThumbnail = -6,
    InputClosed = -7,
    NotImplemented = -8,
    //Non-Fatal Errors
    Success = 0,
    RequestForNonexistentThumbnail = -9,
    UnsufficientMemory = -100007,
    DataError = -100008,
    IoError = -100009,
    CancelledByCallback = -100010,
    BadCrop = -100011,
    TooBig = -100012,
    MempoolOverflow = -100013,
}
