mod image_sizes;
mod imgother;
mod iparams;

use fornax_core::IDecoder;
pub use image_sizes::LibrawImageSizes;
pub use imgother::{LibrawGpsInfo, LibrawImgOther};
pub use iparams::LibrawIParams;
use std::ffi::CString;
use std::path::Path;

use crate::ILibrawErrors;

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
    pub fn open_buffer(&self, buf: &[u8]) -> miette::Result<()> {
        Self::check_run(
            unsafe {
                libraw_sys::libraw_open_buffer(self.imgdata, buf.as_ptr() as *const _, buf.len())
            },
            "libraw_open_buffer",
        )?;
        Ok(())
    }

    pub fn open_file(&self, fname: &Path) -> miette::Result<()> {
        let c_string =
            CString::new(fname.to_string_lossy().to_string()).expect("CString::new failed");
        Self::check_run(
            unsafe { libraw_sys::libraw_open_file(self.imgdata, c_string.as_ptr() as *const _) },
            "libraw_open_file",
        )?;
        Ok(())
    }

    pub fn unpack(&self) -> miette::Result<()> {
        Self::check_run(
            unsafe { libraw_sys::libraw_unpack(self.imgdata) },
            "libraw_unpack",
        )?;
        Ok(())
    }

    // data structure
    pub fn imgother(&self) -> miette::Result<LibrawImgOther> {
        LibrawImgOther::new(self.imgdata)
    }
    pub fn image_sizes(&self) -> miette::Result<LibrawImageSizes> {
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
    fn decode_file(&self, file: &Path) -> miette::Result<()> {
        self.open_file(file)?;
        self.unpack()?;
        Ok(())
    }

    fn decode_buffer(&self, buffer: &[u8]) -> miette::Result<()> {
        self.open_buffer(buffer)?;
        self.unpack()?;
        Ok(())
    }
}
impl ILibrawErrors for Libraw {}
