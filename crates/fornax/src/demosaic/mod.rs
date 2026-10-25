mod linear;

pub use linear::DemosaicLinear;

use crate::{BayerPattern, LumaImage, RgbImage};

trait IDemosaic {
    fn demosaic(&self, bayer_image: &LumaImage, pattern: BayerPattern) -> RgbImage;
}

#[cfg_attr(feature = "serde", derive(serde::Serialize, serde::Deserialize))]
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Demosaicer {
    Linear,
}
impl Demosaicer {
    fn demosaic(&self, bayer_image: &LumaImage, pattern: BayerPattern) -> RgbImage {
        match self {
            Demosaicer::Linear => DemosaicLinear.demosaic(bayer_image, pattern),
        }
    }
}
