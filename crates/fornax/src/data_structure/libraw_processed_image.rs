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
pub struct LibrawProcessedImage {
    pub processed_image: *mut libraw_sys::libraw_processed_image_t,
}
impl LibrawProcessedImage {
    pub(crate) fn new(
        ptr: *mut libraw_sys::libraw_processed_image_t,
    ) -> miette::Result<LibrawProcessedImage> {
        clerk::debug!("Is processed image null: {}", ptr.is_null());
        let img: LibrawProcessedImage = Self { processed_image: ptr };
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
    pub fn data(&self) -> *const u8 {
        unsafe { (*self.processed_image).data.as_ptr() }
    }
}
impl Drop for LibrawProcessedImage {
    fn drop(&mut self) {
        unsafe { libraw_sys::libraw_dcraw_clear_mem(self.processed_image) }
    }
}