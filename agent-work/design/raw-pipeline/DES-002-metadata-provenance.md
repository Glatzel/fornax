# DES-002 — Extensible Metadata and Provenance

> **Task:** `DES-002`
> **Owner:** primary agent
> **Review:** GitHub task pull request; its open/merged state is authoritative.
> **Pull request:** [#542](https://github.com/Glatzel/fornax/pull/542).
> **Last updated:** 2026-10-24
>
> **Dependency:** `SYN-01`, merged through PR #540.
>
> **Acceptance criteria (authoritative in the task manifest):**
>
> - [x] Concrete types and keys are specified.
> - [x] Authority rules resolve conflicts between metadata sources.
> - [x] A generic map does not replace typed algorithm-critical fields.
> - [x] Provenance and unknown-field conversion rules are explicit.
> - [x] Decisions cite merged SYN-01 evidence.

## 1. Scope and decisions

This proposal defines the metadata ownership, namespacing, provenance, conflict
policy, and unknown-field model for the universal RAW representation. It does
not finalize the complete universal RAW struct, preprocessing API, or decoder
backend trait.

The design follows the [SYN-01 decision table](../../research/notes/SYN-01.md#decision-table):

- typed fields remain authoritative for metadata consumed by decoding,
  preprocessing, and demosaicing;
- file declarations, decoder observations, camera-catalog fallbacks, defaults,
  and derived values remain distinguishable;
- conflicting values are never resolved by map overwrite order; and
- unknown source metadata is preserved without allowing it to redefine typed
  universal semantics.

The names below are proposed pre-`SYN-03` APIs. They are not finalized until
`DES-007` accepts the consolidated design.

## 2. Two-layer metadata model

### 2.1 Typed common metadata

Algorithm-critical fields are concrete named fields. Each optional field uses
`Option<Sourced<T>>`; when present, the provenance is inseparable from the
value.

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct Sourced<T> {
    pub value: T,
    pub provenance: Provenance,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct CommonMetadata {
    pub camera: Option<Sourced<CameraIdentity>>,
    pub sample_encoding: Option<Sourced<SampleEncoding>>,
    pub stored_geometry: Option<Sourced<StoredGeometry>>,
    pub active_area: Option<Sourced<ActiveArea>>,
    pub default_crop: Option<Sourced<DefaultCrop>>,
    pub cfa: Option<Sourced<BayerLayout>>,
    pub black: Option<Sourced<BlackLevels>>,
    pub white: Option<Sourced<WhiteLevels>>,
    pub orientation: Option<Sourced<Orientation>>,
    pub white_balance: Option<Sourced<WhiteBalance>>,
    pub color_matrices: Option<Sourced<ColorMatrices>>,
    pub capture: Option<Sourced<CaptureMetadata>>,
}
```

`ActiveArea`, `BayerLayout`, and their coordinate rules are owned by `DES-003`.
`StoredGeometry` is expected to be shared with `DES-001`; this proposal requires
that it retain width, height, and byte stride explicitly.

Typed fields are not serialized through the extension map. A key that attempts
to redefine one of these concepts is rejected.

### 2.2 Extensible metadata

Less-common and vendor-specific values use a namespaced key-value store:

```rust
use std::collections::BTreeMap;

#[derive(Clone, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub struct MetadataKey(Box<str>);

#[derive(Clone, Debug, PartialEq)]
pub struct MetadataEntry {
    pub key: MetadataKey,
    pub value: MetadataValue,
    pub unit: Option<MetadataUnit>,
    pub coordinate_space: Option<CoordinateSpace>,
    pub provenance: Provenance,
    pub original: Option<OriginalValue>,
}

#[derive(Clone, Debug, Default, PartialEq)]
pub struct ExtensibleMetadata {
    pub entries: BTreeMap<MetadataKey, MetadataEntry>,
    pub conflicts: Vec<MetadataConflict>,
}
```

`BTreeMap` provides deterministic iteration. It does not replace the typed
fields in `CommonMetadata`.

## 3. Key and value model

### 3.1 Namespace and key grammar

`MetadataKey::try_new` accepts lowercase, dot-separated ASCII segments:

```text
<namespace>.<path>[.<path>...]
```

- At least two segments are required.
- Each segment matches `[a-z][a-z0-9_]*`.
- Empty segments, uppercase letters, whitespace, and separator characters are
  rejected.
- Canonical keys are short and semantic; backend or source identity belongs in
  `Provenance`, not in duplicate semantic keys.
- The initial reserved top-level namespaces are `dng`, `tiff`, `capture`,
  `vendor`, and `decoder`.

Examples:

| Key | Meaning |
| --- | --- |
| `dng.black_level_delta_h` | DNG column black delta not promoted to a typed field |
| `vendor.sony.white_balance_note` | Sony-specific supplemental value |
| `capture.as_shot` | Original capture metadata |
| `decoder.libraw.raw_count` | Backend-specific observation retained for diagnostics |

Unknown fields from a source format use a stable source tag identifier when
known, such as `dng.tag_50740`, rather than a generated random key.

### 3.2 Values, units, and coordinate spaces

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum MetadataValue {
    Bool(bool),
    I64(i64),
    U64(u64),
    F64(f64),
    Rational(Rational),
    Text(Box<str>),
    Bytes(Box<[u8]>),
    I64Array(Box<[i64]>),
    U64Array(Box<[u64]>),
    F64Array(Box<[f64]>),
    RationalArray(Box<[Rational]>),
}

#[derive(Clone, Debug, PartialEq)]
pub struct Rational {
    pub numerator: i64,
    pub denominator: std::num::NonZeroU32,
}

#[derive(Clone, Debug, PartialEq)]
pub enum MetadataUnit {
    Pixel,
    Byte,
    Bit,
    StoredSampleCode,
    Normalized,
    Dimensionless,
    Second,
    Kelvin,
    Other(Box<str>),
}

#[derive(Clone, Debug, PartialEq)]
pub enum CoordinateSpace {
    StoredImage,
    ActiveArea,
    DefaultCrop,
    CfaTile,
    CameraRgb,
    XyzD50,
    None,
    Other(Box<str>),
}
```

`F64`, `F64Array`, and floating typed fields do not derive `Eq` because IEEE
NaN values are not equality-stable. Validation must reject NaN where a field
requires an ordered range.

An entry must state a unit and coordinate space when its value depends on
either. Raw sample-code values use `StoredSampleCode`; normalized values use
`Normalized`; pixel rectangles use `Pixel` plus their source coordinate space.

## 4. Provenance and authority

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ProviderId(Box<str>);

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum SourceKind {
    FileTag { tag: u32, path: Box<str> },
    Decoder {
        backend: Box<str>,
        revision: Option<Box<str>>,
        path: Box<str>,
    },
    CameraCatalog {
        catalog: Box<str>,
        revision: Option<Box<str>>,
    },
    FormatDefault { clause: Box<str> },
    Derived { operation: Box<str> },
    UserOverride,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Ord, PartialOrd)]
pub enum AuthorityClass {
    UserOverride,
    FormatDeclaration,
    DecoderObservation,
    CameraCatalogFallback,
    FormatDefault,
    DerivedValue,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Confidence {
    Exact,
    Inferred,
    Unknown,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct Provenance {
    pub provider: ProviderId,
    pub source: SourceKind,
    pub authority: AuthorityClass,
    pub confidence: Confidence,
}
```

`AuthorityClass::UserOverride` is permitted only through an explicit decode
configuration that records the override. It is never enabled implicitly.

Provenance is required for both typed and extensible values. A decoder adapter
that cannot identify the origin of a required typed field returns a structured
metadata error rather than assigning a generic source.

## 5. Conflict resolution

The authority order is field-class specific:

1. An explicit user override, when enabled and valid.
2. A file declaration with normative format meaning.
3. A source-verified decoder observation.
4. A camera-catalog fallback selected by the adapter.
5. A format default.
6. A derived or measured value.

Additional rules:

- DNG declarations and defaults follow DNG semantics before decoder
  convenience; this is the SYN-01 A1-over-A2 rule.
- A camera catalog never overrides a present file declaration silently.
- A measured maximum never replaces a declared white level.
- A black-delta vector is never collapsed into scalar black unless an explicit
  transformation records the derivation.
- Two candidates at the same authority class with unequal required values are a
  conflict; the adapter must not select by insertion order.
- A conflicting optional value is retained as a `MetadataConflict` diagnostic
  and does not silently overwrite the selected typed value.
- A conflict in a required algorithm-critical field aborts decode with
  `MetadataError::Conflict`.
- An extensible key attempting to duplicate a typed concept is rejected.

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub enum TypedMetadataField {
    Camera,
    SampleEncoding,
    StoredGeometry,
    ActiveArea,
    DefaultCrop,
    Cfa,
    Black,
    White,
    Orientation,
    WhiteBalance,
    ColorMatrices,
    Capture,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetadataField {
    Typed(TypedMetadataField),
    Extensible(MetadataKey),
}

#[derive(Clone, Debug, PartialEq)]
pub struct MetadataConflict {
    pub field: MetadataField,
    pub selected: Provenance,
    pub rejected: Vec<(MetadataValue, Provenance)>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub enum MetadataError {
    InvalidKey(Box<str>),
    DuplicateTypedField { attempted: MetadataKey },
    MissingRequiredField(TypedMetadataField),
    InvalidUnit { key: MetadataKey },
    InvalidCoordinateSpace { key: MetadataKey },
    Conflict { field: MetadataField },
}
```

The authoritative mapping is:

| Metadata class | Primary authority | Fallback | Never inferred |
| --- | --- | --- | --- |
| DNG declaration | File tag | DNG default | From a vendor camera catalog |
| Vendor declaration | Parsed file tag | Backend/catalog override with provenance | From another backend's value |
| Decoder-derived value | Documented decoder operation | Measured/diagnostic metadata | As a declared file value |
| Capture metadata | File declaration | Decoder extraction | From image statistics |
| Unknown field | Original raw value | None | Semantic meaning not established by evidence |

## 6. Source-to-universal conversion

Each backend adapter follows the same staged conversion:

1. Retain original field/tag identity, raw bytes, backend revision, and source
   path.
2. Validate format-level meaning and coordinate units.
3. Produce typed common-field candidates for algorithm-critical data.
4. Produce namespaced extensible candidates for the remainder.
5. Resolve candidates through the authority policy.
6. Reject required ambiguity or lossy conversion.
7. Preserve unselected optional candidates as conflict diagnostics.

| Source concept | Universal destination | Conversion rule |
| --- | --- | --- |
| LibRaw `black`/`cblack` | `BlackLevels` | Preserve scalar/per-slot/tile distinctions and source units; do not call a folded backend value the declared DNG tile |
| DNG `BlackLevelDeltaH`/`V` | `BlackLevels` or `dng.black_level_delta_*` | Preserve vectors exactly when exposed; never replace absent support with zero |
| DNG `CFAPlaneColor` | `BayerLayout` plane map | Accept only verified conventional RGB initially |
| rawler `RawImage` fields | Typed candidates plus decoder namespace | Snapshot before mutating scaling; preserve rawler revision |
| LibRaw `raw_image` state note | `Provenance` and sample-state metadata | Record any vendor-specific transformation |
| Unknown TIFF/DNG tag | `dng.tag_<number>` or `tiff.tag_<number>` | Retain raw bytes and source type; do not invent meaning |
| Vendor makernote | `vendor.<make>.<field>` | Retain raw bytes and parsed value when parser semantics are verified |

A typed field and an extensible key may represent different concepts with
similar names only if the design documents that relationship. For example,
declared white level and measured maximum are separate typed values, not one
extensible entry.

## 7. Unknown-field preservation

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct OriginalValue {
    pub raw: Box<[u8]>,
    pub source_type: Option<Box<str>>,
    pub count: Option<u32>,
}
```

Unknown fields are preserved when the backend can provide their raw bytes.
Retention rules:

- the key identifies the source/tag, not an assumed semantic;
- raw bytes and declared source type remain unchanged;
- a parsed convenience value may accompany the raw value only with provenance;
- unknown fields do not participate in typed-field conflict resolution; and
- if a source API cannot expose raw bytes, the adapter records that limitation
  in decoder capabilities rather than fabricating an entry.

Unknown-field preservation must be bounded by a documented byte/allocation
limit to avoid turning malformed files into unbounded metadata allocations.

## 8. Proposed lookup APIs

```rust
impl MetadataKey {
    pub fn try_new(value: &str) -> Result<Self, MetadataError> {
        fn valid_segment(segment: &str) -> bool {
            let mut bytes = segment.bytes();
            matches!(bytes.next(), Some(b'a'..=b'z'))
                && bytes.all(|byte| {
                    byte.is_ascii_lowercase()
                        || byte.is_ascii_digit()
                        || byte == b'_'
                })
        }

        let mut segments = value.split('.');
        let namespace = segments.next().unwrap_or_default();
        let path = segments.next().ok_or_else(|| {
            MetadataError::InvalidKey(value.into())
        })?;
        if !valid_segment(namespace)
            || !valid_segment(path)
            || segments.any(|segment| !valid_segment(segment))
        {
            return Err(MetadataError::InvalidKey(value.into()));
        }
        Ok(Self(value.into()))
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }
}

impl ExtensibleMetadata {
    pub fn insert(&mut self, entry: MetadataEntry) -> Result<(), MetadataError> {
        use std::collections::btree_map::Entry;

        match self.entries.entry(entry.key.clone()) {
            Entry::Occupied(_) => Err(MetadataError::Conflict {
                field: MetadataField::Extensible(entry.key),
            }),
            Entry::Vacant(slot) => {
                slot.insert(entry);
                Ok(())
            }
        }
    }

    pub fn get(&self, key: &MetadataKey) -> Option<&MetadataEntry> {
        self.entries.get(key)
    }
}

impl CommonMetadata {
    pub fn validate_required(&self) -> Result<(), MetadataError> {
        for (field, present) in [
            (
                TypedMetadataField::SampleEncoding,
                self.sample_encoding.is_some(),
            ),
            (
                TypedMetadataField::StoredGeometry,
                self.stored_geometry.is_some(),
            ),
            (TypedMetadataField::Cfa, self.cfa.is_some()),
        ] {
            if !present {
                return Err(MetadataError::MissingRequiredField(field));
            }
        }
        Ok(())
    }
}
```

Typed common fields deliberately have no string-key setter API. Populating them
is the responsibility of typed adapter conversion, not arbitrary metadata
assignment.

## 9. Tests

Metadata tests must cover:

1. Key acceptance for canonical names and rejection for empty segments,
   uppercase, whitespace, and one-segment keys.
2. Deterministic ordering of extension entries.
3. Typed-field provenance surviving adapter conversion.
4. File declaration winning over camera-catalog fallback.
5. Same-authority required conflict returning `MetadataError::Conflict`.
6. Optional conflict retaining a diagnostic and the authoritative value.
7. User override accepted only when explicitly configured.
8. Unknown tag bytes and source type preserved for DNG, TIFF, LibRaw, and vendor
   metadata.
9. Declared white level, adjusted maximum, and measured maximum remaining
   separate.
10. Black repeat vectors not being collapsed into a scalar without an explicit
    derived provenance.
11. Unknown-field allocation limits.

Representative conflict tests are table-driven for each `AuthorityClass` pair
rather than repeated ad hoc branches.

## 10. Integration and open questions

- `DES-001` must decide which shared geometry types are reused by
  `CommonMetadata`.
- `DES-003` owns CFA type construction and plane validation.
- `DES-004` owns the meaning of corrected/normalized processing state.
- `DES-006` decides crate visibility and serialization features.
- The final backend adapter must report which unknown fields it can preserve.
- Serialization stability, maximum metadata size, and whether original raw
  values are retained in memory or behind a feature flag require `DES-007`
  review.
- The exact mapping from LibRaw's canonicalized `cblack` back to a declared
  source tile may be unavailable; the adapter must report the loss rather than
  invent provenance.

## 11. Evidence

- [SYN-01 decision table](../../research/notes/SYN-01.md#decision-table):
  typed metadata, units, provenance, conflicts, unknown fields, and authority
  order.
- [SYN-01 conflict log](../../research/notes/SYN-01.md#conflict-log): DNG,
  LibRaw, rawler, and darktable disagreements.
- [RES-02 §2.7](../../research/RES-02-libraw.md#27-metadata-retention-and-the-safe-wrapper):
  LibRaw metadata retention and wrapper gaps.
- [RES-03 §2.5](../../research/RES-03-dnglab.md#25-dng-versus-vendor-raw-metadata):
  file-declared versus vendor-catalog metadata.

## Acceptance checklist

- [x] Concrete types and keys are specified.
- [x] Authority rules resolve conflicts between metadata sources.
- [x] A generic map does not replace typed algorithm-critical fields.
- [x] Provenance and unknown-field conversion rules are explicit.
- [x] Decisions cite merged SYN-01 evidence.
