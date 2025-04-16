use std::path::Path;

use fornax_core::{BayerPrimitive, IDecoder, IPostProcessor};
pub use fornax_core::{ProcessedImage, NullPostProcessor};
pub use {dnc, libraw};

pub struct Fornax<D, T, P>
where
    D: IDecoder<T>,
    P: IPostProcessor<D, T>,
    T: BayerPrimitive,
{
    _marker: std::marker::PhantomData<T>,
    pub decoder: D,
    pub post_processor: P,
}

impl<D, T, P> Fornax<D, T, P>
where
    D: IDecoder<T>,
    P: IPostProcessor<D, T>,
    T: BayerPrimitive,
{
    pub fn new(decoder: D, post_processor: P) -> Self {
        Self {
            _marker: std::marker::PhantomData,
            decoder,
            post_processor,
        }
    }
    pub fn decode_file(&mut self, file: &Path) -> miette::Result<&mut Self> {
        self.decoder.decode_file(file)?;
        Ok(self)
    }
    pub fn decode_buffer(&mut self, buffer: &[u8]) -> miette::Result<&mut Self> {
        self.decoder.decode_buffer(buffer)?;
        Ok(self)
    }
    pub fn post_process(&mut self) -> miette::Result<fornax_core::ProcessedImage> {
        self.post_processor.post_process(&self.decoder)
    }
}
