use std::slice;

use fornax_core::FornaxPrimitive;
use image::{ImageBuffer, Rgb};

use crate::{
    DCRawParams, Libraw, LibrawError, ProcessedImage, check_raw_alloc, check_run, custom_error,
};

// region:Custom API
impl Libraw {
    pub fn new(params: Option<DCRawParams>) -> Self {
        Self {
            imgdata: Self::libraw_init(),
            params,
        }
    }
    pub fn bayer_pattern(&self) -> Result<fornax_core::BayerPattern, LibrawError> {
        check_raw_alloc!(self.imgdata);
        let pattern0 = self.color(0, 0);
        let pattern1 = self.color(0, 1);
        let pattern2 = self.color(1, 0);
        let pattern3 = self.color(1, 1);
        match (pattern0, pattern1, pattern2, pattern3) {
            (0, 1, 3, 2) => Ok(fornax_core::BayerPattern::RGGB),
            (2, 3, 1, 0) => Ok(fornax_core::BayerPattern::BGGR),
            (1, 0, 2, 3) => Ok(fornax_core::BayerPattern::GRBG),
            (3, 2, 0, 1) => Ok(fornax_core::BayerPattern::GBRG),
            (a, b, c, d) => custom_error!(format!("Unknown bayer pattern: {a}, {b}, {c}, {d}")),
        }
    }
    pub fn get_bayer_image<T>(&self) -> Result<fornax_core::BayerImage<T>, LibrawError>
    where
        T: FornaxPrimitive,
    {
        let pattern = self.bayer_pattern()?;
        let raw_img = self.get_raw_image(true)?;
        let img = ImageBuffer::from_par_fn(raw_img.width(), raw_img.height(), |x, y| {
            let pixel = raw_img.get_pixel(x, y);
            let value = T::from(pixel[0].max(pixel[1]).max(pixel[2]).max(pixel[3])).unwrap();

            let value =
            // u16 -> u8
            if std::any::TypeId::of::<T>() == std::any::TypeId::of::<u8>() {
                value / T::from(255).unwrap()
            }
             // u16 -> u16
            else if std::any::TypeId::of::<T>() == std::any::TypeId::of::<u16>() {
                value
            }
            // u16 -> f32/f64
            else if std::any::TypeId::of::<T>() == std::any::TypeId::of::<f32>()
                || std::any::TypeId::of::<T>() == std::any::TypeId::of::<f64>()
            {
                value / T::from(65535).unwrap()
            } else {
                panic!()
            };

            image::Luma::<T>([value])
        });
        Ok(fornax_core::BayerImage::new(img, pattern))
    }
    pub(crate) fn map_processed_image<O>(
        &self,
        processed: &ProcessedImage,
    ) -> Result<image::ImageBuffer<Rgb<O>, Vec<O>>, LibrawError>
    where
        O: FornaxPrimitive,
    {
        match (processed.colors(), processed.bits()) {
            (3, 8) => {
                let img: image::ImageBuffer<image::Rgb<O>, Vec<O>> = image::ImageBuffer::from_vec(
                    processed.width() as u32,
                    processed.height() as u32,
                    unsafe {
                        slice::from_raw_parts(processed.data(), processed.data_size() as usize)
                            .iter()
                            .copied()
                            .map(|v| {
                                // u8 -> u8
                                if std::any::TypeId::of::<O>() == std::any::TypeId::of::<u8>() {
                                    O::from(v).unwrap()
                                }
                                // u8 -> u16
                                else if std::any::TypeId::of::<O>()
                                    == std::any::TypeId::of::<u16>()
                                {
                                    O::from(v).unwrap() * O::from(255).unwrap()
                                }
                                // u8 -> f32/f64
                                else if std::any::TypeId::of::<O>()
                                    == std::any::TypeId::of::<f32>()
                                    || std::any::TypeId::of::<O>() == std::any::TypeId::of::<f64>()
                                {
                                    O::from(v).unwrap() / O::from(255).unwrap()
                                } else {
                                    panic!()
                                }
                            })
                            .collect()
                    },
                )
                .unwrap();
                Ok(img)
            }
            (3, 16) => {
                let img: image::ImageBuffer<image::Rgb<O>, Vec<O>> = image::ImageBuffer::from_vec(
                    processed.width() as u32,
                    processed.height() as u32,
                    bytemuck::cast_slice::<u8, u16>(unsafe {
                        slice::from_raw_parts(processed.data(), processed.data_size() as usize)
                    })
                    .iter()
                    .copied()
                    .map(|v| {
                        // u16 -> u8
                        if std::any::TypeId::of::<O>() == std::any::TypeId::of::<u8>() {
                            O::from(v).unwrap() / O::from(255).unwrap()
                        }
                        // u16 -> u16
                        else if std::any::TypeId::of::<O>() == std::any::TypeId::of::<u16>() {
                            O::from(v).unwrap()
                        }
                        // u16 -> f32/f64
                        else if std::any::TypeId::of::<O>() == std::any::TypeId::of::<f32>()
                            || std::any::TypeId::of::<O>() == std::any::TypeId::of::<f64>()
                        {
                            O::from(v).unwrap() / O::from(65536).unwrap()
                        } else {
                            panic!()
                        }
                    })
                    .collect(),
                )
                .unwrap();
                Ok(img)
            }
            (c, b) => custom_error!(format!("Unsupported color:{}, bits: {}.", c, b)),
        }
    }
    pub fn get_raw_image(
        &self,
        subtract_black: bool,
    ) -> Result<ImageBuffer<image::Rgba<u16>, Vec<u16>>, LibrawError> {
        self.raw2image()?;
        check_run!(unsafe { libraw_sys::libraw_raw2image(self.imgdata.0) });
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
                    (*self.imgdata.0).image as *const u16,
                    width as usize * height as usize * 4,
                )
                .to_vec()
            })
            .unwrap();
        Ok(img)
    }
    pub fn get_params(&self) -> Result<libraw_sys::libraw_output_params_t, LibrawError> {
        Ok(unsafe { (*self.imgdata.0).params })
    }
}
