use image::math::Rect;

use crate::{BayerPattern, MonoImage, Primitive};

#[derive(Clone, Debug)]
pub struct Metadata<T>
where
    T: Primitive,
{
    pub active_area: Rect,
    pub black_levels: BlackLevels<T>,
    pub white_levels: WhiteLevels,
    pub bayer_pattern: BayerPattern,
}

#[derive(Clone, Debug, Default)]
pub struct BlackLevels<T>
where
    T: Primitive,
{
    pub common: T,
    pub per_slot: [T; 4],
    pub spatial: Option<MonoImage<T>>,
}

#[derive(Clone, Copy, Debug)]
pub enum WhiteLevels {
    Uniform(u32),
    PerSlot([u32; 4]),
}
