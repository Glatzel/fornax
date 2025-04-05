use std::path::PathBuf;

use fornax_traits::IPostProcessor;
pub use fornax_traits::NullPostProcessor;
pub struct Fornax<D, P>
where
    D: fornax_traits::IDecoder,
    P: IPostProcessor<D, fornax_traits::ProcessedImage>,
{
    pub decoder: D,
    pub post_processor: P,
}
impl<D, P> Fornax<D, P>
where
    D: fornax_traits::IDecoder,
    P: IPostProcessor<D, fornax_traits::ProcessedImage>,
{
    pub fn new(decoder: D, post_processor: P) -> Self {
        Self {
            decoder,
            post_processor,
        }
    }
    pub fn decode_file(&mut self, file: PathBuf) -> miette::Result<&mut Self> {
        self.decoder.decode_file(file)?;
        Ok(self)
    }
    pub fn decode_buffer(&mut self, buf: &[u8]) -> miette::Result<&mut Self> {
        self.decoder.decode_buffer(buf)?;
        Ok(self)
    }
    pub fn post_process(&mut self) -> miette::Result<fornax_traits::ProcessedImage> {
        self.post_processor.post_process(&self.decoder)
    }
}
