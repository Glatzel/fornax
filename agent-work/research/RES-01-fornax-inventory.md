# Existing `fornax` Components and Reuse Inventory

> **Task:** `RES-01`
> **Owner:** RES-01 research agent / primary agent review
> **Review:** GitHub task pull request; its open/merged state is authoritative.
> **Last updated:** 2026-10-23
> **Upstream versions:** local repository at commit `46fabfd`; source-bearing
> paths are unchanged from the earlier `d06f426` audit
>
> **Acceptance criteria (authoritative version in the task TOML manifest):**
>
> - [x] Relevant existing components and public contracts are listed with path
>   and symbol.
> - [x] Current limitations and defects are explicit.
> - [x] Reusable implementation assets are classified as direct reuse,
>   adaptation, test/oracle only, or not reusable.
> - [x] The report states that the architecture will be fully refactored and
>   does not prescribe retaining existing boundaries or API shape.
> - [x] Reusable implementation assets are distinguished from architecture.
> - [x] Findings use `[verified]`, `[inference]`, or `[recommendation]` labels.
> - [x] All six required report sections are complete.
>
> **Verification snapshot:** Primary checks completed 2026-10-23; GitHub pull-request review is
> pending. Existing components, public symbols, limitations, implementation
> reuse candidates, and report structure were checked against `46fabfd`. This is
> an inventory for implementation reuse and migration; it is not an architecture
> proposal. The current architecture is intended to be fully refactored.

## 1. Scope and questions

Inventory what the existing repository contains and identify implementation
assets that may be useful when building a replacement architecture. The
questions were:

- What do `IDecoder`, `IPostProcessor`, `BayerImage`, `BayerPattern`,
  `IDemosaic`, and `DemosaicLinear` actually guarantee?
- How do dependencies flow between the crates?
- Which concrete algorithms, tests, fixtures, benchmarks, bindings, and tooling
  may be reusable in the replacement implementation?
- Which existing components are tightly coupled to assumptions that should not
  survive the refactor?
- Which assumptions require non-Bayer layouts to be rejected until a future
  task explicitly extends the pipeline?
- Which assumptions block metadata-rich or black-level-aware processing?

The existing crate boundaries, traits, composition model, and public API are
evidence to inventory, not architecture to preserve. The replacement
architecture will be designed from the required pipeline contracts after
synthesis.

The inventory is evaluated against the current initial scope: conventional 2x2
RGB Bayer data in DNG, Sony ARW, Nikon NEF, and Canon `CR*`. X-Trans,
Foveon/X3, Quad Bayer, four-color, monochrome, non-mosaic payloads, and other
RAW formats are deferred and must be rejected rather than inferred as supported.

## 2. Findings

### 2.1 Current workspace topology (inventory only)

`[verified]` The workspace has resolver 3, edition 2024, package version
`0.0.17`, and members `./crates/*` (`Cargo.toml:1-7`). It contains eight crates:
`fornax-core`, `fornax-dalim`, `libraw`, `libraw-sys`, `fornax`, `fornax-py`,
`dnc`, and `fornax-devtool`.

`[verified]` Dependency direction, from the manifests:

- `fornax-core` → `image`, `thiserror` (`crates/fornax-core/Cargo.toml`).
- `fornax-dalim` → `clerk`, `fornax-core`, `image`, `rayon`, optional `serde`
  (`crates/fornax-dalim/Cargo.toml`).
- `libraw` → `bytemuck`, `clerk`, `envoy`, `fornax-core`, `image`, `jiff`,
  `libraw-sys`, `num_enum`, optional `serde`, `thiserror`
  (`crates/libraw/Cargo.toml`).
- `fornax` → `dnc`, `fornax-core`, `fornax-dalim`, `image`, `libraw`
  (`crates/fornax/Cargo.toml`).
- `fornax-py` → `fornax`, `image`, `numpy`, `pyo3`, `rmp-serde`, `serde`
  (`crates/fornax-py/Cargo.toml`).
