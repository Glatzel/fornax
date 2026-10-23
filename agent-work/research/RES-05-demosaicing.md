# Demosaicing Algorithms and Interfaces

> **Task:** `RES-05`
> **Owner:** RES-05 research agent
> **Review:** GitHub task pull request; its open/merged state is authoritative.
> **Last updated:** 2026-10-23
> **Upstream versions:** `fornax` at commit `46fabfd`;
> `naorunaoru/demosaic` at `5050131e998b7f6414eef02eeb60f74b99d77451`;
> `kaimai-apex/meraraw-demosaic` at `b94ebd4324ce56218e42989a5ed852b32a52003f`;
> darktable at `6556cd7dbb0e69f95d165dda8b69e7270c327f6d`; dnglab/rawler at
> `62b77de1a206d467e910d55199191068b45a9cc1`.
>
> **Acceptance criteria (authoritative version in the task TOML manifest):**
>
> - [x] Candidate algorithms are compared on input assumptions, output
>   semantics, border behavior, complexity, and testability.
> - [x] A recommended initial set is justified.
> - [x] The minimum shared input contract is stated without committing to the
>   final public API.
> - [x] Existing `fornax-dalim` behavior and tests are documented.
> - [x] X-Trans, Foveon, Quad Bayer, four-color, and monochrome handling is
>   explicitly deferred from the initial recommendation.
> - [x] The report contains the six required sections and cites sources.
>
> **Verification snapshot:** RES-05 research agent and primary checks completed
> 2026-10-23; GitHub pull-request review is pending. Algorithm comparison, recommended initial
> set, shared-input boundary, existing `fornax-dalim` tests/behavior, border
> rules, deferred layouts, and licensing were checked against the cited
> implementation and paper evidence. `cargo test -p fornax-dalim --all-features`
> passed 4 tests.

## 1. Scope and questions

This report surveys Bayer demosaicing algorithms that are realistic candidates
for an initial Rust-native `fornax` pipeline. It records each algorithm's data
assumptions, border behavior, numerical output semantics, cost, testability, and
reference implementation. It also audits the existing `fornax-dalim` linear
demosaicer.

The questions were:

- What input representation do the candidates require?
- Which CFA layouts and sample domains do they support?
- How do they handle image borders and output dimensions?
- What numerical range and rounding behavior do they produce?
- Which algorithm-specific options must remain outside a shared interface?
- What is the minimum common contract needed by independent implementations?
- Which algorithms should be implemented first, and why?
- What does the existing `fornax-dalim` implementation already guarantee?

The scope is conventional 2x2 RGB Bayer data. X-Trans, Quad Bayer, four-color
CFA, and monochrome reconstruction are noted as explicit exclusions rather than
hidden through pattern-dependent behavior. Fujifilm X-Trans and Sigma Foveon/X3
are deferred to a future investigation, along with every RAW format outside the
initial DNG, Sony ARW, Nikon NEF, and Canon `CR*` scope.

## 2. Findings

### 2.1 Existing `fornax-dalim` implementation

`[verified]` `DemosaicLinear` is a bilinear demosaicer. For each missing channel
it averages the available orthogonal or diagonal same-color neighbors. Interior
pixels use unsafe unchecked indexing after the caller establishes a one-pixel
border; edge rows/columns use checked reads with a count of available neighbors
(`crates/fornax-dalim/src/demosaic/linear.rs:4-70,74-156,160-235`).

`[verified]` The implementation processes rows in parallel with Rayon, preserves
the input dimensions, and writes an interleaved `image::Rgb<T>` buffer. An empty
input returns an empty output (`linear.rs:239-288`).

`[verified]` It distinguishes `G` and `G2` while interpreting the four supported
2x2 patterns, but writes both green positions to the single output green
channel (`crates/fornax-core/src/bayer.rs:13-78`;
`crates/fornax-dalim/src/demosaic/linear.rs:174-235`).

