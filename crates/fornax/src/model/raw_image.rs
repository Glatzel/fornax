use std::sync::Arc;

use super::plane::validate_finite;
use crate::{
    DecodedRaw, DemosaicError, DemosaicOptions, F32Plane, IDemosaic, PixelSize, RawValidationError,
    RgbImageF32, ValidatedMetadata,
};

/// A preprocessed, demosaic-ready RAW image with a single f32 plane.
///
/// A `RawImage` can only be produced by preprocessing a [`DecodedRaw`].
/// Metadata and provenance remain available, while the payload is guaranteed
/// to use [`F32Plane`].
///
/// # Examples
///
/// ```
/// use fornax::RawImage;
///
/// fn accepts_f32_image(_image: &RawImage) {}
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct RawImage {
    metadata: Arc<ValidatedMetadata>,
    plane: F32Plane,
}

impl RawImage {
    /// Returns immutable validated metadata.
    pub fn metadata(&self) -> &ValidatedMetadata { &self.metadata }

    /// Returns the shared metadata handle for retaining after preprocessing.
    pub fn metadata_arc(&self) -> Arc<ValidatedMetadata> { Arc::clone(&self.metadata) }

    /// Returns the preprocessed single-channel f32 plane.
    pub const fn plane(&self) -> &F32Plane { &self.plane }

    /// Consumes the image and returns its f32 plane.
    pub fn into_plane(self) -> F32Plane { self.plane }

    /// Consumes the image into retained metadata and f32 plane.
    pub fn into_parts(self) -> (Arc<ValidatedMetadata>, F32Plane) { (self.metadata, self.plane) }

    /// Runs the selected demosaicing backend.
    ///
    /// # Errors
    ///
    /// Returns the selected backend's structured error or
    /// [`DemosaicError::InvalidInput`] when the backend returns an invalid
    /// output image.
    pub fn demosaic<D: IDemosaic + ?Sized>(
        &self,
        demosaicer: &D,
        options: &DemosaicOptions,
    ) -> Result<(RgbImageF32, Arc<ValidatedMetadata>), DemosaicError> {
        let output = demosaicer.demosaic(self, options)?;
        validate_rgb_output(&output, self.metadata.stored_geometry().dimensions())?;
        Ok((output, self.metadata_arc()))
    }
}

/// Hidden finalizer used by the single `DecodedRaw::pre_process` entry point.
#[doc(hidden)]
pub fn into_preprocessed(
    image: DecodedRaw,
    plane: F32Plane,
) -> Result<RawImage, RawValidationError> {
    let (metadata, _) = image.into_preprocessing_parts()?;
    metadata.validate_f32_working_plane(&plane)?;
    validate_finite(&plane)?;
    if plane
        .as_raw()
        .iter()
        .any(|sample| !(0.0..=1.0).contains(sample))
    {
        return Err(RawValidationError::InvalidReadinessRange);
    }
    Ok(RawImage { metadata, plane })
}

fn validate_rgb_output(output: &RgbImageF32, expected: PixelSize) -> Result<(), DemosaicError> {
    let (width, height) = output.dimensions();
    if width != expected.width() || height != expected.height() {
        return Err(DemosaicError::InvalidInput(
            "output dimensions do not match the source image".into(),
        ));
    }
    let expected_samples = expected
        .sample_count()
        .ok()
        .and_then(|count| count.checked_mul(3))
        .ok_or_else(|| DemosaicError::InvalidInput("output size overflows usize".into()))?;
    if output.as_raw().len() != expected_samples {
        return Err(DemosaicError::InvalidInput(
            "output sample count does not match its dimensions".into(),
        ));
    }
    if output.as_raw().iter().any(|sample| !sample.is_finite()) {
        return Err(DemosaicError::InvalidInput(
            "output contains a non-finite sample".into(),
        ));
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use image::Rgb;

    use super::*;

    #[test]
    fn rgb_image_uses_image_rs_interleaved_layout() {
        let image = RgbImageF32::from_raw(1, 1, vec![0.1, 0.2, 0.3]).unwrap();
        assert_eq!(image.get_pixel(0, 0), &Rgb([0.1, 0.2, 0.3]));
        assert_eq!(
            validate_rgb_output(&image, PixelSize::new(1, 1).unwrap()),
            Ok(())
        );
    }

    #[test]
    fn rgb_output_validation_rejects_invalid_buffers() {
        let wrong_size = RgbImageF32::from_raw(2, 1, vec![0.0; 6]).unwrap();
        assert!(matches!(
            validate_rgb_output(&wrong_size, PixelSize::new(1, 1).unwrap()),
            Err(DemosaicError::InvalidInput(_))
        ));
        let wrong_count = RgbImageF32::from_raw(1, 1, vec![0.0; 4]).unwrap();
        assert!(matches!(
            validate_rgb_output(&wrong_count, PixelSize::new(1, 1).unwrap()),
            Err(DemosaicError::InvalidInput(_))
        ));
        let non_finite = RgbImageF32::from_raw(1, 1, vec![f32::NAN; 3]).unwrap();
        assert!(matches!(
            validate_rgb_output(&non_finite, PixelSize::new(1, 1).unwrap()),
            Err(DemosaicError::InvalidInput(_))
        ));
    }
}
