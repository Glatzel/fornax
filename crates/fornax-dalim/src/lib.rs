use demosaic::IDemosaic;
use fornax_core::{BayerPrimitive, IDecoder, IPostProcessor};
use image::ImageBuffer;
pub mod demosaic;
pub struct Dalim<T, DM>
where
    DM: IDemosaic<T>,
    T: BayerPrimitive,
{
    _marker: std::marker::PhantomData<T>,
    demosaicer: DM,
}
impl<T, DM> Dalim<T, DM>
where
    DM: IDemosaic<T>,
    T: BayerPrimitive,
{
    pub fn new(demosaicer: DM) -> Self {
        Self {
            _marker: std::marker::PhantomData,
            demosaicer,
        }
    }
}
impl<D, DM> IPostProcessor<D, u16> for Dalim<u16, DM>
where
    D: IDecoder<u16>,
    DM: IDemosaic<u16>,
{
    fn post_process(&self, decoder: &D) -> miette::Result<fornax_core::ProcessedImage> {
        let bayer_image = decoder.bayer_image()?;
        let img: ImageBuffer<image::Rgb<u16>, Vec<u16>> = self.demosaicer.demosaic(&bayer_image);
        Ok(fornax_core::ProcessedImage::Rgb16(img))
    }
}
impl<D, DM> IPostProcessor<D, f32> for Dalim<f32, DM>
where
    D: IDecoder<f32>,
    DM: IDemosaic<f32>,
{
    fn post_process(&self, decoder: &D) -> miette::Result<fornax_core::ProcessedImage> {
        let bayer_image = decoder.bayer_image()?;
        let img: ImageBuffer<image::Rgb<f32>, Vec<f32>> = self.demosaicer.demosaic(&bayer_image);
        Ok(fornax_core::ProcessedImage::RgbF32(img))
    }
}
