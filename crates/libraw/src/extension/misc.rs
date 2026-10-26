use fornax_core::{BayerPattern, DecodedRawActive, DecodedRawFull, MonoImage, Primitive};
use generic_num::num;

use super::super::{Libraw, LibrawError};
use super::error_handling::custom_error;
use crate::extension::error_handling::check_raw_alloc;

// region:Custom API
impl Libraw {
    pub fn bayer_pattern(&self) -> Result<BayerPattern, LibrawError> {
        let filters = self.get_iparams()?.filters();
        match filters {
            0 => {
                custom_error!("non-mosaic RAW is unsupported");
            }
            1 => {
                custom_error!("16x16 CFA is unsupported");
            }
            9 => {
                custom_error!("X-Trans CFA is unsupported");
            }
            2..=8 | 10..=999 => {
                custom_error!(format!("reserved LibRaw CFA descriptor {filters}"));
            }
            _ => {}
        }

        // LibRaw COLOR(row, column) uses two bits per CFA color.
        let color =
            |row: u32, col: u32| -> u32 { (filters >> ((((row << 1) & 14) + col) * 2)) & 3 };

        match (color(0, 0), color(0, 1), color(1, 0), color(1, 1)) {
            (0, 1, 3, 2) => Ok(BayerPattern::RGGB),
            (2, 3, 1, 0) => Ok(BayerPattern::BGGR),
            (1, 0, 2, 3) => Ok(BayerPattern::GRBG),
            (3, 2, 0, 1) => Ok(BayerPattern::GBRG),
            _ => custom_error!("unsupported LibRaw CFA pattern {filters:#010x}"),
        }
    }
    pub fn get_decoded_raw_active<T>(&self) -> Result<DecodedRawActive<T>, LibrawError>
    where
        T: Primitive,
    {
        check_raw_alloc!(self.imgdata_ptr());

        let imgdata_ptr = self.arc_imgdata_ptr();
        let imgdata = &*imgdata_ptr;
        let sizes = &imgdata.rawdata.sizes;

        let raw_w = sizes.raw_width as usize;
        let raw_h = sizes.raw_height as usize;
        let left = sizes.left_margin as usize;
        let top = sizes.top_margin as usize;
        let w = sizes.width as usize;
        let h = sizes.height as usize;
        if w == 0 || h == 0 || left.saturating_add(w) > raw_w || top.saturating_add(h) > raw_h {
            custom_error!(format!(
                "invalid LibRaw active area ({left}, {top}, {w}, {h}) for {raw_w}x{raw_h} raw image"
            ));
        }

        // Source stride in elements. raw_pitch is in bytes; fall back to
        // raw_width. (raw_pitch can be 0 for float/DNG paths, so don't
        // rely on it blindly.)
        let stride = raw_w;

        let image = if !imgdata.rawdata.raw_image.is_null() {
            let src =
                unsafe { std::slice::from_raw_parts(imgdata.rawdata.raw_image, raw_w * raw_h) };
            let mut pixels: Vec<T> = Vec::with_capacity(w * h);
            for y in 0..h {
                let row = &src[(y + top) * stride + left..][..w];
                pixels.extend(row.iter().map(|&p| num!(p)));
            }
            MonoImage::from_vec(w as _, h as _, pixels)
        } else if !imgdata.rawdata.float_image.is_null() {
            let src =
                unsafe { std::slice::from_raw_parts(imgdata.rawdata.float_image, raw_w * raw_h) };
            let mut pixels: Vec<T> = Vec::with_capacity(w * h);
            for y in 0..h {
                let row = &src[(y + top) * stride + left..][..w];
                pixels.extend(row.iter().map(|&p| num!(p)));
            }
            MonoImage::from_vec(w as _, h as _, pixels)
        } else {
            custom_error!("LibRaw decoded data contains neither a mono raw image nor a float image")
        };

        let image = image.ok_or_else(|| {
            LibrawError::Other(
                format!("LibRaw active image data is inconsistent with {w}x{h} dimensions").into(),
            )
        })?;
        Ok(DecodedRawActive::new(image))
    }
    pub fn get_decoded_raw_full<T>(&self) -> Result<DecodedRawFull<T>, LibrawError>
    where
        T: Primitive,
    {
        check_raw_alloc!(self.imgdata_ptr());

        let imgdata_ptr = self.arc_imgdata_ptr();
        let imgdata = &*imgdata_ptr;
        let sizes = &imgdata.rawdata.sizes;

        let w = sizes.raw_width as usize;
        let h = sizes.raw_height as usize;
        if w == 0 || h == 0 {
            custom_error!(format!("invalid LibRaw raw dimensions {w}x{h}"));
        }
        let len = w * h;

        let image = if !imgdata.rawdata.raw_image.is_null() {
            let pixels = unsafe { std::slice::from_raw_parts(imgdata.rawdata.raw_image, len) }
                .iter()
                .map(|&p| num!(p))
                .collect();
            MonoImage::from_vec(w as _, h as _, pixels)
        } else if !imgdata.rawdata.float_image.is_null() {
            let pixels = unsafe { std::slice::from_raw_parts(imgdata.rawdata.float_image, len) }
                .iter()
                .map(|&p| num!(p))
                .collect();
            MonoImage::from_vec(w as _, h as _, pixels)
        } else {
            custom_error!("LibRaw decoded data contains neither a mono raw image nor a float image")
        };

        let image = image.ok_or_else(|| {
            LibrawError::Other(
                format!("LibRaw full image data is inconsistent with {w}x{h} dimensions").into(),
            )
        })?;
        Ok(DecodedRawFull::new(image))
    }
    pub fn get_params(&self) -> Result<libraw_sys::libraw_output_params_t, LibrawError> {
        let imgdata = self.arc_imgdata_ptr();
        Ok(imgdata.params)
    }
}
