# Rust RAW-Processing Ecosystem

> **Task:** `RES-06`
> **Owner:** primary agent
> **Status:** `paused` (evidence and draft acceptance review complete; task
> closure deferred by user request)
> **Last updated:** 2026-10-23
> **Upstream versions:** rawloader `8f2ace2d27c7fa6e7671a7fd0ef11d1b1bd77ddf`;
> rawler as part of dnglab `62b77de1a206d467e910d55199191068b45a9cc1`;
> image-tiff `3302936b08e2260d4c66f40f51170efbb1fded4c`; demosaic
> `5050131e998b7f6414eef02eeb60f74b99d77451`; MeraRAW Demosaic
> `b94ebd4324ce56218e42989a5ed852b32a52003f`.
>
> **Acceptance criteria (authoritative version in the task tracker):**
>
> - [x] Each surveyed project has a source-verified summary of its sample
>   representation, metadata model, CFA handling, and preprocessing.
> - [x] Reusable ideas and reusable code are distinguished, with license
>   constraints recorded.
> - [x] Unmaintained or unsuitable projects are named with a reason.
> - [x] Inferred findings are labeled `[inference]`.
> - [x] Coverage does not duplicate `RES-02` or `RES-03`.
> - [x] The report contains the six required sections and cites sources.
>
> **Acceptance review snapshot:** Primary agent, 2026-10-23. Project coverage, sample and
> metadata models, CFA/preprocessing behavior, reuse boundaries, maintenance
> evidence, unsuitable candidates, and licenses were checked against the
> pinned checkouts. Final task closure was paused before status synchronization.

## 1. Scope and questions

This report surveys maintained Rust projects relevant to RAW decoding, TIFF/DNG
containers, demosaicing, and pipeline API design. It does not repeat LibRaw
internals from `RES-02` or DNG tag semantics from `RES-03`; rawler is examined
here at the public representation/API level.

The questions were:

- Which maintained Rust projects expose RAW samples, CFA metadata, or demosaicing?
- What does each project retain and normalize at its public boundary?
- What is reusable as code, and what is only reusable as a design idea?
- What dependency, maintenance, and licensing constraints apply?
- Which projects are unsuitable as the foundation of `fornax`?
- Which architecture choices should `fornax` adopt or reject?

## 2. Findings

### 2.1 Project summary

| Project | Revision / date | License | Status and role |
| --- | --- | --- | --- |
| rawloader | `8f2ace2d27c7fa6e7671a7fd0ef11d1b1bd77ddf`, 2026-08-16 | LGPL-2.1 | Maintained pure-Rust RAW decoder with many camera formats. Minimal metadata beyond black/white/WB/CFA/crop/orientation. |
| rawler (inside dnglab) | `62b77de1a206d467e910d55199191068b45a9cc1`, 2026-10-23 | LGPL-2.1 | Actively maintained RAW/DNG decoder and developer with a richer `RawImage`, rational black levels, active/default crop, color matrices, opcodes, and demosaic algorithms. |
| image-rs/image-tiff | `3302936b08e2260d4c66f40f51170efbb1fded4c`, 2026-09-06 | MIT | Maintained generic TIFF decoder/encoder used by image-rs. Good container primitives, but not a RAW metadata model. |
| `demosaic` crate | `5050131e998b7f6414eef02eeb60f74b99d77451`, 2026-02-18 | MIT OR Apache-2.0 | Maintained algorithm-focused crate for Bayer, Quad Bayer, and X-Trans. No file decoding or preprocessing metadata. |
| MeraRAW Demosaic | `b94ebd4324ce56218e42989a5ed852b32a52003f`, 2026-07-22 | Apache-2.0 | Early but active application/engine. Useful trait/registry/testing reference; includes a full RAW pipeline beyond `fornax` scope. |

Maintenance is source-based only: each checkout has recent commits and no local
modifications. No published release-support policy was inferred.

