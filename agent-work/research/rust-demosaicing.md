# Rust Demosaicing Crates and Interfaces

> **Task:** `RES-08`
> **Owner:** RES-08 research agent / primary agent review
> **Status:** `review` (ready for user review)
> **Last updated:** 2026-10-23
> **Upstream versions:** `naorunaoru/demosaic`
> `5050131e998b7f6414eef02eeb60f74b99d77451` (2026-02-18);
> `kaimai-apex/meraraw-demosaic`
> `b94ebd4324ce56218e42989a5ed852b32a52003f` (2026-07-22).
>
> **Acceptance criteria (authoritative version in the task tracker):**
>
> - [x] Bayer input/output and border contracts are source-verified.
> - [x] Validation, errors, and allocation behavior are documented.
> - [x] Reusable code and ideas are distinguished with licenses.
> - [x] Maintenance and maturity are supported by evidence.
> - [x] X-Trans, Foveon, Quad Bayer, and other non-target modes are excluded
>   from the initial recommendation.
> - [x] The report contains the six required sections and cites sources.
>
> **Review snapshot:** RES-08 research agent and primary checks completed
> 2026-10-23; user review is pending. Bayer API contracts, border behavior,
> validation, allocation, registry patterns, synthetic-test designs,
> maintenance, and licenses were checked against the pinned revisions.

## 1. Scope and questions

This report evaluates Rust demosaicing crates and API patterns for the initial
conventional 2x2 RGB Bayer pipeline. Algorithm quality and the initial algorithm
set are owned by `RES-05`; this report focuses on API contracts, borders,
validation, allocation, test design, maintenance, and licensing.

The questions were:

- What Bayer input and output types do the crates expose?
- How do they validate dimensions, patterns, and buffers?
- How are borders and small images handled?
- Which operations allocate, and which failures are typed versus panics?
- Which implementation parts and interface ideas are reusable?
- What is the evidence for maintenance and maturity?

The following are deferred and excluded from the initial recommendation:

| Mode | Treatment |
| --- | --- |
| Conventional 2x2 RGB Bayer | In scope. |
| Fujifilm X-Trans | Deferred. Present in references only to identify unsupported paths. |
| Sigma Foveon/X3 | Deferred; neither reference supplies a Foveon demosaic contract. |
| Quad Bayer, four-color CFA, monochrome | Deferred. |

## 2. Findings

### 2.1 `demosaic` crate API and Bayer contract

`[verified]` `demosaic()` accepts a single-channel row-major `f32` mosaic,
width/height, `CfaPattern`, an `Algorithm`, and a caller-owned planar `f32`
output. The expected input length is `width * height`; output layout is
`[R plane, G plane, B plane]` with length `3 * width * height`
(`src/lib.rs:131-175`).

`[verified]` `CfaPattern` provides all four conventional 2x2 Bayer phases through
constructors and includes a crop-shift operation. `color_at` wraps coordinates
modulo the pattern dimensions (`src/cfa.rs:46-74,150-194`).

`[verified]` Before algorithm dispatch, `demosaic()` validates exact input and
output buffer lengths and rejects algorithm/CFA combinations with typed
`DemosaicError` variants. The core Bayer dispatch supports bilinear, MHC, PPG,
AHD, and VNG (`src/lib.rs:140-232`; `src/error.rs:3-63`).

`[verified]` The core `demosaic()` path writes into caller-provided output.
`demosaic_interleaved()` instead allocates a temporary planar `Vec<f32>` of
`3 * width * height`, calls `demosaic()`, and scatters into interleaved HWC
output (`src/lib.rs:234-253`). The layout conversion helpers assert equal lengths
and length divisibility by three, so malformed conversion buffers panic rather
than return `DemosaicError` (`src/lib.rs:256-296`).

### 2.2 `demosaic` Bayer borders and tests

`[verified]` The Bayer bilinear path clips its 3x3 window to image bounds and
divides by the number of samples found for each channel, returning zero when a
channel has no sample (`src/bayer/bilinear_impl.rs:7-37`).

`[verified]` The Bayer MHC path evaluates fixed 5x5 kernels at every pixel and
clamps each source coordinate to the image bounds. It does not switch to a
separate bilinear border implementation (`src/bayer/mhc_impl.rs:64-85`). This is
a border policy to preserve if the code or behavior is reused; naive coordinate
wrap would change both CFA phase and edge values.

