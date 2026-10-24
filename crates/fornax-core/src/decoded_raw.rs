use std::sync::Arc;

use crate::{
    BlackLevels, ProviderId, RawValidationError, SampleFormat, SampleSpec, Sourced, StoredPlane,
    ValidatedMetadata, WhiteLevels,
};

/// Decoder identity retained with a decoded RAW image.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct DecoderProvenance {
    backend: ProviderId,
    revision: Option<Box<str>>,
}

impl DecoderProvenance {
    /// Creates decoder provenance.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::{DecoderProvenance, ProviderId};
    ///
    /// let decoder = DecoderProvenance::new(
    ///     ProviderId::try_new("libraw").unwrap(),
    ///     Some("0.22.2".into()),
    /// );
    /// assert_eq!(decoder.backend().as_str(), "libraw");
    /// ```
    pub const fn new(backend: ProviderId, revision: Option<Box<str>>) -> Self {
        Self { backend, revision }
    }

    /// Returns the decoder backend identifier.
    pub const fn backend(&self) -> &ProviderId { &self.backend }

    /// Returns the optional decoder revision.
    pub fn revision(&self) -> Option<&str> { self.revision.as_deref() }
}

/// Whether standard black/white metadata remains valid after decoded
/// processing.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum MetadataBasis {
    /// The original file-declared black and white metadata remains valid.
    OriginalLevelsValid,
    /// Custom processing supplied replacement levels.
    ReplacedLevels {
        /// Replacement black levels.
        black: Box<Sourced<BlackLevels>>,
        /// Replacement white levels.
        white: Box<Sourced<WhiteLevels>>,
    },
    /// Metadata no longer describes the sample basis.
    Incompatible,
}

/// A decoder-produced RAW image before standard preprocessing.
///
/// `DecodedRaw` owns one compact sensor plane, immutable validated metadata,
/// and decoder provenance. Its plane may be native u8, u16, u32, or f32. An
/// integer plane may be converted explicitly to working f32 while remaining a
/// `DecodedRaw` for custom processing.
///
/// Standard preprocessing consumes an owned `DecodedRaw` through one public
/// entry point in `fornax-preprocess` and returns a distinct f32-only
/// [`crate::RawImage`].
///
/// # Examples
///
/// ```
/// use fornax_core::{DecodedRaw, RawImage};
///
/// fn accepts_distinct_types(_decoded: &DecodedRaw, _preprocessed: &RawImage) {}
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct DecodedRaw {
    metadata: Arc<ValidatedMetadata>,
    plane: StoredPlane,
    decoder: DecoderProvenance,
    metadata_basis: MetadataBasis,
}

impl DecodedRaw {
    /// Constructs a decoded RAW image and validates its native plane.
    ///
    /// # Errors
    ///
    /// Returns a structured error when plane dimensions, container, spec,
    /// or sample values do not satisfy the metadata contract.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::RawValidationError;
    ///
    /// let error = RawValidationError::MissingMetadata("stored_geometry");
    /// assert_eq!(
    ///     error.to_string(),
    ///     "missing required metadata: stored_geometry"
    /// );
    /// ```
    pub fn try_new(
        metadata: ValidatedMetadata,
        plane: StoredPlane,
        decoder: DecoderProvenance,
    ) -> Result<Self, RawValidationError> {
        metadata.validate_native_plane(&plane)?;
        Ok(Self {
            metadata: Arc::new(metadata),
            plane,
            decoder,
            metadata_basis: MetadataBasis::OriginalLevelsValid,
        })
    }

    /// Returns immutable validated metadata.
    pub fn metadata(&self) -> &ValidatedMetadata { &self.metadata }

    /// Returns the shared metadata handle for retaining after processing.
    pub fn metadata_arc(&self) -> Arc<ValidatedMetadata> { Arc::clone(&self.metadata) }

    /// Returns the current stored plane.
    pub const fn plane(&self) -> &StoredPlane { &self.plane }