### 2.2 rawloader

`[verified]` The public API is `decode_file` / `decode` returning
`RawImage` with string errors (`src/lib.rs:78-136`). `RawImage` contains:

- make/model strings and cleaned names;
- full width, height, and components per pixel;
- four WB coefficients in RGBE order;
- four `u16` white and black levels in RGBE order;
- a 4x3 XYZ-to-camera matrix;
- a general `CFA` pattern;
- crop borders in top/right/bottom/left order;
- masked black-area rectangles;
- `Orientation`;
- `RawImageData::{Integer(Vec<u16>), Float(Vec<f32>)}`.

Sources: `src/decoders/image.rs:5-51,55-112`.

`[verified]` `CFA` stores a pattern name, width, height, color count, and a
48x48 expanded lookup table. Pattern size is inferred from string length:
2x2, 6x6, 2x8, or 12x12; other lengths panic in `CFA::new`
(`src/decoders/cfa.rs:21-112`).

`[verified]` rawloader does not define a stride separate from width, a typed
sample-domain/processing state, arbitrary black-level patterns, active-area
origin as a tagged type, or structured decoding error categories. Integer and
float data are either fully stored codes or decoder-produced values depending on
format; the representation does not identify the domain beyond the enum type.

`[inference]` rawloader is useful as a decoder backend/reference but should not
be adopted as the universal representation. Its fixed four-level metadata and
unstructured errors cannot express the DNG and vendor metadata needed by the
target preprocessing semantics.

### 2.3 rawler

`[verified]` rawler exposes `RawImage` with:

- integer or float sample storage;
- width, height, channels, camera names, WB, white level, and rational
  `BlackLevel {levels, cpp, width, height}`;
- CFA plus plane-color mapping;
- `RawPhotometricInterpretation::{BlackIsZero, Cfa, LinearRaw}`;
- `active_area`, `crop_area`, black areas, orientation;
- color matrices, preserved DNG tags, and three opcode lists;
- linearization table and capture metadata.

Sources: `rawler/src/rawimage.rs:203-276`; DNG-specific fields were covered in
`RES-03`.

`[verified]` `RawImage::apply_scaling` applies linearization, converts samples
to float, applies black/white normalization, and resets the black/white metadata
(`rawler/src/rawimage.rs:546-576`). This mutating API is an important
double-correction hazard for a universal pipeline.

`[verified]` rawler defines a demosaicing trait over a mosaic, CFA, plane-color
mapping, and ROI, returning an `N`-channel `Color2D<T,N>`
(`rawler/src/imgop/sensor/mod.rs:49-66`). Implementations include Bayer
bilinear, PPG, superpixel, and X-Trans bilinear/Markesteijn. Unsupported CFA
layouts can panic inside algorithm dispatch rather than returning one common
algorithm error (`rawler/src/imgop/sensor/mod.rs:29-43`;
`rawler/src/imgop/sensor/bayer/ppg.rs:35-50`).

`[inference]` rawler is the strongest evidence source for a rich adaptive
decoder, but its monolithic mutating `RawImage` and panic-prone algorithm
preconditions should not be copied wholesale. `fornax` should use its specific
ideas while keeping sample metadata and processing state explicit.

### 2.4 image-rs/image-tiff

`[verified]` image-tiff provides a typed generic TIFF `Decoder<R: Read + Seek>`,
image dimensions, a `ColorType`, `Tag`-keyed directories, sample formats, limits,
row/plane stride preferences, strip/tile decoding, and structured
`TiffError`/format/unsupported/usage errors
(`src/decoder/mod.rs:15-100,246,861-900,1473-1500,1845-1945`;
`src/tags.rs:80-120;187-400`).

`[verified]` Its color model is generic TIFF/display imagery: gray, RGB, palette,
RGBA, CMYK, YCbCr, Lab, and multiband (`src/lib.rs:22-83`). It does not expose
RAW CFA semantics, active/default crop, black levels, or preprocessing state.
The package is MIT licensed (`Cargo.toml`).

