macro_rules! check_run {
    ($code:expr) => {
        match crate::LibrawErrors::from($code) {
            crate::LibrawErrors::Success => {
                clerk::debug!("{}", $code);
            }
            crate::LibrawErrors::UnspecifiedError
            | crate::LibrawErrors::FileUnsupported
            | crate::LibrawErrors::RequestForNonexistentImage
            | crate::LibrawErrors::OutOfOrderCall
            | crate::LibrawErrors::NoThumbnail
            | crate::LibrawErrors::UnsupportedThumbnail
            | crate::LibrawErrors::InputClosed
            | crate::LibrawErrors::NotImplemented
            | crate::LibrawErrors::RequestForNonexistentThumbnail => {
                clerk::warn!("{}", $code);
            }
            crate::LibrawErrors::UnknownError
            | crate::LibrawErrors::UnsufficientMemory
            | crate::LibrawErrors::DataError
            | crate::LibrawErrors::IoError
            | crate::LibrawErrors::CancelledByCallback
            | crate::LibrawErrors::BadCrop
            | crate::LibrawErrors::TooBig
            | crate::LibrawErrors::MempoolOverflow => miette::bail!("{}", $code),
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
