# Consolidate RAW Crates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make `fornax-raw` the single high-level RAW crate by absorbing
`fornax-raw-core`, `fornax-dng`, `fornax-raw-decoder`, and
`fornax-raw-pipeline`, with one shared error family and a public DNG writer
module.

**Architecture:** All raw contracts, DNG handling, native decoders, and pipeline
stages live under `crates/fornax-raw`. `fornax-core` remains unchanged.
Raw-specific development helpers move from `fornax-devtool` into `fornax-raw`
bench support so `fornax-devtool` no longer depends on raw contracts.

**Tech Stack:** Rust 2024, Cargo workspace, `thiserror`, `image`, Rayon,
Criterion, DNG/TIFF/codecs, and the existing pure-Rust vendor decoders.

**Spec:** This document's Goal, Scope, Approaches, and Decisions sections.

## Global Constraints

- Support conventional 2x2 RGB Bayer input only in preprocessing and demosaicing.
- Do not change decoder algorithms, accepted RAW formats, or DNG writer output.
- `fornax-core` is unchanged.
- Remove the four old packages rather than retaining compatibility crates.
- Expose writer APIs only as `fornax_raw::writer::{...}`; do not re-export them
  from the `fornax_raw` root.
- Use one public `RawError`; keep `DecodeError`, `PreprocessError`,
  `DemosaicError`, `PostProcessError`, and `DngWriteError` as aliases.
- Preserve existing feature behavior: default `mmap` and `large-page-hint`,
  plus `metadata-json` and `zstd`.
- Never commit binary files or hard-linked fixture payloads.

## Review Focus

- Preserve codec and decoder behavior when replacing external crate imports
  with `crate::` paths.
- Keep the writer reachable at `fornax_raw::writer` after its source moves under
  `dng`, and keep all writer types absent from the crate root.
- Keep all stage errors pattern-compatible through aliases while flattening
  demosaic and writer variants into `RawError`.
- Keep feature propagation equivalent after combining the four manifests.
- Avoid a Cargo package cycle between `fornax-raw` and `fornax-devtool`.

## Approaches

1. Keep the four old crates and add facade re-exports. This preserves package
   structure and does not meet the consolidation request.
2. Move all shared contracts and implementations into `fornax-raw` while
   leaving raw-specific helpers in `fornax-devtool`. This creates a Cargo cycle
   because the consolidated raw crate needs `fornax-devtool` as a development
   dependency while `fornax-devtool` needs raw types.
3. Move all four implementations and contracts into `fornax-raw`, and move the
   raw-specific helpers out of `fornax-devtool` into raw benchmark support.
   This keeps one public RAW crate and an acyclic workspace graph.

Selected approach: 3.

## Public API Shape

```rust
pub use fornax_core::{MonoImage, RgbImage};
pub use bayer::{BayerChannel, BayerPattern};
pub use error::{
    DecodeError, DemosaicError, PostProcessError, PreprocessError, RawError,
    RawValidationError,
};
pub use format::RawFormat;
pub use metadata::{EncodingMetadata, EncodingSampleFormat, ProcessMetadata};
pub use primitive::{CroppedMosaic, NormalizedMosaic, RawFloat, RawMosaic};
pub mod decoder;
pub use demosaic::{Demosaic, IDemosaic, LazyDemosaic, OutputColorSpace};
pub use postprocessor::PostProcessor;
pub use preprocessor::Preprocessor;

pub mod dng;
pub mod writer {
    pub use crate::dng::writer::*;
}
```

`DngWriteError` is a type alias for `RawError` defined in `dng::writer`.
Writer types are intentionally absent from the crate root.

---

### Task 1: Pin the consolidated public API with failing tests

**Files:**
- Create: `crates/fornax-raw/tests/raw_api.rs`
- Modify: `crates/fornax-raw/src/lib.rs` only after the RED run

**Interfaces:**
- Produces: public trait bounds proving all stage error names resolve to
  `RawError`.
