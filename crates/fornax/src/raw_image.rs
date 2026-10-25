use crate::{BayerPattern, IDemosaic, MonoImage, Primitive, RgbImage};

#[derive(Clone, Debug, PartialEq)]
pub struct RawImage<T>
where
    T: Primitive,
{
    plane: MonoImage<T>,
}

impl<T> RawImage<T>
where
    T: Primitive,
{
    pub fn new(plane: MonoImage<T>) -> Self { Self { plane } }
    pub fn image(&self) -> &MonoImage<T> { &self.plane }
    pub fn into_image(self) -> MonoImage<T> { self.plane }
    pub fn demosaic<D>(self, algorithm: &D, pattern: BayerPattern) -> RgbImage<T>
    where
        D: IDemosaic<T>,
    {
        algorithm.demosaic(self.image(), pattern)
    }
}
