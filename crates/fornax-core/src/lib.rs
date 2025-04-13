use std::path::Path;

pub trait IDecoder {
    fn decode_file(&self, file: &Path) -> miette::Result<()>;
    fn decode_buffer(&self, buffer: &[u8]) -> miette::Result<()>;
}

pub trait IPostProcessor<D>
where
    D: IDecoder,
{
    fn post_process(&self, decoder: &D) -> miette::Result<FornaxProcessedImage>;
}

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
pub enum FornaxRawImage {
    Null,
    Mono16(image::ImageBuffer<image::Luma<u16>, Vec<u16>>),
    Rgb16(image::ImageBuffer<image::Rgb<u16>, Vec<u16>>),
    Rgba16(image::ImageBuffer<image::Rgba<u16>, Vec<u16>>),
    MonoF32(image::ImageBuffer<image::Luma<f32>, Vec<f32>>),
    RgbF32(image::ImageBuffer<image::Rgb<f32>, Vec<f32>>),
    RgbaF32(image::ImageBuffer<image::Rgba<f32>, Vec<f32>>),
}
impl FornaxRawImage {
    pub fn to_dynamic_image(self) -> image::DynamicImage {
        match self {
            FornaxRawImage::Null => panic!("Raw image is null."),
            FornaxRawImage::Mono16(image_buffer) => image::DynamicImage::from(image_buffer),
            FornaxRawImage::Rgb16(image_buffer) => image::DynamicImage::from(image_buffer),
            FornaxRawImage::Rgba16(image_buffer) => image::DynamicImage::from(image_buffer),
            FornaxRawImage::MonoF32(image_buffer) => image::DynamicImage::from(image_buffer),
            FornaxRawImage::RgbF32(image_buffer) => image::DynamicImage::from(image_buffer),
            FornaxRawImage::RgbaF32(image_buffer) => image::DynamicImage::from(image_buffer),
        }
    }
}
/// A generic null post processor.
pub struct NullPostProcessor {}
impl<D> IPostProcessor<D> for NullPostProcessor
where
    D: IDecoder,
{
    fn post_process(&self, _decoded: &D) -> miette::Result<FornaxProcessedImage> {
        Ok(FornaxProcessedImage::Null)
    }
}
