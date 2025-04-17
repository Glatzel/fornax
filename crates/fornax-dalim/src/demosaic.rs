mod linear;
use fornax_core::FornaxPrimitive;
pub use linear::DemosaicLinear;
pub trait IDemosaic<T>
where
    T: FornaxPrimitive,
{
    fn demosaic(
        &self,
        bayer_image: &fornax_core::BayerImage<T>,
    ) -> image::ImageBuffer<image::Rgb<T>, Vec<T>>;
}
