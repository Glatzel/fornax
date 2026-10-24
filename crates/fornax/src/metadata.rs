use std::collections::BTreeMap;
use std::num::NonZeroU32;

use crate::{
    ActiveArea, BayerLayout, PixelSize, RawValidationError, SampleSpec, StoredGeometry, StoredPlane,
};

/// A finite rational metadata value.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Rational {
    numerator: i64,
    denominator: NonZeroU32,
}

impl Rational {
    /// Creates a rational value with a non-zero denominator.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use fornax::Rational;
    ///
    /// let value = Rational::new(1, NonZeroU32::new(2).unwrap());
    /// assert_eq!(value.as_f64(), 0.5);
    /// ```
    pub const fn new(numerator: i64, denominator: NonZeroU32) -> Self {
        Self {
            numerator,
            denominator,
        }
    }

    /// Returns the numerator.
    pub const fn numerator(self) -> i64 { self.numerator }

    /// Returns the non-zero denominator.
    pub const fn denominator(self) -> NonZeroU32 { self.denominator }

    /// Converts the exact components to `f64` for scalar preprocessing.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use fornax::Rational;
    ///
    /// assert_eq!(Rational::new(3, NonZeroU32::new(4).unwrap()).as_f64(), 0.75);
    /// ```
    pub fn as_f64(self) -> f64 { self.numerator as f64 / f64::from(self.denominator.get()) }

    fn is_nonnegative(self) -> bool { self.numerator >= 0 }
}

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
    /// use fornax::{DefaultCrop, Rational};
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
    ) -> Result<Self, crate::MetadataError> {
        if width.as_f64() <= 0.0 || height.as_f64() <= 0.0 {
            return Err(crate::MetadataError::InvalidValue);
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
    /// use fornax::Orientation;
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
    /// use fornax::{Rational, WhiteBalance};
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
    pub fn try_new(camera_neutral: Vec<Rational>) -> Result<Self, crate::MetadataError> {
        if camera_neutral.len() != 3 || camera_neutral.iter().any(|value| value.as_f64() <= 0.0) {
            return Err(crate::MetadataError::InvalidValue);
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

/// A validated provider identity for provenance.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ProviderId(Box<str>);

impl ProviderId {
    /// Creates a non-empty provider identity.
    ///
    /// # Errors
    ///
    /// Returns [`crate::MetadataError::InvalidValue`] for an empty identifier.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::ProviderId;
    ///
    /// assert_eq!(ProviderId::try_new("libraw").unwrap().as_str(), "libraw");
    /// assert!(ProviderId::try_new("").is_err());
    /// ```
    pub fn try_new(value: impl Into<Box<str>>) -> Result<Self, crate::MetadataError> {
        let value = value.into();
        if value.is_empty() {
            Err(crate::MetadataError::InvalidValue)
        } else {
            Ok(Self(value))
        }
    }

    /// Returns the provider identifier.
    pub fn as_str(&self) -> &str { &self.0 }
}

/// Decoder identity retained as part of validated image provenance.
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
    /// use fornax::{DecoderProvenance, ProviderId};
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

/// The source mechanism that produced a metadata value.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum SourceKind {
    /// A typed file tag.
    FileTag {
        /// Numeric format tag.
        tag: u32,
        /// Source path supporting the tag interpretation.
        path: Box<str>,
    },
    /// A decoder observation.
    Decoder {
        /// Decoder backend identifier.
        backend: Box<str>,
        /// Optional decoder revision.
        revision: Option<Box<str>>,
        /// Decoder field or API path.
        path: Box<str>,
    },
    /// A camera catalog fallback.
    CameraCatalog {
        /// Catalog name.
        catalog: Box<str>,
        /// Optional catalog revision.
        revision: Option<Box<str>>,
    },
    /// A specified format default.
    FormatDefault {
        /// Specification clause or tag default.
        clause: Box<str>,
    },
    /// A value derived by a named operation.
    Derived {
        /// Operation name.
        operation: Box<str>,
    },
    /// An explicit user override.
    UserOverride,
}

/// Metadata authority from strongest to weakest.
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum AuthorityClass {
    /// Explicit user override.
    UserOverride,
    /// Normative file declaration.
    FormatDeclaration,
    /// Source-verified decoder observation.
    DecoderObservation,
    /// Camera catalog fallback.
    CameraCatalogFallback,
    /// Format default.
    FormatDefault,
    /// Derived or measured value.
    DerivedValue,
}

impl AuthorityClass {
    /// Returns the precedence rank, where zero is strongest.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::AuthorityClass;
    ///
    /// assert!(
    ///     AuthorityClass::FormatDeclaration.rank() < AuthorityClass::CameraCatalogFallback.rank()
    /// );
    /// ```
    pub const fn rank(self) -> u8 {
        match self {
            Self::UserOverride => 0,
            Self::FormatDeclaration => 1,
            Self::DecoderObservation => 2,
            Self::CameraCatalogFallback => 3,
            Self::FormatDefault => 4,
            Self::DerivedValue => 5,
        }
    }
}

/// Confidence in a metadata interpretation.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum Confidence {
    /// Source semantics are exact.
    Exact,
    /// Source semantics are inferred from verified behavior.
    Inferred,
    /// Source semantics are unavailable.
    Unknown,
}

/// Source identity, authority, and confidence for a metadata value.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Provenance {
    provider: ProviderId,
    source: SourceKind,
    authority: AuthorityClass,
    confidence: Confidence,
}

impl Provenance {
    /// Creates provenance for a metadata value.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::{AuthorityClass, Confidence, Provenance, ProviderId, SourceKind};
    ///
    /// let provenance = Provenance::new(
    ///     ProviderId::try_new("dnglab").unwrap(),
    ///     SourceKind::FormatDefault {
    ///         clause: "DNG 1.7".into(),
    ///     },
    ///     AuthorityClass::FormatDefault,
    ///     Confidence::Exact,
    /// );
    /// assert_eq!(provenance.authority(), AuthorityClass::FormatDefault);
    /// ```
    pub const fn new(
        provider: ProviderId,
        source: SourceKind,
        authority: AuthorityClass,
        confidence: Confidence,
    ) -> Self {
        Self {
            provider,
            source,
            authority,
            confidence,
        }
    }

    /// Returns the provider identity.
    pub const fn provider(&self) -> &ProviderId { &self.provider }

    /// Returns the source mechanism.
    pub const fn source(&self) -> &SourceKind { &self.source }

    /// Returns the authority class.
    pub const fn authority(&self) -> AuthorityClass { self.authority }

    /// Returns interpretation confidence.
    pub const fn confidence(&self) -> Confidence { self.confidence }
}

/// A typed metadata value that retains its provenance.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct Sourced<T> {
    /// The selected value.
    pub value: T,
    /// Where and how the value was obtained.
    pub provenance: Provenance,
}

