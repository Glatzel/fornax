mod errors;
mod image_sizes;
mod imgother;
mod iparams;
mod libraw_version;
mod output_params;
#[cfg(feature = "presets")]
mod presets;
mod processed_image;
mod utils;
use std::ffi::CString;
use std::path::PathBuf;

pub use image_sizes::ImageSizes;
pub use imgother::{GpsInfo, ImgOther};
pub use iparams::IParams;
pub use libraw_version::{LIBRAW_VERSION, LibrawVersion};
pub use output_params::{
    FbddNoiserd, HighlightMode, OutputBps, OutputColor, OutputParams, UserFlip, UserQual,
};
pub use processed_image::{ImageFormats, ProcessedImage};

pub struct Fornax {
    pub(crate) imgdata: *mut libraw_sys::libraw_data_t,
}
impl Fornax {
    // util
    fn check_run(exit_code: i32) -> miette::Result<()> {
        let result = errors::FornaxErrors::try_from(exit_code)?;
        result.report()?;
        Ok(())
    }
    pub fn new() -> Self {
        let imgdata = unsafe { libraw_sys::libraw_init(0) };
        Self { imgdata }
    }
    // io
    pub fn open_buffer(&self, buf: &[u8]) -> miette::Result<()> {
        Self::check_run(unsafe {
            libraw_sys::libraw_open_buffer(self.imgdata, buf.as_ptr() as *const _, buf.len())
        })?;
        Ok(())
    }

    pub fn open_file(&self, fname: PathBuf) -> miette::Result<()> {
        let c_string =
            CString::new(fname.to_string_lossy().to_string()).expect("CString::new failed");
        Self::check_run(unsafe {
            libraw_sys::libraw_open_file(self.imgdata, c_string.as_ptr() as *const _)
        })?;
        Ok(())
    }

    pub fn unpack(&self) -> miette::Result<()> {
        Self::check_run(unsafe { libraw_sys::libraw_unpack(self.imgdata) })?;
        Ok(())
    }

    // data structure
    pub fn imgother(&self) -> miette::Result<ImgOther> {
        ImgOther::new(self.imgdata)
    }
    pub fn image_sizes(&self) -> miette::Result<ImageSizes> {
        ImageSizes::new(self.imgdata)
    }
    pub fn dcraw_process(
        &mut self,
        params: Option<&OutputParams>,
    ) -> miette::Result<ProcessedImage> {
        if let Some(params) = params {
            params.set_output_params(self.imgdata)?
        };
        Self::check_run(unsafe { libraw_sys::libraw_dcraw_process(self.imgdata) })?;
        let mut result = 0i32;
        let processed: *mut libraw_sys::libraw_processed_image_t =
            unsafe { libraw_sys::libraw_dcraw_make_mem_image(self.imgdata, &mut result) };
        Self::check_run(result)?;

        let processed = ProcessedImage::new(processed)?;
        Ok(processed)
    }
    pub fn iparams(&self) -> miette::Result<IParams> {
        IParams::new(self.imgdata)
    }
}
impl Drop for Fornax {
    fn drop(&mut self) {
        unsafe { libraw_sys::libraw_close(self.imgdata) }
    }
}
impl Default for Fornax {
    fn default() -> Self {
        Self::new()
    }
}