`[verified]` The public `IDemosaic<T>` trait takes `&BayerImage<T>` and returns
`ImageBuffer<Rgb<T>, Vec<T>>` directly, with no `Result`, options, or metadata
output. `BayerImage<T>` contains only an `image` cell buffer and one of the four
`BayerPattern` values; it has no CFA origin, black/white levels, or sample-domain
state (`crates/fornax-dalim/src/demosaic.rs:4-16`;
`crates/fornax-core/src/bayer.rs:101-150`).

`[verified]` Four unit tests cover RGGB, BGGR, GRBG, and GBRG on one 3x3 `f32`
mosaic with exact expected output (`linear.rs:292-470`). The only benchmark
uses a 1920x1080 random `f32` RGGB mosaic (`crates/fornax-dalim/benches/demosaic.rs:13-25`).

`[verified]` On 2026-10-23, `cargo test -p fornax-dalim --all-features` passed
all four unit tests; the doc-test target contained no runnable examples. This
confirms the checked-in all-pattern tests, not larger images or other sample
types.

`[inference]` Although the trait is generic over `T: FornaxPrimitive`, averaging
with integer `T` performs integer division and therefore truncates fractional
results. No integer demosaic test was found. A final interface must either
specify this rounding behavior or require a floating/intermediate accumulator.

`[inference]` A 1x1 or sufficiently narrow image can have zero neighbors in a
direction. The checked helpers compute `sum / count` without a zero-count guard,
so an `f32`/`f64` run can produce `NaN` and an integer run can divide by zero
(`linear.rs:74-156`). The code avoids out-of-bounds reads, but no test or
documented border contract covers these degenerate shapes.

### 2.2 Candidate comparison

| Algorithm | Input assumptions | Border behavior | Complexity / memory | Testability and risks |
| --- | --- | --- | --- | --- |
| **Bilinear** | Single-channel 2x2 Bayer mosaic in a linear sample domain; all four RGB Bayer phases | Existing `fornax` logic averages available neighbors, so edge values use fewer samples. Other reference implementations clamp/replicate coordinates or use a reduction. | `O(width*height)`, constant 3x3 work, one output buffer. | Easiest oracle: known-channel preservation and linear ramps have exact/closed-form expectations. Soft detail and zipper artifacts are expected, not defects. |
| **Malvar-He-Cutler (MHC)** | Same single-channel 2x2 Bayer input and linear-domain samples. Uses a fixed 5x5 support; known sensors are copied unchanged. | A correct implementation requires a 2-pixel border. The `demosaic` crate clamps out-of-range coordinates; MeraRAW falls back to bilinear within two pixels of an edge. | `O(width*height)` with fixed 5x5 sparse kernels; two or three small filters per pixel, one output buffer. | Deterministic and highly testable: constant fields, known-channel identity, linear gradients, kernel coefficient checks, and explicit border policy. Can overshoot/undershoot, so clipping policy must be explicit. |
| **PPG (Patterned Pixel Grouping)** | RGB 2x2 Bayer; needs channel balance assumptions even if not formally required. | Rawler explicitly interpolates a 3-pixel border by local averaging before the interior passes (`rawler/src/imgop/sensor/bayer/ppg.rs:60-105`). | `O(width*height)` with several local passes and gradient terms; more arithmetic than MHC but still bounded memory. | More state/stages to test. Compare color differences and edge/zone-plate PSNR, not just flats. Licensing of reference implementations varies. |
| **RCD (Ratio Corrected Demosaicing)** | RGB 2x2 Bayer, float working data; directional ratios and green interpolation. Best treated as a quality-tier algorithm after bilinear/MHC. | Darktable documents a 10-pixel tile border, a 9-pixel outer margin, and filtering of incomplete border regions (`src/iop/demosaicing/rcd.c:47-63`). | `O(width*height)` asymptotic but high constant, directional fields and tile scratch memory. | Strong quality candidate, but needs large border fixtures, tiled-vs-untiled equivalence, and careful epsilon/negative-input tests. |
| **AHD** | RGB 2x2 Bayer; homogeneity maps compare directional candidates. | Requires a larger neighborhood and explicit border fallback. | Higher constant and scratch memory than MHC. | Useful as a high-quality reference, but harder to validate and likely premature for the first implementation wave. |
| **LMMSE** | RGB 2x2 Bayer; local directional interpolation plus an iterative/nonlocal refinement. | Requires neighborhood margin and explicit handling of outer rows/columns. | Higher memory and compute; parameters affect output. | Strong high-ISO candidate in darktable, but algorithm options and reference outputs complicate the shared interface. |
| **X-Trans / Quad Bayer / four-color CFA** | Non-2x2 or spatially different patterns | Pattern-specific | Often requires larger neighborhoods, tiling, and different CFA indexing. | Explicitly outside this report's initial sequential-Bayer contract. Reject rather than silently treating as RGGB. |

