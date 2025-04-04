use std::path::PathBuf;

pub trait IDecoder {
    fn decode_file(&mut self, file: PathBuf) -> miette::Result<()>;
    fn decode_buffer(&mut self, buf: &[u8]) -> miette::Result<()>;
}
pub trait IPostProcessor<D, O>
where
    D: IDecoder,
{
    fn post_process(&self, decoder: &D) -> miette::Result<O>;
}
pub enum ProcessedImage {
    Mono8(image::ImageBuffer<image::Luma<u8>, Vec<u8>>),
    Mono16(image::ImageBuffer<image::Luma<u16>, Vec<u16>>),
    Rgb8(image::ImageBuffer<image::Rgb<u8>, Vec<u8>>),
    Rgb16(image::ImageBuffer<image::Rgb<u16>, Vec<u16>>),
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
impl<D, O> IPostProcessor<D, O> for NullPostProcessor
where
    D: IDecoder,
{
    fn post_process(&self, _decoded: &D) -> miette::Result<O> {
        miette::bail!("None.")
    }
}
