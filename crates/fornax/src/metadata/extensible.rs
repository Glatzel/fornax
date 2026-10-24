//! Extensible metadata fields, values, units, and conflict records.

use std::collections::BTreeMap;

use super::provenance::Provenance;
use super::value::Rational;
use crate::error::MetadataError;

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
    /// use fornax::metadata::MetadataKey;
    ///
    /// assert_eq!(
    ///     MetadataKey::try_new("dng.tag_50740").unwrap().as_str(),
    ///     "dng.tag_50740"
    /// );
    /// assert!(MetadataKey::try_new("DNG.tag").is_err());
    /// ```
    pub fn try_new(value: impl Into<Box<str>>) -> Result<Self, MetadataError> {
        let value = value.into();
        let mut segments = value.split('.');
        let Some(namespace) = segments.next() else {
            return Err(MetadataError::InvalidKey(value));
        };
        let Some(first_path) = segments.next() else {
            return Err(MetadataError::InvalidKey(value));
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
            Err(MetadataError::InvalidKey(value))
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
    /// use fornax::metadata::OriginalValue;
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
    pub(super) fn validate(&self) -> Result<(), MetadataError> {
        if !self.value.is_finite() {
            return Err(MetadataError::InvalidValue);
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
            Err(MetadataError::AllocationLimit)
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
    /// use fornax::metadata::{
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
    pub fn insert(&mut self, entry: MetadataEntry) -> Result<(), MetadataError> {
        if is_reserved_typed_key(entry.key.as_str()) {
            return Err(MetadataError::DuplicateTypedField(
                entry.key.as_str().into(),
            ));
        }
        entry.validate()?;
        if self.entries.contains_key(&entry.key) {
            return Err(MetadataError::Conflict(MetadataField::Extensible(
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
    /// use fornax::metadata::{ExtensibleMetadata, MetadataKey};
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
    /// use fornax::metadata::{
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

    pub(super) fn validate(&self) -> Result<(), MetadataError> {
        for (key, entry) in &self.entries {
            if key != &entry.key {
                return Err(MetadataError::InvalidValue);
            }
            if is_reserved_typed_key(key.as_str()) {
                return Err(MetadataError::DuplicateTypedField(key.as_str().into()));
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
