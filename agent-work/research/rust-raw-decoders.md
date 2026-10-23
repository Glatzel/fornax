# Rust RAW Decoder Crates

> **Task:** `RES-06`
> **Owner:** primary agent
> **Status:** `paused` (split evidence preserved; acceptance review deferred)
> **Last updated:** 2026-10-23
> **Upstream versions:** rawloader
> `8f2ace2d27c7fa6e7671a7fd0ef11d1b1bd77ddf`; rawler as part of dnglab
> `62b77de1a206d467e910d55199191068b45a9cc1`.
>
> **Acceptance criteria (authoritative version in the task tracker):**
>
> - [x] Each decoder has a source-verified sample and metadata model.
> - [x] Coverage of DNG, Sony ARW, Nikon NEF, and Canon `CR*` is explicit.
> - [x] Conventional Bayer behavior and non-Bayer handling are explicit.
> - [x] Reusable code and ideas are distinguished with license constraints.
> - [x] Maintenance and adapter risks are supported by evidence.
> - [x] Inferred findings are labeled `[inference]`.
> - [x] The report contains the six required sections and cites sources.

## 1. Scope and questions

This report evaluates Rust RAW decoder crates as optional backends or as design
evidence for the conventional Bayer pipeline. It does not repeat LibRaw
internals (`RES-02`), DNG tag semantics (`RES-03`), TIFF parsing internals
(`RES-07`), or demosaicing APIs (`RES-08`).

The questions were:

- What does each decoder expose at its public boundary?
- Does it preserve stored sensor codes, crop/CFA alignment, and preprocessing
  state separately?
- Which initial target formats are verified for the exact inspected revision?
- How does each decoder handle non-Bayer payloads that `fornax` defers?
- What can be reused as code versus only as an API or test idea?

## 2. Findings

### 2.1 Target-format evidence

| Decoder | DNG | Sony ARW | Nikon NEF | Canon `CR*` | Evidence level |
| --- | --- | --- | --- | --- | --- |
| rawloader | Unknown | Unknown | Unknown | Unknown | The audit inspected the public decoder API and catalog shape, not one fixture per target format. |
| rawler | Unknown for the exact Bayer fixture matrix | Unknown | Unknown | Unknown | DNG internals were inspected under `RES-03`, but a target-format decode matrix was not run. |

The API and representation findings below are source-verified. The absence of a
fixture run means format support must not be inferred from a catalog name alone.

### 2.2 rawloader

`[verified]` The public API is `decode_file` / `decode`, returning `RawImage`
with string errors (`src/lib.rs:78-136`). `RawImage` contains:

- make/model strings and cleaned names;
- full width, height, and components per pixel;
- four WB coefficients in RGBE order;
- four `u16` white and black levels in RGBE order;
- a 4x3 XYZ-to-camera matrix;
- a general CFA pattern;
- crop borders in top/right/bottom/left order;
- masked black-area rectangles;
- orientation; and
- `RawImageData::{Integer(Vec<u16>), Float(Vec<f32>)}`.

Source: `src/decoders/image.rs:5-51,55-112`.

`[verified]` `CFA` stores a pattern name, width, height, color count, and an
expanded lookup table. Pattern size is inferred from string length: 2x2, 6x6,
2x8, or 12x12; other lengths panic (`src/decoders/cfa.rs:21-112`). This includes
layouts outside the conventional Bayer target.

`[verified]` rawloader does not expose an explicit stride, a typed sample-domain
or processing state, arbitrary black-level patterns, an active-area type, or
structured decoder error categories. Integer and float buffers do not identify
whether values are stored codes or decoder-produced samples.

`[inference]` rawloader is useful behind an adapter or as a format-catalog
reference, but its fixed four-level metadata and string errors are insufficient
for the universal representation.

### 2.3 rawler

