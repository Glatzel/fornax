pub struct Fornax<D, M, P>
where
    D: fornax_traits::IDecoder<M>,
    P: fornax_traits::IPostProcessor<M, fornax_traits::ProcessedImage>,
{
    pub decoder: D,
    pub post_processor: P,
}