impl<T> Sourced<T> {
    /// Creates a sourced value.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::{AuthorityClass, Confidence, Provenance, ProviderId, SourceKind, Sourced};
    ///
    /// let provenance = Provenance::new(
    ///     ProviderId::try_new("decoder").unwrap(),
    ///     SourceKind::Derived {
    ///         operation: "unpack".into(),
    ///     },
    ///     AuthorityClass::DecoderObservation,
    ///     Confidence::Exact,
    /// );
    /// let value = Sourced::new(12_u8, provenance);
    /// assert_eq!(value.value, 12);
    /// ```
    pub const fn new(value: T, provenance: Provenance) -> Self { Self { value, provenance } }
}

/// Selects the authoritative candidate or returns a same-authority conflict.
///
/// # Errors
///
/// Returns [`crate::MetadataError::Conflict`] when candidates at the same
/// authority disagree.
///
/// # Examples
///
/// ```
/// use fornax::{
///     AuthorityClass, Confidence, MetadataField, Provenance, ProviderId, SourceKind, Sourced,
///     TypedMetadataField, resolve_sourced,
/// };
///
/// let decoder = SourceKind::Decoder {
///     backend: "dnglab".into(),
///     revision: None,
///     path: "active_area".into(),
/// };
/// let declaration = SourceKind::FormatDefault {
///     clause: "ActiveArea".into(),
/// };
/// let candidate = |value, source, authority| {
///     Sourced::new(
///         value,
///         Provenance::new(
///             ProviderId::try_new("test").unwrap(),
///             source,
///             authority,
///             Confidence::Exact,
///         ),
///     )
/// };
/// let selected = resolve_sourced(
///     MetadataField::Typed(TypedMetadataField::ActiveArea),
///     candidate(1, decoder, AuthorityClass::DecoderObservation),
///     candidate(2, declaration, AuthorityClass::FormatDefault),
/// )
/// .unwrap();
/// assert_eq!(selected.value, 1);
/// ```
pub fn resolve_sourced<T: PartialEq>(
    field: MetadataField,
    left: Sourced<T>,
    right: Sourced<T>,
) -> Result<Sourced<T>, crate::MetadataError> {
    let left_rank = left.provenance.authority().rank();
    let right_rank = right.provenance.authority().rank();
    match left_rank.cmp(&right_rank) {
        std::cmp::Ordering::Less => Ok(left),
        std::cmp::Ordering::Greater => Ok(right),
        std::cmp::Ordering::Equal if left.value == right.value => Ok(left),
        std::cmp::Ordering::Equal => Err(crate::MetadataError::Conflict(field)),
    }
}

/// A namespaced key for extensible metadata.
#[derive(Clone, Debug, Eq, Ord, PartialEq, PartialOrd)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MetadataKey(Box<str>);

