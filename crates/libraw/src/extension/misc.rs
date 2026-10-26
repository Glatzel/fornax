use std::slice;

use fornax_core::{BayerPattern, MonoImage, Primitive};
use image::ImageBuffer;

use super::super::{Libraw, LibrawError};
use super::error_handling::{check_run, custom_error};

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
        let color = |row: u32, col: u32| -> u32 { (filters >> (((row << 1) & 14) + col) * 2) & 3 };

        match (color(0, 0), color(0, 1), color(1, 0), color(1, 1)) {
            (0, 1, 3, 2) => Ok(BayerPattern::RGGB),
            (2, 3, 1, 0) => Ok(BayerPattern::BGGR),
            (1, 0, 2, 3) => Ok(BayerPattern::GRBG),
            (3, 2, 0, 1) => Ok(BayerPattern::GBRG),
            _ => custom_error!("unsupported LibRaw CFA pattern {filters:#010x}"),
        }
    }
    pub fn get_bayer_image<T>(&self) -> Result<MonoImage<T>, LibrawError>
    where
        T: Primitive + 'static,
    {
        let raw_img = self.get_raw_image(true)?;
        let img = ImageBuffer::from_par_fn(raw_img.width(), raw_img.height(), |x, y| {
            let pixel = raw_img.get_pixel(x, y);
            let value = T::from(pixel[0].max(pixel[1]).max(pixel[2]).max(pixel[3])).unwrap();

            let value =
            // u16 -> u8
            if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u8>() {
                value / T::from(255).unwrap()
            }
             // u16 -> u16
            else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<u16>() {
                value
            }
            // u16 -> f32/f64
            else if core::any::TypeId::of::<T>() == core::any::TypeId::of::<f32>()
                || core::any::TypeId::of::<T>() == core::any::TypeId::of::<f64>()
            {
                value / T::from(65535).unwrap()
            } else {
                panic!()
            };

            image::Luma::<T>([value])
        });
        Ok(img)
    }
    pub fn get_raw_image(
        &self,
        subtract_black: bool,
    ) -> Result<ImageBuffer<image::Rgba<u16>, Vec<u16>>, LibrawError> {
        self.raw2image()?;
        check_run!(unsafe { libraw_sys::libraw_raw2image(self.imgdata_ptr()) });
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
