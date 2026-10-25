mod linear;
use linear::DemosaicLinear;

use crate::alias::{MonoImage, RgbImage};
use crate::{BayerPattern, Primitive};
pub trait IDemosaic<T>
where
    T: Primitive,
{
    fn demosaic(&self, bayer_image: &MonoImage<T>, pattern: BayerPattern) -> RgbImage<T>;
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Demosaic {
    Linear,
}
impl<T> IDemosaic<T> for Demosaic
where
    T: Primitive,
{
    fn demosaic(&self, bayer_image: &MonoImage<T>, pattern: BayerPattern) -> RgbImage<T>
    where
        T: Primitive,
    {
        match self {
            Demosaic::Linear => DemosaicLinear.demosaic(bayer_image, pattern),
        }
    }
}