The inspected Rust `demosaic` crate exposes a compact API over `f32`:
single-channel row-major input, CFA descriptor, an algorithm enum, and
preallocated planar output. It validates input/output sizes and unsupported
algorithm/CFA combinations with structured errors
(`src/lib.rs:137-220,230-270`). Its MHC implementation clamps each 5x5 kernel
coordinate to the image bounds rather than switching to a separate border
algorithm (`src/bayer/mhc_impl.rs:64-85`); it preserves the known sample and
applies fixed 5x5 kernels at every pixel (`src/bayer/mhc_impl.rs:12-62,87-145`).

The inspected MeraRAW implementation uses a narrow stateless `Demosaic` trait:
stable ID, supported CFA, minimum size, cost class, preflight buffer/pattern
checks, and `run(&RawImage, &DemosaicOptions) -> Result<RgbImage, ...>`
(`crates/miro-demosaic/src/lib.rs:48-145`). It demonstrates a useful distinction
between shared algorithm metadata and algorithm-specific tuning
(`lib.rs:145-205`). Its MHC implementation notes the same 2-pixel border
fallback (`crates/miro-demosaic/src/kaimai/malvar.rs:1-50`).

### 2.3 Shared input requirements

`[inference]` All candidates recommended here require at least:

- a two-dimensional, contiguous or explicitly strided single-channel mosaic;
- width and height;
- a CFA descriptor that maps `(x,y)` to an RGB channel, including a phase/origin
  that remains valid after cropping;
- a sample-domain marker indicating the values are suitable linear sensor
  samples and not display-encoded pixels;
- ownership and lifetime information that does not require an extra input
  copy;
- an output buffer or allocator policy;
- structured failure for unsupported CFA layout, invalid dimensions, invalid
  stride, or buffer-size mismatch.

`[recommendation]` The shared contract should preserve the known mosaic sample
unchanged at its own pixel and produce one RGB value at every input coordinate.
Border policy should be explicit and common: either require halo/valid-area
metadata, define a deterministic fallback (such as bilinear), or reject images
too small for the algorithm.

`[recommendation]` Do not put algorithm-specific knobs such as RCD thresholds,
MHC clipping choices, LMMSE iterations, or tiling parameters into the minimum
shared input type. Keep them on the concrete algorithm configuration.

### 2.4 Output semantics

`[verified]` Existing `fornax-dalim` output is interleaved RGB with the same
scalar type and dimensions as the input. No output metadata or processing-state
marker is returned (`linear.rs:239-288`).

`[verified]` The Rust `demosaic` crate's primary output is planar `f32` CHW,
with an optional conversion to interleaved HWC. It explicitly rejects size and
unsupported pattern/algorithm combinations (`src/lib.rs:137-220`).

`[verified]` rawler's trait returns a typed `Color2D<T,N>` for a requested ROI;
its Bayer bilinear implementation supports a four-channel result and crops the
input ROI before processing (`rawler/src/imgop/sensor/mod.rs:49-66`;
`rawler/src/imgop/sensor/bayer/bilinear.rs:13-75`).

`[inference]` A universal demosaiced result should state:

- channel order (`R,G,B` versus four-plane `R,G1,G2,B`);
- interleaved versus planar layout;
- numeric type and whether it is the same linear domain as the input;
- output dimensions and crop/ROI semantics;
- whether out-of-range estimates are preserved, clipped to zero, or clipped to
  both zero and one;
- how metadata is preserved or referenced.

No final public output type is proposed by this research task.

### 2.5 Algorithm selection

