use std::path::Path;

pub use fornax_core::{FornaxProcessedImage, NullPostProcessor};
use fornax_core::{IDecoder, IPostProcessor};
pub use {dnc, libraw};

pub struct Fornax<D, P>
where
    D: IDecoder,
    P: IPostProcessor,
{
    pub decoder: D,
    pub post_processor: P,
}

impl<D, P> Fornax<D, P>
where
    D: IDecoder,
    P: IPostProcessor,
{
    pub fn new(decoder: D, post_processor: P) -> Self {
        Self {
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
    pub fn post_process(&mut self) -> miette::Result<fornax_core::FornaxProcessedImage> {
        self.post_processor.post_process()
    }
}
