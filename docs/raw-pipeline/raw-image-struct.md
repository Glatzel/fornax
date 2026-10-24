# Key Structures and Fields

This file is the normative field-level reference for the persistent RAW image
model. The task-owned DES-001 proposal records the drafting history and
rationale; field names, units, and invariants in this file are authoritative.

## Structure class diagram

```mermaid
classDiagram
    direction LR
    class RawImage {
        <<Decode context>>
        +metadata
        +plane
        +stage
    }
    class ValidatedMetadata {
        <<struct>>
        +common
        +extensible
    }
    class CommonMetadata {
        <<struct>>
    }
    class ExtensibleMetadata {
        <<struct>>
    }
    class StoredGeometry
    class BayerLayout
    class BlackLevels
    class WhiteLevels
    class Decoded {
        <<Decode phase>>
        +decoder
        +operations
        +metadata_basis
        +storage
    }
    class StoredPlane {
        <<Decode payload>>
        U8
        U16
        U32
        F32
    }
    class Linearized {
        <<Preprocess phase>>
    }
    class BlackCorrected {
        <<Preprocess phase>>
    }
    class Normalized {
        <<Preprocess phase>>
    }
    class DemosaicReady {
        <<Preprocess phase>>
    }
    class DemosaicOutput {
        <<Demosaic phase>>
        +planar_rgb_f32
    }

    RawImage *-- ValidatedMetadata
    RawImage *-- StoredPlane
    RawImage *-- Decoded
    ValidatedMetadata *-- CommonMetadata
    ValidatedMetadata *-- ExtensibleMetadata
    CommonMetadata *-- StoredGeometry
    CommonMetadata *-- BayerLayout
    CommonMetadata *-- BlackLevels
    CommonMetadata *-- WhiteLevels
    Decoded ..> Linearized
    Linearized ..> BlackCorrected
    BlackCorrected ..> Normalized
    Normalized ..> DemosaicReady
    DemosaicReady ..> DemosaicOutput
```

## Core image

| Type/field | Type | Unit | Invariant |
| --- | --- | --- | --- |
| `RawImage<S>::metadata` | `Arc<ValidatedMetadata>` | N/A | Immutable during processing |
| `RawImage<S>::plane` | `S::Plane` | samples | Matches declared dimensions and stage |
| `RawImage<S>::stage` | `S` | N/A | Carries state/provenance; no implicit stage recovery |
| `StoredPlane` | `U8`, `U16`, `U32`, or `F32` variant | sample | Container compatible with `SampleEncoding` |
| `F32Plane` | `ImageBuffer<Luma<f32>, Vec<f32>>` | sample | Preprocessing/demosaic stages only |

## Geometry and encoding

| Type/field | Type | Unit | Invariant |
| --- | --- | --- | --- |
| `PixelSize::{width,height}` | `u32` | pixels | Both non-zero for a stored image |
| `PixelCoord::{row,col}` | `u32` | pixel | Stored raster coordinate |
| `PixelRect::{top,left,bottom,right}` | `u32` | half-open pixels | `top < bottom`, `left < right`, inside stored size |
| `SampleEncoding::bits_per_sample` | `u8` | bits | Non-zero and at most 32; semantic source precision |
| `SampleEncoding::format` | `SampleFormat` | enum | Unsigned integer or float |
| `StoredGeometry::dimensions` | `PixelSize` | pixels | Must equal payload dimensions |
| `StoredGeometry::source_row_stride_bytes` | `NonZeroU64` | bytes | Preserves original backend stride |

`StoredGeometry::source_row_stride_bytes` describes the backend allocation
before compacting. The canonical `ImageBuffer` is tightly packed.

## CFA, crop, and levels