`[verified]` The crate has integration coverage for solid-color Bayer bilinear
across all four phases, Bayer bilinear color separation, all Bayer algorithm
dispatch entries, buffer-size errors, unsupported algorithm/CFA pairs, and
planar/interleaved conversion (`tests/integration.rs:55-69,136-225,232-298`).
MHC has dedicated unit tests for known-channel preservation and uniform
reconstruction across all four Bayer phases
(`src/bayer/mhc_impl.rs:133-210`).

### 2.3 MeraRAW Demosaic API and validation

`[verified]` MeraRAW's `Demosaic` trait is stateless with respect to each
invocation and exposes stable ID/name/description, pattern support, cost class,
implementation status, minimum size, and
`run(&RawImage, &DemosaicOptions) -> Result<RgbImage, DemosaicError>`
(`crates/miro-demosaic/src/lib.rs:45-105`).

`[verified]` `Demosaic::check` validates CFA support, minimum dimensions, and
`raw.data.len() == width * height`, returning `UnsupportedPattern`, `TooSmall`,
or `BadBuffer` (`crates/miro-demosaic/src/lib.rs:80-104,199-217`). Bayer
algorithms call this check before dispatch; unsupported patterns are rejected
rather than silently treated as RGGB
(`crates/miro-demosaic/src/kaimai/mod.rs:99-121,243-248`).

`[verified]` `DemosaicOptions` separates shared post-processing controls and
algorithm tuning from algorithm implementations, while the registry supports
runtime capability inspection and safe iteration over stubs
(`crates/miro-demosaic/src/lib.rs:107-197,219-274`).

`[verified]` At this revision, the registry has nine implemented entries:
passthrough, nearest, built-in bilinear, and six Bayer-only adapters carried
over from the author's prior engine. VNG, PPG, AHD, DCB, AMaZE, RCD (the
standalone module), Quad Bayer, and Markesteijn report the default `Stub` status
until their implementations are complete. The README's nine-working-algorithm
claim matches the source registry statuses, not every registered name
(`crates/miro-demosaic/src/lib.rs:219-274`;
`crates/miro-demosaic/src/kaimai/mod.rs:127-174`).

### 2.4 MeraRAW output, borders, allocation, and tests

`[verified]` MeraRAW's output is an interleaved `RgbImage` with three `f32`
channels per pixel and nominal scene-linear `[0.0, 1.0]` values. `RgbImage::new`
allocates with `vec![0.0; width * height * 3]`
(`crates/miro-core/src/image.rs:3-19`). The error enum declares `Alloc`, but the
inspected output/scratch helpers use infallible `Vec` allocation; allocation
failure is therefore not reliably converted into `DemosaicError::Alloc`
(`crates/miro-demosaic/src/lib.rs:199-217`).

`[verified]` Shared border code fills an outer margin with bilinear estimates
before a larger-neighborhood algorithm processes the interior. If the margin
covers the frame, it fills the whole image bilinearly. The gather returns
`None` when no sample of the requested CFA color exists, avoiding division by a
zero count (`crates/miro-demosaic/src/common/borders.rs:17-116`).

`[verified]` Tiling carries a per-algorithm halo and stitches independently
written inner tiles; the helper allocates one scratch vector per tile and copies
results into the output (`crates/miro-demosaic/src/common/tiling.rs:31-100`).

`[verified]` Tests cover flat fields, tiny Bayer images through the implemented
registry, finite borders/corners, unsupported patterns, clean stub failures,
and bilinear tiled-versus-untiled equality. The synthetic harness provides
mosaic round trips, flat/gradient/edge/zone-plate/stripe scenes, PSNR, SSIM, and
zipper counting
(`crates/miro-demosaic/tests/correctness.rs:24-189`;
`crates/miro-demosaic/src/testing.rs:11-227`;
`crates/miro-demosaic/src/bilinear.rs:107-125`).

### 2.5 MeraRAW pipeline boundary and recurrence risk

`[verified]` MeraRAW's documented application order is
`crop -> white balance -> demosaic -> camera-to-XYZ -> ... -> display`. The
demosaic trait itself does not apply black subtraction, normalization, white
balance, color conversion, tone mapping, or display rendering; those are
pipeline concerns outside this crate boundary.

`[verified]` MeraRAW's `kaimai` adapter clones the single-channel mosaic into a
legacy `CfaImage`, runs a separate infallible algorithm trait, converts the
result back to `RgbImage`, and applies optional post-processing. This is useful
as an adapter precedent but adds copies and a second execution model
(`crates/miro-demosaic/src/kaimai/mod.rs:44-81,109-121`).

`[inference]` A `fornax` interface should keep one explicit input domain,
allocator policy, and error model. Importing MeraRAW's whole pipeline or its
legacy adapter would make stage boundaries and metadata conservation harder to
audit.

