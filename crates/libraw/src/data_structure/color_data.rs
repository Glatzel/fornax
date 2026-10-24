use std::sync::Arc;

use crate::extension::error_handling::check_raw_alloc;
use crate::{ImgdataPtr, Libraw, LibrawError};

/// Native LibRaw color and level data for an active decode context.
#[derive(Debug, Clone)]
pub struct ColorData {
    arc_imgdata_ptr: Arc<ImgdataPtr>,
}

impl Libraw {
    /// Returns native color and level data.
    ///
    /// # Errors
    ///
    /// Returns an error when LibRaw has not allocated a raw image.
    pub fn get_color_data(&self) -> Result<ColorData, LibrawError> {
        check_raw_alloc!(self.imgdata_ptr());
        ColorData::new(self.arc_imgdata_ptr())
    }
}

impl ColorData {
    pub(crate) const fn new(imgdata: Arc<ImgdataPtr>) -> Result<Self, LibrawError> {
        Ok(Self {
            arc_imgdata_ptr: imgdata,
        })
    }

    /// Returns the source bits-per-sample value.
    pub fn raw_bps(&self) -> u32 { unsafe { (*self.arc_imgdata_ptr.ptr()).color.raw_bps } }

    /// Returns the common black-level component.
    pub fn black(&self) -> u32 { unsafe { (*self.arc_imgdata_ptr.ptr()).color.black } }

    /// Returns the four color offsets and the repeating-pattern dimensions.
    pub fn cblack(&self) -> [u32; 6] {
        let cblack = unsafe { (*self.arc_imgdata_ptr.ptr()).color.cblack };
        [
            cblack[0], cblack[1], cblack[2], cblack[3], cblack[4], cblack[5],
        ]
    }

    /// Returns LibRaw's processing maximum.
    pub fn maximum(&self) -> u32 { unsafe { (*self.arc_imgdata_ptr.ptr()).color.maximum } }

    /// Returns LibRaw's measured data maximum.
    pub fn data_maximum(&self) -> u32 {
        unsafe { (*self.arc_imgdata_ptr.ptr()).color.data_maximum }
    }

    /// Returns the four per-color linear maxima.
    pub fn linear_max(&self) -> [u32; 4] {
        unsafe { (*self.arc_imgdata_ptr.ptr()).color.linear_max }
    }

    /// Returns the four DNG-declared white levels.
    pub fn dng_whitelevel(&self) -> [u32; 4] {
        unsafe {
            (*self.arc_imgdata_ptr.ptr())
                .color
                .dng_levels
                .dng_whitelevel
        }
    }

    /// Returns DNG crop origin and size as `[x, y, width, height]`.
    pub fn dng_default_crop(&self) -> [u16; 4] {
        unsafe { (*self.arc_imgdata_ptr.ptr()).color.dng_levels.default_crop }
    }
}