- `libraw-sys` → `libc`; `bindgen` is a build dependency
  (`crates/libraw-sys/Cargo.toml`).
- `dnc` → `clerk`, optional `serde`, `thiserror`; `fornax-devtool` → `clerk`.

`[verified]` `fornax-core` publicly exposes `image::ImageBuffer` and
`image::Luma` through `BayerImage` and `IDecoder`
(`crates/fornax-core/src/lib.rs:6-11`,
`crates/fornax-core/src/bayer.rs:111-150`). The core data model is therefore
already coupled to the `image` crate.

`[verified]` `libraw` depends on `fornax-core`, and `fornax` depends on both
`libraw` and `fornax-dalim`. The dependency direction is
`fornax-core ← fornax-dalim ← fornax`, with `libraw → fornax-core` as a sibling
backend that still leaks its own types through `fornax`'s `pub use libraw;`
(`crates/fornax/src/lib.rs:8`).

### 2.2 Decoder and processor contracts

`[verified]` `IDecoder<T>` is defined at
`crates/fornax-core/src/decoder.rs:19`. It has three methods:

- `decode_file(&self, file: &Path) -> Result<(), FornaxError>`.
- `decode_buffer(&self, buffer: &[u8]) -> Result<(), FornaxError>`.
- `bayer_image(&self) -> Result<BayerImage<T>, FornaxError>`.

`[verified]` All three take `&self`, so implementations must mutate interior
state to hold the decoded image. The trait has no metadata return type, no
sample-domain description, and no distinction between raw sensor samples and
processed pixels.

`[verified]` `IPostProcessor<D, T, O>` is defined at
`crates/fornax-core/src/post_processor.rs:22`. It takes a decoder reference and
returns `ImageBuffer<Rgb<O>, Vec<O>>`. `NullPostProcessor` is declared at line
57, but its implementation calls `unimplemented!` at line 66, so it panics if
used.

`[verified]` `FornaxError` is a tuple struct wrapping `String`
(`crates/fornax-core/src/error.rs:5`). There are no structured variants for
invalid layouts, missing metadata, or unsupported CFA patterns.

`[verified]` `FornaxPrimitive` is implemented for `u8`, `u16`, `f32`, and `f64`
(`crates/fornax-core/src/primitive.rs:15-27`). Signed 16-bit and unsigned 32-bit
samples are commented out, which limits signed or high-bit-depth sensor data.

### 2.3 Bayer model

`[verified]` `BayerChannel` has four variants: `R`, `G`, `B`, and `G2`
(`crates/fornax-core/src/bayer.rs:13-18`). `BayerPattern` has exactly `RGGB`,
`BGGR`, `GRBG`, and `GBRG` (`bayer.rs:40-49`). `as_mask()` returns the 2×2
channel order (`bayer.rs:51-78`).

`[verified]` `BayerImage<T>` wraps an `image::ImageBuffer<Luma<T>, Vec<T>>` and
a `BayerPattern` (`bayer.rs:111-128`). Its public surface is only `new`,
`mosaic`, and `pattern` (`bayer.rs:131-150`). It carries no black level, white
level, active area, crop, margins, CFA origin, camera metadata, or provenance.

`[verified]` There is no representation for X-Trans, monochrome, four-color, or
non-mosaic RAW data in `fornax-core`. There is also no structured
unsupported-layout error: decode failures can only use the string-wrapped
`FornaxError`, and the type boundary cannot explain which layout or metadata
condition caused rejection.

### 2.4 Demosaicing

`[verified]` `IDemosaic<T>` is defined at `crates/fornax-dalim/src/demosaic.rs:4`.
It takes `&BayerImage<T>` and returns
`image::ImageBuffer<image::Rgb<T>, Vec<T>>` (`demosaic.rs:8-11`). It cannot fail:
there is no `Result`, no unsupported-pattern error, and no metadata output.

