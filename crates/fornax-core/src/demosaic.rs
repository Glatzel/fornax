use crate::{BayerPattern, MonoImage, Primitive, RgbImage};

pub trait IDemosaic<T>
where
    T: Primitive,
{
    fn demosaic(&self, bayer_image: &MonoImage<T>, pattern: BayerPattern) -> RgbImage<T>;
}