`[inference]` image-tiff is a credible low-level dependency for parsing TIFF IFD
structure and sample layout, including a DNG container, but it cannot be the RAW
representation or replace a DNG adapter. Adding it would still require careful
support for DNG-specific photometric values, metadata, compression, and
provenance.

### 2.5 `demosaic` crate

`[verified]` The `demosaic` crate exposes a buffer-oriented `f32` API:
single-channel row-major CFA input, dimensions, `CfaPattern`, an `Algorithm`
enum, and caller-provided planar output. It validates input/output lengths and
returns typed errors for size mismatch, unsupported combinations, and small
X-Trans images (`src/lib.rs:137-270`).

`[verified]` It supports Bayer bilinear, MHC, PPG, AHD, and VNG; Quad Bayer
bilinear/Quad-PPG; and X-Trans bilinear/Markesteijn/DHT. CFA patterns include a
shift operation for crop offsets (`README.md`; `src/cfa.rs`). It does not parse
RAW files, preprocess black levels, or retain metadata.

`[inference]` Its algorithm/buffer separation is a useful API precedent, but
its single `Algorithm` enum couples the registry to the crate. `fornax` should
keep independently implementable strategies without making shared types depend
on one closed enum.

### 2.6 MeraRAW Demosaic

`[verified]` MeraRAW separates a core RAW/image model, a demosaic crate, I/O, and
UI crates. Its demosaic trait has stable ID/name/description, supported CFA,
cost, minimum size, implementation status, and `run` returning a typed result.
`check` validates pattern support, minimum dimensions, and buffer length
(`crates/miro-demosaic/src/lib.rs:48-145`).

`[verified]` `DemosaicOptions` keeps shared post-processing controls and
algorithm-specific tuning outside individual algorithm implementations, and
the registry supports stubs and runtime capability inspection
(`crates/miro-demosaic/src/lib.rs:145-275`).

`[verified]` Its application pipeline is explicitly
`crop -> white balance -> demosaic -> camera-to-XYZ -> ... -> display`
(`README.md`, "Architecture"). That is broader than `fornax`: white balance,
color conversion, tone mapping, and display orientation are outside the target
pipeline.

`[inference]` MeraRAW is useful for trait shape, validation, registry, and
synthetic-quality-test ideas. Its project is young, some algorithms are stubs
or carried from a prior engine, and its full pipeline decisions must not be
imported as `fornax` scope.

### 2.7 Unsuitable or limited projects

`[verified]` No explicitly unmaintained repository was included solely to
satisfy coverage. Two maintained projects are unsuitable as the central
representation:

- **image-tiff** is a general TIFF library, not a RAW metadata/preprocessing
  model.
- **MeraRAW as a whole** is an application with a wider processing pipeline and
  a young API; only its demosaic abstractions and test ideas are relevant.

`[inference]` rawloader is limited rather than unmaintained: it remains active
but exposes too little metadata for the target universal model. rawler is
feature-rich but cannot be treated as a drop-in universal layer because of its
mutating state, panics, and LGPL-2.1 license.

### 2.8 Reusable code versus reusable ideas

| Project | Reusable code | Reusable ideas | Constraint |
| --- | --- | --- | --- |
| rawloader | Potentially, behind an adapter | Format-specific decoder catalog; simple CFA shift; clean camera naming | LGPL-2.1; fixed `[u16;4]` levels; string errors; incomplete metadata |
| rawler | Potentially as an optional decoder backend | Rational black tiles, active/default crop separation, CFA plane mapping, typed photometric interpretation, ROI demosaic trait | LGPL-2.1; mutating scaling; panic-prone algorithm APIs |
| image-tiff | Yes, as a low-level TIFF dependency | Explicit row/plane stride, decoding limits, structured errors, strip/tile API | MIT; no RAW/CFA semantics |
| `demosaic` crate | Potentially for benchmark comparison or license-compatible algorithms | Buffer validation, planar/interleaved output conversions, algorithm enum/trait lessons | MIT/Apache-2.0; no metadata or preprocessing |
| MeraRAW | Possible Apache-2.0 components after audit | Stateless demosaic trait, registry/capability reporting, synthetic ground-truth tests | Apache-2.0; app scope and young status |

