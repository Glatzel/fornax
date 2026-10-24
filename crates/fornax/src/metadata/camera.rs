//! Camera, crop, capture, orientation, white-balance, and color metadata.

use super::value::Rational;
use crate::error::MetadataError;

/// Camera make, model, and normalized catalog identity.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CameraIdentity {
    /// Camera manufacturer as declared by the source.
    pub make: Option<Box<str>>,
    /// Camera model as declared by the source.
    pub model: Option<Box<str>>,
    /// Normalized manufacturer used for catalog lookup.
    pub normalized_make: Option<Box<str>>,
    /// Normalized model used for catalog lookup.
    pub normalized_model: Option<Box<str>>,
}

/// Descriptive default crop in pre-scale RAW coordinates.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct DefaultCrop {
    origin_x: Rational,
    origin_y: Rational,
    width: Rational,
    height: Rational,
}

impl DefaultCrop {
    /// Creates a crop with positive width and height.
    ///
    /// # Errors
    ///
    /// Returns [`crate::MetadataError::InvalidValue`] for non-positive crop
    /// dimensions.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use fornax::metadata::{DefaultCrop, Rational};
    ///
    /// let zero = Rational::new(0, NonZeroU32::new(1).unwrap());
    /// let size = Rational::new(20, NonZeroU32::new(1).unwrap());
    /// assert_eq!(
    ///     DefaultCrop::try_new(zero, zero, size, size)
    ///         .unwrap()
    ///         .width(),
    ///     size
    /// );
    /// ```
    pub fn try_new(
        origin_x: Rational,
        origin_y: Rational,
        width: Rational,
        height: Rational,
    ) -> Result<Self, MetadataError> {
        if width.as_f64() <= 0.0 || height.as_f64() <= 0.0 {
            return Err(MetadataError::InvalidValue);
        }
        Ok(Self {
            origin_x,
            origin_y,
            width,
            height,
        })
    }

    /// Returns the horizontal crop origin.
    pub const fn origin_x(self) -> Rational { self.origin_x }

    /// Returns the vertical crop origin.
    pub const fn origin_y(self) -> Rational { self.origin_y }

    /// Returns the crop width.
    pub const fn width(self) -> Rational { self.width }

    /// Returns the crop height.
    pub const fn height(self) -> Rational { self.height }
}

/// A validated TIFF orientation value in `1..=8`.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Orientation(u8);

impl Orientation {
    /// Creates a TIFF orientation.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::metadata::Orientation;
    ///
    /// assert_eq!(Orientation::new(6).unwrap().value(), 6);
    /// assert!(Orientation::new(0).is_none());
    /// ```
    pub const fn new(value: u8) -> Option<Self> {
        if value >= 1 && value <= 8 {
            Some(Self(value))
        } else {
            None
        }
    }

    /// Returns the TIFF orientation integer.
    pub const fn value(self) -> u8 { self.0 }
}

/// Camera-neutral white-balance metadata, preserved but not applied.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WhiteBalance {
    camera_neutral: Box<[Rational]>,
}

impl WhiteBalance {
    /// Creates RGB camera-neutral coefficients.
    ///
    /// # Errors
    ///
    /// Returns [`crate::MetadataError::InvalidValue`] unless exactly three
    /// positive coefficients are provided.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use fornax::metadata::{Rational, WhiteBalance};
    ///
    /// let one = Rational::new(1, NonZeroU32::new(1).unwrap());
    /// assert_eq!(
    ///     WhiteBalance::try_new(vec![one; 3])
    ///         .unwrap()
    ///         .camera_neutral()
    ///         .len(),
    ///     3
    /// );
    /// ```
    pub fn try_new(camera_neutral: Vec<Rational>) -> Result<Self, MetadataError> {
        if camera_neutral.len() != 3 || camera_neutral.iter().any(|value| value.as_f64() <= 0.0) {
            return Err(MetadataError::InvalidValue);
        }
        Ok(Self {
            camera_neutral: camera_neutral.into_boxed_slice(),
        })
    }

    /// Returns the RGB camera-neutral coefficients.
    pub fn camera_neutral(&self) -> &[Rational] { &self.camera_neutral }
}

/// Color matrices preserved as metadata but never applied by this pipeline.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ColorMatrices {
    /// Row-major camera-to-XYZ(D50) matrix.
    pub camera_to_xyz_d50: Option<Box<[Rational]>>,
    /// Row-major camera color matrix.
    pub camera_matrix: Option<Box<[Rational]>>,
}

/// Capture metadata that is not consumed before demosaicing.
#[derive(Clone, Debug, Default, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CaptureMetadata {
    /// Source capture timestamp.
    pub timestamp: Option<Box<str>>,
    /// Exposure time in seconds.
    pub exposure_time: Option<Rational>,
    /// ISO speed metadata.
    pub iso_speed: Option<Rational>,
}