`[verified]` `DemosaicLinear` is the only implementation
(`crates/fornax-dalim/src/demosaic/linear.rs:239`). It uses the pattern mask,
unchecked neighbor reads on interior pixels, and checked reads on borders. The
tests cover all four Bayer patterns: `test_linear_rggb` (line 294),
`test_linear_bggr` (line 338), `test_linear_grbg` (line 382), and
`test_linear_gbrg` (line 426). A Criterion benchmark is at
`crates/fornax-dalim/benches/demosaic.rs:1-26`.

`[verified]` `DalimParams` contains only a `Demosaicer` enum
(`crates/fornax-dalim/src/lib.rs:10-12`), and `Dalim` dispatches only to
`Demosaicer::Linear` (`lib.rs:40-43`). There is no preprocessing stage.

### 2.5 LibRaw wrapper

`[verified]` `Libraw` wraps a raw `libraw_data_t` pointer with `Arc<ImgdataPtr>`
and closes it on `Drop` (`crates/libraw/src/data_structure/data.rs:5-38`).
`Rawdata` is an enum of `image::ImageBuffer` values: mono/RGB/RGBA in `u16` and
`f32` (`crates/libraw/src/data_structure/rawdata.rs:9-15`).

`[verified]` `Rawdata::get_rawdata` is `pub(crate)` and receives only `width`
and `height` (`rawdata.rs:18-22`). It checks `rawdata.raw_image`,
`float_image`, `color3_image`, `color4_image`, `float3_image`, and
`float4_image` and attempts to convert them into an `image::ImageBuffer`
(`rawdata.rs:25-114`). It does not return the LibRaw `ImageSizes`, `IParams`,
`libraw_colordata_t`, or any black/white/crop metadata with the buffer.

`[verified]` The `color4_image` branch is defective: line 66 tests
`color4_image.is_null()` instead of `!color4_image.is_null()`. A non-null
four-channel buffer therefore skips this branch and is rejected later unless a
float pointer is also populated; a null pointer that reaches the branch would
feed `slice::from_raw_parts` with a null address (`rawdata.rs:66-80`). The
`Rgba16` variant is present but its normal pointer path is not safe to use until
this condition is fixed and tested.

`[recommendation]` Fix the `color4_image` condition and add a synthetic pointer
test for each `Rawdata` variant before using `Rawdata` as a decoder adapter.

`[verified]` `get_rawdata` is called with `raw_width` and `raw_height` from
`ImageSizes` (`crates/libraw/src/extension/missing_structures.rs:6-16`). The
wrapper exposes `raw_pitch()` (`image_sizes.rs:54`) but the conversion does not
use it.

`[verified]` `ImageSizes` exposes `raw_height`, `raw_width`, `height`, `width`,
`top_margin`, `left_margin`, `raw_pitch`, and `flip`
(`crates/libraw/src/data_structure/image_sizes.rs:32-65`). `IParams` exposes
make, model, normalized names, `raw_count`, `is_foveon`, `dng_version`,
`colors`, `filters`, X-Trans tables, and `cdesc`
(`crates/libraw/src/data_structure/iparams.rs:43-110`).

`[verified]` The wrapper has `color(row, col)`, `libraw_subtract_black()`,
`raw2image()`, `dcraw_process()`, and a processed-image accessor
(`crates/libraw/src/c_api/auxiliary.rs:24-60`). `libraw_subtract_black` calls the
LibRaw C function directly (`auxiliary.rs:27-30`).

`[verified]` The wrapper does **not** expose the LibRaw black-level fields
`color.black`, `color.cblack`, `color.maximum`, `color.linear_max`, or
`color.dng_levels`. A search of `crates/libraw/src` for those field accesses
returns only the DNG/black-related user parameters in
`extension/dcraw.rs:81-84`, not read accessors.

### 2.6 High-level manager and Python bindings

`[verified]` `Fornax<D, T, P, O>` combines a decoder and post-processor
(`crates/fornax/src/lib.rs:29-40`). Its public methods are `new` (line 59),
`decode_file` (line 79), `decode_buffer` (line 95), and `post_process` (line
106). It exposes no metadata, preprocessing control, or stage inspection.

