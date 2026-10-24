use std::sync::Arc;

use crate::plane::validate_finite;
use crate::{
    BlackLevels, F32Plane, ProviderId, RawValidationError, SampleFormat, Sourced, StoredPlane,
    ValidatedMetadata, WhiteLevels,
};

mod sealed {
    pub trait Sealed {}
}

/// The processing state of a [`RawImage`].
pub trait RawStage: sealed::Sealed {
    /// The payload representation valid for this stage.
    type Plane;

    /// Returns the named processing stage.
    fn kind() -> StageKind;
}

/// A named processing stage for diagnostics and metadata.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum StageKind {
    /// Backend-decoded native or custom `f32` samples.
    Decoded,
    /// Linearized `f32` samples.
    Linearized,
    /// Black-subtracted, unclipped `f32` samples.
    BlackCorrected,
    /// Normalized, unclipped `f32` samples.
    Normalized,
    /// Clipped `f32` samples in `[0, 1]` ready for demosaicing.
    DemosaicReady,
}

/// Decoder identity retained with a decoded image.
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

/// A decoded-plane operation that remains in the `Decoded` stage.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct OperationProvenance {
    operation: Box<str>,
    provider: ProviderId,
}

impl OperationProvenance {
    /// Creates operation provenance for custom decoded-plane processing.
    ///
    /// # Errors
    ///
    /// Returns [`crate::MetadataError::InvalidValue`] for an empty operation
    /// name.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::{OperationProvenance, ProviderId};
    ///
    /// let operation = OperationProvenance::try_new(
    ///     "linearize_baseline",
    ///     ProviderId::try_new("example").unwrap(),
    /// )
    /// .unwrap();
    /// assert_eq!(operation.operation(), "linearize_baseline");
    /// ```
    pub fn try_new(
        operation: impl Into<Box<str>>,
        provider: ProviderId,
    ) -> Result<Self, crate::MetadataError> {
        let operation = operation.into();
        if operation.is_empty() {
            Err(crate::MetadataError::InvalidValue)
        } else {
            Ok(Self {
                operation,
                provider,
            })
        }
    }

    /// Returns the operation name.
    pub fn operation(&self) -> &str { &self.operation }

    /// Returns the operation provider.
    pub const fn provider(&self) -> &ProviderId { &self.provider }

    fn system_conversion() -> Self {
        Self {
            operation: "convert_to_f32".into(),
            provider: ProviderId::try_new("fornax-core")
                .expect("the built-in provider identifier is non-empty"),
        }
    }
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

/// Storage state while an image remains decoded.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum DecodedStorage {
    /// A native u8, u16, u32, or source-f32 plane.
    Native,
    /// A working f32 plane derived from an integer source or custom operation.
    F32Working,
}

/// Decode-stage state carried alongside the payload.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Decoded {
    decoder: DecoderProvenance,
    operations: Box<[OperationProvenance]>,
    metadata_basis: MetadataBasis,
    storage: DecodedStorage,
}

impl Decoded {
    /// Returns the decoder provenance.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::{DecoderProvenance, ProviderId, RawStage};
    ///
    /// assert_eq!(
    ///     <fornax_core::Decoded as RawStage>::kind(),
    ///     fornax_core::StageKind::Decoded
    /// );
    /// ```
    pub const fn decoder(&self) -> &DecoderProvenance { &self.decoder }

    /// Returns operations applied while the image remained decoded.
    pub fn operations(&self) -> &[OperationProvenance] { &self.operations }

    /// Returns the validity basis for black and white metadata.
    pub const fn metadata_basis(&self) -> &MetadataBasis { &self.metadata_basis }

    /// Returns whether samples use native or working f32 storage.
    pub const fn storage(&self) -> DecodedStorage { self.storage }
}

/// Linearization-stage marker.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Linearized(());

/// Black-corrected-stage marker; samples remain unclipped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct BlackCorrected(());

/// Normalization-stage marker; samples remain unclipped.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Normalized(());

/// Demosaic-ready stage marker with finite samples in `[0, 1]`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct DemosaicReady(());

macro_rules! impl_stage {
    ($stage:ty, $plane:ty, $kind:expr) => {
        impl sealed::Sealed for $stage {}

        impl RawStage for $stage {
            type Plane = $plane;

            fn kind() -> StageKind { $kind }
        }
    };
}

impl_stage!(Decoded, StoredPlane, StageKind::Decoded);
impl_stage!(Linearized, F32Plane, StageKind::Linearized);
impl_stage!(BlackCorrected, F32Plane, StageKind::BlackCorrected);
impl_stage!(Normalized, F32Plane, StageKind::Normalized);
impl_stage!(DemosaicReady, F32Plane, StageKind::DemosaicReady);