impl MetadataKey {
    /// Parses a lowercase, dot-separated metadata key with at least two
    /// segments in a registered top-level namespace.
    ///
    /// # Errors
    ///
    /// Returns [`crate::MetadataError::InvalidKey`] for an unregistered
    /// namespace or non-canonical segments.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::MetadataKey;
    ///
    /// assert_eq!(
    ///     MetadataKey::try_new("dng.tag_50740").unwrap().as_str(),
    ///     "dng.tag_50740"
    /// );
    /// assert!(MetadataKey::try_new("DNG.tag").is_err());
    /// ```
    pub fn try_new(value: impl Into<Box<str>>) -> Result<Self, crate::MetadataError> {
        let value = value.into();
        let mut segments = value.split('.');
        let Some(namespace) = segments.next() else {
            return Err(crate::MetadataError::InvalidKey(value));
        };
        let Some(first_path) = segments.next() else {
            return Err(crate::MetadataError::InvalidKey(value));
        };
        let valid = matches!(
            namespace,
            "dng" | "tiff" | "capture" | "vendor" | "decoder" | "raw"
        ) && valid_key_segment(namespace)
            && valid_key_segment(first_path)
            && segments.all(valid_key_segment);
        if valid {
            Ok(Self(value))
        } else {
            Err(crate::MetadataError::InvalidKey(value))
        }
    }

    /// Returns the canonical key text.
    pub fn as_str(&self) -> &str { &self.0 }
}

fn valid_key_segment(segment: &str) -> bool {
    let mut bytes = segment.bytes();
    matches!(bytes.next(), Some(b'a'..=b'z'))
        && bytes.all(|byte| byte.is_ascii_lowercase() || byte.is_ascii_digit() || byte == b'_')
}

/// A value retained in the extensible metadata layer.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum MetadataValue {
    /// Boolean value.
    Bool(bool),
    /// Signed integer value.
    I64(i64),
    /// Unsigned integer value.
    U64(u64),
    /// Floating-point value.
    F64(f64),
    /// Exact rational value.
    Rational(Rational),
    /// UTF-8 text.
    Text(Box<str>),
    /// Original bytes.
    Bytes(Box<[u8]>),
    /// Signed integer array.
    I64Array(Box<[i64]>),
    /// Unsigned integer array.
    U64Array(Box<[u64]>),
    /// Floating-point array.
    F64Array(Box<[f64]>),
    /// Exact rational array.
    RationalArray(Box<[Rational]>),
}

impl MetadataValue {
    fn is_finite(&self) -> bool {
        match self {
            Self::F64(value) => value.is_finite(),
            Self::F64Array(values) => values.iter().all(|value| value.is_finite()),
            Self::Bool(_)
            | Self::I64(_)
            | Self::U64(_)
            | Self::Rational(_)
            | Self::Text(_)
            | Self::Bytes(_)
            | Self::I64Array(_)
            | Self::U64Array(_)
            | Self::RationalArray(_) => true,
        }
    }

    fn allocation_bytes(&self) -> usize {
        match self {
            Self::Text(value) => value.len(),
            Self::Bytes(value) => value.len(),
            Self::I64Array(values) => values.len().saturating_mul(size_of::<i64>()),
            Self::U64Array(values) => values.len().saturating_mul(size_of::<u64>()),
            Self::F64Array(values) => values.len().saturating_mul(size_of::<f64>()),
            Self::RationalArray(values) => values.len().saturating_mul(size_of::<Rational>()),
            Self::Bool(_) | Self::I64(_) | Self::U64(_) | Self::F64(_) | Self::Rational(_) => 0,
        }
    }
}

/// The physical unit of an extensible metadata value.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum MetadataUnit {
    /// Pixels.
    Pixel,
    /// Bytes.
    Byte,
    /// Bits.
    Bit,
    /// Stored RAW sample codes.
    StoredSampleCode,
    /// Normalized dimensionless values.
    Normalized,
    /// Dimensionless values.
    Dimensionless,
    /// Seconds.
    Second,
    /// Kelvin.
    Kelvin,
    /// Another explicit unit.
    Other(Box<str>),
}

/// The coordinate space of an extensible metadata value.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum CoordinateSpace {
    /// Stored sensor coordinates.
    StoredImage,
    /// ActiveArea-relative coordinates.
    ActiveArea,
    /// Default-crop-relative coordinates.
    DefaultCrop,
    /// CFA repeat-tile coordinates.
    CfaTile,
    /// Camera RGB coordinates.
    CameraRgb,
    /// XYZ coordinates adapted to D50.
    XyzD50,
    /// No coordinate meaning.
    None,
    /// Another explicit coordinate space.
    Other(Box<str>),
}

/// Original source bytes and type for an unknown field.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct OriginalValue {
    /// Original bytes.
    pub raw: Box<[u8]>,
    /// Source type name.
    pub source_type: Option<Box<str>>,
    /// Declared element count.
    pub count: Option<u32>,
}