| Type/field | Type | Unit | Invariant |
| --- | --- | --- | --- |
| `BayerLayout::pattern` | `BayerPattern` | enum | `RGGB`, `BGGR`, `GRBG`, or `GBRG` |
| `BayerLayout::active_area` | `ActiveArea` | stored pixels | Pattern origin is ActiveArea top-left |
| `DefaultCrop::{origin_x,origin_y,width,height}` | `Rational` | pre-scale raw coordinates | Kept separate from ActiveArea |
| `Orientation` | `u8` newtype | TIFF enum | Valid range 1–8 |
| `BlackLevels::common` | `Rational` | stored sample code | Scalar component |
| `BlackLevels::per_slot` | `[Rational; 4]` | stored sample code | CFA slots remain distinct |
| `BlackLevels::repeat` | `Option<BlackTile>` | stored sample code | ActiveArea-relative tile |
| `BlackTile::{rows,columns,samples_per_plane}` | `NonZeroU32` | samples | Value count matches dimensions × planes |
| `WhiteLevels::declared` | `Box<[Rational]>` | stored sample code | Source-declared saturation |
| `WhiteLevels::adjusted_maximum` | `Option<Box<[Rational]>>` | stored sample code | Decoder-adjusted value kept separate |
| `WhiteLevels::measured_maximum` | `Option<Box<[Rational]>>` | stored sample code | Observed value kept separate |

`BlackLevels`, `BlackTile`, and `WhiteLevels` are private-field types with
validating constructors. Scalar black levels are non-negative. A black tile's
value count must equal `rows * columns * samples_per_plane`. Declared white
levels contain either one scalar or four CFA-slot values and must be positive;
optional adjusted and measured arrays use the same length as the declared
array. This keeps both green CFA positions distinguishable without treating the
common scalar component as one of those slots.

## Stage types

| Stage | Plane | Meaning |
| --- | --- | --- |
| `Decoded` | `StoredPlane` | Native source; optional `convert_to_f32`/custom operation keeps this state, records `DecodedStorage::{Native, F32Working}`, operations, and metadata basis |
| `Linearized` | `F32Plane` | LUT applied once or identity |
| `BlackCorrected` | `F32Plane` | Black subtracted once; unclipped |
| `Normalized` | `F32Plane` | Linear-reference semantics; unclipped |
| `DemosaicReady` | `F32Plane` | Finite f32 values in `[0,1]` |

## Validation fields and errors

`ValidatedMetadata` is constructed only after checking required metadata,
geometry, CFA, level lengths, sample encoding, and provenance conflicts.
The required typed fields are sample encoding, stored geometry, ActiveArea, and
CFA. Their stored dimensions and ActiveArea must agree. The original row stride
is retained as nonzero provenance and is not assumed to equal the compact
`ImageBuffer` stride. Extensible keys use the `raw.*` names blocked for typed
concepts and cannot override those fields.

`RawValidationError` is the stage/model boundary error. It distinguishes
missing metadata, invalid geometry/encoding, unsupported sample widths,
non-mosaic/non-2x2/non-RGB CFA layouts, invalid levels, non-finite samples,
invalid demosaic-ready values, metadata conflicts, and incompatible
custom-processing bases. `MetadataError` reports key grammar, duplicate typed
concepts, candidate conflicts, invalid values, and allocation limits.

The optional `serde` feature serializes the validated metadata and stage
descriptors. It deliberately does not derive deserialization for the model:
deserializing private invariants without a schema-aware validating decoder
would bypass construction checks. Plane serialization and stable schema
versioning remain follow-up design work.

Unsupported input is rejected at the adapter/constructor boundary. No code
guesses a missing CFA origin, fabricates black levels, converts an unknown CFA
to RGB, or treats a display image as a sensor plane.

## Stage construction boundary

`DecodedRaw::try_new` is the public decode boundary and validates the native
plane against its metadata. The preprocessing crate uses the doc-hidden
`fornax_core::transition` module to convert an owned `DecodedRaw` into
`LinearizedRaw`, then to `BlackCorrectedRaw`, `NormalizedRaw`, and
`DemosaicReadyMosaic`. Each transition consumes its source stage. Only
`RawImage<Decoded>` has the standard-preprocessing entry point, and a custom
`MetadataBasis::Incompatible` image is rejected before linearization.

## Related guides

- [Raw image processing flow](raw-image-process.md)
- [Precision by phase](precision.md)
