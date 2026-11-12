# Merge Preprocessing and Postprocessing Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Merge the production preprocessing and postprocessing implementations
into `fornax-raw-pipeline` without changing pipeline behavior. PR #690 later
deleted the reduced compatibility packages.

**Architecture:** A new `fornax-raw-pipeline` crate owns normalization,
demosaic protocols and algorithms, color processing, and both stage
benchmarks. The `fornax-raw` facade adds a `raw_pipeline` module and keeps its
`preprocessor` and `postprocessor` module paths as re-exports; `fornax-cli`
uses the new facade module, and `compare-demosaic` imports the new crate
directly.

**Tech Stack:** Rust 2024, Cargo workspace, Rayon, Criterion, `fornax-raw-core`

**Spec:** `docs/references/workspace-refactor-baseline.md`, Step 2 only

## Scope

In scope:

- Create `crates/fornax-raw-pipeline`.
- Move the normalization implementation from `fornax-raw-pipeline`.
- Move the demosaic protocol, all demosaic algorithms, `Demosaic`, `LmmseMode`, `OutputColorSpace`, and `PostProcessor` from `fornax-raw-pipeline`.
- Preserve the public item names `Preprocessor`, `Demosaic`, `LmmseMode`, `IDemosaic`, `LazyDemosaic`, `OutputColorSpace`, and `PostProcessor`.
- Move both Criterion benchmark targets to the new production crate.
- Update `fornax`, `fornax-cli`, and `compare-demosaic` to consume `fornax-raw-pipeline`.
- Keep the former compatibility re-export packages during this historical step
  because its required validation ran their package tests.
- Update architecture/component documentation and CI benchmark routing for the merged crate.

Out of scope:

- `TODO.md` and Step 3 work.
- Neutral buffer shape and vendor codec placement.
- Codec output, TIFF raster ownership, DNG constants, format crates, workspace shape, or publication changes.
- Any demosaic algorithm, color-matrix, white-balance, normalization, or default-behavior change.
- A new name for either retired implementation beyond `fornax-raw-pipeline`.

## Review Focus

- A consumer importing one of the required symbols from `fornax_raw_pipeline` must compile and retain its existing behavior.
- The former compatibility crate imports must remain usable through reduced
  compatibility packages during this historical step.
- `fornax_raw::preprocessor`, `fornax_raw::postprocessor`, and the new `fornax_raw::raw_pipeline` facade must all resolve to the same moved types.
- Both moved Criterion targets must compile even though one package now contains the preprocessor and demosaic benchmarks.
- The decoder golden hashes must remain byte-for-byte unchanged by the crate move.

## Task 1: Create the merged pipeline crate with TDD

**Files:**

- Create: `crates/fornax-raw-pipeline/Cargo.toml`
- Create: `crates/fornax-raw-pipeline/src/lib.rs`
- Create: `crates/fornax-raw-pipeline/src/preprocessor.rs`
- Create: `crates/fornax-raw-pipeline/src/postprocessor.rs`
- Create: `crates/fornax-raw-pipeline/src/demosaic.rs`
- Move the demosaic module tree into `crates/fornax-raw-pipeline/src/demosaic/*`.
- Move the preprocessing benchmark into `crates/fornax-raw-pipeline/benches/preprocessor.rs`.
- Move the demosaic benchmark into `crates/fornax-raw-pipeline/benches/demosaic.rs`.
- Move: `crates/fornax-raw-pipeline/tests/protocol.rs` coverage into the new crate or retain it in the compatibility package
- Test: `crates/fornax-raw-pipeline/tests/public_api.rs`
- Modify: `Cargo.toml`

**Interfaces:**

- Consumes: `fornax_raw_core::{CroppedMosaic, MonoImage, NormalizedMosaic, Primitive, RgbImage}` and core stage traits/errors.
- Produces: `fornax_raw_pipeline::{Preprocessor, Demosaic, LmmseMode, IDemosaic, LazyDemosaic, OutputColorSpace, PostProcessor}` with the same signatures and module-private implementation code as the source crates.

- [x] **Step 1: Add a failing public API and behavior test**

Create `crates/fornax-raw-pipeline/tests/public_api.rs`:

