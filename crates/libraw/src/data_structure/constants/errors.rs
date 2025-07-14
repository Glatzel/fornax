use num_enum::{FromPrimitive, IntoPrimitive};

///All functions returning integer numbers must return either errno or one of
/// the following error codes.
#[derive(Debug, Clone, IntoPrimitive, FromPrimitive)]
#[repr(i32)]
pub enum LibrawErrors {
    #[num_enum(default)]
    UnknownError = 1,
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

impl std::fmt::Display for LibrawErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let info = crate::Libraw::strerror(self.clone().into());
        let text = format!(
            "Libraw exit code: {}. {}",
            Into::<i32>::into(self.clone()),
            info
        );
        write!(f, "{text}")
    }
}