/// A RAW sensor plane and its immutable metadata in a consuming stage.
///
/// # Examples
///
/// ```
/// use fornax_core::{DemosaicReady, RawStage};
///
/// assert_eq!(DemosaicReady::kind(), fornax_core::StageKind::DemosaicReady);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct RawImage<S: RawStage> {
    metadata: Arc<ValidatedMetadata>,
    plane: S::Plane,
    stage: S,
}

impl<S: RawStage> RawImage<S> {
    /// Returns immutable validated metadata.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::{DemosaicReady, RawStage};
    ///
    /// assert_eq!(DemosaicReady::kind(), fornax_core::StageKind::DemosaicReady);
    /// ```
    pub fn metadata(&self) -> &ValidatedMetadata { &self.metadata }

    /// Returns the shared metadata handle for retaining after processing.
    pub fn metadata_arc(&self) -> Arc<ValidatedMetadata> { Arc::clone(&self.metadata) }

    /// Returns the stage-specific payload.
    pub fn plane(&self) -> &S::Plane { &self.plane }

    /// Returns the stage state.
    pub const fn stage(&self) -> &S { &self.stage }

    /// Returns the stage kind.
    pub fn stage_kind(&self) -> StageKind { S::kind() }

    /// Consumes the image and returns its payload.
    ///
    /// # Examples
    ///
    /// ```
    /// use image::{ImageBuffer, Luma};
    ///
    /// let plane = ImageBuffer::<Luma<f32>, Vec<f32>>::from_raw(1, 1, vec![0.5]).unwrap();
    /// assert_eq!(plane.as_raw().len(), 1);
    /// ```
    pub fn into_plane(self) -> S::Plane { self.plane }

    /// Consumes the image into retained metadata and payload.
    pub fn into_parts(self) -> (Arc<ValidatedMetadata>, S::Plane) { (self.metadata, self.plane) }
}

/// A decoded RAW image, including native or custom f32 storage.
pub type DecodedRaw = RawImage<Decoded>;
/// A linearized RAW image.
pub type LinearizedRaw = RawImage<Linearized>;
/// A black-corrected RAW image.
pub type BlackCorrectedRaw = RawImage<BlackCorrected>;
/// A normalized RAW image.
pub type NormalizedRaw = RawImage<Normalized>;
/// A clipped Bayer mosaic ready for demosaicing.
pub type DemosaicReadyMosaic = RawImage<DemosaicReady>;

impl RawImage<Decoded> {
    /// Constructs a decoded image and validates its native plane.
    ///
    /// # Errors
    ///
    /// Returns a structured error when plane dimensions, container, encoding,
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
            stage: Decoded {
                decoder,
                operations: Box::new([]),
                metadata_basis: MetadataBasis::OriginalLevelsValid,
                storage: DecodedStorage::Native,
            },
        })
    }

    /// Returns decoder provenance.
    pub const fn decoder(&self) -> &DecoderProvenance { self.stage.decoder() }

    /// Returns operations applied while decoded.
    pub fn operations(&self) -> &[OperationProvenance] { self.stage.operations() }

    /// Returns the black/white metadata validity basis.
    pub const fn metadata_basis(&self) -> &MetadataBasis { self.stage.metadata_basis() }

    /// Returns the decoded storage state.
    pub const fn storage(&self) -> DecodedStorage { self.stage.storage() }

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
    /// assert!(matches!(
    ///     RawValidationError::UnsupportedSampleEncoding {
    ///         bits_per_sample: 32,
    ///         format: fornax_core::SampleFormat::UnsignedInteger,
    ///     },
    ///     RawValidationError::UnsupportedSampleEncoding { .. }
    /// ));
    /// ```
    pub fn convert_to_f32(mut self) -> Result<Self, RawValidationError> {
        if self.stage.storage == DecodedStorage::F32Working {
            return Err(RawValidationError::InvalidStageTransition);
        }
        let encoding = self.metadata.sample_encoding();
        if matches!(self.plane, StoredPlane::F32(_)) {
            if encoding.format() != SampleFormat::Float {
                return Err(RawValidationError::UnsupportedPlaneEncoding);
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

        self.plane = StoredPlane::F32(self.plane.into_f32(encoding, false)?);
        self.metadata
            .validate_f32_working_plane(match &self.plane {
                StoredPlane::F32(plane) => plane,
                StoredPlane::U8(_) | StoredPlane::U16(_) | StoredPlane::U32(_) => unreachable!(),
            })?;
        self.stage.storage = DecodedStorage::F32Working;
        let mut operations = self.stage.operations.into_vec();
        operations.push(OperationProvenance::system_conversion());
        self.stage.operations = operations.into_boxed_slice();
        Ok(self)
    }

    /// Replaces the f32 plane after custom processing while remaining decoded.
    ///
    /// # Errors
    ///
    /// Returns a typed error unless the image is already on the f32 working
    /// path and the replacement preserves geometry and finite samples.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_core::RawValidationError;
    ///
    /// assert_eq!(
    ///     RawValidationError::NonFiniteSample.to_string(),
    ///     "RAW plane contains a non-finite sample"
    /// );
    /// ```
    pub fn with_custom_f32_plane(
        mut self,
        plane: F32Plane,
        operation: OperationProvenance,
        metadata_basis: MetadataBasis,
    ) -> Result<Self, RawValidationError> {
        let has_f32_plane = matches!(self.plane, StoredPlane::F32(_));
        if !has_f32_plane
            || (self.stage.storage == DecodedStorage::Native
                && self.metadata.sample_encoding().format() != SampleFormat::Float)
        {
            return Err(RawValidationError::InvalidStageTransition);
        }
        self.metadata.validate_f32_working_plane(&plane)?;
        self.plane = StoredPlane::F32(plane);
        self.stage.storage = DecodedStorage::F32Working;
        self.stage.metadata_basis = metadata_basis;
        let mut operations = self.stage.operations.into_vec();
        operations.push(operation);
        self.stage.operations = operations.into_boxed_slice();
        Ok(self)
    }

    /// Verifies that standard preprocessing may consume this decoded image.
    ///
    /// # Errors
    ///
    /// Returns [`RawValidationError::InvalidCustomMetadataBasis`] when a custom
    /// operation invalidated black/white metadata without replacements.
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
        if matches!(self.metadata_basis(), MetadataBasis::Incompatible) {
            Err(RawValidationError::InvalidCustomMetadataBasis)
        } else {
            Ok(())
        }
    }
}

