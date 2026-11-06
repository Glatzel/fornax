use image::math::Rect;

use crate::primitive::MonoImage;
use crate::protocol::IPostProcessMetadata;
use crate::{BayerPattern, Primitive};

/// A row-major 3x3 color matrix.
pub type Matrix3x3 = [[f64; 3]; 3];

/// Camera calibration matrix from a DNG `CameraCalibration` tag.
#[derive(Clone, Copy, Debug, Default, PartialEq)]
pub enum CameraCalibration {
    /// DNG's default identity calibration when the tag is absent.
    #[default]
    Identity,

    /// A matrix stored by the corresponding `CameraCalibration` tag.
    Matrix(Matrix3x3),
}

impl CameraCalibration {
    /// Returns the calibration matrix, or `None` for DNG's identity default.
    pub const fn matrix(self) -> Option<Matrix3x3> {
        match self {
            Self::Identity => None,
            Self::Matrix(matrix) => Some(matrix),
        }
    }
}

/// Normalized metadata for a single-sample Bayer RAW image.
///
/// The geometry and level fields correspond to the same concepts as the
/// corresponding DNG raw-image tags. Coordinates use pixels with `x` as the
/// column and `y` as the row.
#[derive(Clone, Debug)]
pub struct ProcessMetadata<T>
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
    /// The origin and size are relative to [`ProcessMetadata::active_area`].
    pub default_crop: Rect,

    pub levels: Levels<T>,

    /// CFA layout described by DNG `CFARepeatPatternDim`, `CFAPattern`, and
    /// `CFAPlaneColor`. Cropped.
    pub bayer_pattern: BayerPattern,

    ///green normalized
    pub wb_coeffs: Option<(T, T)>,

    /// DNG `ColorMatrix1`, mapping XYZ to camera-native RGB.
    pub color_matrix_1: Option<Matrix3x3>,

    /// DNG `ColorMatrix2`, mapping XYZ to camera-native RGB.
    pub color_matrix_2: Option<Matrix3x3>,

    /// DNG `ForwardMatrix1`, mapping white-balanced camera RGB to XYZ.
    pub forward_matrix_1: Option<Matrix3x3>,

    /// DNG `ForwardMatrix2`, mapping white-balanced camera RGB to XYZ.
    pub forward_matrix_2: Option<Matrix3x3>,

    /// DNG `CameraCalibration1`.
    pub camera_calibration_1: CameraCalibration,

    /// DNG `CameraCalibration2`.
    pub camera_calibration_2: CameraCalibration,
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

impl<T> IPostProcessMetadata<T> for ProcessMetadata<T>
where
    T: Primitive,
{
    fn bayer_pattern(&self) -> BayerPattern { self.bayer_pattern }

    fn wb_coeffs(&self) -> &Option<(T, T)> { &self.wb_coeffs }

    fn color_matrix_1(&self) -> Option<&Matrix3x3> { self.color_matrix_1.as_ref() }

    fn color_matrix_2(&self) -> Option<&Matrix3x3> { self.color_matrix_2.as_ref() }

    fn forward_matrix_1(&self) -> Option<&Matrix3x3> { self.forward_matrix_1.as_ref() }

    fn forward_matrix_2(&self) -> Option<&Matrix3x3> { self.forward_matrix_2.as_ref() }

    fn camera_calibration_1(&self) -> CameraCalibration { self.camera_calibration_1 }

    fn camera_calibration_2(&self) -> CameraCalibration { self.camera_calibration_2 }
}