```rust
use fornax_raw_core::metadata::Levels;
use fornax_raw_core::protocol::IPreprocessor;
use fornax_raw_core::{BayerPattern, CroppedMosaic, MonoImage};
use fornax_raw_pipeline::{
    Demosaic, IDemosaic, LazyDemosaic, LmmseMode, OutputColorSpace, PostProcessor, Preprocessor,
};

#[test]
fn merged_pipeline_exposes_existing_stage_api() -> mischief::Result<()> {
    let cropped = CroppedMosaic::new(MonoImage::from_raw(4, 4, vec![0.5_f32; 16]).unwrap());
    let levels = Levels {
        black: MonoImage::from_raw(2, 2, vec![0.0; 4]).unwrap(),
        black_delta_h: Vec::new(),
        black_delta_v: Vec::new(),
        white: 1.0,
    };
    let normalized = Preprocessor.process(cropped, &levels)?;
    let rgb = Demosaic::Linear
        .demosaic(normalized.image(), BayerPattern::RGGB)?
        .collect();
    assert_eq!(rgb.dimensions(), (4, 4));

    let processor = PostProcessor::<f32, Demosaic>::default();
    assert_eq!(processor.output_color, OutputColorSpace::Raw);
    assert_eq!(LmmseMode::default(), LmmseMode::Median);
    Ok(())
}
```

- [x] **Step 2: Run the test to verify the missing crate fails**

Run: `cargo test -p fornax-raw-pipeline --test public_api`

Expected: Cargo fails because package `fornax-raw-pipeline` is not a workspace member or dependency.

- [x] **Step 3: Add the package and move the implementations unchanged**

Create the crate manifest with the union of the old production dependencies
and dev-dependencies. Move normalization into `src/preprocessor.rs`; split the
former postprocessor `lib.rs` into `src/postprocessor.rs` plus a small
`src/lib.rs`; move the demosaic module tree unchanged. Update only crate-local
import paths and rustdoc examples to `fornax_raw_pipeline::...`. Move both
benchmark files and update their imports.

- [x] **Step 4: Run the focused API test and moved crate tests**

Run: `cargo test -p fornax-raw-pipeline`

Expected: PASS, including the moved algorithm tests, postprocessor tests, protocol tests, and doctests.

- [x] **Step 5: Compile both benchmark targets**

Run: `cargo bench -p fornax-raw-pipeline --no-run`

Expected: PASS with both `preprocessor` and `demosaic` benchmark executables built.

## Task 2: Redirect consumers and reduce the old packages

**Files:**

- Modify: `crates/fornax-raw/Cargo.toml`
- Modify: `crates/fornax-raw/src/lib.rs`
- Create: `crates/fornax-raw/src/raw_pipeline.rs`
- Modify: `crates/fornax-raw/src/preprocessor.rs`
- Modify: `crates/fornax-raw/src/postprocessor.rs`
- Modify: `crates/fornax-cli/src/config.rs`
- Modify: `crates/fornax-cli/src/convert.rs`
- Modify: `crates/compare-demosaic/Cargo.toml`
- Modify: `crates/compare-demosaic/src/cases.rs`
- Modify: `crates/compare-demosaic/src/reference.rs`
- Modify: `crates/fornax-raw-pipeline/Cargo.toml`
- Replace implementation in: `crates/fornax-raw-pipeline/src/lib.rs`
- Modify: `crates/fornax-raw-pipeline/README.md`

**Interfaces:**

- Consumes: all symbols produced by Task 1.
- Produces: `fornax_raw::raw_pipeline::*`; existing `fornax_raw::preprocessor::Preprocessor` and `fornax_raw::postprocessor::{Demosaic, IDemosaic, LazyDemosaic, OutputColorSpace, PostProcessor}`; and compatibility re-exports from the two old crate names.

- [x] **Step 1: Add a failing facade/compatibility test**

Add compile-time usage in the existing facade or compatibility test coverage for:

```rust
let _: fornax_raw::raw_pipeline::Demosaic = fornax_raw::postprocessor::Demosaic::default();
let _: fornax_raw_pipeline::Preprocessor = fornax_raw::Preprocessor;
let _: fornax_raw_pipeline::Demosaic = fornax_raw::raw_pipeline::Demosaic::default();
```

Run: `cargo test -p fornax-raw -p fornax-raw-pipeline`

Expected: FAIL because `fornax_raw::raw_pipeline` is not yet exported.

- [x] **Step 2: Update the facade and consumers**

Export `fornax_raw_pipeline` as `fornax_raw::raw_pipeline`, preserve the existing facade modules as re-exports, and update the CLI and `compare-demosaic` imports. Keep all required public item names in `preprocessor` and `postprocessor`.

- [x] **Step 3: Reduce old packages to compatibility packages**

Remove their implementation modules and Criterion targets. Re-export the moved public API from `fornax-raw-pipeline`. Keep or add integration tests that import through the old package names.

- [x] **Step 4: Run affected packages and the workspace**

Run:

