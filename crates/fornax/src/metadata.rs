
use image::math::Rect;

use crate::{BayerPattern, MonoImage, Primitive};

#[derive(Clone, Debug)]
pub struct Metadata<T>
where
    T: Primitive,
{
    /// Usable sensor area.
    ///
    /// DNG reference: `ActiveArea` (tag 50829).
    pub active_area: Rect,

    pub black_levels: BlackLevels<T>,

    /// Sensor saturation levels.
    ///
    /// DNG reference: `WhiteLevel` (tag 50717).
    pub white_levels: WhiteLevels,

    /// Color filter array pattern.
    ///
    /// DNG reference: `CFAPattern` (tag 33422).
    pub bayer_pattern: BayerPattern,
}

#[derive(Clone, Debug, Default)]
pub struct BlackLevels<T>
where
    T: Primitive,
{
    /// Common black-level offset.
    ///
    /// DNG reference: baseline values in `BlackLevel` (tag 50714).
    pub common: T,

    /// Black-level offsets for the four Bayer slots.
    ///
    /// DNG reference: `BlackLevelRepeatDim` (tag 50713) and
    /// `BlackLevel` (tag 50714), when the repeating pattern is 2 × 2.
    pub per_slot: [T; 4],

    /// Spatial black-level correction map.
    ///
    /// DNG reference: `BlackLevel` (tag 50714) for a general repeating
    /// pattern, or `BlackLevelDeltaH` (tag 50715) and
    /// `BlackLevelDeltaV` (tag 50716) for per-column and per-row deltas.
    pub spatial: Option<MonoImage<T>>,
}

#[derive(Clone, Copy, Debug)]
pub enum WhiteLevels {
    /// One white level for the image.
    ///
    /// DNG reference: `WhiteLevel` (tag 50717).
    Uniform(u32),

    /// White levels for the four Bayer slots.
    ///
    /// DNG reference: `WhiteLevel` (tag 50717), represented per CFA slot
    /// by this API rather than as a DNG-defined Bayer-specific structure.
    PerSlot([u32; 4]),
}
