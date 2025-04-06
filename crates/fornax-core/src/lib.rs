/// A trait for performing decoding on object of a generic type `I`.
///
/// # Type Parameters
/// - `I`: The type of the item that the operation will be performed on.
///
/// # Methods
/// - `decode`: Decode raw image file on an object of type `I`.
pub trait IDecoder<I> {
    fn decode(&mut self, input: I) -> miette::Result<()>;
}
/// A trait for performing decoding on object of a generic type `I`.
///
/// # Type Parameters
/// - `D`: The type of raw image decoder.
/// - `I`: The type of raw image decoder input.
/// - `O`: The type of post process output.
///
/// # Methods
/// - `post_process`: Perform post process on a decoded raw image object of type `I`.
pub trait IPostProcessor<D, I, O>
where
    D: IDecoder<I>,
{
    fn post_process(&self, decoder: &D) -> miette::Result<O>;
}
/// Basic raw image.
pub enum FornaxProcessedImage {
    Mono8(image::ImageBuffer<image::Luma<u8>, Vec<u8>>),
    Mono16(image::ImageBuffer<image::Luma<u16>, Vec<u16>>),
    Rgb8(image::ImageBuffer<image::Rgb<u8>, Vec<u8>>),
    Rgb16(image::ImageBuffer<image::Rgb<u16>, Vec<u16>>),
}
impl FornaxProcessedImage {
    pub fn to_dynamic(self) -> image::DynamicImage {
        match self {
            FornaxProcessedImage::Mono8(image_buffer) => image::DynamicImage::from(image_buffer),
            FornaxProcessedImage::Mono16(image_buffer) => image::DynamicImage::from(image_buffer),
            FornaxProcessedImage::Rgb8(image_buffer) => image::DynamicImage::from(image_buffer),
            FornaxProcessedImage::Rgb16(image_buffer) => image::DynamicImage::from(image_buffer),
        }
    }
}

/// A generic null post processor.
pub struct NullPostProcessor {}
impl<D, I> IPostProcessor<D, I, ()> for NullPostProcessor
where
    D: IDecoder<I>,
{
    fn post_process(&self, _decoded: &D) -> miette::Result<()> {
        miette::bail!("None processor.")
    }
}