`[recommendation]` Prefer independent shared types in `fornax`, with decoder
adapters for any optional backend. Reusing a whole ecosystem RAW type would
leak format-specific assumptions and licensing constraints into the core.

### 2.9 Testing and API observations

`[verified]` `demosaic` and MeraRAW both expose buffer-size, CFA-support, and
dimension errors rather than panicking. MeraRAW adds synthetic mosaicing, PSNR,
SSIM, zipper metrics, and tiled/untiled comparison (`crates/miro-demosaic/src/testing.rs`,
`tests/correctness.rs`).

`[inference]` These are directly applicable to `fornax` test design:

- generate known RGB ground truth;
- mosaic it with each supported CFA phase;
- demosaic;
- compare deterministic metrics and selected exact pixels;
- test invalid buffers and unsupported layouts;
- compare tiled and untiled paths;
- keep visual checks supplementary.

## 3. Evidence

| Finding | Evidence |
| --- | --- |
| rawloader public decode and `RawImage` | `rawloader/src/lib.rs:78-136`; `src/decoders/image.rs:5-112` |
| rawloader CFA and pattern-size limits | `rawloader/src/decoders/cfa.rs:21-112` |
| rawloader license/revision | `rawloader/Cargo.toml`; `LICENSE`; Git commit |
| rawler rich representation | dnglab `rawler/src/rawimage.rs:203-276` |
| rawler mutating scaling and double-correction risk | dnglab `rawler/src/rawimage.rs:546-576` |
| rawler demosaic trait and panic behavior | dnglab `rawler/src/imgop/sensor/mod.rs:49-66`; Bayer implementations |
| image-tiff types, errors, strides, limits | image-tiff `src/decoder/mod.rs:15-100,1473-1500,1845-1945`; `src/tags.rs` |
| image-tiff generic color/RAW omission | image-tiff `src/lib.rs:22-83`; `Cargo.toml` |
| `demosaic` public buffer API | demosaic crate `src/lib.rs:137-270` |
| `demosaic` algorithms and license | demosaic crate `README.md`; `Cargo.toml` |
| MeraRAW demosaic trait/options/registry | MeraRAW `crates/miro-demosaic/src/lib.rs:48-275` |
| MeraRAW broader pipeline | MeraRAW `README.md`, "Architecture" |
| Synthetic quality test approach | MeraRAW `crates/miro-demosaic/src/testing.rs`; `tests/correctness.rs` |

All paths are relative to their checkout under
`agent-work/references/other/` or `agent-work/references/dnglab/`.

## 4. Implications for fornax

`[recommendation]` Keep `fornax-core` independent of rawloader, rawler,
image-tiff, and all decoder-specific types. Any dependency should sit behind a
decoder adapter or a dedicated optional TIFF/DNG parsing crate after design.

`[recommendation]` Define one universal RAW sample/metadata model with explicit
storage domain, dimensions/stride, CFA mapping and origin, black/white metadata,
active/default crop, orientation, provenance, and processing state. No surveyed
type satisfies all of those requirements.

`[recommendation]` Model decoder failures as structured errors. Avoid public
panics for unsupported CFA patterns and buffer mismatches; `demosaic` and MeraRAW
demonstrate the value of validation before algorithm dispatch.

`[recommendation]` Keep demosaicing strategies open for independent
implementation. A stateless algorithm trait/registry or explicit strategy
object can learn from MeraRAW, but final object safety, allocation, options, and
metadata-return decisions belong to design after synthesis.