`[recommendation]` Implement **bilinear** first because it already exists in
`fornax-dalim`, preserves known samples, is cheap, covers all four Bayer phases,
and provides a deterministic baseline for every other algorithm.

`[recommendation]` Implement **Malvar-He-Cutler** second. It has a fixed,
published 5x5 kernel, modest memory, a straightforward bilinear border fallback,
and an independent MIT/Apache Rust reference suitable for cross-checking. It
offers a meaningful quality/cost improvement without the adaptive-field and
tiling complexity of RCD.

`[recommendation]` Treat **RCD** as the first quality-tier candidate after the
shared interface and baseline tests are stable. It is backed by a maintained
implementation and is darktable's default Bayer method, but its 10-pixel border,
directional ratio fields, tiling, and higher constant cost make it a poor first
algorithm to use for interface discovery.

`[recommendation]` Do not include AHD, LMMSE, VNG, PPG, AMaZE, or Markesteijn in
the first implementation set. They remain candidates for later evidence-driven
selection; inclusion merely because they are well known would add options and
test burden without a concrete quality requirement.

### 2.6 Licensing and reuse

`[verified]` The `demosaic` crate is MIT OR Apache-2.0, and its MHC
implementation is suitable as a behavioral reference and potentially as
license-compatible code with attribution and notice handling. MeraRAW is
Apache-2.0. rawler/dnglab is LGPL-2.1. darktable is GPL-3.0
(`demosaic` `Cargo.toml`; MeraRAW `LICENSE`;
`dnglab/rawler/Cargo.toml`; darktable `LICENSE`).

`[recommendation]` Prefer an independent implementation from the published
algorithm and synthetic tests. If code is copied from a reference, preserve its
license and notices and record the exact revision. Do not port GPL-3.0
darktable RCD code into a BSD-3-Clause `fornax` crate without a deliberate
license decision.

## 3. Evidence

| Claim | Evidence |
| --- | --- |
| Existing bilinear arithmetic and checked borders | `crates/fornax-dalim/src/demosaic/linear.rs:4-235` |
| Existing public trait/output | `crates/fornax-dalim/src/demosaic.rs:4-16`; `crates/fornax-core/src/bayer.rs:101-150` |
| Existing all-pattern tests | `crates/fornax-dalim/src/demosaic/linear.rs:292-470` |
| Existing benchmark | `crates/fornax-dalim/benches/demosaic.rs:13-25` |
| Rust `demosaic` API and errors | `agent-work/references/other/demosaic-crate/src/lib.rs:137-270` |
| MHC 5x5 kernels and conservative border clamping | `agent-work/references/other/demosaic-crate/src/bayer/mhc_impl.rs:1-145` |
| AHD directional passes and homogeneity selection | `agent-work/references/other/demosaic-crate/src/bayer/ahd_impl.rs:5-57` |
| MeraRAW trait and validation | `agent-work/references/other/meraraw-demosaic/crates/miro-demosaic/src/lib.rs:48-205` |
| MeraRAW MHC provenance and border | `agent-work/references/other/meraraw-demosaic/crates/miro-demosaic/src/kaimai/malvar.rs:1-50` |
| LMMSE directional variance weighting | `agent-work/references/other/meraraw-demosaic/crates/miro-demosaic/src/kaimai/lmmse.rs:7-28,36-75`; darktable `src/iop/demosaicing/lmmse.c:19-65` |
| rawler demosaicing trait and ROI semantics | `agent-work/references/dnglab/rawler/src/imgop/sensor/mod.rs:49-66` |
| rawler PPG border fallback | `agent-work/references/dnglab/rawler/src/imgop/sensor/bayer/ppg.rs:23-105` |
| darktable RCD border/tile notes | `agent-work/references/darktable/src/iop/demosaicing/rcd.c:20-65` |
| darktable demosaic method set | `agent-work/references/darktable/src/iop/demosaic.c:55-172,928-971` |

## 4. Implications for fornax

`[recommendation]` Separate the demosaic input contract from the concrete image
container and error strategy. A stable implementation should be able to consume
a backend-independent linear mosaic, dimensions/stride, and CFA descriptor and
return an explicit result without reaching into a decoder.

