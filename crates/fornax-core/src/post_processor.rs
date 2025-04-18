use image::Rgb;

pub trait IPostProcessor<D, T, O>
where
    D: crate::IDecoder<T>,
    T: crate::FornaxPrimitive,
    O: crate::FornaxPrimitive,
{
    fn post_process(&self, decoder: &D) -> miette::Result<image::ImageBuffer<Rgb<O>, Vec<O>>>;
}

/// A generic null post processor.
pub struct NullPostProcessor {}
impl<D> IPostProcessor<D, u8, u8> for NullPostProcessor
where
    D: crate::IDecoder<u8>,
{
    fn post_process(
        &self,
        _decoded: &D,
    ) -> miette::Result<image::ImageBuffer<image::Rgb<u8>, Vec<u8>>> {
        unimplemented!("Null post processor doesn't return image.")
    }
}