- Produces: public writer path `fornax_raw::writer`.

- [x] **Step 1: Add the failing facade test**

```rust
use fornax_raw::{
    DecodeError, DemosaicError, PostProcessError, PreprocessError, RawError,
    RawValidationError,
};
use fornax_raw::writer::{DngWriteError, DngWriter, DngWriterCompression};

#[test]
fn stage_errors_are_aliases_of_raw_error() {
    let errors: [RawError; 5] = [
        DecodeError::Unsupported("decode".into()),
        PreprocessError::EmptyImage,
        DemosaicError::InvalidInput("demosaic".into()),
        PostProcessError::InvalidColorTransform("color".into()),
        DngWriteError::WriteInvalidMetadata { detail: "writer".into() },
    ];
    assert!(matches!(errors[4], RawError::WriteInvalidMetadata { .. }));
    let _: DecodeError = RawError::InvalidOutput(RawValidationError::InvalidImage);
}

#[test]
fn dng_writer_is_exposed_only_through_the_writer_module() {
    let _: DngWriter = DngWriter::new(DngWriterCompression::Uncompressed);
}
```

- [x] **Step 2: Verify RED**

Run:

```bash
cargo test -p fornax-raw --test raw_api
```

Expected: compilation fails because `DecodeError`, `RawError`, and
`fornax_raw::writer` are not yet exported by `fornax-raw`.

### Task 2: Absorb `fornax-raw-core` and unify stage errors

**Files:**
- Move: `crates/fornax-raw-core/src/{bayer,crop_cast,error,format,metadata,primitive,protocol,utils}.rs`
  to `crates/fornax-raw/src/`
- Move: `crates/fornax-raw-core/tests/crop_cast.rs`
  to `crates/fornax-raw/tests/core_crop_cast.rs`
- Modify: `crates/fornax-raw/src/lib.rs`
- Modify: `crates/fornax-raw/src/error.rs`
- Modify: `crates/fornax-raw/Cargo.toml`

**Interfaces:**
- Produces: `RawError`, the single enum used by every stage.
- Produces: aliases `DecodeError`, `PreprocessError`, `DemosaicError`,
  `PostProcessError`, and `DngWriteError`.
- Produces: root exports for `RawMosaic`, `RawFloat`, `RawFormat`,
  `CroppedMosaic`, `NormalizedMosaic`, `MonoImage`, `RgbImage`,
  `BayerPattern`, and `BayerChannel`.

- [x] **Step 1: Move the raw-core modules into `fornax-raw`**

Move files without changing algorithm bodies. Retain `fornax_core::MonoImage`
and `fornax_core::RgbImage` imports so `fornax-core` remains unchanged.

- [x] **Step 2: Replace the separate error enums**

In `crates/fornax-raw/src/error.rs`, define `RawError` with these variants:

```rust
InvalidInput(Box<str>)
Unsupported(Box<str>)
Backend(Box<str>)
InvalidMetadata(MetadataValidationError)
InvalidOutput(RawValidationError)
Other(Box<str>)
EmptyImage
ImageTooLarge { width: usize, height: usize }
InvalidSampleCount { expected: usize, actual: usize }
InvalidBlackPattern
BlackDeltaWidthMismatch { expected: usize, actual: usize }
BlackDeltaHeightMismatch { expected: usize, actual: usize }
InvalidColorTransform(Box<str>)
WriteUnsupported { operation: Box<str> }
WriteIo { operation: Box<str>, detail: Box<str> }
WriteInvalidMetadata { detail: Box<str> }
```

Add:

```rust
pub type DecodeError = RawError;
pub type PreprocessError = RawError;
pub type DemosaicError = RawError;
pub type PostProcessError = RawError;
```

Keep `RawValidationError` and `MetadataValidationError` as typed inner errors.
Remove the nested `PostProcessError::Demosaic` conversion; demosaic failures now
return their direct `RawError` variants. Define
`pub type DngWriteError = RawError` in `dng::writer`, so the writer-specific
alias is exposed only through the writer module.