impl OriginalValue {
    /// Creates an original-value record.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::OriginalValue;
    ///
    /// let original = OriginalValue::new(vec![1, 2], Some("SHORT".into()), Some(1));
    /// assert_eq!(original.raw.len(), 2);
    /// ```
    pub fn new(raw: Vec<u8>, source_type: Option<Box<str>>, count: Option<u32>) -> Self {
        Self {
            raw: raw.into_boxed_slice(),
            source_type,
            count,
        }
    }
}

/// One namespaced extensible metadata entry.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MetadataEntry {
    /// Canonical namespaced key.
    pub key: MetadataKey,
    /// Selected value.
    pub value: MetadataValue,
    /// Optional physical unit.
    pub unit: Option<MetadataUnit>,
    /// Optional coordinate space.
    pub coordinate_space: Option<CoordinateSpace>,
    /// Value provenance.
    pub provenance: Provenance,
    /// Original source data when available.
    pub original: Option<OriginalValue>,
}

impl MetadataEntry {
    fn validate(&self) -> Result<(), crate::MetadataError> {
        if !self.value.is_finite() {
            return Err(crate::MetadataError::InvalidValue);
        }
        let total = self
            .value
            .allocation_bytes()
            .saturating_add(
                self.original
                    .as_ref()
                    .map_or(0, |original| original.raw.len()),
            )
            .saturating_add(
                self.original
                    .as_ref()
                    .and_then(|original| original.source_type.as_ref())
                    .map_or(0, |source_type| source_type.len()),
            );
        if total > 16 * 1024 * 1024 {
            Err(crate::MetadataError::AllocationLimit)
        } else {
            Ok(())
        }
    }
}

/// A typed metadata concept that may be duplicated or conflicted.
#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum TypedMetadataField {
    /// Camera identity.
    Camera,
    /// Sample precision and format.
    SampleSpec,
    /// Stored dimensions and stride.
    StoredGeometry,
    /// ActiveArea.
    ActiveArea,
    /// Default crop.
    DefaultCrop,
    /// CFA layout.
    Cfa,
    /// Black levels.
    Black,
    /// White levels.
    White,
    /// Orientation.
    Orientation,
    /// White balance.
    WhiteBalance,
    /// Color matrices.
    ColorMatrices,
    /// Capture metadata.
    Capture,
}

/// A typed or extensible metadata identity.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum MetadataField {
    /// A typed field.
    Typed(TypedMetadataField),
    /// An extensible entry.
    Extensible(MetadataKey),
}

/// A retained optional metadata conflict.
#[derive(Clone, Debug, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct MetadataConflict {
    /// Field whose authority was resolved.
    pub field: MetadataField,
    /// Provenance of the selected candidate.
    pub selected: Provenance,
    /// Rejected values and their provenance.
    pub rejected: Vec<(MetadataValue, Provenance)>,
}

/// Namespaced metadata that cannot redefine typed algorithm-critical fields.
#[derive(Clone, Debug, Default, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct ExtensibleMetadata {
    entries: BTreeMap<MetadataKey, MetadataEntry>,
    conflicts: Vec<MetadataConflict>,
}

impl ExtensibleMetadata {
    /// Inserts a metadata entry.
    ///
    /// # Errors
    ///
    /// Returns a typed error for invalid values, duplicate keys, or keys that
    /// redefine typed fields.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::{
    ///     AuthorityClass, Confidence, ExtensibleMetadata, MetadataEntry, MetadataKey, MetadataValue,
    ///     Provenance, ProviderId, SourceKind,
    /// };
    ///
    /// let key = MetadataKey::try_new("dng.tag_50740").unwrap();
    /// let entry = MetadataEntry {
    ///     key: key.clone(),
    ///     value: MetadataValue::U64(1),
    ///     unit: None,
    ///     coordinate_space: None,
    ///     provenance: Provenance::new(
    ///         ProviderId::try_new("dnglab").unwrap(),
    ///         SourceKind::UserOverride,
    ///         AuthorityClass::UserOverride,
    ///         Confidence::Exact,
    ///     ),
    ///     original: None,
    /// };
    /// let mut metadata = ExtensibleMetadata::default();
    /// metadata.insert(entry).unwrap();
    /// assert!(metadata.get(&key).is_some());
    /// ```
    pub fn insert(&mut self, entry: MetadataEntry) -> Result<(), crate::MetadataError> {
        if is_reserved_typed_key(entry.key.as_str()) {
            return Err(crate::MetadataError::DuplicateTypedField(
                entry.key.as_str().into(),
            ));
        }
        entry.validate()?;
        if self.entries.contains_key(&entry.key) {
            return Err(crate::MetadataError::Conflict(MetadataField::Extensible(
                entry.key,
            )));
        }
        self.entries.insert(entry.key.clone(), entry);
        Ok(())
    }

