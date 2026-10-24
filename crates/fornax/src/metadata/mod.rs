//! Typed and extensible RAW metadata with source provenance.
//!
//! # Examples
//!
//! ```
//! use std::num::NonZeroU32;
//!
//! use fornax::metadata::{BlackLevels, Rational};
//!
//! let zero = Rational::new(0, NonZeroU32::new(1).unwrap());
//! assert_eq!(
//!     BlackLevels::try_new(zero, [zero; 4], None)
//!         .unwrap()
//!         .common(),
//!     zero
//! );
//! ```

mod camera;
mod cfa;
mod extensible;
mod levels;
mod provenance;
#[cfg(test)]
mod tests;
mod value;

pub use camera::{
    CameraIdentity, CaptureMetadata, ColorMatrices, DefaultCrop, Orientation, WhiteBalance,
};
pub use cfa::{BayerLayout, BayerPattern, BayerSlot, CfaColor, CfaShape};
pub use extensible::{
    CoordinateSpace, ExtensibleMetadata, MetadataConflict, MetadataEntry, MetadataField,
    MetadataKey, MetadataUnit, MetadataValue, OriginalValue, TypedMetadataField,
};
use levels::validate_level_relationship;
pub use levels::{BlackLevels, BlackTile, WhiteLevels};
pub use provenance::{
    AuthorityClass, Confidence, DecoderProvenance, Provenance, ProviderId, SourceKind, Sourced,
    resolve_sourced,
};
pub use value::Rational;

use crate::{ActiveArea, PixelSize, RawValidationError, SampleSpec, StoredGeometry, StoredPlane};

/// Typed metadata candidates before required fields are validated.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct CommonMetadata {
    /// Camera identity.
    pub camera: Option<Sourced<CameraIdentity>>,
    /// Sample precision and format.
    pub sample_spec: Option<Sourced<SampleSpec>>,
    /// Compact dimensions and original row stride.
    pub stored_geometry: Option<Sourced<StoredGeometry>>,
    /// Active image rectangle.
    pub active_area: Option<Sourced<ActiveArea>>,
    /// Descriptive default crop.
    pub default_crop: Option<Sourced<DefaultCrop>>,
    /// Bayer layout anchored at the ActiveArea.
    pub cfa: Option<Sourced<BayerLayout>>,
    /// Black-level metadata.
    pub black: Option<Sourced<BlackLevels>>,
    /// White-level metadata.
    pub white: Option<Sourced<WhiteLevels>>,
    /// TIFF orientation.
    pub orientation: Option<Sourced<Orientation>>,
    /// White-balance metadata.
    pub white_balance: Option<Sourced<WhiteBalance>>,
    /// Color matrices.
    pub color_matrices: Option<Sourced<ColorMatrices>>,
    /// Capture metadata.
    pub capture: Option<Sourced<CaptureMetadata>>,
}

impl CommonMetadata {
    fn validate_required(&self) -> Result<(), crate::MetadataError> {
        for (field, present) in [
            (TypedMetadataField::SampleSpec, self.sample_spec.is_some()),
            (
                TypedMetadataField::StoredGeometry,
                self.stored_geometry.is_some(),
            ),
            (TypedMetadataField::ActiveArea, self.active_area.is_some()),
            (TypedMetadataField::Cfa, self.cfa.is_some()),
            (TypedMetadataField::Black, self.black.is_some()),
            (TypedMetadataField::White, self.white.is_some()),
        ] {
            if !present {
                return Err(crate::MetadataError::MissingRequiredField(field));
            }
        }
        Ok(())
    }
}

/// Typed and extensible metadata that has passed structural validation.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ValidatedMetadata {
    common: CommonMetadata,
    extensible: ExtensibleMetadata,
    decoder: DecoderProvenance,
}

