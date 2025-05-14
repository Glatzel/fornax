use std::ffi::CString;

use miette::IntoDiagnostic;

use crate::{DCRawOutputBps, DCRawOutputColor, DCRawParams};

impl DCRawParams {
    pub(crate) fn set_output_params(
        &self,
        imgdata: *mut libraw_sys::libraw_data_t,
    ) -> miette::Result<()> {
        if let Some(graybox) = self.greybox {
            unsafe { (*imgdata).params.greybox = graybox };
        }
        if let Some(cropbox) = self.cropbox {
            unsafe { (*imgdata).params.cropbox = cropbox };
        }
        if let Some(aber) = self.aber {
            unsafe {
                (*imgdata).params.aber[0] = aber[0];
                (*imgdata).params.aber[2] = aber[1];
            }
        }
        if let Some(gamm) = self.gamm {
            unsafe {
                (*imgdata).params.gamm[0] = gamm[0];
                (*imgdata).params.gamm[1] = gamm[1];
            }
        }
        if let Some(user_mul) = self.user_mul {
            unsafe { (*imgdata).params.user_mul = user_mul };
        }
        if let Some(bright) = self.bright {
            unsafe { (*imgdata).params.bright = bright };
        }
        if let Some(threshold) = self.threshold {
            unsafe { (*imgdata).params.threshold = threshold };
        }
        if let Some(half_size) = self.half_size {
            unsafe { (*imgdata).params.half_size = half_size as i32 };
        }
        if let Some(four_color_rgb) = self.four_color_rgb {
            unsafe { (*imgdata).params.four_color_rgb = four_color_rgb as i32 };
        }
        if let Some(highlight) = self.highlight {
            unsafe { (*imgdata).params.highlight = highlight.into() };
        }
        if let Some(use_auto_wb) = self.use_auto_wb {
            unsafe { (*imgdata).params.use_auto_wb = use_auto_wb as i32 };
        }
        if let Some(use_camera_wb) = self.use_camera_wb {
            unsafe { (*imgdata).params.use_camera_wb = use_camera_wb as i32 };
        }
        if let Some(use_camera_matrix) = self.use_camera_matrix {
            unsafe { (*imgdata).params.use_camera_matrix = use_camera_matrix.into() };
        }
        if let Some(output_color) = self.output_color {
            unsafe { (*imgdata).params.output_color = output_color.into() };
        }
        if let Some(output_profile) = &self.output_profile {
            unsafe {
                (*imgdata).params.output_profile = CString::new(output_profile.to_str().unwrap())
                    .into_diagnostic()?
                    .into_raw();
            }
        }
        if let Some(camera_profile) = &self.camera_profile {
            unsafe {
                (*imgdata).params.camera_profile = CString::new(camera_profile.to_str().unwrap())
                    .into_diagnostic()?
                    .into_raw();
            }
        }
        if let Some(bad_pixels) = &self.bad_pixels {
            unsafe {
                (*imgdata).params.bad_pixels = CString::new(bad_pixels.to_str().unwrap())
                    .into_diagnostic()?
                    .into_raw();
            }
        }
        if let Some(dark_frame) = &self.dark_frame {
            unsafe {
                (*imgdata).params.dark_frame = CString::new(dark_frame.to_str().unwrap())
                    .into_diagnostic()?
                    .into_raw();
            }
        }
        if let Some(output_bps) = self.output_bps {
            unsafe { (*imgdata).params.output_bps = output_bps.into() };
        }
        if let Some(output_tiff) = self.output_tiff {
            unsafe { (*imgdata).params.output_tiff = output_tiff.into() };
        }
        if let Some(user_flip) = self.user_flip {
            unsafe { (*imgdata).params.user_flip = user_flip.into() };
        }
        if let Some(user_qual) = self.user_qual {
            unsafe { (*imgdata).params.user_qual = user_qual.into() };
        }
        if let Some(user_black) = self.user_black {
            unsafe { (*imgdata).params.user_black = user_black };
        }
        if let Some(user_cblack) = self.user_cblack {
            unsafe { (*imgdata).params.user_cblack = user_cblack };
        }
        if let Some(user_black) = self.user_black {
            unsafe { (*imgdata).params.user_black = user_black };
        }
        if let Some(user_sat) = self.user_sat {
            unsafe { (*imgdata).params.user_sat = user_sat };
        }
        if let Some(med_passes) = self.med_passes {
            unsafe { (*imgdata).params.med_passes = med_passes };
        }
        if let Some(no_auto_bright) = self.no_auto_bright {
            unsafe { (*imgdata).params.no_auto_bright = no_auto_bright as i32 };
        }
        if let Some(auto_bright_thr) = self.auto_bright_thr {
            unsafe { (*imgdata).params.auto_bright_thr = auto_bright_thr };
        }
        if let Some(adjust_maximum_thr) = self.adjust_maximum_thr {
            unsafe { (*imgdata).params.adjust_maximum_thr = adjust_maximum_thr };
        }
        if let Some(use_fuji_rotate) = self.use_fuji_rotate {
            unsafe { (*imgdata).params.use_fuji_rotate = use_fuji_rotate as i32 };
        }
        if let Some(dcb_iterations) = self.dcb_iterations {
            unsafe { (*imgdata).params.dcb_iterations = dcb_iterations };
        }
        if let Some(dcb_enhance_fl) = self.dcb_enhance_fl {
            unsafe { (*imgdata).params.dcb_enhance_fl = dcb_enhance_fl };
        }
        if let Some(fbdd_noiserd) = self.fbdd_noiserd {
            unsafe { (*imgdata).params.fbdd_noiserd = fbdd_noiserd.into() };
        }
        if let Some(exp_correc) = self.exp_correc {
            unsafe { (*imgdata).params.exp_correc = exp_correc };
        }
        if let Some(exp_shift) = self.exp_shift {
            unsafe { (*imgdata).params.exp_shift = exp_shift };
        }
        if let Some(exp_correc) = self.exp_correc {
            unsafe { (*imgdata).params.exp_correc = exp_correc };
        }
        if let Some(exp_preser) = self.exp_preser {
            unsafe { (*imgdata).params.exp_preser = exp_preser };
        }
        if let Some(no_auto_scale) = self.no_auto_scale {
            unsafe { (*imgdata).params.no_auto_scale = no_auto_scale as i32 };
        }
        if let Some(no_interpolation) = self.no_interpolation {
            unsafe { (*imgdata).params.no_interpolation = no_interpolation as i32 };
        }
        Ok(())
    }
}
// presets
impl DCRawParams {
    /// Match output to cg workflow.
    /// - `gamm` = `[1.0, 1.0]`
    /// - `output_color`: ACES
    /// - `output_bps`: 16bit
    pub fn preset_cg() -> Self {
        Self {
            gamm: Some([1.0, 1.0]),
            output_color: Some(DCRawOutputColor::ACES),
            output_bps: Some(DCRawOutputBps::_16bit),
            ..Default::default()
        }
    }
}