    /// Looks up an extensible entry.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::{ExtensibleMetadata, MetadataKey};
    ///
    /// let metadata = ExtensibleMetadata::default();
    /// assert!(
    ///     metadata
    ///         .get(&MetadataKey::try_new("vendor.unknown").unwrap())
    ///         .is_none()
    /// );
    /// ```
    pub fn get(&self, key: &MetadataKey) -> Option<&MetadataEntry> { self.entries.get(key) }

    /// Iterates entries in deterministic key order.
    pub fn entries(&self) -> impl Iterator<Item = (&MetadataKey, &MetadataEntry)> {
        self.entries.iter()
    }

    /// Returns retained optional conflicts.
    pub fn conflicts(&self) -> &[MetadataConflict] { &self.conflicts }

    /// Records a resolved optional metadata conflict for diagnostics.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax::{
    ///     AuthorityClass, Confidence, ExtensibleMetadata, MetadataConflict, MetadataField,
    ///     Provenance, ProviderId, SourceKind, TypedMetadataField,
    /// };
    ///
    /// let mut metadata = ExtensibleMetadata::default();
    /// metadata.record_conflict(MetadataConflict {
    ///     field: MetadataField::Typed(TypedMetadataField::White),
    ///     selected: Provenance::new(
    ///         ProviderId::try_new("dnglab").unwrap(),
    ///         SourceKind::FormatDefault {
    ///             clause: "DNG".into(),
    ///         },
    ///         AuthorityClass::FormatDefault,
    ///         Confidence::Exact,
    ///     ),
    ///     rejected: Vec::new(),
    /// });
    /// assert_eq!(metadata.conflicts().len(), 1);
    /// ```
    pub fn record_conflict(&mut self, conflict: MetadataConflict) { self.conflicts.push(conflict); }

    fn validate(&self) -> Result<(), crate::MetadataError> {
        for (key, entry) in &self.entries {
            if key != &entry.key {
                return Err(crate::MetadataError::InvalidValue);
            }
            if is_reserved_typed_key(key.as_str()) {
                return Err(crate::MetadataError::DuplicateTypedField(
                    key.as_str().into(),
                ));
            }
            entry.validate()?;
        }
        Ok(())
    }
}

fn is_reserved_typed_key(key: &str) -> bool {
    matches!(
        key,
        "raw.camera"
            | "raw.sample_spec"
            | "raw.stored_geometry"
            | "raw.active_area"
            | "raw.default_crop"
            | "raw.cfa"
            | "raw.black"
            | "raw.white"
            | "raw.orientation"
            | "raw.white_balance"
            | "raw.color_matrices"
            | "raw.capture"
    )
}

/// A finite repeating black-level tile in stored-sample units.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct BlackTile {
    rows: NonZeroU32,
    columns: NonZeroU32,
    samples_per_plane: NonZeroU32,
    values: Box<[Rational]>,
}

impl BlackTile {
    /// Creates a black-level tile with an exact value count.
    ///
    /// # Errors
    ///
    /// Returns [`crate::MetadataError::InvalidLevels`] for negative values or
    /// when `values` does not contain `rows * columns * samples_per_plane`
    /// entries.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use fornax::{BlackTile, Rational};
    ///
    /// let one = NonZeroU32::new(1).unwrap();
    /// let black = Rational::new(64, one);
    /// assert!(BlackTile::try_new(one, one, one, vec![black]).is_ok());
    /// ```
    pub fn try_new(
        rows: NonZeroU32,
        columns: NonZeroU32,
        samples_per_plane: NonZeroU32,
        values: Vec<Rational>,
    ) -> Result<Self, crate::MetadataError> {
        let expected = u64::from(rows.get())
            .checked_mul(u64::from(columns.get()))
            .and_then(|count| count.checked_mul(u64::from(samples_per_plane.get())))
            .ok_or(crate::MetadataError::InvalidLevels)?;
        if u64::try_from(values.len()) != Ok(expected)
            || values.iter().any(|value| !value.is_nonnegative())
        {
            return Err(crate::MetadataError::InvalidLevels);
        }
        Ok(Self {
            rows,
            columns,
            samples_per_plane,
            values: values.into_boxed_slice(),
        })
    }

    /// Returns the repeat row count.
    pub const fn rows(&self) -> NonZeroU32 { self.rows }

    /// Returns the repeat column count.
    pub const fn columns(&self) -> NonZeroU32 { self.columns }

    /// Returns the sample-plane count per tile position.
    pub const fn samples_per_plane(&self) -> NonZeroU32 { self.samples_per_plane }

    /// Returns tile values in row-major order.
    pub fn values(&self) -> &[Rational] { &self.values }
}

/// Scalar, four-slot, and repeating black-level metadata.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct BlackLevels {
    common: Rational,
    per_slot: [Rational; 4],
    repeat: Option<BlackTile>,
}

