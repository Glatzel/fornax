//! Metadata provenance, authority, confidence, and conflict resolution.

use super::extensible::MetadataField;
use crate::error::MetadataError;

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
    /// use fornax::metadata::ProviderId;
    ///
    /// assert_eq!(ProviderId::try_new("libraw").unwrap().as_str(), "libraw");
    /// assert!(ProviderId::try_new("").is_err());
    /// ```
    pub fn try_new(value: impl Into<Box<str>>) -> Result<Self, MetadataError> {
        let value = value.into();
        if value.is_empty() {
            Err(MetadataError::InvalidValue)
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
    /// use fornax::metadata::{DecoderProvenance, ProviderId};
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
    /// use fornax::metadata::AuthorityClass;
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
    /// use fornax::metadata::{AuthorityClass, Confidence, Provenance, ProviderId, SourceKind};
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
    /// use fornax::metadata::{
    ///     AuthorityClass, Confidence, Provenance, ProviderId, SourceKind, Sourced,
    /// };
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
/// use fornax::metadata::{
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
pub fn resolve_sourced<T>(
    field: MetadataField,
    left: Sourced<T>,
    right: Sourced<T>,
) -> Result<Sourced<T>, MetadataError> where T: PartialEq {
    let left_rank = left.provenance.authority().rank();
    let right_rank = right.provenance.authority().rank();
    match left_rank.cmp(&right_rank) {
        std::cmp::Ordering::Less => Ok(left),
        std::cmp::Ordering::Greater => Ok(right),
        std::cmp::Ordering::Equal if left.value == right.value => Ok(left),
        std::cmp::Ordering::Equal => Err(MetadataError::Conflict(field)),
    }
}