impl ValidatedMetadata {
    /// Validates required typed fields and cross-field invariants.
    ///
    /// # Errors
    ///
    /// Returns a structured error for missing metadata, inconsistent geometry,
    /// invalid levels, typed/extensible duplication, or invalid values.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::metadata::{
    ///     CommonMetadata, DecoderProvenance, ExtensibleMetadata, ProviderId, ValidatedMetadata,
    /// };
    ///
    /// let decoder = DecoderProvenance::new(ProviderId::try_new("test").unwrap(), None);
    /// assert!(
    ///     ValidatedMetadata::try_new(
    ///         CommonMetadata::default(),
    ///         ExtensibleMetadata::default(),
    ///         decoder
    ///     )
    ///     .is_err()
    /// );
    /// ```
    pub fn try_new(
        common: CommonMetadata,
        extensible: ExtensibleMetadata,
        decoder: DecoderProvenance,
    ) -> Result<Self, RawValidationError> {
        common.validate_required()?;
        extensible.validate()?;
        if let Some(black) = &common.black {
            black.value.validate()?;
        }
        if let Some(white) = &common.white {
            white.value.validate()?;
        }
        let black = common
            .black
            .as_ref()
            .ok_or(RawValidationError::MissingMetadata("black"))?;
        let white = common
            .white
            .as_ref()
            .ok_or(RawValidationError::MissingMetadata("white"))?;
        validate_level_relationship(&black.value, &white.value)?;

        let geometry = common
            .stored_geometry
            .as_ref()
            .ok_or(RawValidationError::MissingMetadata("stored_geometry"))?
            .value;
        let active_area = common
            .active_area
            .as_ref()
            .ok_or(RawValidationError::MissingMetadata("active_area"))?
            .value;
        let cfa = common
            .cfa
            .as_ref()
            .ok_or(RawValidationError::MissingMetadata("cfa"))?
            .value;

        if cfa.active_area() != active_area {
            return Err(RawValidationError::InvalidCfa);
        }
        let dimensions = geometry.dimensions();
        let rect = active_area.rect();
        if rect.bottom() > dimensions.height() || rect.right() > dimensions.width() {
            return Err(RawValidationError::InvalidActiveArea);
        }
        Ok(Self {
            common,
            extensible,
            decoder,
        })
    }

    /// Returns validated typed metadata.
    ///
    /// The returned structure retains optional fields because its source
    /// representation is also used to describe incomplete decode candidates.
    /// The required fields are guaranteed present.
    ///
    /// # Examples
    ///
    /// ```
    /// # use fornax::metadata::{
    /// #     CommonMetadata, DecoderProvenance, ExtensibleMetadata, ProviderId, ValidatedMetadata,
    /// # };
    /// # let decoder = DecoderProvenance::new(ProviderId::try_new("test").unwrap(), None);
    /// # let validated =
    /// #     ValidatedMetadata::try_new(CommonMetadata::default(), ExtensibleMetadata::default(), decoder);
    /// # assert!(validated.is_err());
    /// ```
    pub const fn common(&self) -> &CommonMetadata { &self.common }

    /// Returns the validated extensible metadata layer.
    pub const fn extensible(&self) -> &ExtensibleMetadata { &self.extensible }

    /// Returns decoder provenance retained for the image lifecycle.
    pub const fn decoder(&self) -> &DecoderProvenance { &self.decoder }

    /// Returns required validated black levels.
    pub fn black(&self) -> &BlackLevels {
        &self
            .common
            .black
            .as_ref()
            .expect("validated metadata has black levels")
            .value
    }

    /// Returns required validated white levels.
    pub fn white(&self) -> &WhiteLevels {
        &self
            .common
            .white
            .as_ref()
            .expect("validated metadata has white levels")
            .value
    }

    /// Returns the required sample specification.
    pub fn sample_spec(&self) -> SampleSpec {
        self.common
            .sample_spec
            .as_ref()
            .expect("validated metadata has sample specification")
            .value
    }

    /// Returns the required compact geometry.
    pub fn stored_geometry(&self) -> StoredGeometry {
        self.common
            .stored_geometry
            .as_ref()
            .expect("validated metadata has stored geometry")
            .value
    }

    /// Returns the required ActiveArea.
    pub fn active_area(&self) -> ActiveArea {
        self.common
            .active_area
            .as_ref()
            .expect("validated metadata has ActiveArea")
            .value
    }

    /// Returns the required Bayer layout.
    pub fn cfa(&self) -> BayerLayout {
        self.common
            .cfa
            .as_ref()
            .expect("validated metadata has CFA")
            .value
    }

    /// Validates a native stored plane against validated geometry and spec.
    pub(crate) fn validate_native_plane(
        &self,
        plane: &StoredPlane,
    ) -> Result<(), RawValidationError> {
        let dimensions = plane.dimensions()?;
        if dimensions != self.stored_geometry().dimensions() {
            return Err(RawValidationError::InvalidGeometry);
        }
        plane.validate_native_spec(self.sample_spec())
    }

    pub(crate) fn validate_f32_working_plane(
        &self,
        plane: &crate::F32Plane,
    ) -> Result<(), RawValidationError> {
        let dimensions = plane.dimensions();
        let dimensions = PixelSize::new(dimensions.0, dimensions.1)
            .ok_or(RawValidationError::InvalidGeometry)?;
        if dimensions != self.stored_geometry().dimensions()
            || plane.as_raw().len() != dimensions.sample_count()?
            || !self.sample_spec().is_f32_exact()
        {
            return Err(RawValidationError::InvalidGeometry);
        }
        crate::decoded_raw::plane::validate_finite(plane)
    }
}
