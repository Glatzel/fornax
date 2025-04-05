use std::marker::PhantomData;
use std::path::PathBuf;

pub struct Fornax<D, M, P>
where
    D: fornax_traits::IDecoder<M>,
    P: fornax_traits::IPostProcessor<M, fornax_traits::ProcessedImage>,
{
    _marker: PhantomData<M>,
    pub decoder: D,
    pub post_processor: Option<P>,
}
impl<D, M, P> Fornax<D, M, P>
where
    D: fornax_traits::IDecoder<M>,
    P: fornax_traits::IPostProcessor<M, fornax_traits::ProcessedImage>,
{
    pub fn new(decoder: D) -> Self {
        Self {
            _marker: PhantomData::<M>,
            decoder,
            post_processor: None,
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
    pub fn decoded(&mut self) -> miette::Result<M> {
        self.decoder.decoded()
    }
    pub fn post_process(&mut self) -> miette::Result<fornax_traits::ProcessedImage> {
        let decoded = self.decoded()?;
        self.post_processor = Some(P::from_decoded(decoded));
        let processed_image = (self.post_processor).as_mut().unwrap().post_process()?;
        Ok(processed_image)
    }
}
