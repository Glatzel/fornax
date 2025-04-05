use std::path::PathBuf;

pub trait IDecoder<M> {
    fn decode_file(&mut self, file: PathBuf) -> miette::Result<()>;
    fn decode_buffer(&mut self, buf: &[u8]) -> miette::Result<()>;
    fn decoded(&mut self) -> miette::Result<M>;
}
pub trait IPostProcessor<M, O> {
    fn from_decoded(decoded: M) -> Self;
    fn post_process(&mut self) -> miette::Result<O>;
}
pub enum ProcessedImage {
    Mono8(image::ImageBuffer<image::Luma<u8>, Vec<u8>>),
    Mono16(image::ImageBuffer<image::Luma<u16>, Vec<u16>>),
    Rgb8(image::ImageBuffer<image::Rgb<u8>, Vec<u8>>),
    Rgb16(image::ImageBuffer<image::Luma<u16>, Vec<u16>>),
}
impl ProcessedImage {
    pub fn to_dynamic(self) -> image::DynamicImage {
        match self {
            ProcessedImage::Mono8(image_buffer) => image::DynamicImage::from(image_buffer),
            ProcessedImage::Mono16(image_buffer) => image::DynamicImage::from(image_buffer),
            ProcessedImage::Rgb8(image_buffer) => image::DynamicImage::from(image_buffer),
            ProcessedImage::Rgb16(image_buffer) => image::DynamicImage::from(image_buffer),
        }
    }
}
pub struct NullPostProcessor {}
impl<M, O> IPostProcessor<M, O> for NullPostProcessor {
    fn from_decoded(_decoded: M) -> Self {
        Self {}
    }

    fn post_process(&mut self) -> miette::Result<O> {
        miette::bail!("None.")
    }
}