/// Hidden transition operations used by the later preprocessing crate.
///
/// These functions enforce consuming transitions and payload invariants without
/// adding preprocessing algorithms to `fornax-core`.
#[doc(hidden)]
pub mod transition {
    use super::*;

    /// Creates a linearized image from a decoded image.
    pub fn linearized_from_decoded(
        decoded: DecodedRaw,
        plane: F32Plane,
    ) -> Result<LinearizedRaw, RawValidationError> {
        decoded.require_standard_preprocessing()?;
        decoded.metadata.validate_f32_working_plane(&plane)?;
        Ok(RawImage {
            metadata: decoded.metadata,
            plane,
            stage: Linearized(()),
        })
    }

    /// Creates a black-corrected image from a linearized image.
    pub fn black_corrected_from_linearized(
        linearized: LinearizedRaw,
        plane: F32Plane,
    ) -> Result<BlackCorrectedRaw, RawValidationError> {
        linearized.metadata.validate_f32_working_plane(&plane)?;
        Ok(RawImage {
            metadata: linearized.metadata,
            plane,
            stage: BlackCorrected(()),
        })
    }

    /// Creates a normalized image from a black-corrected image.
    pub fn normalized_from_black_corrected(
        corrected: BlackCorrectedRaw,
        plane: F32Plane,
    ) -> Result<NormalizedRaw, RawValidationError> {
        corrected.metadata.validate_f32_working_plane(&plane)?;
        Ok(RawImage {
            metadata: corrected.metadata,
            plane,
            stage: Normalized(()),
        })
    }

    /// Validates and creates a demosaic-ready image from a normalized image.
    pub fn demosaic_ready_from_normalized(
        normalized: NormalizedRaw,
        plane: F32Plane,
    ) -> Result<DemosaicReadyMosaic, RawValidationError> {
        normalized.metadata.validate_f32_working_plane(&plane)?;
        validate_finite(&plane)?;
        if plane
            .as_raw()
            .iter()
            .any(|sample| !(0.0..=1.0).contains(sample))
        {
            return Err(RawValidationError::InvalidReadinessRange);
        }
        Ok(RawImage {
            metadata: normalized.metadata,
            plane,
            stage: DemosaicReady(()),
        })
    }
}

#[cfg(test)]
mod tests {
    use std::num::{NonZeroU32, NonZeroU64};

    use image::ImageBuffer;

