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

## 1. Scope and decisions

This proposal defines the typed CFA model used between decoding,
CFA-dependent preprocessing, and Bayer demosaicing. It does not define the
sample buffer, black-level values, decoder adapter API, or demosaicing
algorithms.

The design follows the [SYN-01 decision table](../../research/notes/SYN-01.md#decision-table):

- initial pipeline support is limited to conventional 2×2 RGB Bayer phases;
- DNG CFA phase originates at ActiveArea top-left;
- stored coordinates, ActiveArea coordinates, and crop coordinates remain
  distinct; and
- an unsupported layout is a structured error rather than a reinterpretation.

The proposed names are design candidates. They are not final public API until
`SYN-03` and `DES-007` approve the consolidated design.

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

## 4. General CFA boundary

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

## 5. Validation and errors

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

During normal validated-pipeline operation, `BayerLayout::slot_at` never
returns `InvalidCfaSlot`: the modulo operation and constructor guarantee a
valid 2×2 slot. The method retains the checked constructor so misuse through
future constructors fails structurally rather than panicking.

No public CFA operation panics for malformed input. Indexing is checked by type
construction; unsupported layouts fail at the adapter boundary.

## 6. Test plan

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

## 7. Integration decisions and risks

- `DES-001` should own or re-export the shared pixel-size, coordinate, and
  rectangle types rather than duplicating them.
- `DES-004` can use the slot index for per-position black selection while
  using `CfaColor` for channel interpretation.
- `DES-005` should accept a validated Bayer layout and the preprocessed mosaic;
  it should not recover CFA phase from buffer-local coordinates.
- Backends must retain CFA plane mapping and ActiveArea origin during adapter
  conversion. Rawler's common RGB assumption and LibRaw's packed filters are
  not sufficient universal representations.
- This proposal does not decide whether future CfaPattern variants are
  represented by an enum or trait. It preserves `CfaShape` as the general
  boundary so adding those variants does not require pretending they are 2×2.

## 8. Open questions

- Should `PixelSize`, `PixelCoord`, and `ActiveArea` live in the universal
  representation module or a shared geometry module?
- Should a stored-coordinate phase be cached on `BayerLayout`, or recomputed
  from ActiveArea for each lookup?
- How should adapter provenance retain non-Bayer plane mappings after
  structured rejection?
- Does the final demosaicing API need a borrowed tile LUT for hot loops, or are
  slot lookups sufficient after validation?

## 9. Evidence

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

## Acceptance checklist

- [x] Concrete types, methods, coordinate conventions, and index formulas are
  specified.
- [x] Bounds and validation behavior are explicit.
- [x] RGGB, BGGR, GRBG, and GBRG tests are planned for every supported
  pattern.
- [x] Shared APIs do not hide a 2×2-only assumption.
- [x] Decisions cite merged SYN-01 evidence.
