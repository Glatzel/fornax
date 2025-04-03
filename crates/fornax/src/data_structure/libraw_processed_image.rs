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
    pub ptr: *mut libraw_sys::libraw_processed_image_t,
}
impl LibrawProcessedImage {
    pub(crate) fn new(
        ptr: *mut libraw_sys::libraw_processed_image_t,
    ) -> miette::Result<LibrawProcessedImage> {
        clerk::debug!("Is processed image null: {}", ptr.is_null());
        let img: LibrawProcessedImage = Self { ptr };
        Ok(img)
    }
    pub fn image_type(&self) -> miette::Result<ImageFormats> {
        match unsafe { (*self.ptr).type_ } {
            1i32 => Ok(ImageFormats::ImageBitmap),
            2i32 => Ok(ImageFormats::LibrawImageJpeg),
            t => miette::bail!("Unknow image format: {t}"),
        }
    }
    pub fn height(&self) -> u16 {
        unsafe { (*self.ptr).height }
    }
    pub fn width(&self) -> u16 {
        unsafe { (*self.ptr).width }
    }
    pub fn colors(&self) -> u16 {
        unsafe { (*self.ptr).colors }
    }
    pub fn bits(&self) -> u16 {
        unsafe { (*self.ptr).bits }
    }
    pub fn data_size(&self) -> u32 {
        unsafe { (*self.ptr).data_size }
    }
    pub fn data(&self) -> *const u8 {
        unsafe { (*self.ptr).data.as_ptr() }
    }
}
