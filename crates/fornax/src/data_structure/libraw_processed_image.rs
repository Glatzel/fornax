pub enum ImageFormats {
    ImageBitmap = 1,
    LibrawImageJpeg = 2,
}
pub struct LibrawProcessedImage {
    pub image_type: ImageFormats,
    pub height: u16,
    pub width: u16,
    pub colors: u16,
    pub bis: u16,
    pub data_size: u32,
    pub data: [u8; 1],
}
impl LibrawProcessedImage {
    pub(crate) fn new(
        processed_image: &libraw_sys::libraw_processed_image_t,
    ) -> miette::Result<LibrawProcessedImage> {
        let img: LibrawProcessedImage = Self {
            image_type: match processed_image.type_ {
                1 => ImageFormats::ImageBitmap,
                2 => ImageFormats::LibrawImageJpeg,
                t => miette::bail!("Unknow processed_image type: {t}"),
            },
            height: processed_image.height,
            width: processed_image.width,
            colors: processed_image.colors,
            bis: processed_image.bits,
            data_size: processed_image.data_size,
            data: processed_image.data,
        };
        Ok(img)
    }
}
