use std::sync::Arc;

use crate::plane::validate_finite;
use crate::{
    DecodedRaw, DecoderProvenance, F32Plane, MetadataBasis, RawValidationError, ValidatedMetadata,
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
/// use fornax_core::RawImage;
///
/// fn accepts_f32_image(_image: &RawImage) {}
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct RawImage {
    metadata: Arc<ValidatedMetadata>,
    plane: F32Plane,
    decoder: DecoderProvenance,
    metadata_basis: MetadataBasis,
}

impl RawImage {
    /// Returns immutable validated metadata.
    pub fn metadata(&self) -> &ValidatedMetadata { &self.metadata }

    /// Returns the shared metadata handle for retaining after preprocessing.
    pub fn metadata_arc(&self) -> Arc<ValidatedMetadata> { Arc::clone(&self.metadata) }

    /// Returns the preprocessed single-channel f32 plane.
    pub const fn plane(&self) -> &F32Plane { &self.plane }

    /// Returns decoder provenance.
    pub const fn decoder(&self) -> &DecoderProvenance { &self.decoder }

    /// Returns the black/white metadata validity basis retained from decoding.
    pub const fn metadata_basis(&self) -> &MetadataBasis { &self.metadata_basis }

    /// Verifies the f32 Bayer mosaic is finite and in `[0, 1]`.
    ///
    /// # Errors
    ///
    /// Returns a typed error if geometry is inconsistent or any sample is
    /// non-finite or outside `[0, 1]`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::RawValidationError;
    ///
    /// assert_eq!(
    ///     RawValidationError::InvalidReadinessRange.to_string(),
    ///     "demosaic-ready sample is outside [0, 1]"
    /// );
    /// ```
    pub fn validate_for_demosaic(&self) -> Result<(), RawValidationError> {
        self.metadata.validate_f32_working_plane(&self.plane)?;
        validate_finite(&self.plane)?;
        if self
            .plane
            .as_raw()
            .iter()
            .any(|sample| !(0.0..=1.0).contains(sample))
        {
            return Err(RawValidationError::InvalidReadinessRange);
        }
        Ok(())
    }

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
    image.require_standard_preprocessing()?;
    image.metadata().validate_f32_working_plane(&plane)?;
    validate_finite(&plane)?;
    if plane
        .as_raw()
        .iter()
        .any(|sample| !(0.0..=1.0).contains(sample))
    {
        return Err(RawValidationError::InvalidReadinessRange);
    }
    let (metadata, decoder, metadata_basis) = image.into_preprocessing_parts();
    Ok(RawImage {
        metadata,
        plane,
        decoder,
        metadata_basis,
    })
}
