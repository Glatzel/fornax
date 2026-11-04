# DEC-MIGRATE and DEC-DNG finalization

**Tasks:** `DEC-MIGRATE`, `DEC-DNG`

## Goal

Finalize the branch around the implemented pure-Rust DNG MVP: decoding,
typed metadata preservation, conventional 2x2 Bayer handoff, and uncompressed
DNG export. Refresh all active documentation and comparison fixtures for the
DNG-only architecture, add fixture-backed regression coverage, and leave every
repository check green or explicitly reported.

## Scope

In scope:

- Existing `fornax_core::Decoder` and `fornax_core::writer` behavior only; no
  public API changes and no new code documentation.
- Integration coverage for the accepted
  `external/raw-images/images/colorchart-5D2-6000K.dng` fixture.
- DNG-based demosaic comparison baseline, README output, snapshots, and
  waivers.
- Active agent, architecture, decoder, README, TODO, and plan documentation.
- Removal of orphaned DNC/libraw-bindgen migration tooling.

Out of scope:

- Byte-identical source-preserving lossless export; it requires decoded-source
  state that the current public API does not expose.
- BigTIFF, Deflate, JPEG XL, opcode, linearization, corpus, and non-2x2 CFA
  support beyond the code already present.
- Changes to the excluded `fornax-py` API or implementation.
- Human-facing book content and historical changelog entries.

## Task checklist

### Task 1: Add fixture-backed DNG integration coverage

- [x] Add `tempfile.workspace = true` to `fornax-core` dev dependencies.
- [x] Add `crates/fornax-core/tests/dng_roundtrip.rs`.
- [x] Assert full-frame dimensions, active/default crop geometry, full GBRG
  and active RGGB patterns, black/white levels, representative metadata, and
  the accepted sample hash.
- [x] Export uncompressed DNG and assert exact sample and selected-tag
  round-trip.
- [x] Assert default lossless export returns the existing typed `Unsupported`
  error.
- [x] Run the integration test and fix only defects it exposes.

### Task 2: Repair branch checks without API or code-doc additions

- [x] Correct the five existing doctest imports from `fornax_decoder` to their
  current `fornax_core` paths.
- [x] Replace repeated `MosaicData` variant names with `Self` in
  `MosaicData::dimensions`.
- [x] Remove the unnecessary semicolon after the compression match.
- [x] Run nightly rustfmt.
- [x] Normalize trailing whitespace and CRLF artifacts reported by
  `git diff --check`.
- [x] Run doctests, exact clippy, and formatting checks.

### Task 3: Refresh demosaic comparisons for the DNG fixture

- [x] Run the full comparison using the accepted DNG and inspect verdict
  changes.
- [x] Replace `crates/compare-demosaic/baseline/canonical.json` with the new
  deterministic full-run output.
- [x] Update existing waivers only where the same DNG rows still pass all
  limits and the measured bounds/evidence can be updated. Remove unused
  waivers. Do not add a new accepted outlier without human review.
- [x] Regenerate marker-controlled `crates/compare-demosaic/README.md`
  sections and update the input prose.
- [x] Accept DNG-derived Insta checksum snapshots after confirming only
  expected checksum values changed.
- [x] Run comparison tests, `readme --check`, full run, and baseline diff.

### Task 4: Reconcile active documentation and migration cleanup

- [x] Update `AGENTS.md` and `crates/compare-demosaic/AGENTS.md` for the
  `fornax-core` decoder ownership, DNG fixture, and legacy-backend removal.
- [x] Update active architecture, decoder format/status, benchmark, corpus,
  reference-note, root README, and crate README guidance.
- [x] Rewrite this plan's progress, files, validation, and decisions as work
  completes.
- [x] Remove `DEC-DNG` and `DEC-MIGRATE` from `TODO.md`.
- [x] Remove the orphaned libraw bindgen workflow/script and DNC
  installer/CI steps.
- [x] Remove the unused workspace `bindgen` dependency and refresh the lock
  file.

### Task 5: Full verification and handoff

- [x] Run the complete validation matrix below with LibRaw environment
  variables set.
- [x] Run task-specific benchmark compilation and the pipeline example.
- [x] Verify production dependency trees do not include removed legacy
  decoder crates.
- [x] Inspect the full diff and commit on `task/dec-dng-roundtrip`.
- [x] Push the branch and create or report the semantic pull request.

## Files expected to change

