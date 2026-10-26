use crate::{BayerPattern, MonoImage, Primitive};
use image::math::Rect;

#[derive(Clone, Debug)]
pub struct Metadata<T>
where
    T: Primitive,
{
    /// Usable sensor area, relative to the full decoded image.
    pub active_area: Rect,

    /// Recommended output crop, relative to the active area.
    pub default_crop: Rect,

    pub levels: Levels<T>,
    pub bayer_pattern: BayerPattern,
}

#[derive(Clone, Debug)]
pub struct Levels<T>
where
    T: Primitive,
{
    /// Repeating black-level pattern.
    pub black: MonoImage<T>,

    /// Per-column black-level adjustments.
    pub black_delta_h: Vec<T>,

    /// Per-row black-level adjustments.
    pub black_delta_v: Vec<T>,

    /// White level for single-sample Bayer RAW.
    pub white: T,
}