    /// Returns mutable access to the decoded stored plane for custom
    /// processing.
    ///
    /// Mutations are not validated immediately. Call
    /// [`Self::set_metadata_basis`] when the sample transformation changes
    /// the validity of black/white metadata, then call
    /// [`Self::require_standard_preprocessing`] before preprocessing or
    /// demosaicing the image.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::{DecodedRaw, StoredPlane};
    ///
    /// fn clear_f32_plane(image: &mut DecodedRaw) {
    ///     if let StoredPlane::F32(samples) = image.plane_mut() {
    ///         samples.as_mut().fill(0.0);
    ///     }
    /// }
    /// ```
    pub fn plane_mut(&mut self) -> &mut StoredPlane { &mut self.plane }

    /// Returns declared sample specification.
    pub fn sample_spec(&self) -> SampleSpec { self.metadata.sample_spec() }

    /// Returns decoder provenance.
    pub const fn decoder(&self) -> &DecoderProvenance { &self.decoder }

    /// Returns the black/white metadata validity basis.
    pub const fn metadata_basis(&self) -> &MetadataBasis { &self.metadata_basis }

    /// Replaces the validity basis after custom sample processing.
    ///
    /// The caller must supply replacement levels when transformations change
    /// the sample basis, or [`MetadataBasis::Incompatible`] when standard
    /// preprocessing must not consume the image.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::{DecodedRaw, MetadataBasis};
    ///
    /// fn reject_standard_preprocessing(image: &mut DecodedRaw) {
    ///     image.set_metadata_basis(MetadataBasis::Incompatible);
    /// }
    /// ```
    pub fn set_metadata_basis(&mut self, metadata_basis: MetadataBasis) {
        self.metadata_basis = metadata_basis;
    }

    /// Converts integer samples to working f32 storage.
    ///
    /// Native floating-point images are already on the f32 path and are
    /// returned unchanged. Integer conversion is exact only when the declared
    /// precision fits f32's 24-bit significand.
    ///
    /// # Errors
    ///
    /// Returns a typed error for repeated conversion, wider integer sources,
    /// incompatible geometry, or non-finite samples.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::RawValidationError;
    ///
    /// assert_eq!(
    ///     RawValidationError::InvalidStorageState.to_string(),
    ///     "invalid decoded RAW sample storage"
    /// );
    /// ```
    pub fn convert_to_f32(mut self) -> Result<Self, RawValidationError> {
        let spec = self.metadata.sample_spec();
        if matches!(self.plane, StoredPlane::F32(_)) {
            if spec.format() != SampleFormat::Float {
                return Err(RawValidationError::InvalidStorageState);
            }
            self.metadata
                .validate_f32_working_plane(match &self.plane {
                    StoredPlane::F32(plane) => plane,
                    StoredPlane::U8(_) | StoredPlane::U16(_) | StoredPlane::U32(_) => {
                        unreachable!()
                    }
                })?;
            return Ok(self);
        }

        self.plane = StoredPlane::F32(self.plane.into_f32(spec)?);
        self.metadata
            .validate_f32_working_plane(match &self.plane {
                StoredPlane::F32(plane) => plane,
                StoredPlane::U8(_) | StoredPlane::U16(_) | StoredPlane::U32(_) => unreachable!(),
            })?;
        Ok(self)
    }

    /// Verifies that the single standard preprocessing pass may consume this
    /// image.
    ///
    /// # Errors
    ///
    /// Returns [`RawValidationError::InvalidCustomMetadataBasis`] when custom
    /// processing invalidated black/white metadata without replacements.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::RawValidationError;
    ///
    /// assert_eq!(
    ///     RawValidationError::InvalidCustomMetadataBasis.to_string(),
    ///     "custom processing metadata basis is incompatible with standard preprocessing"
    /// );
    /// ```
    pub fn require_standard_preprocessing(&self) -> Result<(), RawValidationError> {
        if matches!(self.metadata_basis, MetadataBasis::Incompatible) {
            return Err(RawValidationError::InvalidCustomMetadataBasis);
        }
        match &self.plane {
            StoredPlane::U8(_) | StoredPlane::U16(_) | StoredPlane::U32(_) => {
                self.metadata.validate_native_plane(&self.plane)
            }
            StoredPlane::F32(plane) => self.metadata.validate_f32_working_plane(plane),
        }
    }

    /// Consumes the image and returns its stored plane.
    pub fn into_plane(self) -> StoredPlane { self.plane }

