mod image_sizes;
mod imgother;
mod iparams;

use std::ffi::CString;
use std::path::PathBuf;

use fornax_core::IDecoder;
pub use image_sizes::LibrawImageSizes;
pub use imgother::{LibrawGpsInfo, LibrawImgOther};
pub use iparams::LibrawIParams;

#[derive(Debug)]
pub struct Libraw {
    pub(crate) imgdata: *mut libraw_sys::libraw_data_t,
}

impl Libraw {
    pub fn new() -> Self {
        let imgdata = unsafe { libraw_sys::libraw_init(0) };
        Self { imgdata }
    }

    // io
    pub fn open_buffer(&mut self, buf: &[u8]) -> miette::Result<()> {
        crate::check_run(unsafe {
            libraw_sys::libraw_open_buffer(self.imgdata, buf.as_ptr() as *const _, buf.len())
        })?;
        Ok(())
    }

    pub fn open_file(&mut self, fname: PathBuf) -> miette::Result<()> {
        let c_string =
            CString::new(fname.to_string_lossy().to_string()).expect("CString::new failed");
        crate::check_run(unsafe {
            libraw_sys::libraw_open_file(self.imgdata, c_string.as_ptr() as *const _)
        })?;
        Ok(())
    }

    pub fn unpack(&mut self) -> miette::Result<()> {
        crate::check_run(unsafe { libraw_sys::libraw_unpack(self.imgdata) })?;
        Ok(())
    }

    // data structure
    pub fn imgother(&mut self) -> miette::Result<LibrawImgOther> {
        LibrawImgOther::new(self.imgdata)
    }
    pub fn image_sizes(&mut self) -> miette::Result<LibrawImageSizes> {
        LibrawImageSizes::new(self.imgdata)
    }
    pub fn iparams(&self) -> miette::Result<LibrawIParams> {
        LibrawIParams::new(self.imgdata)
    }
}
impl Drop for Libraw {
    fn drop(&mut self) {
        unsafe { libraw_sys::libraw_close(self.imgdata) }
    }
}
impl Default for Libraw {
    fn default() -> Self {
        Self::new()
    }
}
impl crate::IDCRaw for Libraw {
    fn imgdata(&self) -> miette::Result<*mut libraw_sys::libraw_data_t> {
        Ok(self.imgdata)
    }
}
impl IDecoder for Libraw {
    fn decode_file(&mut self, file: PathBuf) -> miette::Result<()> {
        let c_string =
            CString::new(file.to_string_lossy().to_string()).expect("CString::new failed");
        crate::check_run(unsafe {
            libraw_sys::libraw_open_file(self.imgdata, c_string.as_ptr() as *const _)
        })?;
        crate::check_run(unsafe { libraw_sys::libraw_unpack(self.imgdata) })?;
        Ok(())
    }

    fn decode_buffer(&mut self, buf: &[u8]) -> miette::Result<()> {
        crate::check_run(unsafe {
            libraw_sys::libraw_open_buffer(self.imgdata, buf.as_ptr() as *const _, buf.len())
        })?;
        crate::check_run(unsafe { libraw_sys::libraw_unpack(self.imgdata) })?;
        Ok(())
    }
}
