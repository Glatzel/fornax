use image::math::Rect;

use crate::{BayerPattern, MonoImage, Primitive};
/// Normalized metadata for a single-sample Bayer RAW image.
///
/// The geometry and level fields correspond to the same concepts as the
/// corresponding DNG raw-image tags. Coordinates use pixels with `x` as the
/// column and `y` as the row.
#[derive(Clone, Debug)]
pub struct Metadata<T>
where
    T: Primitive,
{
    /// Usable sensor area, corresponding to DNG `ActiveArea`.
    ///
    /// The origin is relative to the full raw frame. In DNG tag order this is
    /// `(top, left, bottom, right)`; this rectangle exposes the same area as
    /// `(x, y, width, height)`.
    pub active_area: Rect,

    /// Recommended output crop from DNG `DefaultCropOrigin` and
    /// `DefaultCropSize`.
    ///
    /// The origin and size are relative to [`Metadata::active_area`].
    pub default_crop: Rect,

    pub levels: Levels<T>,

    /// CFA layout described by DNG `CFARepeatPatternDim`, `CFAPattern`, and
    /// `CFAPlaneColor`.
    pub bayer_pattern: BayerPattern,
}

/// Black and white levels for a single-sample Bayer RAW image.
#[derive(Clone, Debug)]
pub struct Levels<T>
where
    T: Primitive,
{
    /// Repeating black-level pattern from DNG `BlackLevel`, with its repeat
    /// shape given by `BlackLevelRepeatDim`.
    ///
    /// The pattern is applied tile-by-tile from the top-left raw pixel.
    pub black: MonoImage<T>,

    /// Per-column black-level adjustments from DNG `BlackLevelDeltaH`.
    ///
    /// LibRaw folds these deltas into its scalar black level and does not
    /// retain the individual tag values, so decoders using LibRaw leave this
    /// vector empty.
    pub black_delta_h: Vec<T>,

    /// Per-row black-level adjustments from DNG `BlackLevelDeltaV`.
    ///
    /// LibRaw folds these deltas into its scalar black level and does not
    /// retain the individual tag values, so decoders using LibRaw leave this
    /// vector empty.
    pub black_delta_v: Vec<T>,

    /// White level from DNG `WhiteLevel` for single-sample Bayer RAW.
    pub white: T,
}
