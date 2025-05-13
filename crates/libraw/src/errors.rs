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

impl LibrawErrors {
    pub(crate) fn report(&self, task: &str) -> miette::Result<()> {
        match self {
            LibrawErrors::Success => {
                clerk::debug!("Task: {}. {}", task, self);
            }
            LibrawErrors::UnspecifiedError
            | LibrawErrors::FileUnsupported
            | LibrawErrors::RequestForNonexistentImage
            | LibrawErrors::OutOfOrderCall
            | LibrawErrors::NoThumbnail
            | LibrawErrors::UnsupportedThumbnail
            | LibrawErrors::InputClosed
            | LibrawErrors::NotImplemented
            | LibrawErrors::RequestForNonexistentThumbnail => {
                clerk::warn!("Task: {}. {}", task, self);
            }
            LibrawErrors::UnknownError
            | LibrawErrors::UnsufficientMemory
            | LibrawErrors::DataError
            | LibrawErrors::IoError
            | LibrawErrors::CancelledByCallback
            | LibrawErrors::BadCrop
            | LibrawErrors::TooBig
            | LibrawErrors::MempoolOverflow => miette::bail!("Task: {}. {}", task, self),
        };
        Ok(())
    }
}
pub(crate) trait ILibrawErrors {
    fn check_run(exit_code: i32, task: &str) -> miette::Result<i32> {
        let result = crate::errors::LibrawErrors::try_from(exit_code)?;
        result.report(task)?;
        Ok(exit_code)
    }
    fn check_raw_alloc(imgdata: *mut libraw_sys::libraw_data_t) -> miette::Result<()> {
        if unsafe { (*imgdata).rawdata.raw_alloc }.is_null() {
            miette::bail!("imgdata is null.")
        }
        Ok(())
    }
}
