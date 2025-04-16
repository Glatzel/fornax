pub trait IPostProcessor<D, T>
where
    D: crate::IDecoder<T>,
    T: crate::FornaxPrimitive,
{
    fn post_process(&self, decoder: &D) -> miette::Result<crate::ProcessedImage>;
}

/// A generic null post processor.
pub struct NullPostProcessor {}
impl<D> IPostProcessor<D, u8> for NullPostProcessor
where
    D: crate::IDecoder<u8>,
{
    fn post_process(&self, _decoded: &D) -> miette::Result<crate::ProcessedImage> {
        Ok(crate::ProcessedImage::Null)
    }
}