`[verified]` The Python binding has a `py_process` entry point and a `fornax_py`
module (`crates/fornax-py/src/lib.rs:74-153`). Python tests cover DNC, LibRaw
default/custom processing, and LibRaw+Dalim (`crates/fornax-py/tests/test_fornax.py:20-115`).

`[verified]` `dnc` is re-exported only on Windows and macOS
(`crates/fornax/src/lib.rs:3-5`), so DNG conversion is not a portable backend.

### 2.7 CI and conventions

`[verified]` CI runs cargo doc with `RUSTFLAGS=-Dwarnings`, clippy and
workspace tests with nightly and all features excluding `fornax-py`, then
`scripts/maturin-develop.ps1` and pytest
(`.github/workflows/ci.yml:26-71`). Formatting uses unstable rustfmt options
(`rustfmt.toml`), so nightly rustfmt is required.

## 3. Evidence

| Claim | Evidence |
| --- | --- |
| Workspace layout and version | `Cargo.toml:1-7` |
| Crate dependency manifests | `crates/*/Cargo.toml` |
| Core public exports | `crates/fornax-core/src/lib.rs:6-11` |
| Decoder contract | `crates/fornax-core/src/decoder.rs:19-31` |
| Post-processor contract | `crates/fornax-core/src/post_processor.rs:22-66` |
| Error type | `crates/fornax-core/src/error.rs:5` |
| Primitive types | `crates/fornax-core/src/primitive.rs:4-27` |
| Bayer types and buffer | `crates/fornax-core/src/bayer.rs:13-150` |
| Demosaic trait | `crates/fornax-dalim/src/demosaic.rs:4-16` |
| Linear demosaic and tests | `crates/fornax-dalim/src/demosaic/linear.rs:239-470` |
| LibRaw ownership and close | `crates/libraw/src/data_structure/data.rs:5-38` |
| Rawdata conversion | `crates/libraw/src/data_structure/rawdata.rs:9-114` |
| Image sizes and flip | `crates/libraw/src/data_structure/image_sizes.rs:9-65` |
| LibRaw info parameters | `crates/libraw/src/data_structure/iparams.rs:33-110` |
| LibRaw C API calls | `crates/libraw/src/c_api/auxiliary.rs:24-60` |
| High-level manager | `crates/fornax/src/lib.rs:29-110` |
| Python module | `crates/fornax-py/src/lib.rs:74-153` |
| CI commands | `.github/workflows/ci.yml:26-71` |

## 4. Implications for fornax

`[recommendation]` Treat the current architecture as legacy input only. The
replacement must define new crate boundaries, contracts, data ownership,
metadata semantics, and error handling from the research and synthesis
findings. The existing architecture is expected to be fully refactored.

| Existing asset | Reuse class | Implementation implication |
| --- | --- | --- |
| `DemosaicLinear` interpolation core | Adapt | Useful algorithmic baseline, but it needs explicit border behavior, zero-neighbor handling, integer rounding policy, and a new input/output contract. |
| Bayer pattern/channel concepts | Adapt | The four-phase model is relevant, but the replacement must own CFA origin, crop phase, channel interpretation, and unsupported-layout errors. |
| Linear-demosaic unit tests | Test/oracle only | Useful expected values for four Bayer phases after the algorithm contract is redesigned. |
| Criterion benchmark scaffolding | Adapt | Reuse the benchmark idea and harness after adapting buffers and algorithm interfaces. |
| LibRaw FFI declarations and ownership plumbing | Adapt | Potentially reusable behind a new decoder adapter. Do not expose LibRaw types or the current `Rawdata` model downstream. |
| `ImageSizes` and `IParams` accessors | Adapt | Useful evidence for a backend adapter, not a universal metadata API. |
| `Rawdata` buffer enum | Not reusable as architecture | Loses stride, crop, CFA, black/white, and provenance semantics; also contains the inverted `color4_image` condition. |
| `IDecoder` and `IPostProcessor` | Not reusable as architecture | Stateful decode-then-query and direct post-processing obscure metadata and prevent an explicit preprocessing stage. |
| `BayerImage<T>` | Not reusable as architecture | Lacks the metadata, sample-domain, stride, and processing-state invariants required by the target pipeline. |
| `Fornax<D, T, P, O>` manager | Not reusable as architecture | Couples decoding and post-processing without explicit pipeline stages or metadata flow. |
| `FornaxError(String)` | Not reusable as architecture | Cannot express structured decode, metadata, CFA, or stage errors. |
| `NullPostProcessor` | Not reusable | Its implementation panics despite being described as a no-op. |
| Current crate topology | Not reusable as architecture | The new design must justify its own boundaries from pipeline responsibilities. |
| Python packaging and CI/tooling structure | Adapt | Potentially reusable build and test infrastructure after the public API is redesigned. |
| DNC wrapper | Conditional | Keep only if a separately scoped implementation task requires vendor DNG conversion; it is not part of the core architecture. |

