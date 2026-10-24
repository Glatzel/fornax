# DES-001 — Universal RAW Representation

> **Task:** `DES-001`
> **Owner:** primary agent
> **Review:** GitHub task pull request; its open/merged state is authoritative.
> **Pull request:** [#545](https://github.com/Glatzel/fornax/pull/545).
> **Last updated:** 2026-10-24
>
> **Dependencies:** `SYN-01` (#540), `SYN-02` (#541), and `DES-003` (#543),
> all merged.
>
> **Acceptance criteria (authoritative in the task manifest):**
>
> - [x] Concrete Rust types, fields, units, invariants, and validation rules
>   are specified.
> - [x] Unsupported CFA and non-mosaic behavior has structured rejection
>   semantics.
> - [x] Stage invariants before and after preprocessing and demosaicing are
>   explicit.
> - [x] Representative code and signatures are included.
> - [x] Decisions cite merged SYN-01 and SYN-02 task PRs.

## 1. Scope and decisions

This proposal defines the backend-independent RAW image model produced by a
decoder adapter and consumed by preprocessing. It incorporates the metadata
model from DES-002, the CFA and payload rules from DES-003, and the numerical
stage contract from SYN-02.

The proposed names are pre-`SYN-03`/`DES-007` candidates. They are not final
public API.

Core decisions:

- a RAW image is a typed sensor plane plus validated immutable metadata;
- stored native samples use u8, u16, or f32 payloads and preserve declared bit
  depth separately;
- every transition to a corrected/normalized stage consumes the prior typed
  value;
- ActiveArea, CFA phase, and black-level coordinates remain tied to stored
  sensor coordinates;
- unknown metadata is preserved behind a separate extension map; and
- only conventional 2×2 RGB Bayer inputs enter the initial pipeline.

## 2. Representative Rust types

The shared geometry types are the canonical copies of the matching provisional
types in DES-003.

```rust
use std::num::{NonZeroU8, NonZeroU32, NonZeroU64};
use std::sync::Arc;
use image::{ImageBuffer, Luma};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PixelSize {
    pub width: u32,
    pub height: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PixelCoord {
    pub row: u32,
    pub col: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PixelRect {
    pub top: u32,
    pub left: u32,
    pub bottom: u32,
    pub right: u32,
}

impl PixelRect {
    pub const fn is_empty(self) -> bool {
        self.top >= self.bottom || self.left >= self.right
    }

    pub const fn contains(self, point: PixelCoord) -> bool {
        point.row >= self.top
            && point.row < self.bottom
            && point.col >= self.left
            && point.col < self.right
    }
}

pub type U8Plane = ImageBuffer<Luma<u8>, Vec<u8>>;
pub type U16Plane = ImageBuffer<Luma<u16>, Vec<u16>>;
pub type U32Plane = ImageBuffer<Luma<u32>, Vec<u32>>;
pub type F32Plane = ImageBuffer<Luma<f32>, Vec<f32>>;

#[derive(Clone, Debug, PartialEq)]
pub enum StoredPlane {
    U8(U8Plane),
    U16(U16Plane),
    U32(U32Plane),
    F32(F32Plane),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SampleFormat {
    UnsignedInteger,
    Float,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct SampleEncoding {
    pub bits_per_sample: NonZeroU8,
    pub format: SampleFormat,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct StoredGeometry {
    pub dimensions: PixelSize,
    /// Row stride of the backend allocation before compacting into ImageBuffer.
    pub source_row_stride_bytes: NonZeroU64,
}
```

`PixelRect` uses half-open bounds. `StoredGeometry::dimensions` describes the
compact stored plane; `source_row_stride_bytes` preserves the original backend
layout for provenance. A packed `ImageBuffer` has an implicit
`width * bytes_per_sample` canonical row stride.

### 2.1 Metadata-bearing levels

DES-002 owns `Sourced<T>`, `Rational`, provenance, and `ExtensibleMetadata`.
This proposal concretizes the typed level values:

```rust
#[derive(Clone, Debug, PartialEq)]
pub struct BlackTile {
    pub rows: NonZeroU32,
    pub columns: NonZeroU32,
    pub samples_per_plane: NonZeroU32,
    pub values: Box<[Rational]>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct BlackLevels {
    pub common: Rational,
    pub per_slot: [Rational; 4],
    pub repeat: Option<BlackTile>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct WhiteLevels {
    pub declared: Box<[Rational]>,
    pub adjusted_maximum: Option<Box<[Rational]>>,
    pub measured_maximum: Option<Box<[Rational]>>,
}
```

All level values are stored-sample coordinates/units. They are evaluated in
`f64` only during preprocessing. The components are additive exactly when the
adapter's provenance states that decomposition; a DNG tile can be retained as
one repeat component with common/per-slot zeros.

### 2.2 Camera and crop metadata

```rust
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct CameraIdentity {
    pub make: Option<Box<str>>,
    pub model: Option<Box<str>>,
    pub normalized_make: Option<Box<str>>,
    pub normalized_model: Option<Box<str>>,
}

#[derive(Clone, Debug, PartialEq)]
pub struct DefaultCrop {
    pub origin_x: Rational,
    pub origin_y: Rational,
    pub width: Rational,
    pub height: Rational,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Orientation(u8);

impl Orientation {
    pub const fn new(value: u8) -> Option<Self> {
        if value >= 1 && value <= 8 {
            Some(Self(value))
        } else {
            None
        }
    }

    pub const fn value(self) -> u8 {
        self.0
    }
}
```

WB coefficients, color matrices, calibration data, and less-common capture
metadata remain `Sourced`/extensible values from DES-002. They are not applied
by this model.

### 2.3 Validated metadata

```rust
pub struct ValidatedMetadata {
    common: CommonMetadata,
    extensible: ExtensibleMetadata,
}

impl ValidatedMetadata {
    pub fn try_new(
        _common: CommonMetadata,
        _extensible: ExtensibleMetadata,
    ) -> Result<Self, RawValidationError> {
        todo!("validation design")
    }

    pub fn common(&self) -> &CommonMetadata {
        &self.common
    }

    pub fn extensible(&self) -> &ExtensibleMetadata {
        &self.extensible
    }
}
```

Validation requirements:

- sample encoding, stored geometry, ActiveArea, and CFA are present;
- ActiveArea lies within stored dimensions and matches the area embedded in
  `BayerLayout`;
- the CFA is conventional 2×2 RGB Bayer;
- the image buffer dimensions equal `StoredGeometry::dimensions`;
- the sample container fits the declared bit depth and format;
- black/white level vector lengths match the supported CFA/plane shape;
- no duplicate typed concepts appear in the extension map; and
- orientation, crop, and provenance structures are internally consistent.

The exact `CommonMetadata` field composition belongs to DES-002. Required fields
become private validated fields or non-optional equivalents after construction.

## 3. Stage-typed image

```rust
pub trait RawStage {
    type Plane;
}

pub struct Decoded {
    pub decoder: DecoderProvenance,
    pub operations: Box<[OperationProvenance]>,
    pub metadata_basis: MetadataBasis,
}

pub struct Linearized;
pub struct BlackCorrected;
pub struct Normalized;
pub struct DemosaicReady;

impl RawStage for Decoded { type Plane = StoredPlane; }
impl RawStage for Linearized { type Plane = F32Plane; }
impl RawStage for BlackCorrected { type Plane = F32Plane; }
impl RawStage for Normalized { type Plane = F32Plane; }
impl RawStage for DemosaicReady { type Plane = F32Plane; }

pub struct RawImage<S: RawStage> {
    metadata: Arc<ValidatedMetadata>,
    plane: S::Plane,
    stage: S,
}

impl<S: RawStage> RawImage<S> {
    pub fn metadata(&self) -> &ValidatedMetadata { &self.metadata }
    pub fn plane(&self) -> &S::Plane { &self.plane }
    pub fn stage(&self) -> &S { &self.stage }
}

pub type DecodedRaw = RawImage<Decoded>;
pub type LinearizedRaw = RawImage<Linearized>;
pub type BlackCorrectedRaw = RawImage<BlackCorrected>;
pub type NormalizedRaw = RawImage<Normalized>;
pub type DemosaicReadyMosaic = RawImage<DemosaicReady>;
```

The stage type carries non-interchangeable processing semantics. Metadata is
immutable behind `Arc`; operations consume the source image by value and return
the next stage. User f32 conversion/custom processing remains in `Decoded`; it
does not introduce another pipeline stage.

`MetadataBasis` is:

```rust
#[derive(Clone, Debug, PartialEq)]
pub enum MetadataBasis {
    OriginalLevelsValid,
    ReplacedLevels {
        black: Sourced<BlackLevels>,
        white: Sourced<WhiteLevels>,
    },
    Incompatible,
}
```

`Incompatible` decoded/custom output is valid as a user-processed image but
cannot enter standard preprocessing. `OriginalLevelsValid` is accepted only
when the custom operation contract proves it.

## 4. Source container validation

| Stored container | Accepted semantic encoding | Rules |
| --- | --- | --- |
| `U8Plane` | Unsigned integer, `bits_per_sample <= 8` | Values are stored codes, not implicitly normalized |
| `U16Plane` | Unsigned integer, `bits_per_sample <= 16` | Unpacked values are retained without semantic left shift |
| `U32Plane` | Unsigned integer, `16 < bits_per_sample <= 32` | Used only when backend/source evidence supports wider integer samples |
| `F32Plane` | Float, source declares a floating encoding | Used natively for floating DNG; no u16 round-trip |

Signed integers, integer samples above 32 bits, and f64 source planes are
rejected with `UnsupportedSampleEncoding` until a separately verified storage
path exists. Container size is not proof of semantic precision: an 8-bit source
in u8 and a 12-bit source in u16 retain their declared bit depth. A u32 source
is not exact in f32 when it needs more than 24 significant bits.
`convert_to_f32` must reject such u32 sources rather than silently lose
precision; a wider working path is future work.

Every compact `ImageBuffer` must have exact `width * height` sample count. A
padded backend allocation is copied row-by-row before canonical storage; source
stride remains in `StoredGeometry`/provenance.

## 5. Coordinate and metadata invariants

The universal model has four coordinate concepts:

- stored raster coordinates starting at `(0,0)`;
- ActiveArea coordinates derived from the ActiveArea top-left;
- DefaultCrop coordinates relative to ActiveArea, before `DefaultScale`;
- CFA tile coordinates relative to ActiveArea.

Invariants:

- ActiveArea is half-open and contained in stored dimensions;
- CFA and black-pattern phase use ActiveArea-relative coordinates;
- crop is descriptive and never rebases the stored image implicitly;
- orientation is descriptive and never rotates/mirrors the sensor plane;
- default crop is not merged with ActiveArea or masked areas;
- black and white values use stored sample units;
- camera and color metadata do not mutate samples; and
- every consumed metadata value has provenance.

## 6. Structured rejection

```rust
#[derive(Debug, PartialEq)]
pub enum RawValidationError {
    MissingMetadata(&'static str),
    InvalidGeometry,
    InvalidSampleEncoding,
    UnsupportedSampleEncoding {
        bits: u8,
        format: SampleFormat,
    },
    UnsupportedCfa {
        rows: u32,
        columns: u32,
    },
    UnsupportedNonMosaic,
    InvalidLevels,
    MetadataConflict,
    InvalidCustomMetadataBasis,
}
```

Non-Bayer CFA, monochrome, LinearRaw, Foveon/X3, Quad Bayer, four-color, and
non-mosaic payloads are unsupported structured outcomes, not inferred RGB.

## 7. Stage invariants

| Stage | Plane | Required invariants | Allowed next stage |
| --- | --- | --- | --- |
| `Decoded` | u8/u16/u32/f32 native or f32 after explicit conversion/custom processing | Validated source geometry/encoding/CFA/levels; conversion and operations remain in `Decoded` with `MetadataBasis`; no implied correction | Standard preprocessing only with valid/replaced basis |
| `Linearized` | f32 | Linearization applied once or identity; geometry/phase unchanged | Black selection/subtraction |
| `BlackCorrected` | f32 | Black subtracted once; signed values allowed; geometry/phase unchanged | Normalization |
| `Normalized` | f32 | Unclipped signed `f32`; denominator validated; no second correction | Final clipping |
| `DemosaicReady` | f32 | Finite values in `[0,1]`, active Bayer geometry/phase, immutable preprocessed state | Demosaicing |
| Demosaic output | f32 RGB | Algorithm-defined planar/interleaved output; metadata reference retained; no preprocessing rerun | End of pipeline scope |

## 8. Representative constructors

```rust
impl ValidatedMetadata {
    pub fn validate_plane(&self, _plane: &StoredPlane) -> Result<(), RawValidationError> {
        todo!("plane validation design")
    }
}

impl DecodedRaw {
    pub fn try_new(
        _metadata: ValidatedMetadata,
        _plane: StoredPlane,
        _decoder: DecoderProvenance,
    ) -> Result<Self, RawValidationError> {
        todo!("constructor design")
    }

    pub fn convert_to_f32(
        self,
    ) -> Result<Self, RawValidationError> {
        todo!("conversion design")
    }
}
```

No constructor accepts arbitrary width/height separately from the metadata and
plane; geometry cannot silently diverge from the payload.

## 9. Example flow

```text
decoder adapter
  -> StoredPlane { U8 | U16 | U32 | F32 } + ValidatedMetadata
  -> DecodedRaw
       -> [optional] convert_to_f32/user processing, still DecodedRaw
       -> LinearizedRaw
       -> BlackCorrectedRaw
       -> NormalizedRaw
       -> DemosaicReadyMosaic
       -> f32 demosaic output
```

Malformed or unsupported input fails at adapter/constructor boundaries with
structured errors. Preprocessing cannot recover missing semantics by guessing.

## 10. Testing plan

1. Construct valid u8, u16, u32, and f32 planes with exact dimensions and
   encoding.
2. Reject u8 with >8 declared bits, u16 with >16 declared bits, u32 with >32
   declared bits, and unsupported signed/f64 containers.
3. Reject padded allocations passed as compact buffers; verify correct
   row-by-row compaction.
4. Exhaustively verify u8/u16 conversion to f32 is lossless; verify u32
   conversion succeeds only when it fits f32's 24-bit significand and rejects
   wider values.
5. Verify floating DNG remains on the native f32 path.
6. Test ActiveArea validity, crop separation, orientation preservation, and
   active-relative CFA/black phase at odd origins.
7. Reject all non-Bayer/non-mosaic layouts with structured variants.
8. Verify u8/u16/u32/f32 custom processing remains `DecodedRaw`, records
   operation provenance, and rejects stale metadata basis.
9. Verify each preprocessing transition consumes the previous state and cannot
   be applied twice.
10. Verify `DemosaicReadyMosaic` is finite f32 in `[0,1]` and retains Bayer
    phase/metadata.

## 11. Open questions and integration

- DES-002 must convert its optional `CommonMetadata` fields into the
  non-optional validated representation used here.
- DES-003 owns the finalized CFA module location; DES-001 owns shared geometry
  to prevent duplicate coordinate types.
- DES-004 owns concrete operation structs and the mutable custom-processing API
  used while the image remains in `DecodedRaw`.
- DES-005 owns planar/interleaved ordered RGB output and metadata retention.
- Exact `DefaultScale` and fractional-crop handling remain unresolved.
- General black tiles, row/column deltas, and DNG opcode subsets remain
  unsupported gates.
- Signed integer and >32-bit integer source support require decoder/source
  evidence and a separate design update.

## 12. Evidence

- [SYN-01 decision table](../../research/notes/SYN-01.md#decision-table):
  sample state, units, ranges, coordinates, CFA, black/white metadata,
  provenance, and backend boundaries.
- [SYN-01 conflict log](../../research/notes/SYN-01.md#conflict-log):
  coordinate/crop, black-shape, clipping, and double-correction conflicts.
- [SYN-02 numeric model](../../research/notes/SYN-02.md#numerical-model):
  f32 storage, f64 scalar arithmetic, ordering, clipping, and custom processing.
- [SYN-02 processing states](../../research/notes/SYN-02.md#processing-state-constraints):
  type/state separation and double-correction prevention.
- [DES-002 §2–§5](../raw-pipeline/DES-002-metadata-provenance.md#2-two-layer-metadata-model):
  typed metadata, extensible keys, provenance, and conflicts.
- [DES-003 §2–§5](../raw-pipeline/DES-003-cfa-indexing.md#2-proposed-types):
  geometry, CFA indexing, image-rs payload containers, and u8/u16/f32 rules.

## Acceptance checklist

- [x] Concrete Rust types, fields, units, invariants, and validation rules are
  specified.
- [x] Unsupported CFA and non-mosaic behavior has structured rejection
  semantics.
- [x] Stage invariants before and after preprocessing and demosaicing are
  explicit.
- [x] Representative code and signatures are included.
- [x] Decisions cite merged SYN-01 and SYN-02 task PRs.
