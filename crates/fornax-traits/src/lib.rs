use std::path::PathBuf;

pub trait IDecoder<M> {
    fn decode_file(&mut self, file: PathBuf) -> miette::Result<()>;
    fn decode_buffer(&mut self, buf: &[u8]) -> miette::Result<()>;
    fn decoded(&mut self) -> miette::Result<M>;
}
pub trait IPostProcessor<M, O> {
    fn post_process(&mut self, decoded: M) -> miette::Result<O>;
}
pub enum ProcessedImage {
    Mono8(image::ImageBuffer<image::Luma<u8>, Vec<u8>>),
    Mono16(image::ImageBuffer<image::Luma<u16>, Vec<u16>>),
    Rgb8(image::ImageBuffer<image::Rgb<u8>, Vec<u8>>),
    Rgb16(image::ImageBuffer<image::Luma<u16>, Vec<u16>>),
}
