# DES-003 — Bayer CFA Model and Indexing

> **Task:** `DES-003`
> **Owner:** primary agent
> **Review:** GitHub task pull request; its open/merged state is authoritative.
> **Pull request:** [#543](https://github.com/Glatzel/fornax/pull/543).
> **Last updated:** 2026-10-24
>
> **Dependency:** `SYN-01`, merged through PR #540.
>
> **Acceptance criteria (authoritative in the task manifest):**
>
> - [x] Concrete types, methods, coordinate conventions, and index formulas
>   are specified.
> - [x] Bounds and validation behavior are explicit.
> - [x] RGGB, BGGR, GRBG, and GBRG tests are planned for every supported
>   pattern.
> - [x] Shared APIs do not hide a 2×2-only assumption.
> - [x] Decisions cite merged SYN-01 evidence.
> - [x] Image-rs container suitability and row-stride limitations are
>   evaluated.
> - [x] Native u8/u16/f32 storage, explicit custom-processing conversion, and
>   preprocessing/demosaic precision are justified from decoder evidence.

## 1. Scope and decisions

This proposal defines the typed CFA model used between decoding,
CFA-dependent preprocessing, and Bayer demosaicing. It also evaluates image-rs
as the pixel payload container and distinguishes native source precision from
the `f32` preprocessing domain. It does not define the black-level algorithm,
decoder adapter API, or demosaicing algorithms.

The design follows the [SYN-01 decision table](../../research/notes/SYN-01.md#decision-table):

- initial pipeline support is limited to conventional 2×2 RGB Bayer phases;
- DNG CFA phase originates at ActiveArea top-left;
- stored coordinates, ActiveArea coordinates, and crop coordinates remain
  distinct; and
- an unsupported layout is a structured error rather than a reinterpretation.

The names are retained as design history. The persistent normative design is
under repository-root `docs/raw-pipeline/`.

## 2. Proposed types

```rust
use core::num::NonZeroU32;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PixelSize {
    width: u32,
    height: u32,
}

impl PixelSize {
    pub const fn new(width: u32, height: u32) -> Option<Self> {
        if width == 0 || height == 0 {
            None
        } else {
            Some(Self { width, height })
        }
    }

    pub const fn width(self) -> u32 {
        self.width
    }

    pub const fn height(self) -> u32 {
        self.height
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct PixelCoord {
    pub row: u32,
    pub col: u32,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct ActiveArea {
    top: u32,
    left: u32,
    bottom: u32,
    right: u32,
}

impl ActiveArea {
    pub fn new(
        stored: PixelSize,
        top: u32,
        left: u32,
        bottom: u32,
        right: u32,
    ) -> Result<Self, CfaError> {
        if top >= bottom
            || left >= right
            || bottom > stored.height()
            || right > stored.width()
        {
            return Err(CfaError::InvalidActiveArea);
        }
        Ok(Self {
            top,
            left,
            bottom,
            right,
        })
    }

    pub const fn top(self) -> u32 { self.top }
    pub const fn left(self) -> u32 { self.left }
    pub const fn bottom(self) -> u32 { self.bottom }
    pub const fn right(self) -> u32 { self.right }

    pub const fn contains(self, point: PixelCoord) -> bool {
        point.row >= self.top
            && point.row < self.bottom
            && point.col >= self.left
            && point.col < self.right
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CfaShape {
    rows: NonZeroU32,
    columns: NonZeroU32,
}

impl CfaShape {
    pub const BAYER_2X2: Self = Self {
        // SAFETY: 2 is non-zero.
        rows: unsafe { NonZeroU32::new_unchecked(2) },
        // SAFETY: 2 is non-zero.
        columns: unsafe { NonZeroU32::new_unchecked(2) },
    };

    pub fn new(rows: u32, columns: u32) -> Result<Self, CfaError> {
        Ok(Self {
            rows: NonZeroU32::new(rows).ok_or(CfaError::InvalidCfaShape)?,
            columns: NonZeroU32::new(columns).ok_or(CfaError::InvalidCfaShape)?,
        })
    }

    pub const fn rows(self) -> u32 { self.rows.get() }
    pub const fn columns(self) -> u32 { self.columns.get() }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CfaColor {
    Red,
    Green,
    Blue,
    Other(u8),
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct CfaSlot {
    row: u32,
    column: u32,
}

impl CfaSlot {
    pub fn new(row: u32, column: u32, shape: CfaShape) -> Result<Self, CfaError> {
        if row >= shape.rows() || column >= shape.columns() {
            return Err(CfaError::InvalidCfaSlot);
        }
        Ok(Self { row, column })
    }

    pub const fn row(self) -> u32 { self.row }
    pub const fn column(self) -> u32 { self.column }

    pub fn index(self, shape: CfaShape) -> usize {
        (self.row as usize) * (shape.columns() as usize) + self.column as usize
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BayerSlot(u8);

impl BayerSlot {
    pub const fn from_local(row: u32, col: u32) -> Self {
        Self((((row & 1) << 1) | (col & 1)) as u8)
    }

    pub const fn index(self) -> u8 {
        self.0
    }

    pub const fn row(self) -> u32 {
        ((self.0 >> 1) & 1) as u32
    }

    pub const fn column(self) -> u32 {
        (self.0 & 1) as u32
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BayerPattern {
    Rggb,
    Bggr,
    Grbg,
    Gbrg,
}

impl BayerPattern {
    pub const fn shape(self) -> CfaShape {
        CfaShape::BAYER_2X2
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct BayerLayout {
    pattern: BayerPattern,
    active_area: ActiveArea,
}

#[derive(Debug, Eq, PartialEq)]
pub enum CfaError {
    InvalidActiveArea,
    InvalidCfaShape,
    InvalidCfaSlot,
    CoordOutsideActiveArea,
    UnsupportedLayout { observed: CfaShape },
}
```

The final implementation may place `PixelSize`, `PixelCoord`, and
`ActiveArea` in a shared geometry module. They are repeated here only to make
the CFA contract concrete.

## 3. Channel lookup and coordinates

`BayerPattern` owns only color order. `BayerLayout` adds the ActiveArea origin.
Both are required to map a stored sensor coordinate to a CFA slot.

The local slot is:

```text
local_row = (stored.row - active.top) mod 2
local_col = (stored.col - active.left) mod 2
slot_index = 2 * local_row + local_col
```

The phase lookup for one complete tile is:

| Pattern | slot 0 `(0,0)` | slot 1 `(0,1)` | slot 2 `(1,0)` | slot 3 `(1,1)` |
| --- | --- | --- | --- | --- |
| `RGGB` | Red | Green | Green | Blue |
| `BGGR` | Blue | Green | Green | Red |
| `GRBG` | Green | Red | Blue | Green |
| `GBRG` | Green | Blue | Red | Green |

Per-position black metadata indexes the four slots directly. This keeps the
two green positions distinct for preprocessing while exposing one
`CfaColor::Green` result to demosaicing.

Proposed methods:

```rust
impl BayerPattern {
    pub const fn color_at(self, slot: BayerSlot) -> CfaColor {
        match (self, slot.row(), slot.column()) {
            (Self::Rggb, 0, 0) | (Self::Bggr, 1, 1) |
            (Self::Grbg, 0, 1) | (Self::Gbrg, 1, 0) => CfaColor::Red,
            (Self::Rggb, 1, 1) | (Self::Bggr, 0, 0) |
            (Self::Grbg, 1, 0) | (Self::Gbrg, 0, 1) => CfaColor::Blue,
            _ => CfaColor::Green,
        }
    }
}

impl BayerLayout {
    pub fn new(pattern: BayerPattern, active_area: ActiveArea) -> Self {
        Self { pattern, active_area }
    }

    pub const fn pattern(self) -> BayerPattern { self.pattern }
    pub const fn active_area(self) -> ActiveArea { self.active_area }

    pub fn slot_at(&self, stored: PixelCoord) -> Result<BayerSlot, CfaError> {
        if !self.active_area.contains(stored) {
            return Err(CfaError::CoordOutsideActiveArea);
        }
        let row = (stored.row - self.active_area.top()) & 1;
        let col = (stored.col - self.active_area.left()) & 1;
        Ok(BayerSlot::from_local(row, col))
    }

    pub fn color_at(&self, stored: PixelCoord) -> Result<CfaColor, CfaError> {
        self.slot_at(stored).map(|slot| self.pattern.color_at(slot))
    }
}
```

The subtraction in `slot_at` is reached only after `contains` succeeds, so it
cannot underflow. Callers must validate `PixelCoord` against the stored buffer
separately; this type validates ActiveArea membership only.

### Stored-coordinate lookup

Some backends expose a phase already rebased to stored-buffer `(0,0)`.
Adapters may construct that representation only through an explicit conversion:

```text
stored_phase[(r, c)] =
    active_phase[(r - active.top) mod 2, (c - active.left) mod 2]
```

The backends' packed filter integers are not part of this model. DNG CFA tags,
rawler's plane mapping, and LibRaw's packed `filters` must be validated and
translated by their adapters.

## 4. Image-rs payload container

The workspace already depends on image-rs `0.25.9` at commit `d11aaf5`
(`Cargo.lock:910-912`). The inspected API offers two relevant containers:

- `ImageBuffer<Luma<T>, Vec<T>>` is an owned, row-major, tightly packed payload
  with width and height (`image src/images/buffer.rs:492-546`).
- `FlatSamples<Buffer>` stores an explicit `SampleLayout`, including height
  stride, and can represent padded, borrowed, or owned layouts
  (`image src/images/flat.rs:81-117,471-529`).

`ImageBuffer` is suitable as the compact stored payload container, but not as
the universal RAW type:

```rust,ignore
use image::{ImageBuffer, Luma};

pub type StoredU8Plane = ImageBuffer<Luma<u8>, Vec<u8>>;
pub type StoredU16Plane = ImageBuffer<Luma<u16>, Vec<u16>>;
pub type StoredF32Plane = ImageBuffer<Luma<f32>, Vec<f32>>;
pub type PreprocessedPlane = ImageBuffer<Luma<f32>, Vec<f32>>;

pub enum StoredPlane {
    U8(StoredU8Plane),
    U16(StoredU16Plane),
    F32(StoredF32Plane),
}
```

Constraints and recommendation:

- `ImageBuffer` is tightly packed and has no row-pitch field. A backend buffer
  with `row_stride != width * bytes_per_sample` must be copied row-by-row into
  a compact `ImageBuffer`; its original stride is retained in source
  provenance.
- `ImageBuffer::from_raw` accepts any container at least as large as
  `width * height`; callers must use exact-length containers or explicitly
  compact them first. Passing a padded raw allocation directly misinterprets
  row starts.
- `FlatSamples` is preferred for inspecting or borrowing backend buffers with
  padding. `FlatSamples::try_into_buffer` succeeds only for row-major packed
  layouts, which makes the compacting boundary explicit.
- `ImageBuffer` carries an inherent display color-space field initialized to
  sRGB. RAW use must remain an opaque payload limited to pixel/sample access;
  it must not call color-conversion, encoding, or display APIs. Preserve CFA,
  processing state, and color metadata in the separate universal model.
- `ImageBuffer` has no ActiveArea, CFA phase, black levels, processing state,
  or backend provenance. These remain wrapper metadata.
- `get_pixel_checked`/`get_pixel_mut_checked` should be used by the wrapper to
  avoid image-rs's panicking unchecked accessors.

Recommended boundary:

```text
backend borrowed/owned samples + source stride
    -> adapter validation (FlatSamples/view or checked copy)
    -> compact owned ImageBuffer<Luma<T>, Vec<T>>
    -> RawPlane wrapper carrying geometry, sample encoding, CFA, and state
```

This reuses image-rs for storage and checked pixel access without treating a
display-oriented image container as the RAW semantic model.

## 5. Source precision and preprocessing type

`u16` is a common integer storage container in the inspected decoder surfaces,
not a statement that every RAW sample has 16 bits of precision:

- LibRaw exposes single-plane `raw_image` as `ushort*` and separate float image
  pointers; source reports require preserving the actual sample encoding
  ([RES-02 §2.2](../../research/RES-02-libraw.md#22-decode-lifecycle-and-buffer-meaning)).
- rawloader stores `RawImageData::{Integer(Vec<u16>), Float(Vec<f32>)}`
  ([RES-06 §2.3](../../research/RES-06-rust-raw-decoders.md#23-rawloader)).
- rawler stores integer or floating samples and retains the DNG
  `BitsPerSample`/`SampleFormat`; DNG floating samples can declare 16, 24, or
  32 bits while the populated backend container remains `f32`
  ([RES-03 §2.2](../../research/RES-03-dnglab.md#22-dng-tag-mapping);
  [RES-06 §2.4](../../research/RES-06-rust-raw-decoders.md#24-rawler)).

Therefore:

- The decoded representation supports native `u8` and `u16` integer planes:
  `ImageBuffer<Luma<u8>, Vec<u8>>` for 8-bit sources and
  `ImageBuffer<Luma<u16>, Vec<u16>>` for 9–16-bit sources.
- The declared bit depth remains part of `SampleEncoding`, so 10-, 12-, 14-,
  and 16-bit sources are not silently promoted to semantic 16-bit data.
  Packed source values are unpacked without an extra left shift unless the
  source specification/decoder says otherwise.
- Floating DNG and another verified floating decoder path use
  `ImageBuffer<Luma<f32>, Vec<f32>>` natively. The decoder does not force a
  float DNG through `u16`.
- Integer widths that cannot fit losslessly in `u16` must use a verified wider
  variant or return a structured unsupported-encoding error. Signed integers
  and `f64` are not established initial decoder outputs.
- Preprocessing and demosaic input use
  `ImageBuffer<Luma<f32>, Vec<f32>>`.

Every native unsigned integer up to 16 bits is exactly representable in `f32`
because `f32` has 24 bits of significand precision. This makes an explicit
`u8`/`u16` to `f32` conversion lossless for the initial integer sources.
`f32` is not universally higher precision: integers above 24 significant bits
and some `f64` source values can lose information when stored as `f32`.

### Custom processing before preprocessing

The decoded source should expose these opt-in conversions without changing the
default storage:

```text
DecodedRaw -> into_f32_for_custom_processing() -> DecodedF32Raw
DecodedF32Raw -> user transformation -> CustomProcessedF32Raw
CustomProcessedF32Raw -> standard preprocessing (only when metadata basis matches)
```

Requirements:

- `u8` and `u16` conversion to `f32` is explicit, lossless, and preserves
  geometry, CFA, original bit depth, and provenance.
- An already-floating decoded plane is used as `f32` directly.
- A custom transformation returns a distinct processing state and operation
  provenance; it cannot masquerade as unmodified decoded data.
- Standard black/white preprocessing may follow only if the custom operation
  states that black/white metadata remains valid in the transformed domain.
  Otherwise the custom operation must provide transformed levels or standard
  preprocessing is rejected.

### Preprocessing precision

The standard preprocessing contract uses `f32` sample storage with widened
scalar arithmetic where needed:

- convert validated `u8`/`u16` source samples to `f32` before the preprocessed
  working buffer;
- retain native `f32` source samples without an additional quantizing
  round-trip;
- compute black/white level arithmetic in `f64` to avoid avoidable
  intermediate rounding;
- store unclipped corrected/normalized values as `f32` for `SYN-02`'s explicit
  clipping stage; and
- finalize the demosaic-ready mosaic as clamped `f32`.

High-bit-depth or `f64` inputs require an explicitly approved wider path; they
must not silently pass through `f32` with a false precision claim.

### Demosaic precision

Demosaicing should use `f32` input and output buffers, not `f64`:

- the selected Rust `demosaic` API already accepts single-channel `f32` and
  writes planar `f32` output
  ([RES-08 §2.1](../../research/RES-08-rust-demosaicing.md#21-demosaic-crate-api-and-bayer-contract));
- normalized initial samples occupy `[0,1]`, where `f32` has substantially
  finer spacing than a 16-bit source step;
- `f64` buffers double memory and bandwidth without improving the quantized
  f32 preprocessing input; and
- an individual algorithm may use `f64` accumulators or kernel intermediates
  only when benchmarks/tests justify the cost. This remains internal and does
  not change the shared `f32` input/output contract.

Higher-order demosaic algorithms can overshoot or undershoot the input range.
Their output remains linear `f32`; clipping is an explicit algorithm/output
policy, not a reason to switch to `f64`.

## 6. General CFA boundary

The universal metadata may need to preserve the observed CFA shape before
rejecting an unsupported sensor. The proposed shared shape type is `CfaShape`;
its dimensions are explicit and are not fixed to 2×2.

```text
decoder CFA metadata
    -> adapter validates standard 2x2 RGB Bayer
    -> BayerLayout

decoder CFA metadata, unsupported
    -> adapter returns UnsupportedLayout { observed: CfaShape }
```

`BayerLayout` is therefore deliberately a validated projection, not the only
CFA type known to the system. Demosaicing APIs accept `&BayerLayout`; future
layout variants can evolve through a separate enum or trait without changing
the meaning of this type. The initial adapter must not silently map a larger
CFA tile to four values.

The `Other(u8)` color exists to represent metadata while diagnosing malformed
or unsupported inputs. A valid `BayerLayout` can produce only `Red`, `Green`,
or `Blue`.

## 7. Validation and errors

| Condition | Result |
| --- | --- |
| Stored width or height is zero | `InvalidActiveArea` when used to construct that area; `PixelSize::new` returns `None` |
| ActiveArea is empty or exceeds stored dimensions | `InvalidActiveArea` |
| CFA repeat dimensions are zero | `InvalidCfaShape` |
| CFA repeat dimensions are not 2×2 | `UnsupportedLayout { observed }` |
| CFA contains non-RGB/unknown plane mapping | `UnsupportedLayout { observed }` with plane mapping retained in adapter provenance |
| CFA layout is non-rectangular | `UnsupportedLayout { observed }` |
| Coordinate is outside ActiveArea | `CoordOutsideActiveArea` |
| Slot coordinate is outside the pattern tile | `InvalidCfaSlot` |
| Custom processing changes sample basis without transformed black/white metadata | Structured preprocessing-basis error; original black levels are not applied |

During normal validated-pipeline operation, `BayerLayout::slot_at` never
returns `InvalidCfaSlot`: the modulo operation and constructor guarantee a
valid 2×2 slot. The method retains the checked constructor so misuse through
future constructors fails structurally rather than panicking.

No public CFA operation panics for malformed input. Indexing is checked by type
construction; unsupported layouts fail at the adapter boundary.

## 8. Test plan

Each supported phase must test the same properties:

1. The four local slots map to the expected color order in the table above.
2. ActiveArea origins `(0,0)`, `(1,0)`, `(0,1)`, `(1,1)`, and an odd origin
   such as `(3,5)` preserve the phase at the first active sample.
3. Sampling all stored coordinates in a 6×8 active region matches the formula
   `(row - top) mod 2`, `(col - left) mod 2`.
4. A coordinate immediately outside each ActiveArea edge returns
   `CoordOutsideActiveArea`.
5. A shifted stored-coordinate lookup agrees with direct ActiveArea lookup.
6. Two green slots remain distinguishable by slot index even though
   `color_at` returns `CfaColor::Green` for both.

The required pattern matrix is:

| Test | `RGGB` | `BGGR` | `GRBG` | `GBRG` |
| --- | --- | --- | --- | --- |
| Tile LUT | `R,G,G,B` | `B,G,G,R` | `G,R,B,G` | `G,B,R,G` |
| First active phase at odd origin | shifted LUT | shifted LUT | shifted LUT | shifted LUT |
| Full active sweep | yes | yes | yes | yes |
| Outside-active rejection | yes | yes | yes | yes |

Additional negative tests cover zero dimensions, reversed ActiveArea bounds,
out-of-buffer bounds, and malformed CFA shapes/planes.

Precision and container tests:

1. Every `u8` value converts exactly to `f32`.
2. Every `u16` value converts exactly to `f32`.
3. An 8-bit source remains semantic 8-bit after native `u8` storage or `f32`
   conversion; the container does not rewrite `bits_per_sample`.
4. A floating DNG enters the `f32` path without a `u16` round-trip.
5. Custom `f32` processing returns a distinct state and cannot be passed to
   standard preprocessing with stale black/white metadata.
6. Demosaic tests use `f32` input/output; no `f64` buffer is required by the
   shared interface.

## 9. Integration decisions and risks

- `DES-001` should own or re-export the shared pixel-size, coordinate, and
  rectangle types rather than duplicating them.
- `DES-004` can use the slot index for per-position black selection while
  using `CfaColor` for channel interpretation.
- `DES-004` owns the custom-processing state/provenance guard that determines
  whether standard black/white preprocessing remains valid after a user
  transformation.
- `DES-005` should accept a validated Bayer layout and the preprocessed mosaic;
  it should not recover CFA phase from buffer-local coordinates.
- Backends must retain CFA plane mapping and ActiveArea origin during adapter
  conversion. Rawler's common RGB assumption and LibRaw's packed filters are
  not sufficient universal representations.
- This proposal does not decide whether future CfaPattern variants are
  represented by an enum or trait. It preserves `CfaShape` as the general
  boundary so adding those variants does not require pretending they are 2×2.

## 10. Open questions

- Should `PixelSize`, `PixelCoord`, and `ActiveArea` live in the universal
  representation module or a shared geometry module?
- Should the compact payload use image-rs `ImageBuffer` directly or an opaque
  one-field wrapper that prevents accidental display/color operations?
- Should a stored-coordinate phase be cached on `BayerLayout`, or recomputed
  from ActiveArea for each lookup?
- How should adapter provenance retain non-Bayer plane mappings after
  structured rejection?
- Does the final demosaicing API need a borrowed tile LUT for hot loops, or are
  slot lookups sufficient after validation?

## 11. Evidence

- [SYN-01 decision table](../../research/notes/SYN-01.md#decision-table):
  stored coordinates, ActiveArea, CFA phase, per-position black slots, and
  universal/backend-specific boundaries.
- [SYN-01 conflict log](../../research/notes/SYN-01.md#conflict-log): DNG CFA
  origin, plane mapping, and crop-phase conflicts.
- [RES-03 §2.2–§2.3](../../research/RES-03-dnglab.md#22-dng-tag-mapping):
  CFA tag types, ActiveArea origin, half-open rectangles, and rawler phase
  conversion.
- [RES-06 §2.3–§2.4](../../research/RES-06-rust-raw-decoders.md#23-rawloader):
  rawloader/rawler CFA representations and unsupported-layout risks.
- `Cargo.lock:910-912`: image-rs `0.25.9` at
  `d11aaf5f12c948606d5865cacb30da016ef175c2`.
- image-rs `src/images/buffer.rs:492-546`: owned row-major `ImageBuffer`,
  dimensions, raw-container acceptance, and checked pixel access.
- image-rs `src/images/flat.rs:81-117,471-529,827-851`: explicit sample
  strides, `FlatSamples`, and packed-buffer conversion requirements.
- [RES-02 §2.2](../../research/RES-02-libraw.md#22-decode-lifecycle-and-buffer-meaning);
  [RES-03 §2.2](../../research/RES-03-dnglab.md#22-dng-tag-mapping);
  [RES-06 §2.3–§2.4](../../research/RES-06-rust-raw-decoders.md#23-rawloader):
  native integer/float storage and bit-depth provenance.

## Acceptance checklist

- [x] Concrete types, methods, coordinate conventions, and index formulas are
  specified.
- [x] Bounds and validation behavior are explicit.
- [x] RGGB, BGGR, GRBG, and GBRG tests are planned for every supported
  pattern.
- [x] Shared APIs do not hide a 2×2-only assumption.
- [x] Decisions cite merged SYN-01 evidence.
- [x] Image-rs container suitability and row-stride limitations are evaluated.
- [x] Native u8/u16/f32 storage, explicit custom-processing conversion, and
  preprocessing/demosaic precision are justified from decoder evidence.
