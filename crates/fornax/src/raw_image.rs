use image::math::Rect;
use image::{GenericImageView, SubImage};

use crate::alias::{BayerImage, LumaImage};
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
    pub fn image<'a>(&'a self, active_area: Rect) -> BayerImage<'a, T> {
        self.plane.view(active_area)
    }
    pub const fn full_image(&self) -> &LumaImage<T> { &self.plane }
    pub fn into_full_image(self) -> LumaImage<T> { self.plane }
    pub fn demosaic<D>(&self, algorithm: D, pattern: BayerPattern, active_area: Rect)
    where
        D: IDemosaic<T>,
    {
        algorithm.demosaic(self.image(active_area), pattern)
    }
}
