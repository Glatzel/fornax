# Decoder Benchmark and Devtool Controls Implementation Plan

This archived plan documents the completed Decoder Benchmark and Devtool Controls work.

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Make decoder comparisons semantically fair and consistent with the
shared Fornax benchmark controls.

**Architecture:** `fornax-devtool` owns reusable environment parsing. The
decoder benchmark consumes its sample-bit selection, adds decoder selection
and composite cases, and reports metadata coverage. The Rawler adapter gains
the same active-area crop and synthesized DNG metadata needed for a meaningful
comparison.

**Tech Stack:** Rust 2024, Criterion compatibility harness,
`fornax-devtool`, `rawler`, and synthetic `RawImage` data.

## Scope

In scope:

- default benchmark sample type `f32`;
- `FORNAX_BENCH_BITS` reuse in `bench-decoder`;
- `FORNAX_BENCH_DECODER` selection;
- composite decode-plus-mosaic cases;
- Rawler active-area cropping;
- Rawler DNG metadata synthesis and caching;
- benchmark README and CI path updates.

Out of scope:

- decoder algorithm optimizations;
- lazy preprocessing;
- TIFF/DNG crate migration;
- fixed corpus acquisition or baseline files.

## Task Checklist

- [ ] Add failing tests for the default bit selection and decoder-list parser.
- [ ] Switch `BenchParams` to default to `32` and update stage READMEs.
- [ ] Add Rawler crop and metadata regression tests.
- [ ] Implement Rawler cropping and cached synthesized metadata.
- [x] Refactor decoder benchmarks to use `BenchParams`, decoder selection, and
      exactly the five `IDecoder` methods.
- [ ] Add `bench-decoder/README.md`.
- [ ] Update CI path filters.
- [ ] Run validation and record measured results.

## Files Expected to Change

- `crates/fornax-devtool/src/bench.rs`
- `crates/fornax-devtool/README.md`
- `crates/bench-decoder/benches/decoder.rs`
- `crates/bench-decoder/README.md`
- `crates/fornax-decoder-rawler/src/lib.rs`
- `crates/fornax-raw-pipeline/README.md`
- `.github/workflows/ci.yml`
- `docs/architecture/overview.md`
- `docs/architecture/decoder.md`
- `docs/decoder/benchmark.md`

## Interfaces

- `BenchParams::from_env()` produces `f32` by default and preserves explicit
  `32,64` ordering.
- `bench-decoder` reads `FORNAX_BENCH_DECODER` as a comma-separated subset of
  `fornax` and `rawler`; an unset or empty value selects both.
- Benchmark names follow `<decoder>/<sample>/<IDecoder method>`.
- `RawlerDecoder::cropped_mosaic()` returns samples cropped to
  `ProcessMetadata::active_area` and converts the Bayer pattern relative to
  that crop.
- `RawlerDecoder::dng_metadata()` returns a cached synthesized DNG map
  containing dimensions, CFA, crops, levels, white balance, camera identity,
  and available color matrices.

## Validation

```bash
cargo test -p fornax-devtool
cargo test -p fornax-decoder-rawler
cargo test --workspace --all-features --exclude compare-demosaic
cargo bench -p bench-decoder --no-run
cargo fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --exclude compare-demosaic
cargo clippy --workspace --all-features --all-targets --exclude compare-demosaic -- <root AGENTS.md lint flags>
git diff --check
```

`FORNAX_BENCH_BITS=32 FORNAX_BENCH_DECODER=fornax,rawler
cargo bench -p bench-decoder --bench decoder` is run when the ignored
colorchart fixture is available. Individual and composite results are recorded
in the pull request; no timing baseline is committed.

## Decisions and Notes

- Rawler metadata previously came only from `RawImage::dng_tags`, which is
  empty for the colorchart DNG. Synthesizing metadata from the decoded
  `RawImage` makes the adapter complete enough for DNG output and makes the
  benchmark visible rather than comparing against an empty map.
- Rawler currently returns the full frame from `cropped_mosaic`. Cropping is
  required before comparing decoder accessor performance.
- `FORNAX_BENCH_DECODER` is parsed by a benchmark-local function; no separate
  support library is needed.

### Task 1: Shared Sample-Type Default

**Files:** `crates/fornax-devtool/src/bench.rs`,
`crates/fornax-devtool/README.md`, `crates/fornax-raw-pipeline/README.md`

- [x] Add a test proving an unset-equivalent default is `[F32]`.
- [x] Change the default from `32,64` to `32`.
- [x] Update all three benchmark READMEs.

### Task 2: Rawler Crop and Metadata

**Files:** `crates/fornax-decoder-rawler/src/lib.rs`

- [x] Add failing tests for active-area cropping and non-empty synthesized DNG
      metadata.
- [x] Cache synthesized metadata in `RawlerDecoder`.
- [x] Crop and cast only the active area in `cropped_mosaic`.

### Task 3: Decoder Selection and Five IDecoder Cases

**Files:** `crates/bench-decoder/benches/decoder.rs`,
`crates/bench-decoder/README.md`

- [x] Add failing parser tests for `FORNAX_BENCH_DECODER`.
- [x] Consume `BenchParams::bits`.
- [x] Add exactly five IDecoder method cases and metadata coverage reporting.
- [x] Document every environment variable and command.

### Task 4: CI and Architecture Documentation

**Files:** `.github/workflows/ci.yml`, `docs/architecture/overview.md`,
`docs/architecture/decoder.md`, `docs/decoder/benchmark.md`

- [x] Update decoder benchmark path filters and documented behavior.

### Task 5: Full Validation

**Files:** no expected source changes.

- [x] Run package tests, benchmark compilation, and the repository validation
      suite.
- [x] Record results in this plan.

## Validation Results

- `cargo test --workspace --all-features --exclude compare-demosaic`: 130
  passed across 17 suites.
- `cargo +nightly fmt --check`: passed.
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features
  --exclude compare-demosaic --no-deps`: passed.
- The root clippy flag set: no issues.
- `cargo bench -p bench-decoder --bench decoder --no-run`: passed.
- Source validation confirms exactly five benchmark functions and no
  `bench-decoder/src/lib.rs`.
- An invalid `FORNAX_BENCH_DECODER` value fails at benchmark startup with the
  documented error.
- `git diff --check` and YAML parsing of `.github/workflows/ci.yml`: passed.
- The ignored colorchart fixture is absent from this worktree, so no timing
  run was possible locally. The benchmark remains executable when setup
  supplies `FORNAX_BENCH_RAW`.
- No Markdown lint executable is installed in the active environment.
