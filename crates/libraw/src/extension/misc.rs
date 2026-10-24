use std::slice;

use image::ImageBuffer;

use crate::{Libraw, LibrawError};

// region:Custom API
impl Libraw {
    pub fn get_raw_image(
        &self,
        subtract_black: bool,
    ) -> Result<ImageBuffer<image::Rgba<u16>, Vec<u16>>, LibrawError> {
        self.raw2image()?;
        if subtract_black {
            self.libraw_subtract_black()?;
        }

        let size = self.get_image_sizes()?;
        let width = size.iwidth();
        let height = size.iheight();
        clerk::debug!("Width: {width}, Height: {height}");

        clerk::debug!("Found rgba16 raw image.");
        let img: ImageBuffer<image::Rgba<u16>, Vec<u16>> =
            ImageBuffer::from_vec(width as u32, height as u32, unsafe {
                slice::from_raw_parts(
                    (*self.imgdata_ptr()).image as *const u16,
                    width as usize * height as usize * 4,
                )
                .to_vec()
            })
            .unwrap();
        Ok(img)
    }
    pub fn get_params(&self) -> Result<libraw_sys::libraw_output_params_t, LibrawError> {
        Ok(unsafe { (*self.imgdata_ptr()).params })
    }
}