`[recommendation]` Consider image-tiff only if a later DNG implementation needs
a maintained low-level TIFF parser. Do not add it merely because DNG is
TIFF-based; the explored DNG-specific decoder and tag requirements remain the
larger part of the work.

`[recommendation]` Adopt the synthetic-ground-truth and tiled/untiled test
patterns, but avoid importing the full MeraRAW/darktable processing pipeline.

`[recommendation]` Treat licenses as architectural constraints:

- image-tiff (MIT), `demosaic` (MIT/Apache-2.0), and MeraRAW (Apache-2.0) are
  the most straightforward code dependencies;
- rawloader/rawler are LGPL-2.1 and require a deliberate linking/distribution
  decision;
- darktable is GPL-3.0 and should remain an evidence source unless licensing
  changes.

## 5. Open questions and limitations

- No exhaustive crates.io or GitHub census was attempted. The survey covers the
  maintained projects relevant to the target decode/preprocess/demosaic stack.
- rawler's complete error and panic surface was not exhaustively audited; only
  the public representation and representative demosaic preconditions were
  inspected.
- image-tiff may support enough raw TIFF structures to parse parts of DNG, but
  no end-to-end DNG decode experiment was performed.
- The `demosaic` crate and MeraRAW have different algorithm registries and
  maturity levels. Comparative quality was not independently reproduced.
- No published maintenance policy or long-term support commitment was found for
  the surveyed projects; commit recency is the only maintenance evidence.
- MeraRAW's README contains implementation-status claims that may not equal
  every source path's implementation state; the report relies on the inspected
  trait/API and test architecture, not on its marketing table for algorithm
  guarantees.
- No direct dependency decision is finalized here. Crate architecture and
  licensing implementation choices belong to design tasks after synthesis.

## 6. References

- rawloader: `https://github.com/pedrocr/rawloader.git` at
  `8f2ace2d27c7fa6e7671a7fd0ef11d1b1bd77ddf`.
  - Local checkout: `agent-work/references/other/rawloader/`.
  - `src/lib.rs`, `src/decoders/image.rs`, `src/decoders/cfa.rs`.
  - License: LGPL-2.1.
- rawler / dnglab: `https://github.com/dnglab/dnglab.git` at
  `62b77de1a206d467e910d55199191068b45a9cc1`.
  - Local checkout: `agent-work/references/dnglab/rawler/`.
  - `src/rawimage.rs`, `src/imgop/sensor/`, `src/imgop/develop.rs`.
  - License: LGPL-2.1.
- image-rs/image-tiff: `https://github.com/image-rs/image-tiff.git` at
  `3302936b08e2260d4c66f40f51170efbb1fded4c`.
  - Local checkout: `agent-work/references/other/image-tiff/`.
  - `src/lib.rs`, `src/decoder/`, `src/tags.rs`.
  - License: MIT.
- `demosaic`: `https://github.com/naorunaoru/demosaic.git` at
  `5050131e998b7f6414eef02eeb60f74b99d77451`.
  - Local checkout: `agent-work/references/other/demosaic-crate/`.
  - `src/lib.rs`, `src/bayer/`, `src/xtrans/`, tests and benchmarks.
  - License: MIT OR Apache-2.0.
- MeraRAW Demosaic: `https://github.com/kaimai-apex/meraraw-demosaic.git` at
  `b94ebd4324ce56218e42989a5ed852b32a52003f`.
  - Local checkout: `agent-work/references/other/meraraw-demosaic/`.
  - `crates/miro-demosaic/`, `crates/miro-core/`, correctness tests.
  - License: Apache-2.0.
- darktable (demosaic and pipeline cross-check only):
  `https://github.com/darktable-org/darktable.git` at
  `6556cd7dbb0e69f95d165dda8b69e7270c327f6d`; GPL-3.0.