- [x] **Step 3: Make `fornax-raw` own the raw-core dependencies**

Copy the required dependency entries from `fornax-raw-core/Cargo.toml` into
`fornax-raw/Cargo.toml`, except `fornax-raw-core` itself. Keep the existing
`fornax-core` dependency.

- [x] **Step 4: Run the focused checks**

Run:

```bash
cargo test -p fornax-raw --test raw_api stage_errors_are_aliases_of_raw_error
cargo test -p fornax-raw --test core_crop_cast
```

Expected: the error-alias test passes; the `writer` test still fails to compile
until Task 4.

### Task 3: Absorb `fornax-raw-pipeline`

**Files:**
- Move: `crates/fornax-raw-pipeline/src/demosaic.rs` and `src/demosaic/`
  to `crates/fornax-raw/src/`
- Move: `crates/fornax-raw-pipeline/src/preprocessor.rs`
  to `crates/fornax-raw/src/preprocessor.rs`
- Move: `crates/fornax-raw-pipeline/src/postprocessor.rs`
  to `crates/fornax-raw/src/postprocessor.rs`
- Move: pipeline benchmark files to `crates/fornax-raw/benches/`
- Modify: `crates/fornax-raw/src/lib.rs`
- Modify: `crates/fornax-raw/Cargo.toml`

**Interfaces:**
- Consumes: root `RawError`, `RawFloat`, `RawMosaic`, `BayerPattern`, and
  metadata/protocol types from Task 2.
- Produces: root exports `Demosaic`, `IDemosaic`, `LazyDemosaic`,
  `OutputColorSpace`, `PostProcessor`, and `Preprocessor`.
- Produces: the compatibility module `fornax_raw::raw_pipeline`.

- [x] **Step 1: Replace pipeline crate imports**

In moved pipeline source, replace external `fornax_raw_core::` imports with
`crate::`. Replace the wrapper `raw_pipeline.rs` with a module that re-exports
the merged pipeline API.

- [x] **Step 2: Replace the failing wrapper modules**

Keep `fornax_raw::preprocessor` and `fornax_raw::postprocessor` as real modules
containing the moved implementations. Keep `demosaic` private and preserve the
existing root re-exports for its public types.

- [x] **Step 3: Move pipeline benchmark declarations**

Add `demosaic` and `preprocessor` bench targets to
`crates/fornax-raw/Cargo.toml`. Move their files unchanged except crate imports.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p fornax-raw --lib demosaic
cargo test -p fornax-raw --test raw_pipeline
cargo bench -p fornax-raw --bench demosaic --no-run
cargo bench -p fornax-raw --bench preprocessor --no-run
```

Expected: all commands pass; the existing Bayer invariants remain green.

### Task 4: Absorb `fornax-dng` and expose the writer module

**Files:**
- Move: `crates/fornax-dng/src/lib.rs` to
  `crates/fornax-raw/src/dng/mod.rs`
- Move: the remaining `crates/fornax-dng/src/` files to
  `crates/fornax-raw/src/dng/`
- Move: DNG tests to distinct names under `crates/fornax-raw/tests/`
- Move: `crates/fornax-dng/benches/writer.rs`
  to `crates/fornax-raw/benches/writer.rs`
- Modify: `crates/fornax-raw/src/lib.rs`
- Modify: DNG source imports
- Modify: `crates/fornax-raw/Cargo.toml`

**Interfaces:**
- Consumes: root raw contracts and `RawError` from Task 2.
- Produces: `pub mod dng` for DNG metadata, constants, and decoding.
- Produces: `pub mod writer { pub use crate::dng::writer::*; }`.
- Produces: writer types only through `fornax_raw::writer`, not the crate root.

- [x] **Step 1: Update moved DNG imports**

Replace `fornax_raw_core::` with `crate::`. Replace moved internal references
such as `crate::metadata`, `crate::writer`, `crate::sample_buffer`,
`crate::constants`, and `crate::dng_decoder` with their `crate::dng::...`
equivalents.

- [x] **Step 2: Replace `DngWriteError` with the alias**

Delete the standalone enum. Re-export the `RawError` alias as
`DngWriteError` from `dng::writer`. Map writer failures as follows:

```rust
TiffError -> RawError::WriteInvalidMetadata
CodecError::InvalidInput -> RawError::WriteInvalidMetadata
CodecError::Unsupported -> RawError::WriteUnsupported
File create/write -> RawError::WriteIo
Unsupported operation -> RawError::WriteUnsupported
```

- [x] **Step 3: Add the public writer facade**

In `crates/fornax-raw/src/lib.rs`:

```rust
pub mod writer {
    pub use crate::dng::writer::*;
}
```

Do not add `DngWriter`, `DngWriterCompression`, `DngVersion`, or
`DngWriteError` to root `pub use` declarations.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p fornax-raw --test raw_api
cargo test -p fornax-raw --test dng_constants
cargo test -p fornax-raw --test dng_directory_metadata
cargo test -p fornax-raw --test dng_writer_boundary
cargo bench -p fornax-raw --bench writer --no-run
```

