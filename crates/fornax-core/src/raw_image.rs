use crate::error::DemosaicError;
use crate::{BayerPattern, IDemosaic, MonoImage, Primitive, RgbImage};

#[derive(Clone, Debug, PartialEq)]
pub struct RawImage<T>(MonoImage<T>)
where
    T: Primitive;

impl<T> RawImage<T>
where
    T: Primitive,
{
    pub const fn new(plane: MonoImage<T>) -> Self { Self(plane) }
    pub const fn image(&self) -> &MonoImage<T> { &self.0 }
    pub fn into_image(self) -> MonoImage<T> { self.0 }
    pub fn demosaic<D>(
        self,
        algorithm: &D,
        pattern: BayerPattern,
    ) -> Result<RgbImage<T>, DemosaicError>
    where
        D: IDemosaic<T>,
    {
        algorithm.demosaic(self.image(), pattern)
    }
}