    use super::*;
    use crate::{
        ActiveArea, AuthorityClass, BayerLayout, BayerPattern, CommonMetadata, Confidence,
        ExtensibleMetadata, PixelSize, Provenance, SampleEncoding, SourceKind, StoredGeometry,
        U16Plane,
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
            sample_encoding: Some(sourced(
                SampleEncoding::try_new(16, SampleFormat::UnsignedInteger).unwrap(),
            )),
            stored_geometry: Some(sourced(geometry)),
            active_area: Some(sourced(active_area)),
            cfa: Some(sourced(BayerLayout::new(BayerPattern::Gbrg, active_area))),
            ..CommonMetadata::default()
        };
        let metadata = ValidatedMetadata::try_new(common, ExtensibleMetadata::default()).unwrap();
        let plane = StoredPlane::U16(U16Plane::from_raw(2, 2, vec![1, 2, 3, 4]).unwrap());
        RawImage::<Decoded>::try_new(
            metadata,
            plane,
            DecoderProvenance::new(ProviderId::try_new("test").unwrap(), None),
        )
        .unwrap()
    }

    #[test]
    fn conversion_stays_decoded_and_records_provenance() {
        let decoded = decoded().convert_to_f32().unwrap();
        assert_eq!(decoded.stage_kind(), StageKind::Decoded);
        assert_eq!(decoded.storage(), DecodedStorage::F32Working);
        assert_eq!(decoded.operations()[0].operation(), "convert_to_f32");
        assert_eq!(decoded.require_standard_preprocessing(), Ok(()));
        assert_eq!(
            decoded.clone().convert_to_f32(),
            Err(RawValidationError::InvalidStageTransition)
        );
    }

    #[test]
    fn custom_processing_can_reject_standard_metadata() {
        let operation =
            OperationProvenance::try_new("custom", ProviderId::try_new("test").unwrap()).unwrap();
        let custom = decoded()
            .convert_to_f32()
            .unwrap()
            .with_custom_f32_plane(
                ImageBuffer::from_raw(2, 2, vec![0.25; 4]).unwrap(),
                operation,
                MetadataBasis::Incompatible,
            )
            .unwrap();
        assert_eq!(
            custom.require_standard_preprocessing(),
            Err(RawValidationError::InvalidCustomMetadataBasis)
        );
        assert_eq!(
            transition::linearized_from_decoded(
                custom,
                ImageBuffer::from_raw(2, 2, vec![0.25; 4]).unwrap()
            )
            .unwrap_err(),
            RawValidationError::InvalidCustomMetadataBasis
        );
    }

    #[test]
    fn stages_are_consuming_and_readiness_is_checked() {
        let decoded = decoded().convert_to_f32().unwrap();
        let linearized = transition::linearized_from_decoded(
            decoded,
            ImageBuffer::from_raw(2, 2, vec![1.0; 4]).unwrap(),
        )
        .unwrap();
        let corrected = transition::black_corrected_from_linearized(
            linearized,
            ImageBuffer::from_raw(2, 2, vec![-0.5; 4]).unwrap(),
        )
        .unwrap();
        let normalized = transition::normalized_from_black_corrected(
            corrected,
            ImageBuffer::from_raw(2, 2, vec![1.25; 4]).unwrap(),
        )
        .unwrap();
        assert_eq!(
            transition::demosaic_ready_from_normalized(
                normalized.clone(),
                ImageBuffer::from_raw(2, 2, vec![1.25; 4]).unwrap(),
            )
            .unwrap_err(),
            RawValidationError::InvalidReadinessRange
        );
        let ready = transition::demosaic_ready_from_normalized(
            normalized,
            ImageBuffer::from_raw(2, 2, vec![0.75; 4]).unwrap(),
        )
        .unwrap();
        assert_eq!(ready.stage_kind(), StageKind::DemosaicReady);
        assert_eq!(ready.metadata().cfa().pattern(), BayerPattern::Gbrg);
    }

    #[test]
    fn replaced_levels_are_structurally_valid_and_accepted() {
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
        let operation =
            OperationProvenance::try_new("replace_levels", ProviderId::try_new("test").unwrap())
                .unwrap();
        let image = decoded()
            .convert_to_f32()
            .unwrap()
            .with_custom_f32_plane(
                ImageBuffer::from_raw(2, 2, vec![0.5; 4]).unwrap(),
                operation,
                basis,
            )
            .unwrap();
        assert_eq!(image.require_standard_preprocessing(), Ok(()));
    }

    #[test]
    fn decoded_constructor_rejects_wrong_dimensions_and_encoding() {
        let image = decoded();
        let metadata = image.metadata().clone();
        let decoder = DecoderProvenance::new(ProviderId::try_new("test").unwrap(), None);
        assert_eq!(
            RawImage::<Decoded>::try_new(
                metadata.clone(),
                StoredPlane::U16(U16Plane::from_raw(1, 1, vec![1]).unwrap()),
                decoder.clone(),
            )
            .unwrap_err(),
            RawValidationError::InvalidGeometry
        );
        assert_eq!(
            RawImage::<Decoded>::try_new(
                metadata,
                StoredPlane::U8(crate::U8Plane::from_raw(2, 2, vec![1; 4]).unwrap()),
                decoder,
            )
            .unwrap_err(),
            RawValidationError::UnsupportedSampleEncoding {
                bits_per_sample: 16,
                format: SampleFormat::UnsignedInteger,
            }
        );
    }
}
