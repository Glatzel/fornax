use image::math::Rect;

use crate::{BayerPattern, Primitive};

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
    pub black: BlackLevel<T>,

    /// Per-column black-level adjustments.
    pub black_delta_h: Option<Vec<T>>,

    /// Per-row black-level adjustments.
    pub black_delta_v: Option<Vec<T>>,

    /// White level for single-sample Bayer RAW.
    pub white: T,
}

#[derive(Clone, Debug)]
pub struct BlackLevel<T>
where
    T: Primitive,
{
    /// Number of rows in the repeating pattern.
    pub rows: u32,

    /// Number of columns in the repeating pattern.
    pub cols: u32,

    /// Pattern values in row-major order.
    pub values: Vec<T>,
}
