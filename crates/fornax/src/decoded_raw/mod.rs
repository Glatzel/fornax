use std::sync::Arc;

pub mod plane;
mod preprocess;

pub use plane::{F32Plane, SampleFormat, SampleSpec, StoredPlane, U8Plane, U16Plane, U32Plane};

use crate::RawImage;
use crate::error::{PreprocessError, RawValidationError};
use crate::metadata::{DecoderProvenance, ValidatedMetadata};

/// A decoder-produced RAW image before standard preprocessing.
///
/// `DecodedRaw` owns one compact sensor plane, immutable validated metadata,
/// and decoder provenance. Its plane may be native u8, u16, u32, or f32. An
/// integer plane may be converted explicitly to working f32 while remaining a
/// `DecodedRaw` for custom processing.
///
/// Standard preprocessing consumes an owned `DecodedRaw` through
/// [`DecodedRaw::preprocess`] and returns a distinct f32-only
/// [`crate::RawImage`]. Its numerical substeps remain private.
///
/// # Examples
///
/// ```
/// use fornax::{DecodedRaw, RawImage};
///
/// fn accepts_distinct_types(_decoded: &DecodedRaw, _preprocessed: &RawImage) {}
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct DecodedRaw {
    metadata: Arc<ValidatedMetadata>,
    plane: StoredPlane,
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
    /// use fornax::error::RawValidationError;
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
    ) -> Result<Self, RawValidationError> {
        metadata.validate_native_plane(&plane)?;
        Ok(Self {
            metadata: Arc::new(metadata),
            plane,
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
    /// Mutations are not validated immediately. Custom sample edits must
    /// preserve the metadata semantics established by the decoder. Validation
    /// is applied automatically when preprocessing consumes the image.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::{DecodedRaw, StoredPlane};
    ///
    /// fn clear_f32_plane(image: &mut DecodedRaw) {
    ///     if let StoredPlane::F32(samples) = image.plane_mut() {
    ///         samples.as_mut().fill(0.0);
    ///     }
    /// }
    /// ```
    pub const fn plane_mut(&mut self) -> &mut StoredPlane { &mut self.plane }

    /// Returns declared sample specification.
    pub fn sample_spec(&self) -> SampleSpec { self.metadata.sample_spec() }

    /// Returns decoder provenance.
    pub fn decoder(&self) -> &DecoderProvenance { self.metadata.decoder() }

    /// Converts integer samples to working f32 storage.
    ///
    /// Native floating-point images are already on the f32 path and are
    /// returned unchanged. Integer conversion is exact only when the declared
    /// precision fits f32's 24-bit significand and every sample value
    /// round-trips exactly.
    ///
    /// # Errors
    ///
    /// Returns a typed error for repeated conversion, wider or lossy integer
    /// sources, incompatible geometry, or non-finite samples.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::error::RawValidationError;
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

    fn validate_for_preprocessing(&self) -> Result<(), RawValidationError> {
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

    /// Consumes the decoded image through the single standard preprocessing
    /// entry point.
    ///
    /// The operation converts native integer samples to the f32 working domain
    /// when needed, then applies identity linearization, black-level
    /// subtraction, white-level normalization, ActiveArea cropping, and final
    /// clipping in that order. Decoder-validated metadata and provenance are
    /// retained, including the original stored geometry.
    ///
    /// # Errors
    ///
    /// Returns a structured [`PreprocessError`] when the current plane is
    /// invalid, conversion would be lossy, metadata cannot be applied, or an
    /// allocation or numerical operation fails.
    ///
    /// # Examples
    ///
    /// ```
    /// # use std::num::{NonZeroU32, NonZeroU64};
    /// # use fornax::geometry::{ActiveArea, PixelSize, StoredGeometry};
    /// # use fornax::metadata::{
    /// #     AuthorityClass, BlackLevels, CommonMetadata, Confidence, DecoderProvenance,
    /// #     ExtensibleMetadata, Provenance, ProviderId, Rational, SourceKind, Sourced,
    /// #     ValidatedMetadata, WhiteLevels,
    /// # };
    /// # use fornax::metadata::{BayerLayout, BayerPattern};
    /// # use fornax::{DecodedRaw, SampleFormat, SampleSpec, StoredPlane};
    /// # fn input() -> DecodedRaw {
    /// #     let dimensions = PixelSize::new(1, 1).unwrap();
    /// #     let area = ActiveArea::try_new(dimensions, 0, 0, 1, 1).unwrap();
    /// #     let provider = ProviderId::try_new("example").unwrap();
    /// #     let provenance = Provenance::new(
    /// #         provider.clone(),
    /// #         SourceKind::Derived { operation: "example".into() },
    /// #         AuthorityClass::DerivedValue,
    /// #         Confidence::Exact,
    /// #     );
    /// #     let zero = Rational::new(0, NonZeroU32::new(1).unwrap());
    /// #     let one = Rational::new(1, NonZeroU32::new(1).unwrap());
    /// #     let metadata = ValidatedMetadata::try_new(
    /// #         CommonMetadata {
    /// #             sample_spec: Some(Sourced::new(
    /// #                 SampleSpec::try_new(32, SampleFormat::Float).unwrap(),
    /// #                 provenance.clone(),
    /// #             )),
    /// #             stored_geometry: Some(Sourced::new(
    /// #                 StoredGeometry::new(dimensions, NonZeroU64::new(4).unwrap()),
    /// #                 provenance.clone(),
    /// #             )),
    /// #             active_area: Some(Sourced::new(area, provenance.clone())),
    /// #             cfa: Some(Sourced::new(
    /// #                 BayerLayout::new(BayerPattern::Rggb, area),
    /// #                 provenance.clone(),
    /// #             )),
    /// #             black: Some(Sourced::new(
    /// #                 BlackLevels::try_new(zero, [zero; 4], None).unwrap(),
    /// #                 provenance.clone(),
    /// #             )),
    /// #             white: Some(Sourced::new(
    /// #                 WhiteLevels::try_new(vec![one], None, None).unwrap(),
    /// #                 provenance,
    /// #             )),
    /// #             ..CommonMetadata::default()
    /// #         },
    /// #         ExtensibleMetadata::default(),
    /// #         DecoderProvenance::new(provider, None),
    /// #     )
    /// #     .unwrap();
    /// #     let plane = image::ImageBuffer::from_raw(1, 1, vec![0.25]).unwrap();
    /// #     DecodedRaw::try_new(metadata, StoredPlane::F32(plane)).unwrap()
    /// # }
    ///
    /// let output = input().preprocess().unwrap();
    /// assert_eq!(output.plane().get_pixel(0, 0)[0], 0.25);
    /// ```
    pub fn preprocess(self) -> Result<RawImage, PreprocessError> { preprocess::run(self) }
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU32, NonZeroU64};

    use image::ImageBuffer;

    use super::*;
    use crate::geometry::{ActiveArea, PixelSize, StoredGeometry};
    use crate::metadata::{
        AuthorityClass, BayerLayout, BayerPattern, BlackLevels, CommonMetadata, Confidence,
        ExtensibleMetadata, Provenance, ProviderId, Rational, SourceKind, Sourced,
        ValidatedMetadata, WhiteLevels,
    };
    use crate::{RawImage, U16Plane};

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

    fn black_levels() -> BlackLevels {
        let zero = Rational::new(0, NonZeroU32::new(1).unwrap());
        BlackLevels::try_new(zero, [zero; 4], None).unwrap()
    }

    fn white_levels() -> WhiteLevels {
        let white = Rational::new(65_535, NonZeroU32::new(1).unwrap());
        WhiteLevels::try_new(vec![white], None, None).unwrap()
    }

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
            black: Some(sourced(black_levels())),
            white: Some(sourced(white_levels())),
            ..CommonMetadata::default()
        };
        let metadata = ValidatedMetadata::try_new(
            common,
            ExtensibleMetadata::default(),
            DecoderProvenance::new(ProviderId::try_new("test").unwrap(), None),
        )
        .unwrap();
        let plane = StoredPlane::U16(U16Plane::from_raw(2, 2, vec![1, 2, 3, 4]).unwrap());
        DecodedRaw::try_new(metadata, plane).unwrap()
    }

    #[test]
    fn conversion_remains_a_decoded_raw() {
        let decoded = decoded().convert_to_f32().unwrap();
        assert!(matches!(decoded.plane(), StoredPlane::F32(_)));
        decoded.clone().preprocess().unwrap();
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
        custom.preprocess().unwrap();
    }

    #[test]
    fn preprocessing_returns_the_distinct_f32_only_type() {
        let ready: RawImage = decoded().preprocess().unwrap();
        let expected = [(4.0_f64 / 65_535.0) as f32];
        assert_eq!(ready.plane().as_raw(), &expected);
        assert_eq!(ready.plane().dimensions(), (1, 1));
        assert_eq!(ready.metadata().cfa().pattern(), BayerPattern::Gbrg);
        assert_eq!(ready.metadata().decoder().backend().as_str(), "test");
    }

    #[test]
    fn preprocessing_revalidates_user_mutated_plane() {
        let mut image = decoded();
        *image.plane_mut() = StoredPlane::U8(crate::U8Plane::from_raw(2, 2, vec![1; 4]).unwrap());
        assert!(matches!(
            image.preprocess(),
            Err(PreprocessError::InvalidInput(_))
        ));
    }

    #[test]
    fn constructor_rejects_wrong_dimensions_and_spec() {
        let image = decoded();
        let metadata = image.metadata().clone();
        assert_eq!(
            DecodedRaw::try_new(
                metadata.clone(),
                StoredPlane::U16(U16Plane::from_raw(1, 1, vec![1]).unwrap()),
            )
            .unwrap_err(),
            RawValidationError::InvalidGeometry
        );
        assert_eq!(
            DecodedRaw::try_new(
                metadata,
                StoredPlane::U8(crate::U8Plane::from_raw(2, 2, vec![1; 4]).unwrap()),
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
                black: Some(sourced(black_levels())),
                white: Some(sourced(white_levels())),
                ..CommonMetadata::default()
            };
            ValidatedMetadata::try_new(
                common,
                ExtensibleMetadata::default(),
                DecoderProvenance::new(ProviderId::try_new("test").unwrap(), None),
            )
            .unwrap()
        };
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
            let image = DecodedRaw::try_new(metadata(spec), plane).unwrap();
            assert_eq!(
                image.plane().dimensions().unwrap(),
                PixelSize::new(1, 1).unwrap()
            );
        }
    }
}
