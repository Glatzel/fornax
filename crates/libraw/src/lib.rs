mod errors;
mod image_sizes;
mod imgother;
mod iparams;
mod libraw_version;
mod output_params;
mod processed_image;
mod utils;
use std::ffi::CString;
use std::path::PathBuf;

use fornax_core::{IDecoder, IPostProcessor, ProcessedImage};
pub use image_sizes::LibrawImageSizes;
pub use imgother::{LibrawGpsInfo, LibrawImgOther};
pub use iparams::LibrawIParams;
pub use libraw_version::{LIBRAW_VERSION, LibrawVersion};
pub use output_params::{
    DCRawFbddNoiserd, DCRawHighlightMode, DCRawOutputBps, DCRawOutputColor, DCRawParams,
    DCRawUserFlip, DCRawUserQual,
};
pub use processed_image::{DcRawProcessedImage, ImageFormats};

pub struct Libraw {
    pub(crate) imgdata: *mut libraw_sys::libraw_data_t,
}
impl Libraw {
    // util
    fn check_run(exit_code: i32) -> miette::Result<()> {
        let result = errors::LibrawErrors::try_from(exit_code)?;
        result.report()?;
        Ok(())
    }
    pub fn new() -> Self {
        let imgdata = unsafe { libraw_sys::libraw_init(0) };
        Self { imgdata }
    }

    // io
    pub fn open_buffer(&mut self, buf: &[u8]) -> miette::Result<()> {
        Self::check_run(unsafe {
            libraw_sys::libraw_open_buffer(self.imgdata, buf.as_ptr() as *const _, buf.len())
        })?;
        Ok(())
    }

    pub fn open_file(&mut self, fname: PathBuf) -> miette::Result<()> {
        let c_string =
            CString::new(fname.to_string_lossy().to_string()).expect("CString::new failed");
        Self::check_run(unsafe {
            libraw_sys::libraw_open_file(self.imgdata, c_string.as_ptr() as *const _)
        })?;
        Ok(())
    }

    pub fn unpack(&mut self) -> miette::Result<()> {
        Self::check_run(unsafe { libraw_sys::libraw_unpack(self.imgdata) })?;
        Ok(())
    }

    // data structure
    pub fn imgother(&mut self) -> miette::Result<LibrawImgOther> {
        LibrawImgOther::new(self.imgdata)
    }
    pub fn image_sizes(&mut self) -> miette::Result<LibrawImageSizes> {
        LibrawImageSizes::new(self.imgdata)
    }
    pub fn set_output_params(&mut self, params: DCRawParams) -> miette::Result<()> {
        params.set_output_params(self.imgdata)?;
        Ok(())
    }
    pub fn dcraw_process(&self) -> miette::Result<DcRawProcessedImage> {
        Self::check_run(unsafe { libraw_sys::libraw_dcraw_process(self.imgdata) })?;
        let mut result = 0i32;
        let processed: *mut libraw_sys::libraw_processed_image_t =
            unsafe { libraw_sys::libraw_dcraw_make_mem_image(self.imgdata, &mut result) };
        Self::check_run(result)?;

        let processed = DcRawProcessedImage::new(processed)?;
        Ok(processed)
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
impl IDecoder for Libraw {
    fn decode_file(&mut self, file: PathBuf) -> miette::Result<()> {
        let c_string =
            CString::new(file.to_string_lossy().to_string()).expect("CString::new failed");
        Libraw::check_run(unsafe {
            libraw_sys::libraw_open_file(self.imgdata, c_string.as_ptr() as *const _)
        })?;
        Libraw::check_run(unsafe { libraw_sys::libraw_unpack(self.imgdata) })?;
        Ok(())
    }

    fn decode_buffer(&mut self, buf: &[u8]) -> miette::Result<()> {
        Libraw::check_run(unsafe {
            libraw_sys::libraw_open_buffer(self.imgdata, buf.as_ptr() as *const _, buf.len())
        })?;
        Libraw::check_run(unsafe { libraw_sys::libraw_unpack(self.imgdata) })?;
        Ok(())
    }
}

pub struct DcRaw {}
impl IPostProcessor<Libraw, ProcessedImage> for DcRaw {
    fn post_process(&self, libraw: &Libraw) -> miette::Result<ProcessedImage> {
        libraw.dcraw_process()?.to_image()
    }
}
