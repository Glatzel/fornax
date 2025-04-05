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
use miette::IntoDiagnostic;
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
    pub fn set_output_params(&self, params: &DCRawParams) -> miette::Result<()> {
        if let Some(graybox) = params.greybox {
            unsafe { (*self.imgdata).params.greybox = graybox };
        }
        if let Some(cropbox) = params.cropbox {
            unsafe { (*self.imgdata).params.cropbox = cropbox };
        }
        if let Some(aber) = params.aber {
            unsafe {
                (*self.imgdata).params.aber[0] = aber[0];
                (*self.imgdata).params.aber[2] = aber[1];
            }
        }
        if let Some(gamm) = params.gamm {
            unsafe {
                (*self.imgdata).params.gamm[0] = gamm[0];
                (*self.imgdata).params.gamm[1] = gamm[1];
            }
        }
        if let Some(user_mul) = params.user_mul {
            unsafe { (*self.imgdata).params.user_mul = user_mul };
        }
        if let Some(bright) = params.bright {
            unsafe { (*self.imgdata).params.bright = bright };
        }
        if let Some(threshold) = params.threshold {
            unsafe { (*self.imgdata).params.threshold = threshold };
        }
        if let Some(half_size) = params.half_size {
            unsafe { (*self.imgdata).params.half_size = half_size as i32 };
        }
        if let Some(four_color_rgb) = params.four_color_rgb {
            unsafe { (*self.imgdata).params.four_color_rgb = four_color_rgb as i32 };
        }
        if let Some(highlight) = params.highlight {
            unsafe { (*self.imgdata).params.highlight = i32::from(highlight) };
        }
        if let Some(use_auto_wb) = params.use_auto_wb {
            unsafe { (*self.imgdata).params.use_auto_wb = use_auto_wb as i32 };
        }
        if let Some(use_camera_wb) = params.use_camera_wb {
            unsafe { (*self.imgdata).params.use_camera_wb = use_camera_wb as i32 };
        }
        if let Some(use_camera_matrix) = params.use_camera_matrix {
            unsafe { (*self.imgdata).params.use_camera_matrix = i32::from(use_camera_matrix) };
        }
        if let Some(output_color) = params.output_color {
            unsafe { (*self.imgdata).params.output_color = i32::from(output_color) };
        }
        if let Some(output_profile) = &params.output_profile {
            unsafe {
                (*self.imgdata).params.output_profile =
                    CString::new(output_profile.to_str().unwrap())
                        .into_diagnostic()?
                        .into_raw();
            }
        }
        if let Some(camera_profile) = &params.camera_profile {
            unsafe {
                (*self.imgdata).params.camera_profile =
                    CString::new(camera_profile.to_str().unwrap())
                        .into_diagnostic()?
                        .into_raw();
            }
        }
        if let Some(bad_pixels) = &params.bad_pixels {
            unsafe {
                (*self.imgdata).params.bad_pixels = CString::new(bad_pixels.to_str().unwrap())
                    .into_diagnostic()?
                    .into_raw();
            }
        }
        if let Some(dark_frame) = &params.dark_frame {
            unsafe {
                (*self.imgdata).params.dark_frame = CString::new(dark_frame.to_str().unwrap())
                    .into_diagnostic()?
                    .into_raw();
            }
        }
        if let Some(output_bps) = params.output_bps {
            unsafe { (*self.imgdata).params.output_bps = i32::from(output_bps) };
        }
        if let Some(output_tiff) = params.output_tiff {
            unsafe { (*self.imgdata).params.output_tiff = i32::from(output_tiff) };
        }
        if let Some(user_flip) = params.user_flip {
            unsafe { (*self.imgdata).params.user_flip = i32::from(user_flip) };
        }
        if let Some(user_qual) = params.user_qual {
            unsafe { (*self.imgdata).params.user_qual = i32::from(user_qual) };
        }
        if let Some(user_black) = params.user_black {
            unsafe { (*self.imgdata).params.user_black = user_black };
        }
        if let Some(user_cblack) = params.user_cblack {
            unsafe { (*self.imgdata).params.user_cblack = user_cblack };
        }
        if let Some(user_black) = params.user_black {
            unsafe { (*self.imgdata).params.user_black = user_black };
        }
        if let Some(user_sat) = params.user_sat {
            unsafe { (*self.imgdata).params.user_sat = user_sat };
        }
        if let Some(med_passes) = params.med_passes {
            unsafe { (*self.imgdata).params.med_passes = med_passes };
        }
        if let Some(no_auto_bright) = params.no_auto_bright {
            unsafe { (*self.imgdata).params.no_auto_bright = no_auto_bright as i32 };
        }
        if let Some(auto_bright_thr) = params.auto_bright_thr {
            unsafe { (*self.imgdata).params.auto_bright_thr = auto_bright_thr };
        }
        if let Some(adjust_maximum_thr) = params.adjust_maximum_thr {
            unsafe { (*self.imgdata).params.adjust_maximum_thr = adjust_maximum_thr };
        }
        if let Some(use_fuji_rotate) = params.use_fuji_rotate {
            unsafe { (*self.imgdata).params.use_fuji_rotate = use_fuji_rotate as i32 };
        }
        if let Some(dcb_iterations) = params.dcb_iterations {
            unsafe { (*self.imgdata).params.dcb_iterations = dcb_iterations };
        }
        if let Some(dcb_enhance_fl) = params.dcb_enhance_fl {
            unsafe { (*self.imgdata).params.dcb_enhance_fl = dcb_enhance_fl };
        }
        if let Some(fbdd_noiserd) = params.fbdd_noiserd {
            unsafe { (*self.imgdata).params.fbdd_noiserd = i32::from(fbdd_noiserd) };
        }
        if let Some(exp_correc) = params.exp_correc {
            unsafe { (*self.imgdata).params.exp_correc = exp_correc };
        }
        if let Some(exp_shift) = params.exp_shift {
            unsafe { (*self.imgdata).params.exp_shift = exp_shift };
        }
        if let Some(exp_correc) = params.exp_correc {
            unsafe { (*self.imgdata).params.exp_correc = exp_correc };
        }
        if let Some(exp_preser) = params.exp_preser {
            unsafe { (*self.imgdata).params.exp_preser = exp_preser };
        }
        if let Some(no_auto_scale) = params.no_auto_scale {
            unsafe { (*self.imgdata).params.no_auto_scale = no_auto_scale as i32 };
        }
        if let Some(no_interpolation) = params.no_interpolation {
            unsafe { (*self.imgdata).params.no_interpolation = no_interpolation as i32 };
        }
        Ok(())
    }
    pub fn dcraw_process(&self) -> miette::Result<DcRawProcessedImage> {
        clerk::debug!("{:?}", unsafe { (*self.imgdata).params });
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

#[derive(Default)]
pub struct DCRaw {
    pub(crate) params: Option<DCRawParams>,
}
impl DCRaw {
    pub fn new(params: DCRawParams) -> Self {
        Self {
            params: Some(params),
        }
    }
}
impl IPostProcessor<Libraw, ProcessedImage> for DCRaw {
    fn post_process(&self, libraw: &Libraw) -> miette::Result<ProcessedImage> {
        if let Some(params) = &self.params {
            libraw.set_output_params(params)?;
        }
        libraw.dcraw_process()?.to_image()
    }
}