impl BlackLevels {
    /// Creates black levels in stored-sample units.
    ///
    /// # Errors
    ///
    /// Returns [`crate::MetadataError::InvalidLevels`] when any scalar is
    /// negative.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use fornax::{BlackLevels, Rational};
    ///
    /// let black = Rational::new(64, NonZeroU32::new(1).unwrap());
    /// assert_eq!(
    ///     BlackLevels::try_new(black, [black; 4], None)
    ///         .unwrap()
    ///         .common(),
    ///     black
    /// );
    /// ```
    pub fn try_new(
        common: Rational,
        per_slot: [Rational; 4],
        repeat: Option<BlackTile>,
    ) -> Result<Self, crate::MetadataError> {
        if !common.is_nonnegative() || per_slot.iter().any(|value| !value.is_nonnegative()) {
            return Err(crate::MetadataError::InvalidLevels);
        }
        Ok(Self {
            common,
            per_slot,
            repeat,
        })
    }

    /// Returns the scalar common component.
    pub const fn common(&self) -> Rational { self.common }

    /// Returns the four CFA-slot components.
    pub const fn per_slot(&self) -> &[Rational; 4] { &self.per_slot }

    /// Returns an optional repeating tile.
    pub const fn repeat(&self) -> Option<&BlackTile> { self.repeat.as_ref() }

    fn validate(&self) -> Result<(), crate::MetadataError> {
        if !self.common.is_nonnegative()
            || self.per_slot.iter().any(|value| !value.is_nonnegative())
        {
            Err(crate::MetadataError::InvalidLevels)
        } else {
            Ok(())
        }
    }

    fn repeat_maximum(&self) -> f64 {
        self.repeat.as_ref().map_or(0.0, |tile| {
            tile.values
                .iter()
                .map(|value| value.as_f64())
                .fold(0.0_f64, f64::max)
        })
    }

    fn effective_slot(&self, slot: usize) -> f64 {
        self.common.as_f64() + self.per_slot[slot].as_f64() + self.repeat_maximum()
    }

    fn effective_maximum(&self) -> f64 {
        (0..4)
            .map(|slot| self.effective_slot(slot))
            .fold(0.0_f64, f64::max)
    }
}

/// Declared, decoder-adjusted, and measured white levels.
#[derive(Clone, Debug, Eq, PartialEq)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub struct WhiteLevels {
    declared: Box<[Rational]>,
    adjusted_maximum: Option<Box<[Rational]>>,
    measured_maximum: Option<Box<[Rational]>>,
}

impl WhiteLevels {
    /// Creates white levels with one scalar or four CFA-slot values.
    ///
    /// # Errors
    ///
    /// Returns [`crate::MetadataError::InvalidLevels`] for unsupported lengths,
    /// non-positive values, or mismatched optional arrays.
    ///
    /// # Examples
    ///
    /// ```
    /// use std::num::NonZeroU32;
    ///
    /// use fornax::{Rational, WhiteLevels};
    ///
    /// let white = Rational::new(4095, NonZeroU32::new(1).unwrap());
    /// assert_eq!(
    ///     WhiteLevels::try_new(vec![white], None, None)
    ///         .unwrap()
    ///         .declared(),
    ///     &[white]
    /// );
    /// ```
    pub fn try_new(
        declared: Vec<Rational>,
        adjusted_maximum: Option<Vec<Rational>>,
        measured_maximum: Option<Vec<Rational>>,
    ) -> Result<Self, crate::MetadataError> {
        let valid_primary =
            matches!(declared.len(), 1 | 4) && declared.iter().all(|value| value.as_f64() > 0.0);
        let positive_adjusted = adjusted_maximum.as_ref().is_none_or(|values| {
            values.len() == declared.len() && values.iter().all(|value| value.as_f64() > 0.0)
        });
        let positive_measured = measured_maximum.as_ref().is_none_or(|values| {
            values.len() == declared.len() && values.iter().all(|value| value.as_f64() > 0.0)
        });
        if !valid_primary || !positive_adjusted || !positive_measured {
            return Err(crate::MetadataError::InvalidLevels);
        }
        Ok(Self {
            declared: declared.into_boxed_slice(),
            adjusted_maximum: adjusted_maximum.map(Vec::into_boxed_slice),
            measured_maximum: measured_maximum.map(Vec::into_boxed_slice),
        })
    }

    /// Returns source-declared white levels.
    pub fn declared(&self) -> &[Rational] { &self.declared }

    /// Returns decoder-adjusted maximum values.
    pub fn adjusted_maximum(&self) -> Option<&[Rational]> { self.adjusted_maximum.as_deref() }

    /// Returns measured maximum values.
    pub fn measured_maximum(&self) -> Option<&[Rational]> { self.measured_maximum.as_deref() }

