use num_enum::{FromPrimitive, IntoPrimitive};

use crate::Libraw;

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
        let info = Libraw::strerror(self.clone().into());
        let text = format!(
            "Libraw exit code: {}. {}",
            Into::<i32>::into(self.clone()),
            info
        );
        write!(f, "{}", text)
    }
}

// pub(crate) trait ILibrawErrors {
//     fn check_run(exit_code: i32, task: &str) -> miette::Result<i32> {
//         let result = crate::errors::LibrawErrors::from(exit_code);
//         result.report(task)?;
//         Ok(exit_code)
//     }
// }
macro_rules! check_run {
    ($code:expr) => {
        match crate::errors::LibrawErrors::from($code) {
            crate::errors::LibrawErrors::Success => {
                clerk::debug!("{}", $code);
            }
            crate::errors::LibrawErrors::UnspecifiedError
            | crate::errors::LibrawErrors::FileUnsupported
            | crate::errors::LibrawErrors::RequestForNonexistentImage
            | crate::errors::LibrawErrors::OutOfOrderCall
            | crate::errors::LibrawErrors::NoThumbnail
            | crate::errors::LibrawErrors::UnsupportedThumbnail
            | crate::errors::LibrawErrors::InputClosed
            | crate::errors::LibrawErrors::NotImplemented
            | crate::errors::LibrawErrors::RequestForNonexistentThumbnail => {
                clerk::warn!("{}", $code);
            }
            crate::errors::LibrawErrors::UnknownError
            | crate::errors::LibrawErrors::UnsufficientMemory
            | crate::errors::LibrawErrors::DataError
            | crate::errors::LibrawErrors::IoError
            | crate::errors::LibrawErrors::CancelledByCallback
            | crate::errors::LibrawErrors::BadCrop
            | crate::errors::LibrawErrors::TooBig
            | crate::errors::LibrawErrors::MempoolOverflow => miette::bail!("{}", $code),
        };
    };
}
pub(crate) use check_run;
macro_rules! check_raw_alloc {
    ($imgdata:expr) => {
        if unsafe { (*$imgdata).rawdata.raw_alloc }.is_null() {
            miette::bail!("imgdata is null.")
        }
    };
}
pub(crate) use check_raw_alloc;
