use demosaic::IDemosaic;
use fornax_core::{FornaxPrimitive, IDecoder, IPostProcessor};
use image::{ImageBuffer, Rgb};
mod demosaic;
pub use demosaic::Demosaicer;
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
pub struct DalimParams {
    pub demosaicer: Demosaicer,
}
pub struct Dalim<T>
where
    T: FornaxPrimitive,
{
    _marker: std::marker::PhantomData<T>,
    params: DalimParams,
}
impl<T> Dalim<T>
where
    T: FornaxPrimitive,
{
    pub fn new(params: DalimParams) -> Self {
        Self {
            _marker: std::marker::PhantomData,
            params,
        }
    }
}
impl<D, T> IPostProcessor<D, T, T> for Dalim<T>
where
    D: IDecoder<T>,
    T: FornaxPrimitive,
{
    fn post_process(&self, decoder: &D) -> miette::Result<ImageBuffer<Rgb<T>, Vec<T>>> {
        let bayer_image = decoder.bayer_image()?;

        let img = match &self.params.demosaicer {
            Demosaicer::Linear => demosaic::DemosaicLinear().demosaic(&bayer_image),
        };
        Ok(img)
    }
}
