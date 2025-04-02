///All functions returning integer numbers must return either errno or one of the following error
/// codes.
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone)]
pub enum LibRawErrors {
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
impl From<i32> for LibRawErrors {
    fn from(value: i32) -> Self {
        match value {
            //Non-Fatal Errors
            0 => Self::Success,
            -1 => Self::UnspecifiedError,
            -2 => Self::FileUnsupported,
            -3 => Self::RequestForNonexistentImage,
            -4 => Self::OutOfOrderCall,
            -5 => Self::NoThumbnail,
            -6 => Self::UnsupportedThumbnail,
            -7 => Self::InputClosed,
            -8 => Self::NotImplemented,
            -9 => Self::RequestForNonexistentThumbnail,

            //Fatal Errors
            -100007 => Self::UnsufficientMemory,
            -100008 => Self::DataError,
            -100009 => Self::IoError,
            -100010 => Self::CancelledByCallback,
            -100011 => Self::BadCrop,
            -100012 => Self::TooBig,
            -100013 => Self::MempoolOverflow,
            code => panic!("Unknow error code: {code}"),
        }
    }
}
impl From<&LibRawErrors> for i32 {
    fn from(enum_value: &LibRawErrors) -> Self {
        match enum_value {
            //Non-Fatal Errors
            LibRawErrors::Success => 0,
            LibRawErrors::UnspecifiedError => -1,
            LibRawErrors::FileUnsupported => -2,
            LibRawErrors::RequestForNonexistentImage => -3,
            LibRawErrors::OutOfOrderCall => -4,
            LibRawErrors::NoThumbnail => -5,
            LibRawErrors::UnsupportedThumbnail => -6,
            LibRawErrors::InputClosed => -7,
            LibRawErrors::NotImplemented => -8,
            LibRawErrors::RequestForNonexistentThumbnail => -9,

            //Fatal Errors
            LibRawErrors::UnsufficientMemory => -100007,
            LibRawErrors::DataError => -100008,
            LibRawErrors::IoError => -100009,
            LibRawErrors::CancelledByCallback => -100010,
            LibRawErrors::BadCrop => -100011,
            LibRawErrors::TooBig => -100012,
            LibRawErrors::MempoolOverflow => -100013,
        }
    }
}
impl std::fmt::Display for LibRawErrors {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        let info= match self {
             //Non-Fatal Errors
            LibRawErrors::Success => r##"
                No error; function terminated successfully.
                "##.trim_start(),
            LibRawErrors::UnspecifiedError => r##"
                An unknown error has been encountered. This code should never be generated.
                "##.trim_start(),
            LibRawErrors::FileUnsupported => r##"
                Unsupported file format (attempt to open a RAW file with a format unknown to the program).
                "##.trim_start(),
            LibRawErrors::RequestForNonexistentImage => r##"
                Attempt to retrieve a RAW image with a number absent in the data file (only for formats supporting storage of several images in a file).
                "##.trim_start(),
            LibRawErrors::OutOfOrderCall => r##"
                API functions have been called in wrong order (e.g., unpack() before open_file() ) or the previous stage has ended with an error (e.g., unpack() is called after open_file() has returned an error).
                "##.trim_start(),
            LibRawErrors::NoThumbnail => r##"
                Returned upon an attempt to retrieve a thumbnail from a file containing no preview.
                "##.trim_start(),
            LibRawErrors::UnsupportedThumbnail => r##"
                RAW file contains a preview of unsupported format.
                "##.trim_start(),
            LibRawErrors::InputClosed => r##"
                Input stream is not available for reading.
                "##.trim_start(),
            LibRawErrors::NotImplemented =>r##"
                Decoder for specific RAW storage/compression format is not implemented.
                "##.trim_start(),
            LibRawErrors::RequestForNonexistentThumbnail => r##"
                Attempt to retrieve a non-existent thumbnail by (invalid) index.
                "##.trim_start(),

            //Fatal Errors
            LibRawErrors::UnsufficientMemory => r##"
                Attempt to get memory from the system has failed.
                All allocated resources will be freed, recycle() will be called, and the LibRaw object will be brought to the state "right after creation."
                "##.trim_start(),
            LibRawErrors::DataError => r##"
                A fatal error emerged during data unpacking.
                All allocated resources will be freed, recycle() will be called, and the LibRaw object will be brought to the state "right after creation."
                "##.trim_start(),
            LibRawErrors::IoError => r##"
                A fatal error emerged during file reading (premature end-of-file encountered or file is corrupt).
                All allocated resources will be freed, recycle() will be called, and the LibRaw object will be brought to the state "right after creation."
                "##.trim_start(),
            LibRawErrors::CancelledByCallback => r##"
                Processing cancelled due to calling application demand (by returning nonzero code from progress callback ).
                All allocated resources will be freed, recycle() will be called, and the LibRaw object will be brought to the state "right after creation."
                "##.trim_start(),
            LibRawErrors::BadCrop => r##"
                The incorrect cropping coordinates are set via params.cropbox[]: the left-top corner of cropping rectangle is outside the image. The processing will be cancelled, all allocated resources will be freed
                "##.trim_start(),
            LibRawErrors::TooBig => r##"
                Raw data size exceeds data limit.
                "##.trim_start(),
            LibRawErrors::MempoolOverflow => r##"
                MempoolOverflow
                "##.trim_start(),
        };
        let text = format!("Exit code: {}.\n{}", i32::from(self), info);
        write!(f, "{}", text)
    }
}

impl LibRawErrors {
    pub(crate) fn report(&self) -> miette::Result<()> {
        match self {
            //Non-Fatal Errors
            LibRawErrors::Success => {
                clerk::info!("{self}");
            }
            LibRawErrors::UnspecifiedError => {
                clerk::warn!("{self}");
            }
            LibRawErrors::FileUnsupported => {
                clerk::warn!("{self}");
            }
            LibRawErrors::RequestForNonexistentImage => {
                clerk::warn!("{self}");
            }
            LibRawErrors::OutOfOrderCall => {
                clerk::warn!("{self}");
            }
            LibRawErrors::NoThumbnail => {
                clerk::warn!("{self}");
            }
            LibRawErrors::UnsupportedThumbnail => {
                clerk::warn!("{self}");
            }
            LibRawErrors::InputClosed => {
                clerk::warn!("{self}");
            }
            LibRawErrors::NotImplemented => {
                clerk::warn!("{self}");
            }
            LibRawErrors::RequestForNonexistentThumbnail => {
                clerk::warn!("{self}");
            }

            //Fatal Errors
            LibRawErrors::UnsufficientMemory => miette::bail!("{self}"),
            LibRawErrors::DataError => miette::bail!("{self}"),
            LibRawErrors::IoError => miette::bail!("{self}"),
            LibRawErrors::CancelledByCallback => miette::bail!("{self}"),
            LibRawErrors::BadCrop => miette::bail!("{self}"),
            LibRawErrors::TooBig => miette::bail!("{self}"),
            LibRawErrors::MempoolOverflow => miette::bail!("{self}"),
        };
        Ok(())
    }
}
