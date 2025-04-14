/// Basic raw image.
pub enum FornaxProcessedImage {
    Null,
    Mono8(image::ImageBuffer<image::Luma<u8>, Vec<u8>>),
    Mono16(image::ImageBuffer<image::Luma<u16>, Vec<u16>>),
    MonoF32(image::ImageBuffer<image::Luma<f32>, Vec<f32>>),
    Rgb8(image::ImageBuffer<image::Rgb<u8>, Vec<u8>>),
    Rgb16(image::ImageBuffer<image::Rgb<u16>, Vec<u16>>),
    RgbF32(image::ImageBuffer<image::Rgb<f32>, Vec<f32>>),
}

impl FornaxProcessedImage {
    pub fn to_dynamic_image(self) -> image::DynamicImage {
        match self {
            FornaxProcessedImage::Null => panic!("Processed image is null."),
            FornaxProcessedImage::Mono8(image_buffer) => image::DynamicImage::from(image_buffer),
            FornaxProcessedImage::Mono16(image_buffer) => image::DynamicImage::from(image_buffer),
            FornaxProcessedImage::MonoF32(image_buffer) => image::DynamicImage::from(image_buffer),
            FornaxProcessedImage::Rgb8(image_buffer) => image::DynamicImage::from(image_buffer),
            FornaxProcessedImage::Rgb16(image_buffer) => image::DynamicImage::from(image_buffer),
            FornaxProcessedImage::RgbF32(image_buffer) => image::DynamicImage::from(image_buffer),
        }
    }
}

