use std::path::Path;

use fornax_core::IPostProcessor;
pub use fornax_core::NullPostProcessor;
pub struct Fornax<D, P>
where
    D: fornax_core::IDecoder,
    P: IPostProcessor<D>,
{
    pub decoder: D,
    pub post_processor: P,
}

impl<D, P> Fornax<D, P>
where
    D: fornax_core::IDecoder,
    P: IPostProcessor<D>,
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
        self.post_processor.post_process(&self.decoder)
    }
}
