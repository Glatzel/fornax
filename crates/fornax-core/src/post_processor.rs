pub trait IPostProcessor<D, T, P, O>
where
    D: crate::IDecoder<T>,
    T: crate::FornaxPrimitive,
    P: image::Pixel,
    O: crate::FornaxPrimitive,
{
    fn post_process(&self, decoder: &D) -> miette::Result<image::ImageBuffer<P, Vec<O>>>;
}

/// A generic null post processor.
pub struct NullPostProcessor {}
impl<D, T> IPostProcessor<D, T, image::Rgb<T>, T> for NullPostProcessor
where
    D: crate::IDecoder<T>,
    T: crate::FornaxPrimitive,
{
    fn post_process(
        &self,
        _decoded: &D,
    ) -> miette::Result<image::ImageBuffer<image::Rgb<T>, Vec<T>>> {
        unimplemented!("Null post processor doesn't return image.")
    }
}