```bash
cargo test -p fornax-raw-pipeline
cargo test -p fornax-raw-pipeline
cargo test -p fornax-raw-pipeline
cargo test -p fornax-cli
cargo test --workspace --all-features --exclude compare-demosaic
```

Expected: every command exits 0 and the moved test count is preserved except tests intentionally relocated between package suites.

## Task 3: Update project documentation and CI routing

**Files:**

- Modify: `AGENTS.md`
- Modify: `README.md`
- Modify: `crates/fornax-raw-core/README.md`
- Modify: `docs/architecture.md`
- Modify: `.github/workflows/ci.yml`
- Modify: `docs/plans/chore-merge-pre-post-pipeline.md`

**Interfaces:**

- Consumes: the final crate and benchmark layout from Tasks 1 and 2.
- Produces: current architecture and CI references that point to `fornax-raw-pipeline`; historical plan files remain historical and are not rewritten.

- [x] **Step 1: Update current architecture and component references**

Describe `fornax-raw-pipeline` as the owner of normalization, demosaic protocols/algorithms, RGB reconstruction, and both stage benchmarks. State that the old crates are compatibility re-exports.

- [x] **Step 2: Retarget CI benchmark change detection**

Route preprocessor/demosaic benchmark changes to one raw-pipeline output, and run `fornax-raw-pipeline` with either `--bench demosaic` or `--bench preprocessor` as appropriate.

- [x] **Step 3: Update the plan checklist and decisions with completed work**

Record deviations and validation results in this plan before final validation.

## Task 4: Full validation and pull request

**Files:**

- Verify only; do not add generated baseline files.

- [x] **Step 1: Verify golden decoder hashes**

Run:

```bash
cargo run -q -p decoder-tracker -- hash --manifest crates/decoder-tracker/golden/selected-manifest.csv --check crates/decoder-tracker/golden/decode-hashes.csv
```

Expected: the check succeeds and reports unchanged hashes.

- [x] **Step 2: Run required package and workspace tests**

Run:

```bash
cargo test -p fornax-raw-pipeline
cargo test -p fornax-raw-pipeline
cargo test -p fornax-cli
cargo test --workspace --all-features --exclude compare-demosaic
```

Expected: every command exits 0.

- [x] **Step 3: Run repository validation**

Run: `scripts/agent-validation.sh`

Expected: manifest checks, workspace tests, formatting, docs, clippy, and decoder benchmark compilation pass. Because `compare-demosaic` is changed, run its validation only if its native setup is available; otherwise record it as skipped.

- [x] **Step 4: Run diff and whitespace checks**

Run: `git diff --check`

Expected: no output and exit 0.

- [x] **Step 5: Inspect and commit the complete diff**

Review the diff for only the intended Step 2 changes, then commit with a semantic message.

- [ ] **Step 6: Push and open one pull request**

Push `chore/merge-pre-post-pipeline` and open one PR against `main` titled `refactor: merge preprocessing and postprocessing`. Include scope, moved symbols, validation results, golden-hash result, and skipped checks. Do not merge, poll, or close the PR.

## Validation

Required commands:

```bash
cargo test -p fornax-raw-pipeline
cargo test -p fornax-raw-pipeline
cargo test -p fornax-cli
cargo test --workspace --all-features --exclude compare-demosaic
scripts/agent-validation.sh
cargo bench -p fornax-raw-pipeline --no-run
cargo run -q -p decoder-tracker -- hash --manifest crates/decoder-tracker/golden/selected-manifest.csv --check crates/decoder-tracker/golden/decode-hashes.csv
git diff --check
```

`compare-demosaic` is not compiled or run unless its native setup is available.

## Decisions and notes

- The former packages remained as reduced re-export packages during this
  historical step; PR #690 deleted them.
- The new crate's public API test passed with 95 package tests, and both Criterion targets compiled.
- The facade/consumer update passed 103 tests across the pipeline, facade, and
  CLI packages.
- The old package names remain unchanged; no additional crate renames are introduced.
- The branch was rebased onto `81aca76` after the workspace-refactor trigger documents were published; the trigger and baseline confirm Step 2 scope.
- Required package tests passed: preprocessor 1, postprocessor 1, CLI 6; workspace tests passed 262 tests across 41 suites.
- `scripts/agent-validation.sh` passed completely, including native `compare-demosaic` docs, clippy, and 150 tests.
- Both `fornax-raw-pipeline` benchmark targets compiled.
- The golden decoder check reported `golden: unchanged (9 rows)`.
- `TODO.md` remains untouched.
- Neutral buffer shape, vendor codec placement, codec output, TIFF ownership, DNG constants, format crates, and publication remain untouched.
- Step 3 is not started by this plan.
