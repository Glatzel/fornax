use image::math::Rect;
use image::{GenericImageView, ImageError, SubImage};

use crate::{LumaImage, RawValidationError};

#[derive(Clone, Debug, PartialEq)]
pub struct RawImage {
    plane: LumaImage,
}

impl RawImage {
    pub fn try_new(plane: LumaImage) -> Result<Self, RawValidationError> {
        let (width, height) = plane.dimensions();
        if width == 0 || height == 0 {
            return Err(RawValidationError::InvalidImage);
        }
        if plane.as_raw().iter().any(|sample| !sample.is_finite()) {
            return Err(RawValidationError::NonFiniteSample);
        }
        Ok(Self { plane })
    }

    pub fn cropped(&self, area: Rect) -> Result<SubImage<&LumaImage>, ImageError> {
        self.plane.try_view(area)
    }

    #[doc(hidden)]
    pub const fn full_image(&self) -> &LumaImage { &self.plane }

    pub(crate) fn into_full_image(self) -> LumaImage { self.plane }
}
