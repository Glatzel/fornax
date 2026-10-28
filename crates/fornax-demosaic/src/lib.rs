mod linear;
use fornax_core::error::DemosaicError;
use fornax_core::protocol::IDemosaic;
use fornax_core::{BayerPattern, MonoImage, Primitive, RgbImage};
use linear::DemosaicLinear;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Demosaic {
    Linear,
}
impl<T> IDemosaic<T> for Demosaic
where
    T: Primitive,
{
    fn demosaic(
        &self,
        bayer_image: &MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<RgbImage<T>, DemosaicError>
    where
        T: Primitive,
    {
        match self {
            Self::Linear => DemosaicLinear.demosaic(bayer_image, pattern),
        }
    }
}
