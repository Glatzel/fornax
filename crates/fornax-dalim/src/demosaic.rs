mod linear;
use fornax_core::BayerPrimitive;
pub use linear::DemosaicLinear;
pub trait IDemosaic<T>
where
    T: BayerPrimitive,
{
    fn demosaic(
        &self,
        bayer_image: &fornax_core::BayerImage<T>,
    ) -> image::ImageBuffer<image::Rgb<T>, Vec<T>>;
}