Expected: all pass and writer behavior is unchanged.

### Task 5: Absorb `fornax-raw-decoder`

**Files:**
- Move: `crates/fornax-raw-decoder/src/decoders/`
  to `crates/fornax-raw/src/decoder/`
- Move: `crates/fornax-raw-decoder/src/lib.rs`
  to `crates/fornax-raw/src/decoder/mod.rs`
- Move: `crates/fornax-raw-decoder/src/dnc.rs`
  to `crates/fornax-raw/src/dnc.rs`
- Move: decoder tests and benches to `crates/fornax-raw/`
- Modify: `crates/fornax-raw/src/lib.rs`
- Modify: `crates/fornax-raw/Cargo.toml`

**Interfaces:**
- Consumes: root raw contracts/errors and `crate::dng`.
- Produces: `fornax_raw::Decoder`.
- Produces: root vendor modules `canon`, `fujifilm`, `hasselblad`, `nikon`,
  `panasonic`, `phase_one`, and `sony`.
- Produces: decoder bench targets `decoder`, `dng_gap_probe`, and
  `flashraw_probe`.

- [x] **Step 1: Preserve vendor module paths**

Declare `pub mod decoder` at the crate root. Keep the seven vendor modules
under `fornax_raw::decoder` so decoder dispatch and format modules share one
namespace.

- [x] **Step 2: Rewrite moved imports**

Replace `fornax_raw_core::` with `crate::` and `fornax_dng::` with
`crate::dng::` throughout moved decoder source, tests, benches, and doctests.
Do not change decoder logic.

- [x] **Step 3: Merge decoder features and benchmark declarations**

