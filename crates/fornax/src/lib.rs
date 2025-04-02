mod data_structure;
#[cfg(feature = "preset")]
mod preset;
use std::ffi::CString;
use std::path::PathBuf;

pub use data_structure::*;
use libraw_sys as sys;

pub struct Fornax {
    pub(crate) imgdata: *mut sys::libraw_data_t,
}
impl Fornax {
    // util
    fn check_run(exit_code: i32) -> miette::Result<()> {
        let result = LibRawErrors::from(exit_code);
        result.report()?;
        Ok(())
    }
    pub fn new() -> Self {
        let imgdata = unsafe { sys::libraw_init(0) };
        Self { imgdata }
    }
    // io
    pub fn open_buffer(&self, buf: &[u8]) -> miette::Result<()> {
        Self::check_run(unsafe {
            sys::libraw_open_buffer(self.imgdata, buf.as_ptr() as *const _, buf.len())
        })?;
        Ok(())
    }

    pub fn open_file(&self, fname: PathBuf) -> miette::Result<()> {
        let c_string =
            CString::new(fname.to_string_lossy().to_string()).expect("CString::new failed");
        Self::check_run(unsafe {
            sys::libraw_open_file(self.imgdata, c_string.as_ptr() as *const _)
        })?;
        Ok(())
    }

    pub fn unpack(&self) -> miette::Result<()> {
        Self::check_run(unsafe { sys::libraw_unpack(self.imgdata) })?;
        Ok(())
    }

    // data structure
    pub fn image_sizes(&self) -> miette::Result<LibrawImageSizes> {
        if unsafe { (*self.imgdata).rawdata.raw_alloc }.is_null() {
            miette::bail!("")
        } else {
            Ok(LibrawImageSizes::new(unsafe { &(*self.imgdata).sizes }))
        }
    }
}
