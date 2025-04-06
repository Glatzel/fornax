use fornax_core::IPostProcessor;
pub use fornax_core::NullPostProcessor;
pub struct Fornax<D, I, P, O>
where
    D: fornax_core::IDecoder<I>,
    P: IPostProcessor<D, I, O>,
{
    _marker: std::marker::PhantomData<(I, O)>,
    pub decoder: D,
    pub post_processor: P,
}

impl<D, I, P, O> Fornax<D, I, P, O>
where
    D: fornax_core::IDecoder<I>,
    P: IPostProcessor<D, I, O>,
{
    pub fn new(decoder: D, post_processor: P) -> Self {
        Self {
            _marker: std::marker::PhantomData,
            decoder,
            post_processor,
        }
    }
    pub fn decode(&mut self, input: I) -> miette::Result<&mut Self> {
        self.decoder.decode(input)?;
        Ok(self)
    }
}

impl<D, I, P> Fornax<D, I, P, fornax_core::FornaxProcessedImage>
where
    D: fornax_core::IDecoder<I>,
    P: IPostProcessor<D, I, fornax_core::FornaxProcessedImage>,
{
    pub fn post_process(&mut self) -> miette::Result<fornax_core::FornaxProcessedImage> {
        self.post_processor.post_process(&self.decoder)
    }
}
