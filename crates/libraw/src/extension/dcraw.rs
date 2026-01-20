use std::sync::Arc;

use envoy::ToCString;

use crate::{DCRawOutputBps, DCRawOutputColor, DCRawParams, ImgdataPointer, LibrawError};

impl DCRawParams {
    #[allow(unused_mut)]
    pub(crate) fn set_output_params(
        &self,
        mut imgdata: Arc<ImgdataPointer>,
    ) -> Result<(), LibrawError> {
        self.greybox
            .inspect(|v| unsafe { (**imgdata).params.greybox = *v });
        self.cropbox
            .inspect(|v| unsafe { (**imgdata).params.cropbox = *v });
        self.aber.inspect(|v| unsafe {
            (**imgdata).params.aber[0] = v[0];
            (**imgdata).params.aber[2] = v[1];
        });
        self.gamm.inspect(|v| unsafe {
            (**imgdata).params.gamm[0] = v[0];
            (**imgdata).params.gamm[1] = v[1];
        });
        self.user_mul
            .inspect(|v| unsafe { (**imgdata).params.user_mul = *v });
        self.bright
            .inspect(|v| unsafe { (**imgdata).params.bright = *v });
        self.threshold
            .inspect(|v| unsafe { (**imgdata).params.threshold = *v });
        self.half_size
            .inspect(|v| unsafe { (**imgdata).params.half_size = *v as i32 });
        self.four_color_rgb
            .inspect(|v| unsafe { (**imgdata).params.four_color_rgb = *v as i32 });
        self.highlight
            .inspect(|v| unsafe { (**imgdata).params.highlight = (*v) as i32 });
        self.use_auto_wb
            .inspect(|v| unsafe { (**imgdata).params.use_auto_wb = *v as i32 });
        self.use_camera_wb
            .inspect(|v| unsafe { (**imgdata).params.use_camera_wb = *v as i32 });
        self.use_camera_matrix
            .inspect(|v| unsafe { (**imgdata).params.use_camera_matrix = (*v) as i32 });
        self.output_color
            .inspect(|v| unsafe { (**imgdata).params.output_color = (*v) as i32 });

        if let Some(v) = self.output_profile.as_ref() {
            unsafe {
                (**imgdata).params.output_profile = v.to_str().unwrap().to_cstring()?.into_raw();
            }
        }
        if let Some(v) = self.camera_profile.as_ref() {
            unsafe {
                (**imgdata).params.camera_profile = v.to_str().unwrap().to_cstring()?.into_raw();
            }
        }

        if let Some(v) = self.bad_pixels.as_ref() {
            unsafe {
                (**imgdata).params.bad_pixels = v.to_str().unwrap().to_cstring()?.into_raw();
            }
        }

        if let Some(v) = self.dark_frame.as_ref() {
            unsafe {
                (**imgdata).params.dark_frame = v.to_str().unwrap().to_cstring()?.into_raw();
            }
        }

        self.output_bps
            .inspect(|v| unsafe { (**imgdata).params.output_bps = (*v) as i32 });
        self.output_tiff
            .inspect(|v| unsafe { (**imgdata).params.output_tiff = (*v) as i32 });
        self.user_flip
            .inspect(|v| unsafe { (**imgdata).params.user_flip = (*v) as i32 });
        self.user_qual
            .inspect(|v| unsafe { (**imgdata).params.user_qual = (*v) as i32 });
        self.user_black
            .inspect(|v| unsafe { (**imgdata).params.user_black = *v });
        self.user_cblack
            .inspect(|v| unsafe { (**imgdata).params.user_cblack = *v });
        self.user_sat
            .inspect(|v| unsafe { (**imgdata).params.user_sat = *v });
        self.med_passes
            .inspect(|v| unsafe { (**imgdata).params.med_passes = *v });
        self.no_auto_bright
            .inspect(|v| unsafe { (**imgdata).params.no_auto_bright = *v as i32 });
        self.auto_bright_thr
            .inspect(|v| unsafe { (**imgdata).params.auto_bright_thr = *v });
        self.adjust_maximum_thr
            .inspect(|v| unsafe { (**imgdata).params.adjust_maximum_thr = *v });
        self.use_fuji_rotate
            .inspect(|v| unsafe { (**imgdata).params.use_fuji_rotate = *v as i32 });
        self.dcb_iterations
            .inspect(|v| unsafe { (**imgdata).params.dcb_iterations = *v });
        self.dcb_enhance_fl
            .inspect(|v| unsafe { (**imgdata).params.dcb_enhance_fl = *v });
        self.fbdd_noiserd
            .inspect(|v| unsafe { (**imgdata).params.fbdd_noiserd = (*v) as i32 });
        self.exp_correc
            .inspect(|v| unsafe { (**imgdata).params.exp_correc = *v });
        self.exp_shift
            .inspect(|v| unsafe { (**imgdata).params.exp_shift = *v });
        self.exp_preser
            .inspect(|v| unsafe { (**imgdata).params.exp_preser = *v });
        // self.use_rawspeed
        //     .inspect(|v| unsafe { (**imgdata).params.use_rawspeed = *v });
        // self.use_dng_sdk
        //     .inspect(|v| unsafe { (**imgdata).params.use_dng_sdk = *v });
        self.no_auto_scale
            .inspect(|v| unsafe { (**imgdata).params.no_auto_scale = *v as i32 });
        self.no_interpolation
            .inspect(|v| unsafe { (**imgdata).params.no_interpolation = *v as i32 });
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
