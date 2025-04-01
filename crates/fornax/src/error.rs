use std::error::Error as StdError;
use std::fmt::{self, Display, Formatter};

use libraw_sys as sys;

pub type Result<T> = std::result::Result<T, LibrawError>;

#[derive(Debug, Clone)]
pub struct LibrawError {
    code: i32,
}

impl LibrawError {
    pub(crate) fn check(code: i32) -> Result<()> {
        if code == sys::LibRaw_errors_LIBRAW_SUCCESS {
            Ok(())
        } else {
            Err(LibrawError { code })
        }
    }
}

impl Display for LibrawError {
    fn fmt(&self, f: &mut Formatter<'_>) -> fmt::Result {
        write!(f, "libraw error: {}", self.code)
    }
}

impl StdError for LibrawError {}