- `AGENTS.md`
- `README.md`
- `TODO.md`
- `.github/workflows/ci.yml`
- `.github/workflows/libraw-bindgen.yml` (delete)
- `Cargo.toml`, `Cargo.lock`
- `crates/fornax-core/Cargo.toml`
- `crates/fornax-core/src/error.rs`
- `crates/fornax-core/src/format.rs`
- `crates/fornax-core/src/metadata/dng.rs`
- `crates/fornax-core/src/primitive.rs`
- `crates/fornax-core/src/writer.rs`
- `crates/fornax-core/tests/dng_roundtrip.rs`
- `crates/fornax-core/README.md`
- `crates/fornax-core/benches/decoder.rs`
- `crates/fornax-preprocessor/README.md`
- `crates/fornax-preprocessor/benches/preprocessor.rs`
- `crates/fornax-postprocessor/benches/demosaic.rs`
- `crates/fornax-postprocessor/benches/support/demosaic_registry.rs`
- `crates/fornax-postprocessor/src/demosaic/linear.rs`
- `crates/fornax-postprocessor/src/demosaic/lmmse.rs`
- `crates/fornax/examples/pipeline.rs`
- `crates/fornax/README.md`
- `crates/compare-demosaic/AGENTS.md`
- `crates/compare-demosaic/README.md`
- `crates/compare-demosaic/baseline/canonical.json`
- `crates/compare-demosaic/src/snapshots/*.snap`
- `crates/compare-demosaic/src/status.rs`
- `crates/compare-demosaic/src/waivers.rs`
- `docs/architecture.md`
- `docs/decoder/architecture.md`
- `docs/decoder/benchmark.md`
- `docs/decoder/corpus.md`
- `docs/decoder/formats/*.md`
- `docs/references/dcb.md`
- `docs/references/dng-lossless-jpeg.md`
- `docs/plans/dec-dng-roundtrip.md`
- `scripts/install-dnc.ps1` (delete)
- `scripts/libraw-bindings.ps1` (delete)

## Validation

Use:

```bash
export LIBRAW_ROOT=/home/glatzel/.pixi/envs/base
export LD_LIBRARY_PATH=/home/glatzel/.pixi/envs/base/lib
export OMP_NUM_THREADS=1
```

Then run:

```bash
cargo test -p fornax-core --test dng_roundtrip
cargo test --workspace --all-features
cargo +nightly fmt --check
cargo clippy --workspace --all-features --all-targets -- <exact AGENTS.md lint list>
RUSTFLAGS=-Dwarnings cargo doc --workspace --all-features --no-deps
cargo bench -p fornax-core --no-run
cargo run -p fornax --example pipeline
cargo test -p compare-demosaic --all-features
cargo run --release -p compare-demosaic -- readme --check
cargo run --release -p compare-demosaic -- run --artifacts
cargo run --release -p compare-demosaic -- diff \
  crates/compare-demosaic/baseline/canonical.json \
  target/compare-demosaic/canonical.json
git diff --check
```

Run `markdownlint-cli2` on changed Markdown if the executable is available;
otherwise report that the repository-configured local lint was unavailable.

Observed on this worktree:

- `cargo test --workspace --all-features`: 261 passed across 16 suites.
- `cargo test -p compare-demosaic --all-features`: 149 passed across 3 suites.
- `cargo clippy` with the exact repository lint list: no issues.
- `cargo +nightly fmt --check`, `git diff --check`, and the docs build with
  warnings denied: passed.
- `cargo bench -p fornax-core --no-run` and the DNG pipeline example: passed.
- `compare-demosaic run --artifacts` followed by `diff`: no differences with
  `OMP_NUM_THREADS=1`.
- Production dependency-tree scan: no `fornax-decoder`, Rawler, LibRaw,
  bindgen, or `clang-sys` dependency.
- `markdownlint-cli2` is not installed in this environment; the configured
  lint could not be run locally.
- Branch pushed to `origin/task/dec-dng-roundtrip`; existing PR updated at
  <https://github.com/Glatzel/fornax/pull/642>.

## Decisions and notes

- The current DNG MVP is the completion boundary. The existing default
  lossless writer intentionally returns `Unsupported`; no source-preserving
  behavior is introduced because it would require an API change.
- The accepted DNG replaces the CR2 as the authoritative comparison input.
- Existing accepted-outlier bounds are not transferable to the DNG without
  remeasurement. New unreviewed outliers remain warnings rather than being
  waived.
- All four old waivers were removed: PPG and darktable RCD now pass without
  outliers, while AAHD/AHD and RawTherapee RCD retain unwaived warnings.
- Canonical comparison runs pin `OMP_NUM_THREADS=1`. LibRaw DHT produced
  different one-code tails with one versus eight OpenMP threads, so the thread
  count is fixed at the process boundary rather than changing reference math.
- `fornax-py` remains excluded and untouched while its public API is
  stabilized.
- External LibRaw/darktable/RawTherapee references remain development-only;
  `libraw-dev` remains required to build `compare-demosaic`.

## Progress

- [x] Initial branch and failure audit completed.
- [x] Integration coverage, code checks, comparison refresh, and active
  documentation cleanup completed.
- [x] Full validation completed and recorded.
- [x] Branch pushed and PR #642 updated.