Keep `mmap` and `zstd`. Add decoder and DNG dev dependencies needed by moved
tests and benches. Add all decoder bench targets and nested support files to the
raw crate.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p fornax-raw --test vendor_modules
cargo test -p fornax-raw --test features
cargo test -p fornax-raw --test decoder_equivalence
cargo bench -p fornax-raw --bench decoder --no-run
cargo bench -p fornax-raw --bench dng_gap_probe --no-run
cargo bench -p fornax-raw --bench flashraw_probe --no-run
```

Expected: all pass or report only the already documented missing optional
fixture as a skipped precondition, never as silent success.

### Task 6: Remove `fornax-devtool`'s raw dependency and update consumers

**Files:**
- Modify: `crates/fornax-devtool/Cargo.toml`
- Modify: `crates/fornax-devtool/src/{lib,bench,dirs}.rs`
- Create: `crates/fornax-raw/benches/support/mod.rs`
- Modify: raw benchmarks to use the local support module
- Modify: `crates/fornax-raw-cli/Cargo.toml` and source imports
- Modify: `crates/decoder-tracker/Cargo.toml` and source imports
- Modify: root `Cargo.toml`
- Delete: `crates/fornax-raw-core/`
- Delete: `crates/fornax-dng/`
- Delete: `crates/fornax-raw-decoder/`
- Delete: `crates/fornax-raw-pipeline/`

**Interfaces:**
- Consumes: all merged `fornax-raw` APIs.
- Produces: workspace member `fornax-raw-cli` and `decoder-tracker` using only
  `fornax-raw`.
- Produces: `fornax-devtool` with no `fornax-raw-core` dependency and no RAW
  contract imports.

- [x] **Step 1: Move raw-specific devtool helpers**

Move `synthetic_mono_image`, `raw_mosaic_bytes`, `raw_file`,
`DngBenchmarkInput`, and `dng_benchmark_inputs` into
`crates/fornax-raw/benches/support/mod.rs`. Keep generic `BenchParams`,
`BenchSampleBits`, `BenchSize`, `root_dir`, `temp_dir`, `setup`, and image
export helpers in `fornax-devtool`.

- [x] **Step 2: Update consumer imports and features**

Use `fornax_raw::error::{...}`, `fornax_raw::metadata::{...}`,
`fornax_raw::protocol::{...}`, `fornax_raw::dng`, and
`fornax_raw::writer::{...}`. Enable `metadata-json` and `zstd` on the single raw
dependency as needed.

- [x] **Step 3: Remove old packages and workspace dependencies**

Delete the four old crate directories and their `[workspace.dependencies]`
entries. Confirm `cargo metadata --no-deps` contains one raw package and no
references to the removed package names.

- [x] **Step 4: Verify GREEN**

Run:

```bash
cargo test -p fornax-devtool
cargo test -p fornax-raw-cli
cargo test -p decoder-tracker
cargo metadata --no-deps --format-version 1
```

Expected: all tests pass; metadata lists only `fornax-raw` for the merged raw
API.

### Task 7: Update current documentation and run full validation

**Files:**
- Modify: `README.md`
- Modify: `.github/workflows/ci.yml`
- Modify: `crates/fornax-raw/README.md`
- Modify: `docs/architecture/overview.md`
- Modify: `docs/architecture/decoder.md`
- Modify: `crates/fornax-codec/README.md`
- Modify: `crates/fornax-tiff/README.md`
- Modify: current `book/` source links and examples that reference removed crates

**Interfaces:**
- Produces: current documentation that names only the merged `fornax-raw`
  surface.
- Preserves historical `docs/archive/` references.

- [x] **Step 1: Update crate map and public paths**

Document `fornax-raw` as owner of core contracts, DNG, native decoders, pipeline
stages, and `fornax_raw::writer`. Remove package-level references to the four
deleted crates from current documentation. Remap CI changed-file filters and
benchmark matrix entries to `fornax-raw`, preserving format-specific decoder
benchmark selection.

- [x] **Step 2: Update commands and doctests**

Replace old package commands with `cargo test/bench -p fornax-raw` and old Rust
paths with `fornax_raw::...` public paths.

- [x] **Step 3: Run focused documentation checks**

Run:

```bash
python3 scripts/check-doc-links.py
cargo test -p fornax-raw --doc
```

Expected: links resolve and all merged doctests pass.

- [ ] **Step 4: Run task validation**

Run:

```bash
cargo test -p fornax-raw
cargo test -p fornax-raw --no-default-features
cargo test -p fornax-raw --all-features
cargo bench -p fornax-raw --no-run
scripts/agent-validation.sh
```

Expected: all commands pass. Record any environment-gated fixture skips and the
exact failing command if a full validation step cannot complete.

- [x] **Step 5: Inspect and commit**

Run:

```bash
git status --short
git diff --check
git diff --stat
```

Confirm no binary or fixture payload is staged, then commit the consolidation as
one semantic change.