    fn validate(&self) -> Result<(), crate::MetadataError> {
        let primary_valid = matches!(self.declared.len(), 1 | 4)
            && self.declared.iter().all(|value| value.as_f64() > 0.0);
        let adjusted_valid = self.adjusted_maximum.as_ref().is_none_or(|values| {
            values.len() == self.declared.len() && values.iter().all(|value| value.as_f64() > 0.0)
        });
        let measured_valid = self.measured_maximum.as_ref().is_none_or(|values| {
            values.len() == self.declared.len() && values.iter().all(|value| value.as_f64() > 0.0)
        });
        if primary_valid && adjusted_valid && measured_valid {
            Ok(())
        } else {
            Err(crate::MetadataError::InvalidLevels)
        }
    }
}

fn validate_level_relationship(
    black: &BlackLevels,
    white: &WhiteLevels,
) -> Result<(), RawValidationError> {
    let valid = match white.declared.as_ref() {
        [scalar] => scalar.as_f64() > black.effective_maximum(),
        values if values.len() == 4 => values
            .iter()
            .enumerate()
            .all(|(slot, value)| value.as_f64() > black.effective_slot(slot)),
        _ => false,
    };
    if valid {
        Ok(())
    } else {
        Err(RawValidationError::Metadata(
            crate::MetadataError::InvalidLevels,
        ))
    }
}

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
    /// use fornax::{
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
    /// # use fornax::{
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
        crate::plane::validate_finite(plane)
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::SampleFormat;

    fn provenance(authority: AuthorityClass) -> Provenance {
        Provenance::new(
            ProviderId::try_new("test").unwrap(),
            SourceKind::Derived {
                operation: "fixture".into(),
            },
            authority,
            Confidence::Exact,
        )
    }

    fn decoder_provenance() -> DecoderProvenance {
        DecoderProvenance::new(ProviderId::try_new("test").unwrap(), None)
    }

    #[test]
    fn metadata_keys_follow_the_canonical_grammar() {
        for valid in [
            "dng.tag_50740",
            "vendor.sony.white_balance_note",
            "capture.as_shot",
        ] {
            assert!(MetadataKey::try_new(valid).is_ok(), "{valid}");
        }
        for invalid in [
            "",
            "key",
            "unknown.tag",
            "Dng.tag",
            "dng..tag",
            "dng.Tag",
            "dng.tag.value!",
        ] {
            assert!(MetadataKey::try_new(invalid).is_err(), "{invalid}");
        }
    }

    #[test]
    fn extensible_metadata_is_ordered_and_rejects_duplicates() {
        let entry = |key: &str| MetadataEntry {
            key: MetadataKey::try_new(key).unwrap(),
            value: MetadataValue::U64(1),
            unit: None,
            coordinate_space: None,
            provenance: provenance(AuthorityClass::DecoderObservation),
            original: None,
        };
        let mut metadata = ExtensibleMetadata::default();
        metadata.insert(entry("vendor.z")).unwrap();
        metadata.insert(entry("dng.a")).unwrap();
        assert_eq!(
            metadata
                .entries()
                .map(|(key, _)| key.as_str())
                .collect::<Vec<_>>(),
            ["dng.a", "vendor.z"]
        );
        assert!(metadata.insert(entry("vendor.z")).is_err());
    }

    #[test]
    fn typed_concepts_cannot_be_redefined_in_the_extension_map() {
        let mut metadata = ExtensibleMetadata::default();
        let entry = MetadataEntry {
            key: MetadataKey::try_new("raw.black").unwrap(),
            value: MetadataValue::U64(0),
            unit: None,
            coordinate_space: None,
            provenance: provenance(AuthorityClass::UserOverride),
            original: None,
        };
        assert!(matches!(
            metadata.insert(entry),
            Err(crate::MetadataError::DuplicateTypedField(_))
        ));
    }

    #[test]
    fn same_authority_candidates_conflict_but_higher_authority_wins() {
        let field = MetadataField::Typed(TypedMetadataField::Black);
        assert_eq!(
            resolve_sourced(
                field.clone(),
                Sourced::new(1, provenance(AuthorityClass::DecoderObservation)),
                Sourced::new(2, provenance(AuthorityClass::DecoderObservation)),
            ),
            Err(crate::MetadataError::Conflict(field.clone()))
        );
        assert_eq!(
            resolve_sourced(
                field,
                Sourced::new(1, provenance(AuthorityClass::DecoderObservation)),
                Sourced::new(2, provenance(AuthorityClass::FormatDefault)),
            )
            .unwrap()
            .value,
            1
        );
    }

    #[test]
    fn levels_enforce_shape_and_value_invariants() {
        let one = NonZeroU32::new(1).unwrap();
        let black = Rational::new(64, one);
        assert!(BlackLevels::try_new(black, [black; 4], None).is_ok());
        assert!(BlackTile::try_new(one, NonZeroU32::new(2).unwrap(), one, vec![black]).is_err());
        assert!(WhiteLevels::try_new(vec![black], None, None).is_ok());
        assert!(WhiteLevels::try_new(vec![black; 2], None, None).is_err());
    }

    #[test]
    fn validated_metadata_enforces_required_and_cross_field_invariants() {
        use std::num::NonZeroU64;

        let dimensions = PixelSize::new(8, 8).unwrap();
        let active_area = ActiveArea::try_new(dimensions, 1, 2, 8, 8).unwrap();
        let different_area = ActiveArea::try_new(dimensions, 0, 0, 8, 8).unwrap();
        let geometry = StoredGeometry::new(dimensions, NonZeroU64::new(16).unwrap());
        let spec = SampleSpec::try_new(16, SampleFormat::UnsignedInteger).unwrap();
        let zero = Rational::new(0, NonZeroU32::new(1).unwrap());
        let white = Rational::new(65_535, NonZeroU32::new(1).unwrap());
        let common = CommonMetadata {
            sample_spec: Some(Sourced::new(
                spec,
                provenance(AuthorityClass::DecoderObservation),
            )),
            stored_geometry: Some(Sourced::new(
                geometry,
                provenance(AuthorityClass::DecoderObservation),
            )),
            active_area: Some(Sourced::new(
                active_area,
                provenance(AuthorityClass::DecoderObservation),
            )),
            cfa: Some(Sourced::new(
                BayerLayout::new(crate::BayerPattern::Rggb, different_area),
                provenance(AuthorityClass::DecoderObservation),
            )),
            black: Some(Sourced::new(
                BlackLevels::try_new(zero, [zero; 4], None).unwrap(),
                provenance(AuthorityClass::DecoderObservation),
            )),
            white: Some(Sourced::new(
                WhiteLevels::try_new(vec![white], None, None).unwrap(),
                provenance(AuthorityClass::DecoderObservation),
            )),
            ..CommonMetadata::default()
        };
        let missing_black = CommonMetadata {
            black: None,
            ..common.clone()
        };
        assert_eq!(
            ValidatedMetadata::try_new(
                missing_black,
                ExtensibleMetadata::default(),
                decoder_provenance()
            ),
            Err(RawValidationError::Metadata(
                crate::MetadataError::MissingRequiredField(TypedMetadataField::Black)
            ))
        );
        let missing_white = CommonMetadata {
            white: None,
            ..common.clone()
        };
        assert_eq!(
            ValidatedMetadata::try_new(
                missing_white,
                ExtensibleMetadata::default(),
                decoder_provenance()
            ),
            Err(RawValidationError::Metadata(
                crate::MetadataError::MissingRequiredField(TypedMetadataField::White)
            ))
        );
        assert_eq!(
            ValidatedMetadata::try_new(common, ExtensibleMetadata::default(), decoder_provenance()),
            Err(RawValidationError::InvalidCfa)
        );
        assert_eq!(
            ValidatedMetadata::try_new(
                CommonMetadata::default(),
                ExtensibleMetadata::default(),
                decoder_provenance()
            ),
            Err(RawValidationError::Metadata(
                crate::MetadataError::MissingRequiredField(TypedMetadataField::SampleSpec)
            ))
        );
    }

    #[test]
    fn validated_metadata_rejects_invalid_black_white_range() {
        use std::num::NonZeroU64;

        let dimensions = PixelSize::new(2, 2).unwrap();
        let active_area = ActiveArea::try_new(dimensions, 0, 0, 2, 2).unwrap();
        let black = Rational::new(100, NonZeroU32::new(1).unwrap());
        let white = Rational::new(50, NonZeroU32::new(1).unwrap());
        let common = CommonMetadata {
            sample_spec: Some(Sourced::new(
                SampleSpec::try_new(16, SampleFormat::UnsignedInteger).unwrap(),
                provenance(AuthorityClass::DecoderObservation),
            )),
            stored_geometry: Some(Sourced::new(
                StoredGeometry::new(dimensions, NonZeroU64::new(4).unwrap()),
                provenance(AuthorityClass::DecoderObservation),
            )),
            active_area: Some(Sourced::new(
                active_area,
                provenance(AuthorityClass::DecoderObservation),
            )),
            cfa: Some(Sourced::new(
                BayerLayout::new(crate::BayerPattern::Rggb, active_area),
                provenance(AuthorityClass::DecoderObservation),
            )),
            black: Some(Sourced::new(
                BlackLevels::try_new(black, [black; 4], None).unwrap(),
                provenance(AuthorityClass::DecoderObservation),
            )),
            white: Some(Sourced::new(
                WhiteLevels::try_new(vec![white], None, None).unwrap(),
                provenance(AuthorityClass::DecoderObservation),
            )),
            ..CommonMetadata::default()
        };
        assert_eq!(
            ValidatedMetadata::try_new(common, ExtensibleMetadata::default(), decoder_provenance()),
            Err(RawValidationError::Metadata(
                crate::MetadataError::InvalidLevels
            ))
        );
    }
}
