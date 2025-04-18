use demosaic::IDemosaic;
use fornax_core::{FornaxPrimitive, IDecoder, IPostProcessor};
use image::{ImageBuffer, Rgb};
pub mod demosaic;
pub struct Dalim<T, DM>
where
    DM: IDemosaic<T>,
    T: FornaxPrimitive,
{
    _marker: std::marker::PhantomData<T>,
    demosaicer: DM,
}
impl<T, DM> Dalim<T, DM>
where
    DM: IDemosaic<T>,
    T: FornaxPrimitive,
{
    pub fn new(demosaicer: DM) -> Self {
        Self {
            _marker: std::marker::PhantomData,
            demosaicer,
        }
    }
}
impl<D, DM, T> IPostProcessor<D, T, T> for Dalim<T, DM>
where
    D: IDecoder<T>,
    DM: IDemosaic<T>,
    T: FornaxPrimitive,
{
    fn post_process(&self, decoder: &D) -> miette::Result<ImageBuffer<Rgb<T>, Vec<T>>> {
        let bayer_image = decoder.bayer_image()?;
        let img = self.demosaicer.demosaic(&bayer_image);
        Ok(img)
    }
}