`[recommendation]` Define border behavior before implementing MHC or RCD.
Bilinear can be the common fallback, but tiled implementations must produce the
same pixels as untiled execution after discarding halo rows/columns.

`[recommendation]` Test every algorithm with:

- constant channel fields;
- known-channel preservation;
- linear gradients where bilinear or MHC has a known result;
- synthetic RGB mosaics with ground truth;
- all four Bayer phases;
- one-pixel and two-pixel dimensions;
- tiled versus untiled equality;
- unsupported pattern and invalid buffer errors;
- deterministic floating-point tolerances and explicit clipping.

`[recommendation]` Preserve metadata by returning or referencing it with the
demosaiced image. Do not use metadata transfer as a reason to keep the current
direct-return trait.

`[recommendation]` Keep demosaicing in the same linear sample domain as the
preprocessed mosaic. Do not apply WB, color matrices, tone curves, or display
gamma as part of an algorithm.

## 5. Open questions and limitations

- No real RAW fixture with independent ground truth was available for
  algorithm-quality comparison. Synthetic mosaics can establish interpolation
  correctness but not universal perceptual superiority.
- The existing `fornax-dalim` tests cover only 3x3 `f32` inputs. Degenerate
  dimensions, integer rounding, and larger multi-tile behavior remain
  unverified.
- The current trait cannot fail, has no options, and returns no metadata. The
  final interface must be designed after `SYN-01` and `SYN-03`.
- MHC can overshoot or undershoot. The reference implementations differ in
  clipping: one inspected path clamps negative estimates while preserving
  highlights; the current `fornax` bilinear path performs typed arithmetic with
  no global clipping policy.
- PPG channel-balance assumptions need reconciliation with the decision not to
  apply WB in `fornax`. The algorithm may be mathematically applicable to
  unbalanced mosaics, but artifacts can be affected.
- RCD's reference implementation is GPL-3.0 in darktable. An independent
  implementation would require a published algorithm description and
  licensing review, not direct code reuse.
- X-Trans, Quad Bayer, four-color CFA, monochrome, and non-mosaic outputs are
  outside this report's initial 2x2 Bayer recommendation.

## 6. References

- H. S. Malvar, L.-W. He, and R. Cutler, "High-Quality Linear Interpolation for
  Demosaicing of Bayer-Patterned Color Images," ICASSP 2004.
  DOI: `10.1109/ICASSP.2004.1326587`.
- Chuan-kai Lin, Patterned Pixel Grouping (PPG) algorithm; rawler records the
  archived author page at
  `https://web.archive.org/web/20160923211135/https://sites.google.com/site/chklin/demosaic/`.
- Luis Sanz Rodríguez, Ratio Corrected Demosaicing, reference implementation:
  `https://github.com/LuisSR/RCD-Demosaicing`.
- Local `fornax` sources:
  `crates/fornax-dalim/src/demosaic/linear.rs`,
  `crates/fornax-dalim/src/demosaic.rs`,
  `crates/fornax-core/src/bayer.rs`,
  `crates/fornax-dalim/benches/demosaic.rs`.
- Rust `demosaic` crate: `https://github.com/naorunaoru/demosaic` at
  `5050131e998b7f6414eef02eeb60f74b99d77451`; `src/lib.rs`,
  `src/bayer/bilinear_impl.rs`, `src/bayer/mhc_impl.rs`, and
  `src/bayer/{ahd,ppg}_impl.rs`; MIT OR Apache-2.0.
- MeraRAW Demosaic: `https://github.com/kaimai-apex/meraraw-demosaic` at
  `b94ebd4324ce56218e42989a5ed852b32a52003f`;
  `crates/miro-demosaic/src/`, including `kaimai/lmmse.rs`; Apache-2.0.
- dnglab/rawler: `https://github.com/dnglab/dnglab` at
  `62b77de1a206d467e910d55199191068b45a9cc1`;
  `rawler/src/imgop/sensor/`; LGPL-2.1.
- darktable: `https://github.com/darktable-org/darktable` at
  `6556cd7dbb0e69f95d165dda8b69e7270c327f6d`;
  `src/iop/demosaic.c`, `src/iop/demosaicing/`; GPL-3.0.
