mod linear;
use linear::DemosaicLinear;

use crate::alias::{LumaImage, RgbImage};
use crate::{BayerPattern, Primitive};
pub trait IDemosaic<T>
where
    T: Primitive,
{
    fn demosaic(&self, bayer_image: &LumaImage<T>, pattern: BayerPattern) -> RgbImage<T>;
}
#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Demosaicer {
    Linear,
}
impl Demosaicer {
    fn demosaic<T>(&self, bayer_image: &LumaImage<T>, pattern: BayerPattern) -> RgbImage<T>
    where
        T: Primitive,
    {
        match self {
            Demosaicer::Linear => DemosaicLinear.demosaic(bayer_image, pattern),
        }
    }
}
