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
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Demosaicer {
    Linear,
}

// impl Demosaicer {
//     fn demosaic<T>(
//         &self,
//         bayer_image: &fornax_core::BayerImage<T>,
//     ) -> image::ImageBuffer<image::Rgb<T>, Vec<T>>
//     where
//         T: FornaxPrimitive,
//     {
//         todo!()
        // if width == 0 || height == 0 {
        //     return img;
        // }
//     }
// }
