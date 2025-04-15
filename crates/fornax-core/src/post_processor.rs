pub trait IPostProcessor<D>
where
    D: crate::IDecoder,
{
    fn post_process(&self, decoder: &D) -> miette::Result<crate::FornaxProcessedImage>;
}

/// A generic null post processor.
pub struct NullPostProcessor {}
impl<D> IPostProcessor<D> for NullPostProcessor
where
    D: crate::IDecoder,
{
    fn post_process(&self, _decoded: &D) -> miette::Result<crate::FornaxProcessedImage> {
        Ok(crate::FornaxProcessedImage::Null)
    }
}
