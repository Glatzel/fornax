pub mod data_structure;
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
        let result = LibRawErrors::try_from(exit_code)?;
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
        LibrawImageSizes::new(self.imgdata)
    }
    pub fn dcraw_process(
        &mut self,
        params: Option<&LibrawOutputParams>,
    ) -> miette::Result<LibrawProcessedImage> {
        if let Some(params) = params {
            params.set_output_params(self.imgdata)?
        };

        let mut result = 0i32;
        let processed: *mut libraw_sys::libraw_processed_image_t =
            unsafe { sys::libraw_dcraw_make_mem_image(self.imgdata, &mut result) };
        Self::check_run(result)?;

        let processed = LibrawProcessedImage::new(processed)?;
        Ok(processed)
    }
}
impl Drop for Fornax {
    fn drop(&mut self) {
        unsafe { sys::libraw_close(self.imgdata) }
    }
}
impl Default for Fornax {
    fn default() -> Self {
        Self::new()
    }
}