`[recommendation]` Do not carry `BayerImage`, `IDecoder`, `IPostProcessor`, or
`Fornax<D, T, P, O>` forward merely for compatibility. Carry code only when the
new design independently requires that behavior and the adaptation has explicit
tests, licensing review, and invariant preservation.

`[recommendation]` Record every reused implementation asset during design and
implementation with its source path, target location, semantic changes, tests,
and migration cost. An asset is not reusable merely because it compiles in the
old architecture.

## 5. Open questions and limitations

- `[inference]` The current `BayerImage`/`IDecoder` split assumes the decoder
  can return a fully formed Bayer image without metadata. This is workable for
  the current LibRaw path but cannot express black/white levels or crop.
- `[verified]` No integration fixtures for real RAW files were found under
  `crates/*/tests`; only Python tests and unit tests exist. This limits
  regression coverage. In particular, the initial Bayer DNG/ARW/NEF/CR* matrix
  is not established by repository-local fixtures.
- `[verified]` There is no documentation for the public traits beyond doc
  comments, and no design record for the existing API.
- `[inference]` `Rawdata::get_rawdata` assumes the raw buffer is contiguous for
  `raw_width * raw_height`; the wrapper exposes `raw_pitch` but does not use it.
  `RES-02` confirms that `raw_pitch` is the authoritative row stride and that
  zero-margin contiguous buffers do not establish the general case.
- `[verified]` The wrapper contains conversion branches for `raw_image`,
  `float_image`, `color3_image`, `color4_image`, `float3_image`, and
  `float4_image`, but the `color4_image` condition is inverted. Fixture-level
  reachability for each pointer is not covered by the current test suite.
- `[verified]` `NullPostProcessor` panics by design (`unimplemented!`), so it is
  not a no-op despite its name.

## 6. References

- Local repository: `/home/glatzel/project/fornax`, with the current inventory
  at commit `46fabfd`; Rust sources and manifests are unchanged from `d06f426`.
- Workspace manifest: `Cargo.toml`.
- Crate manifests: `crates/fornax-core/Cargo.toml`,
  `crates/fornax-dalim/Cargo.toml`, `crates/libraw/Cargo.toml`,
  `crates/libraw-sys/Cargo.toml`, `crates/fornax/Cargo.toml`,
  `crates/fornax-py/Cargo.toml`, `crates/dnc/Cargo.toml`,
  `crates/fornax-devtool/Cargo.toml`.
- Core sources: `crates/fornax-core/src/{lib,bayer,decoder,post_processor,primitive,error}.rs`.
- Demosaic sources: `crates/fornax-dalim/src/{lib,demosaic}.rs`,
  `crates/fornax-dalim/src/demosaic/linear.rs`,
  `crates/fornax-dalim/benches/demosaic.rs`.
- LibRaw wrapper: `crates/libraw/src/{lib,c_api/*,data_structure/*,extension/*}.rs`.
- High-level crate: `crates/fornax/src/lib.rs`.
- Python bindings: `crates/fornax-py/src/lib.rs`,
  `crates/fornax-py/tests/test_fornax.py`.
- CI and formatting: `.github/workflows/ci.yml`, `rustfmt.toml`.