`[verified]` rawler's `RawImage` contains integer or float samples; dimensions,
channels, camera names, WB, white level, and rational
`BlackLevel {levels, cpp, width, height}`; CFA plus plane-color mapping;
`RawPhotometricInterpretation::{BlackIsZero, Cfa, LinearRaw}`; active and crop
areas; black areas; orientation; color matrices; preserved DNG tags; opcode
lists; a linearization table; and capture metadata
(`rawler/src/rawimage.rs:203-276`).

`[verified]` `RawImage::apply_scaling` applies linearization, converts samples
to float, applies black/white normalization, and resets black/white metadata
(`rawler/src/rawimage.rs:546-576`). This mutating method is a double-correction
hazard for a staged API.

`[verified]` rawler's demosaicing interface is separate from the RAW image model,
but unsupported CFA dispatch can panic rather than return one common algorithm
error (`rawler/src/imgop/sensor/mod.rs:29-66`;
`rawler/src/imgop/sensor/bayer/ppg.rs:35-50`).

`[inference]` rawler is the richest decoder evidence in this split, but its
monolithic mutable state and panic-prone preconditions should inform, not define,
`fornax`'s core types.

### 2.4 Reuse and licensing

| Decoder | Reusable code | Reusable ideas | Constraint |
| --- | --- | --- | --- |
| rawloader | Potentially behind an adapter | Camera catalog, CFA shift, clean camera naming | LGPL-2.1; fixed four levels; string errors; incomplete metadata |
| rawler | Potentially as an optional backend | Rational black tiles, active/default crop split, CFA plane mapping, photometric interpretation | LGPL-2.1; mutating scaling; panic-prone APIs |

`[recommendation]` Keep decoder-specific types out of `fornax-core`. Any
rawloader/rawler dependency is an adapter or optional backend decision that must
pass the architectural and license review in `RES-09`.

## 3. Evidence

| Finding | Evidence |
| --- | --- |
| rawloader public decode and `RawImage` | `rawloader/src/lib.rs:78-136`; `src/decoders/image.rs:5-112` |
| rawloader CFA limits | `rawloader/src/decoders/cfa.rs:21-112` |
| rawloader license | `rawloader/Cargo.toml`; `LICENSE` |
| rawler public representation | dnglab `rawler/src/rawimage.rs:203-276` |
| rawler mutating scaling | dnglab `rawler/src/rawimage.rs:546-576` |
| rawler demosaic preconditions | dnglab `rawler/src/imgop/sensor/mod.rs:29-66`; Bayer implementations |
| rawler license | dnglab `rawler/Cargo.toml`; repository license |

All paths are relative to clean pinned submodules under
`agent-work/references/`.

## 4. Implications for fornax

`[recommendation]` The decoder boundary must expose stored sample domain,
dimensions/stride, CFA layout and origin, crop/active areas, black/white
metadata, orientation, provenance, and processing state explicitly.

`[recommendation]` Do not infer target-format support from upstream camera
catalog breadth. Add fixture-based support claims for DNG, ARW, NEF, and `CR*`
only after decode and metadata validation.

`[recommendation]` X-Trans, Foveon, Quad Bayer, four-color, and monochrome paths
are deferred. A decoder adapter may encounter them, but the `fornax` boundary
must reject them with a structured unsupported-layout error.

## 5. Open questions and limitations

- The exact target-format and camera coverage of each decoder was not verified
  through real fixture decoding.
- rawler's complete error and panic surface was not audited.
- No license decision is finalized here.
- A decoder crate's successful parse does not prove CFA or black-level
  correctness; those semantics require separate synthesis and tests.

## 6. References

- rawloader: `https://github.com/pedrocr/rawloader.git` at
  `8f2ace2d27c7fa6e7671a7fd0ef11d1b1bd77ddf`; LGPL-2.1.
- rawler / dnglab: `https://github.com/dnglab/dnglab.git` at
  `62b77de1a206d467e910d55199191068b45a9cc1`; LGPL-2.1.
- Pinned checkouts: `agent-work/references/other/rawloader/` and
  `agent-work/references/dnglab/rawler/`.
