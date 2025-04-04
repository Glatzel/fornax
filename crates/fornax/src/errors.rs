///All functions returning integer numbers must return either errno or one of the following error
/// codes.
#[derive(Debug, Clone)]
pub enum FornaxErrors {
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
impl TryFrom<i32> for FornaxErrors {
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
            code => miette::bail!("Unknow error code: {code}"),
        }
    }
}
impl From<&FornaxErrors> for i32 {
    fn from(enum_value: &FornaxErrors) -> Self {
        match enum_value {
            //Non-Fatal Errors
            FornaxErrors::Success => 0,
            FornaxErrors::UnspecifiedError => -1,
            FornaxErrors::FileUnsupported => -2,
            FornaxErrors::RequestForNonexistentImage => -3,
            FornaxErrors::OutOfOrderCall => -4,
            FornaxErrors::NoThumbnail => -5,
            FornaxErrors::UnsupportedThumbnail => -6,
            FornaxErrors::InputClosed => -7,
            FornaxErrors::NotImplemented => -8,
            FornaxErrors::RequestForNonexistentThumbnail => -9,

            //Fatal Errors
            FornaxErrors::UnsufficientMemory => -100007,
            FornaxErrors::DataError => -100008,
            FornaxErrors::IoError => -100009,
            FornaxErrors::CancelledByCallback => -100010,
            FornaxErrors::BadCrop => -100011,
            FornaxErrors::TooBig => -100012,
            FornaxErrors::MempoolOverflow => -100013,
        }
    }
}
impl std::fmt::Display for FornaxErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let info= match self {
             //Non-Fatal Errors
            FornaxErrors::Success => r##"
                No error; function terminated successfully."##.trim_start(),
            FornaxErrors::UnspecifiedError => r##"
                An unknown error has been encountered. This code should never be generated."##.trim_start(),
            FornaxErrors::FileUnsupported => r##"
                Unsupported file format (attempt to open a RAW file with a format unknown to the program)."##.trim_start(),
            FornaxErrors::RequestForNonexistentImage => r##"
                Attempt to retrieve a RAW image with a number absent in the data file (only for formats supporting storage of several images in a file)."##.trim_start(),
            FornaxErrors::OutOfOrderCall => r##"
                API functions have been called in wrong order (e.g., unpack() before open_file() ) or the previous stage has ended with an error (e.g., unpack() is called after open_file() has returned an error)."##.trim_start(),
            FornaxErrors::NoThumbnail => r##"
                Returned upon an attempt to retrieve a thumbnail from a file containing no preview."##.trim_start(),
            FornaxErrors::UnsupportedThumbnail => r##"
                RAW file contains a preview of unsupported format."##.trim_start(),
            FornaxErrors::InputClosed => r##"
                Input stream is not available for reading."##.trim_start(),
            FornaxErrors::NotImplemented =>r##"
                Decoder for specific RAW storage/compression format is not implemented."##.trim_start(),
            FornaxErrors::RequestForNonexistentThumbnail => r##"
                Attempt to retrieve a non-existent thumbnail by (invalid) index."##.trim_start(),

            //Fatal Errors
            FornaxErrors::UnsufficientMemory => r##"
                Attempt to get memory from the system has failed.
                All allocated resources will be freed, recycle() will be called, and the LibRaw object will be brought to the state "right after creation.""##.trim_start(),
            FornaxErrors::DataError => r##"
                A fatal error emerged during data unpacking.
                All allocated resources will be freed, recycle() will be called, and the LibRaw object will be brought to the state "right after creation.""##.trim_start(),
            FornaxErrors::IoError => r##"
                A fatal error emerged during file reading (premature end-of-file encountered or file is corrupt).
                All allocated resources will be freed, recycle() will be called, and the LibRaw object will be brought to the state "right after creation.""##.trim_start(),
            FornaxErrors::CancelledByCallback => r##"
                Processing cancelled due to calling application demand (by returning nonzero code from progress callback ).
                All allocated resources will be freed, recycle() will be called, and the LibRaw object will be brought to the state "right after creation.""##.trim_start(),
            FornaxErrors::BadCrop => r##"
                The incorrect cropping coordinates are set via params.cropbox[]: the left-top corner of cropping rectangle is outside the image. The processing will be cancelled, all allocated resources will be freed"##.trim_start(),
            FornaxErrors::TooBig => r##"
                Raw data size exceeds data limit."##.trim_start(),
            FornaxErrors::MempoolOverflow => r##"
                MempoolOverflow"##.trim_start(),
        };
        let text = format!("Exit code: {}.\n{}", i32::from(self), info);
        write!(f, "{}", text)
    }
}

impl FornaxErrors {
    pub(crate) fn report(&self) -> miette::Result<()> {
        match self {
            //Non-Fatal Errors
            FornaxErrors::Success => {
                clerk::debug!("{}", self);
            }
            FornaxErrors::UnspecifiedError => {
                clerk::warn!("{}", self);
            }
            FornaxErrors::FileUnsupported => {
                clerk::warn!("{}", self);
            }
            FornaxErrors::RequestForNonexistentImage => {
                clerk::warn!("{}", self);
            }
            FornaxErrors::OutOfOrderCall => {
                clerk::warn!("{}", self);
            }
            FornaxErrors::NoThumbnail => {
                clerk::warn!("{}", self);
            }
            FornaxErrors::UnsupportedThumbnail => {
                clerk::warn!("{}", self);
            }
            FornaxErrors::InputClosed => {
                clerk::warn!("{}", self);
            }
            FornaxErrors::NotImplemented => {
                clerk::warn!("{}", self);
            }
            FornaxErrors::RequestForNonexistentThumbnail => {
                clerk::warn!("{}", self);
            }

            //Fatal Errors
            FornaxErrors::UnsufficientMemory => miette::bail!(self.to_string()),
            FornaxErrors::DataError => miette::bail!(self.to_string()),
            FornaxErrors::IoError => miette::bail!(self.to_string()),
            FornaxErrors::CancelledByCallback => miette::bail!(self.to_string()),
            FornaxErrors::BadCrop => miette::bail!(self.to_string()),
            FornaxErrors::TooBig => miette::bail!(self.to_string()),
            FornaxErrors::MempoolOverflow => miette::bail!(self.to_string()),
        };
        Ok(())
    }
}
