use crate::alias::{LumaImage, RgbImage};
use crate::{BayerPattern, IDemosaic, Primitive};

#[derive(Clone, Debug, PartialEq)]
pub struct RawImage<T>
where
    T: Primitive,
{
    plane: LumaImage<T>,
}

impl<T> RawImage<T>
where
    T: Primitive,
{
    pub fn new(plane: LumaImage<T>) -> Self { Self { plane } }
    pub fn image(&self) -> &LumaImage<T> { &self.plane }
    pub fn into_image(self) -> LumaImage<T> { self.plane }
    pub fn demosaic<D>(self, algorithm: &D, pattern: BayerPattern) -> RgbImage<T>
    where
        D: IDemosaic<T>,
    {
        algorithm.demosaic(self.image(), pattern)
    }
}
