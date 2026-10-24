use std::sync::Arc;

use super::plane::validate_finite;
use crate::{DecodedRaw, F32Plane, RawValidationError, ValidatedMetadata};

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
