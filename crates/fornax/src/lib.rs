use fornax_core::IPostProcessor;
pub use fornax_core::NullPostProcessor;
pub struct Fornax<D, T, P>
where
    D: fornax_core::IDecoder<T>,
    P: IPostProcessor<D, T, fornax_core::FornaxProcessedImage>,
{
    _marker: std::marker::PhantomData<T>,
    pub decoder: D,
    pub post_processor: P,
}
impl<D, T, P> Fornax<D, T, P>
where
    D: fornax_core::IDecoder<T>,
    P: IPostProcessor<D, T, fornax_core::FornaxProcessedImage>,
{
    pub fn new(decoder: D, post_processor: P) -> Self {
        Self {
            _marker: std::marker::PhantomData,
            decoder,
            post_processor,
        }
    }
    pub fn decode(&mut self, input: T) -> miette::Result<&mut Self> {
        self.decoder.decode(input)?;
        Ok(self)
    }

    pub fn post_process(&mut self) -> miette::Result<fornax_core::FornaxProcessedImage> {
        self.post_processor.post_process(&self.decoder)
    }
}
