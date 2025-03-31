use libraw_sys as sys;
use std::{ffi::CString, path::PathBuf};

use crate::{LibrawError, RawImage};
pub type Result<T> = std::result::Result<T, LibrawError>;
pub struct Processor {
    pub(crate) inner: *mut sys::libraw_data_t,
}
impl Processor {
    pub fn new() -> Self {
        let inner = unsafe { sys::libraw_init(0) };
        Self { inner }
    }

    pub fn open_buffer(&self, buf: &[u8]) -> Result<()> {
        LibrawError::check(unsafe {
            sys::libraw_open_buffer(self.inner, buf.as_ptr() as *const _, buf.len())
        })?;
        Ok(())
    }

    pub fn open_file(&self, fname: PathBuf) -> Result<()> {
        let c_string =
            CString::new(fname.to_string_lossy().to_string()).expect("CString::new failed");
        LibrawError::check(unsafe {
            sys::libraw_open_file(self.inner, c_string.as_ptr() as *const _)
        })?;
        Ok(())
    }

    pub fn unpack(&self) -> Result<()> {
        LibrawError::check(unsafe { sys::libraw_unpack(self.inner) })?;
        Ok(())
    }
    pub fn decode(self) -> Result<RawImage> {
        let decoded = RawImage::new(self);
        Ok(decoded)
    }
}
