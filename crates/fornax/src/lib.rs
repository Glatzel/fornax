use std::path::Path;

pub use fornax_core::NullPostProcessor;
use fornax_core::{FornaxPrimitive, IDecoder, IPostProcessor};
use image::{ImageBuffer, Rgb};
pub use {dnc, libraw};

pub struct Fornax<D, T, P, O>
where
    D: IDecoder<T>,
    T: FornaxPrimitive,
    P: IPostProcessor<D, T, O>,
    O: FornaxPrimitive,
{
    _marker_t: std::marker::PhantomData<T>,
    _marker_o: std::marker::PhantomData<O>,
    pub decoder: D,
    pub post_processor: P,
}

impl<D, T, P, O> Fornax<D, T, P, O>
where
    D: IDecoder<T>,
    T: FornaxPrimitive,
    P: IPostProcessor<D, T, O>,
    O: FornaxPrimitive,
{
    pub fn new(decoder: D, post_processor: P) -> Self {
        Self {
            _marker_t: std::marker::PhantomData,
            _marker_o: std::marker::PhantomData,
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
    pub fn post_process(&mut self) -> miette::Result<ImageBuffer<Rgb<O>, Vec<O>>> {
        self.post_processor.post_process(&self.decoder)
    }
}
