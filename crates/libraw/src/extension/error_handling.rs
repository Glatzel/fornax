macro_rules! check_run {
    ($code:expr) => {
        let code = crate::LibrawErrorCode::from($code);
        match &code {
            crate::LibrawErrorCode::Success => {
                clerk::debug!(
                    "{:?}",
                    $crate::LibrawError {
                        code,
                        message: "Success".to_string()
                    }
                );
            }
            crate::LibrawErrorCode::UnspecifiedError
            | crate::LibrawErrorCode::FileUnsupported
            | crate::LibrawErrorCode::RequestForNonexistentImage
            | crate::LibrawErrorCode::OutOfOrderCall
            | crate::LibrawErrorCode::NoThumbnail
            | crate::LibrawErrorCode::UnsupportedThumbnail
            | crate::LibrawErrorCode::InputClosed
            | crate::LibrawErrorCode::NotImplemented
            | crate::LibrawErrorCode::RequestForNonexistentThumbnail => {
                clerk::warn!(
                    "{:?}",
                    $crate::LibrawError {
                        code,
                        message: crate::Libraw::strerror(code as i32)?
                    }
                );
            }
            crate::LibrawErrorCode::UnknownError
            | crate::LibrawErrorCode::OtherError
            | crate::LibrawErrorCode::UnsufficientMemory
            | crate::LibrawErrorCode::DataError
            | crate::LibrawErrorCode::IoError
            | crate::LibrawErrorCode::CancelledByCallback
            | crate::LibrawErrorCode::BadCrop
            | crate::LibrawErrorCode::TooBig
            | crate::LibrawErrorCode::MempoolOverflow => {
                let err = $crate::LibrawError {
                    code,
                    message: crate::Libraw::strerror(code as i32)?,
                };
                clerk::error!("{:?}", err);
                return Err(err);
            }
        };
    };
    ($condition:expr,$message:expr) => {
        if $condition {
            return Err($crate::LibrawError {
                code: $crate::LibrawErrorCode::OtherError,
                message: format!("{}", $message),
            });
        }
    };
}
pub(crate) use check_run;

macro_rules! custom_error {
    ($message:expr) => {
        return Err($crate::LibrawError {
            code: $crate::LibrawErrorCode::OtherError,
            message: format!("{}", $message),
        })
    };
}
pub(crate) use custom_error;

macro_rules! check_raw_alloc {
    ($imgdata:expr) => {
        if unsafe { (**$imgdata).rawdata.raw_alloc }.is_null() {
            return Err($crate::LibrawError {
                code: $crate::LibrawErrorCode::OtherError,
                message: "imagedata pointer is null".to_string(),
            });
        }
    };
}
pub(crate) use check_raw_alloc;