## 3. Evidence

| Finding | Evidence |
| --- | --- |
| `demosaic` public API, routing, and validation | `demosaic/src/lib.rs:131-232` |
| `demosaic` typed errors | `demosaic/src/error.rs:3-63` |
| Bayer CFA constructors and shift | `demosaic/src/cfa.rs:46-74,150-194` |
| Bilinear clipping and zero-count guard | `demosaic/src/bayer/bilinear_impl.rs:7-37` |
| MHC 5x5 coordinate clamping | `demosaic/src/bayer/mhc_impl.rs:64-85` |
| `demosaic` synthetic/integration tests | `demosaic/tests/integration.rs:55-69,136-225,232-298`; `src/bayer/mhc_impl.rs:133-210` |
| `demosaic` temporary allocation and panic-prone conversions | `demosaic/src/lib.rs:234-296` |
| MeraRAW trait, statuses, options, registry | `meraraw-demosaic/crates/miro-demosaic/src/lib.rs:45-274` |
| MeraRAW Bayer adapter validation and copies | `meraraw-demosaic/crates/miro-demosaic/src/kaimai/mod.rs:44-121` |
| MeraRAW output representation and allocation | `meraraw-demosaic/crates/miro-core/src/image.rs:3-19` |
| MeraRAW border and tiling behavior | `meraraw-demosaic/crates/miro-demosaic/src/common/borders.rs:17-116`; `common/tiling.rs:31-100` |
| MeraRAW synthetic and edge-case tests | `meraraw-demosaic/crates/miro-demosaic/tests/correctness.rs:24-189`; `src/testing.rs:11-227`; `src/bilinear.rs:107-125` |
| MeraRAW broader pipeline | `meraraw-demosaic/README.md`, "Architecture" |

## 4. Implications for fornax

`[recommendation]` Define a shared Bayer contract with a single-channel linear
mosaic, dimensions and stride, explicit CFA phase, caller-supplied or explicit
output allocation policy, and typed errors for unsupported layout, invalid
buffer, invalid geometry, and insufficient dimensions.

`[recommendation]` Make border behavior part of each algorithm's tested contract.
Do not inherit clamping, bilinear-ring fallback, or tiling asymmetry by accident.
Tiled and untiled paths must be compared after removing halo overlap.

`[recommendation]` Keep algorithm registration open and inspectable, but keep
shared types independent of one closed enum if independent implementations are a
requirement. MeraRAW's status/capability model and the `demosaic` crate's
algorithm-matrix validation are both useful precedents.

`[recommendation]` Prefer fallible allocation or document infallible allocation
limits explicitly. Treating `Vec` allocation failure as an ordinary `Result` is
not established by either inspected implementation.

`[recommendation]` Reuse synthetic mosaicing, flat-field, border-finites,
tiny-image, unsupported-pattern, stub, and quality-metric test ideas. Reuse code
only after checking the exact license and preserving notices; do not copy the
whole MeraRAW application pipeline.

## 5. Open questions and limitations

- Comparative image quality was not independently reproduced.
- MeraRAW README status claims were checked against registry statuses, not by
  executing every implemented algorithm on every Bayer phase.
- `DemosaicError::Alloc` exists but fallible allocation is not demonstrated by
  the inspected helpers.
- Final trait object safety, allocation strategy, options model, and metadata
  policy belong to design after synthesis.
- `demosaic` interleaved conversion helpers can panic on malformed buffers even
  though the primary `demosaic()` entry validates its buffers.
- X-Trans, Quad Bayer, monochrome, Foveon/X3, four-color CFA, and other
  non-target modes were inspected only enough to exclude them from the initial
  Bayer recommendation.

## 6. References

- `demosaic`: `https://github.com/naorunaoru/demosaic.git` at
  `5050131e998b7f6414eef02eeb60f74b99d77451`; MIT OR Apache-2.0.
  - Local checkout: `agent-work/references/other/demosaic-crate/`.
  - `src/lib.rs`, `src/error.rs`, `src/cfa.rs`, `src/bayer/`,
    `tests/integration.rs`, and `Cargo.toml`.
- MeraRAW Demosaic: `https://github.com/kaimai-apex/meraraw-demosaic.git` at
  `b94ebd4324ce56218e42989a5ed852b32a52003f`; Apache-2.0.
  - Local checkout: `agent-work/references/other/meraraw-demosaic/`.
  - `crates/miro-demosaic/src/`, `crates/miro-core/src/`, tests, README, and
    license.