    /// Consumes the image into retained metadata and stored plane.
    pub fn into_parts(self) -> (Arc<ValidatedMetadata>, StoredPlane) { (self.metadata, self.plane) }

    pub(crate) fn into_preprocessing_parts(
        self,
    ) -> (Arc<ValidatedMetadata>, DecoderProvenance, MetadataBasis) {
        (self.metadata, self.decoder, self.metadata_basis)
    }
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU32, NonZeroU64};

    use image::ImageBuffer;

    use super::*;
    use crate::{
        ActiveArea, AuthorityClass, BayerLayout, BayerPattern, CommonMetadata, Confidence,
        ExtensibleMetadata, PixelSize, Provenance, RawImage, SourceKind, StoredGeometry, U16Plane,
        into_preprocessed,
    };

    fn provenance() -> Provenance {
        Provenance::new(
            ProviderId::try_new("test").unwrap(),
            SourceKind::Derived {
                operation: "fixture".into(),
            },
            AuthorityClass::DerivedValue,
            Confidence::Exact,
        )
    }

    fn sourced<T>(value: T) -> Sourced<T> { Sourced::new(value, provenance()) }

    fn decoded() -> DecodedRaw {
        let dimensions = PixelSize::new(2, 2).unwrap();
        let active_area = ActiveArea::try_new(dimensions, 1, 1, 2, 2).unwrap();
        let geometry = StoredGeometry::new(dimensions, NonZeroU64::new(4).unwrap());
        let common = CommonMetadata {
            sample_spec: Some(sourced(
                SampleSpec::try_new(16, SampleFormat::UnsignedInteger).unwrap(),
            )),
            stored_geometry: Some(sourced(geometry)),
            active_area: Some(sourced(active_area)),
            cfa: Some(sourced(BayerLayout::new(BayerPattern::Gbrg, active_area))),
            ..CommonMetadata::default()
        };
        let metadata = ValidatedMetadata::try_new(common, ExtensibleMetadata::default()).unwrap();
        let plane = StoredPlane::U16(U16Plane::from_raw(2, 2, vec![1, 2, 3, 4]).unwrap());
        DecodedRaw::try_new(
            metadata,
            plane,
            DecoderProvenance::new(ProviderId::try_new("test").unwrap(), None),
        )
        .unwrap()
    }

    #[test]
    fn conversion_remains_a_decoded_raw() {
        let decoded = decoded().convert_to_f32().unwrap();
        assert!(matches!(decoded.plane(), StoredPlane::F32(_)));
        assert_eq!(decoded.require_standard_preprocessing(), Ok(()));
        assert_eq!(
            decoded.clone().convert_to_f32(),
            Err(RawValidationError::InvalidStorageState)
        );
    }

    #[test]
    fn custom_processing_can_mutate_the_decoded_plane() {
        let mut custom = decoded().convert_to_f32().unwrap();
        let StoredPlane::F32(plane) = custom.plane_mut() else {
            panic!("converted image has an f32 plane");
        };
        plane.as_mut().fill(0.25);
        custom.set_metadata_basis(MetadataBasis::Incompatible);
        assert_eq!(
            custom.require_standard_preprocessing(),
            Err(RawValidationError::InvalidCustomMetadataBasis)
        );
    }

    #[test]
    fn preprocessing_returns_the_distinct_f32_only_type() {
        let decoded = decoded().convert_to_f32().unwrap();
        let ready: RawImage =
            into_preprocessed(decoded, ImageBuffer::from_raw(2, 2, vec![0.75; 4]).unwrap())
                .unwrap();
        assert_eq!(ready.plane().as_raw(), &[0.75; 4]);
        assert_eq!(ready.validate_for_demosaic(), Ok(()));
        assert_eq!(ready.metadata().cfa().pattern(), BayerPattern::Gbrg);
    }

    #[test]
    fn readiness_rejects_out_of_range_values() {
        let decoded = decoded().convert_to_f32().unwrap();
        assert_eq!(
            into_preprocessed(decoded, ImageBuffer::from_raw(2, 2, vec![1.25; 4]).unwrap())
                .unwrap_err(),
            RawValidationError::InvalidReadinessRange
        );
    }

    #[test]
    fn replaced_levels_are_accepted() {
        let one = NonZeroU32::new(1).unwrap();
        let black = crate::Rational::new(1, one);
        let white = crate::Rational::new(100, one);
        let basis = MetadataBasis::ReplacedLevels {
            black: Box::new(sourced(
                BlackLevels::try_new(black, [black; 4], None).unwrap(),
            )),
            white: Box::new(sourced(
                WhiteLevels::try_new(vec![white], None, None).unwrap(),
            )),
        };
        let mut image = decoded().convert_to_f32().unwrap();
        let StoredPlane::F32(plane) = image.plane_mut() else {
            panic!("converted image has an f32 plane");
        };
        plane.as_mut().fill(0.5);
        image.set_metadata_basis(basis);
        assert_eq!(image.require_standard_preprocessing(), Ok(()));
    }

    #[test]
    fn preprocessing_revalidates_user_mutated_plane() {
        let mut image = decoded();
        *image.plane_mut() = StoredPlane::U8(crate::U8Plane::from_raw(2, 2, vec![1; 4]).unwrap());
        assert_eq!(
            image.require_standard_preprocessing(),
            Err(RawValidationError::UnsupportedSampleSpec {
                bits_per_sample: 16,
                format: SampleFormat::UnsignedInteger,
            })
        );
    }

    #[test]
    fn constructor_rejects_wrong_dimensions_and_spec() {
        let image = decoded();
        let metadata = image.metadata().clone();
        let decoder = DecoderProvenance::new(ProviderId::try_new("test").unwrap(), None);
        assert_eq!(
            DecodedRaw::try_new(
                metadata.clone(),
                StoredPlane::U16(U16Plane::from_raw(1, 1, vec![1]).unwrap()),
                decoder.clone(),
            )
            .unwrap_err(),
            RawValidationError::InvalidGeometry
        );
        assert_eq!(
            DecodedRaw::try_new(
                metadata,
                StoredPlane::U8(crate::U8Plane::from_raw(2, 2, vec![1; 4]).unwrap()),
                decoder,
            )
            .unwrap_err(),
            RawValidationError::UnsupportedSampleSpec {
                bits_per_sample: 16,
                format: SampleFormat::UnsignedInteger,
            }
        );
    }

    #[test]
    fn decoded_raw_accepts_every_supported_plane() {
        let metadata = |spec| {
            let dimensions = PixelSize::new(1, 1).unwrap();
            let active_area = ActiveArea::try_new(dimensions, 0, 0, 1, 1).unwrap();
            let common = CommonMetadata {
                sample_spec: Some(sourced(spec)),
                stored_geometry: Some(sourced(StoredGeometry::new(
                    dimensions,
                    NonZeroU64::new(4).unwrap(),
                ))),
                active_area: Some(sourced(active_area)),
                cfa: Some(sourced(BayerLayout::new(BayerPattern::Rggb, active_area))),
                ..CommonMetadata::default()
            };
            ValidatedMetadata::try_new(common, ExtensibleMetadata::default()).unwrap()
        };
        let decoder = || DecoderProvenance::new(ProviderId::try_new("test").unwrap(), None);
        let cases = [
            (
                SampleSpec::try_new(8, SampleFormat::UnsignedInteger).unwrap(),
                StoredPlane::U8(ImageBuffer::from_raw(1, 1, vec![1]).unwrap()),
            ),
            (
                SampleSpec::try_new(16, SampleFormat::UnsignedInteger).unwrap(),
                StoredPlane::U16(ImageBuffer::from_raw(1, 1, vec![1]).unwrap()),
            ),
            (
                SampleSpec::try_new(32, SampleFormat::UnsignedInteger).unwrap(),
                StoredPlane::U32(ImageBuffer::from_raw(1, 1, vec![1]).unwrap()),
            ),
            (
                SampleSpec::try_new(32, SampleFormat::Float).unwrap(),
                StoredPlane::F32(ImageBuffer::from_raw(1, 1, vec![0.5]).unwrap()),
            ),
        ];
        for (spec, plane) in cases {
            let image = DecodedRaw::try_new(metadata(spec), plane, decoder()).unwrap();
            assert_eq!(
                image.plane().dimensions().unwrap(),
                PixelSize::new(1, 1).unwrap()
            );
        }
    }
}
