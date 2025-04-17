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
        let info= match self {
             //Non-Fatal Errors
            LibrawErrors::Success => r##"
                No error; function terminated successfully."##.trim_start(),
            LibrawErrors::UnspecifiedError => r##"
                An unknown error has been encountered. This code should never be generated."##.trim_start(),
            LibrawErrors::FileUnsupported => r##"
                Unsupported file format (attempt to open a RAW file with a format unknown to the program)."##.trim_start(),
            LibrawErrors::RequestForNonexistentImage => r##"
                Attempt to retrieve a RAW image with a number absent in the data file (only for formats supporting storage of several images in a file)."##.trim_start(),
            LibrawErrors::OutOfOrderCall => r##"
                API functions have been called in wrong order (e.g., unpack() before open_file() ) or the previous stage has ended with an error (e.g., unpack() is called after open_file() has returned an error)."##.trim_start(),
            LibrawErrors::NoThumbnail => r##"
                Returned upon an attempt to retrieve a thumbnail from a file containing no preview."##.trim_start(),
            LibrawErrors::UnsupportedThumbnail => r##"
                RAW file contains a preview of unsupported format."##.trim_start(),
            LibrawErrors::InputClosed => r##"
                Input stream is not available for reading."##.trim_start(),
            LibrawErrors::NotImplemented =>r##"
                Decoder for specific RAW storage/compression format is not implemented."##.trim_start(),
            LibrawErrors::RequestForNonexistentThumbnail => r##"
                Attempt to retrieve a non-existent thumbnail by (invalid) index."##.trim_start(),

            //Fatal Errors
            LibrawErrors::UnsufficientMemory => r##"
                Attempt to get memory from the system has failed.
                All allocated resources will be freed, recycle() will be called, and the LibRaw object will be brought to the state "right after creation.""##.trim_start(),
            LibrawErrors::DataError => r##"
                A fatal error emerged during data unpacking.
                All allocated resources will be freed, recycle() will be called, and the LibRaw object will be brought to the state "right after creation.""##.trim_start(),
            LibrawErrors::IoError => r##"
                A fatal error emerged during file reading (premature end-of-file encountered or file is corrupt).
                All allocated resources will be freed, recycle() will be called, and the LibRaw object will be brought to the state "right after creation.""##.trim_start(),
            LibrawErrors::CancelledByCallback => r##"
                Processing cancelled due to calling application demand (by returning nonzero code from progress callback ).
                All allocated resources will be freed, recycle() will be called, and the LibRaw object will be brought to the state "right after creation.""##.trim_start(),
            LibrawErrors::BadCrop => r##"
                The incorrect cropping coordinates are set via params.cropbox[]: the left-top corner of cropping rectangle is outside the image. The processing will be cancelled, all allocated resources will be freed"##.trim_start(),
            LibrawErrors::TooBig => r##"
                Raw data size exceeds data limit."##.trim_start(),
            LibrawErrors::MempoolOverflow => r##"
                MempoolOverflow"##.trim_start(),
        };
        let text = format!("Libraw exit code: {}.\n{}", i32::from(self), info);
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
pub trait ILibrawErrors {
    fn check_run(exit_code: i32, task: &str) -> miette::Result<i32> {
        let result = crate::errors::LibrawErrors::try_from(exit_code)?;
        result.report(task)?;
        Ok(exit_code)
    }
}
