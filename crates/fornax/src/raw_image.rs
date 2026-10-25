use image::math::Rect;
use image::{GenericImageView, ImageError, SubImage};

use crate::{F32Plane, RawValidationError};

#[derive(Clone, Debug, PartialEq)]
pub struct RawImage {
    plane: F32Plane,
}

impl RawImage {
    pub fn try_new(plane: F32Plane) -> Result<Self, RawValidationError> {
        let (width, height) = plane.dimensions();
        if width == 0 || height == 0 {
            return Err(RawValidationError::InvalidImage);
        }
        if plane.as_raw().iter().any(|sample| !sample.is_finite()) {
            return Err(RawValidationError::NonFiniteSample);
        }
        Ok(Self { plane })
    }

    pub fn cropped(&self, area: Rect) -> Result<SubImage<&F32Plane>, ImageError> {
        self.plane.try_view(area)
    }

    #[doc(hidden)]
    pub const fn full_image(&self) -> &F32Plane { &self.plane }

    pub(crate) fn into_full_image(self) -> F32Plane { self.plane }
}
