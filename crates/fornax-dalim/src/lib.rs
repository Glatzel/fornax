use demosaic::IDemosaic;
use fornax_core::{FornaxError, FornaxPrimitive, IDecoder, IPostProcessor};
use image::{ImageBuffer, Rgb};

mod demosaic;
pub use demosaic::Demosaicer;
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
pub struct DalimParams {
    pub demosaicer: Demosaicer,
}
#[derive(Debug, Clone, PartialEq, Eq, Hash)]
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
    fn post_process(&self, decoder: &D) -> Result<ImageBuffer<Rgb<T>, Vec<T>>, FornaxError> {
        let bayer_image = decoder.bayer_image()?;

        let img = match &self.params.demosaicer {
            Demosaicer::Linear => demosaic::DemosaicLinear.demosaic(&bayer_image),
        };
        Ok(img)
    }
}
