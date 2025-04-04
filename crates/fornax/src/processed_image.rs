use core::slice;
use std::fmt::Display;
#[derive(Debug)]
pub enum ImageFormats {
    ImageBitmap = 1,
    LibrawImageJpeg = 2,
}
impl Display for ImageFormats {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            ImageFormats::ImageBitmap => write!(f, "ImageBitmap"),
            ImageFormats::LibrawImageJpeg => write!(f, "LibrawImageJpeg"),
        }
    }
}
pub struct ProcessedImage {
    processed_image: *mut libraw_sys::libraw_processed_image_t,
}
impl ProcessedImage {
    pub(crate) fn new(
        ptr: *mut libraw_sys::libraw_processed_image_t,
    ) -> miette::Result<ProcessedImage> {
        clerk::debug!("Is processed image null: {}", ptr.is_null());
        let img: ProcessedImage = Self {
            processed_image: ptr,
        };
        Ok(img)
    }
    pub fn image_type(&self) -> miette::Result<ImageFormats> {
        match unsafe { (*self.processed_image).type_ } {
            1i32 => Ok(ImageFormats::ImageBitmap),
            2i32 => Ok(ImageFormats::LibrawImageJpeg),
            t => miette::bail!("Unknow image format: {t}"),
        }
    }
    pub fn height(&self) -> u16 {
        unsafe { (*self.processed_image).height }
    }
    pub fn width(&self) -> u16 {
        unsafe { (*self.processed_image).width }
    }
    pub fn colors(&self) -> u16 {
        unsafe { (*self.processed_image).colors }
    }
    pub fn bits(&self) -> u16 {
        unsafe { (*self.processed_image).bits }
    }
    pub fn data_size(&self) -> u32 {
        unsafe { (*self.processed_image).data_size }
    }
}
impl Drop for ProcessedImage {
    fn drop(&mut self) {
        unsafe { libraw_sys::libraw_dcraw_clear_mem(self.processed_image) }
    }
}

#[cfg(feature = "image")]
impl ProcessedImage {
    pub fn to_image(&self) -> miette::Result<image::DynamicImage> {
        match (self.colors(), self.bits()) {
            (1, 8) => {
                let img: image::ImageBuffer<image::Luma<u8>, Vec<u8>> =
                    image::ImageBuffer::from_vec(
                        self.width() as u32,
                        self.height() as u32,
                        unsafe {
                            slice::from_raw_parts(
                                (*self.processed_image).data.as_mut_ptr(),
                                self.data_size() as usize,
                            )
                            .to_vec()
                        },
                    )
                    .unwrap();
                Ok(image::DynamicImage::from(img))
            }
            (1, 16) => {
                let img: image::ImageBuffer<image::Luma<u16>, Vec<u16>> =
                    image::ImageBuffer::from_vec(
                        self.width() as u32,
                        self.height() as u32,
                        bytemuck::cast_slice::<u8, u16>(unsafe {
                            slice::from_raw_parts(
                                (*self.processed_image).data.as_mut_ptr(),
                                self.data_size() as usize,
                            )
                        })
                        .to_vec(),
                    )
                    .unwrap();

                Ok(image::DynamicImage::from(img))
            }
            (3, 8) => {
                let img: image::ImageBuffer<image::Rgb<u8>, Vec<u8>> =
                    image::ImageBuffer::from_vec(
                        self.width() as u32,
                        self.height() as u32,
                        unsafe {
                            slice::from_raw_parts(
                                (*self.processed_image).data.as_mut_ptr(),
                                self.data_size() as usize,
                            )
                            .to_vec()
                        },
                    )
                    .unwrap();
                Ok(image::DynamicImage::from(img))
            }
            (3, 16) => {
                let img: image::ImageBuffer<image::Luma<u16>, Vec<u16>> =
                    image::ImageBuffer::from_vec(
                        self.width() as u32,
                        self.height() as u32,
                        bytemuck::cast_slice::<u8, u16>(unsafe {
                            slice::from_raw_parts(
                                (*self.processed_image).data.as_mut_ptr(),
                                self.data_size() as usize,
                            )
                        })
                        .to_vec(),
                    )
                    .unwrap();
                Ok(image::DynamicImage::from(img))
            }
            (c, b) => {
                miette::bail!("Unsupported color:{}, bits: {}.", c, b)
            }
        }
    }
}
