use crate::Libraw;

///All functions returning integer numbers must return either errno or one of the following error
/// codes.
#[derive(Debug, Clone)]
pub enum LibrawErrors {
    //Non-Fatal Errors
    Success,
    UnspecifiedError,
    FileUnsupported,
    RequestForNonexistentImage,
    OutOfOrderCall,
    NoThumbnail,
    UnsupportedThumbnail,
    InputClosed,
    NotImplemented,
    RequestForNonexistentThumbnail,

    //Fatal Errors
    UnsufficientMemory,
    DataError,
    IoError,
    CancelledByCallback,
    BadCrop,
    TooBig,
    MempoolOverflow,
}
impl TryFrom<i32> for LibrawErrors {
    type Error = miette::Report;
    fn try_from(value: i32) -> miette::Result<Self> {
        match value {
            //Non-Fatal Errors
            0 => Ok(Self::Success),
            -1 => Ok(Self::UnspecifiedError),
            -2 => Ok(Self::FileUnsupported),
            -3 => Ok(Self::RequestForNonexistentImage),
            -4 => Ok(Self::OutOfOrderCall),
            -5 => Ok(Self::NoThumbnail),
            -6 => Ok(Self::UnsupportedThumbnail),
            -7 => Ok(Self::InputClosed),
            -8 => Ok(Self::NotImplemented),
            -9 => Ok(Self::RequestForNonexistentThumbnail),

            //Fatal Errors
            -100007 => Ok(Self::UnsufficientMemory),
            -100008 => Ok(Self::DataError),
            -100009 => Ok(Self::IoError),
            -100010 => Ok(Self::CancelledByCallback),
            -100011 => Ok(Self::BadCrop),
            -100012 => Ok(Self::TooBig),
            -100013 => Ok(Self::MempoolOverflow),
            _ => Ok(Self::Success),
        }
    }
}
impl From<&LibrawErrors> for i32 {
    fn from(enum_value: &LibrawErrors) -> Self {
        match enum_value {
            //Non-Fatal Errors
            LibrawErrors::Success => 0,
            LibrawErrors::UnspecifiedError => -1,
            LibrawErrors::FileUnsupported => -2,
            LibrawErrors::RequestForNonexistentImage => -3,
            LibrawErrors::OutOfOrderCall => -4,
            LibrawErrors::NoThumbnail => -5,
            LibrawErrors::UnsupportedThumbnail => -6,
            LibrawErrors::InputClosed => -7,
            LibrawErrors::NotImplemented => -8,
            LibrawErrors::RequestForNonexistentThumbnail => -9,

            //Fatal Errors
            LibrawErrors::UnsufficientMemory => -100007,
            LibrawErrors::DataError => -100008,
            LibrawErrors::IoError => -100009,
            LibrawErrors::CancelledByCallback => -100010,
            LibrawErrors::BadCrop => -100011,
            LibrawErrors::TooBig => -100012,
            LibrawErrors::MempoolOverflow => -100013,
        }
    }
}
impl std::fmt::Display for LibrawErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let info = Libraw::strerror(i32::from(self));
        let text = format!("Libraw exit code: {}. {}", i32::from(self), info);
        write!(f, "{}", text)
    }
}

impl LibrawErrors {
    pub(crate) fn report(&self, task: &str) -> miette::Result<()> {
        match self {
            //Non-Fatal Errors
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

            //Fatal Errors
            LibrawErrors::UnsufficientMemory
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
