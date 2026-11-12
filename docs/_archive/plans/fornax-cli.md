# Independent Fornax CLI Implementation Plan

This archived plan documents the completed Independent Fornax CLI work.

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Replace the pipeline example with a standalone batch RAW converter
that writes DNG, TIFF, JPEG, and optional DNG metadata JSON.

**Architecture:** `fornax-cli` owns argument/config parsing, input discovery,
output naming, bounded file-level parallelism, and output encoding. It uses
Fornax's decoder pipeline for rendered images and `DngWriter` for raw DNG
output. CPU- and memory-bound conversion uses Rayon, not an async runtime.

**Base branch:** `chore/dng-zstd-writer-bench`.

## Scope

In scope:

- standalone `fornax-cli` package with binary `fornax`;
- positional file/directory inputs;
- TOML and JSON config with CLI overrides;
- recursive `RawFormat` discovery and mirrored output paths;
- DNG, 16-bit linear TIFF, and 8-bit preview JPEG output;
- DNG-to-DNG skipping;
- optional DNG metadata sidecar JSON;
- postprocessor algorithm, white-balance, and color-matrix controls;
- bounded file concurrency with default one job;
- tests for config, discovery, naming, skipping, and output encoding.

Out of scope:

- DNC integration;
- X-Trans or non-Bayer output;
- full camera profile/tone rendering;
- watching folders;
- network or GUI features.

## Defaults

- input: positional arguments, or config `inputs`;
- output: `./converted`;
- format: `dng`;
- decoder: `auto`;
- jobs: `1`;
- recursive: `true`;
- existing output: skip;
- metadata JSON: off;
- demosaic: `Demosaic::default()` with default parameters;
- white balance: on;
- camera matrix: on;
- JPEG quality: `90`;
- DNG compression: LosslessJPEG.

## Public CLI

```text
fornax [--config PATH] [OPTIONS] [INPUT...]
```

Required options are unnecessary because defaults provide a usable pipeline.
Explicit flags override config. Config paths resolve relative to the config
file; explicit CLI paths resolve relative to the current directory.

## Task Checklist

- [x] Add failing config/default tests.
- [x] Add failing recursive discovery and mirrored naming tests.
- [x] Add failing conversion tests for skip and metadata sidecars.
- [x] Add failing TIFF/JPEG conversion tests.
- [x] Implement package scaffolding, config merging, and discovery.
- [x] Implement decoder selection and rendered-image conversion.
- [x] Implement DNG synthesis/writing and metadata JSON.
- [x] Implement TIFF/JPEG encoders and bounded job execution.
- [x] Remove the pipeline example and update Pixi/facade documentation.
- [x] Show resolved defaults in `fornax --help`.
- [x] Run full validation and record results.

## Validation Results

- `cargo test -p fornax-cli`: 8 passed.
- `cargo test --workspace --all-features --exclude compare-demosaic`: 150
  passed across 25 suites.
- `cargo run -p fornax-cli -- --help`: passed and exposed all configured
  defaults/options.
- Nightly formatting, repository rustdoc, the root clippy flag set, benchmark
  compilation, and `git diff --check`: passed.
- Fixture-backed external RAW conversion remains unrun because the ignored RAW
  fixtures are absent; integration tests use generated synthetic DNG data,
  including a non-DNG-extension DNG payload through Rawler.
- Post-rebase conflict validation: `cargo test -p fornax-cli` passed 8 tests,
  the workspace all-feature suite passed 160 tests across 28 suites, and
  `scripts/agent-validation.sh` passed with the raw-image fixture initialized.
- Help-defaults follow-up: `cargo test -p fornax-cli` passed 8 tests, and
  `cargo run -p fornax-cli -- --help` showed every resolved default.
  `scripts/agent-validation.sh --fix` passed the full workspace validation.

## Files Expected to Change

- `Cargo.toml`
- `crates/fornax-cli/**`
- `crates/fornax-raw/examples/pipeline.rs`
- `crates/fornax-raw/README.md`
- `pixi.toml`
- `.github/workflows/ci.yml`
- `docs/architecture/overview.md`

## Validation

```bash
cargo test -p fornax-cli
cargo test --workspace --all-features --exclude compare-demosaic
cargo +nightly fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --exclude compare-demosaic
cargo clippy --workspace --all-features --all-targets --exclude compare-demosaic -- <root AGENTS.md flags>
cargo bench -p bench-decoder --no-run
cargo run -p fornax-cli -- --help
git diff --check
```

Fixture-backed direct and directory conversions run when the ignored RAW
fixtures exist. Otherwise tests use an internal fixture decoder.

## Decisions and Notes

- TIFF stores linear RGB; JPEG is a preview with the sRGB transfer curve.
- DNG output preserves raw mosaic samples; rendered formats run preprocess and
  demosaic.
- Directory mapping is `<output>/<input-root-name>/<relative-path>`.
- Explicit duplicate output targets fail before any file is processed.
- After rebasing onto the repaired PR 654 stack, metadata sidecars use the
  merged TIFF `Directory` plus `directory_to_json_value`.
- The existing `color_matrix = true` contract maps to
  `OutputColorSpace::Srgb`; disabling it maps to `OutputColorSpace::Raw`.
- Rendered output uses `Preprocessor::process` followed by
  `IPostProcessor::process`.
- Help text documents defaults without setting Clap defaults on the optional
  CLI fields, preserving explicit CLI > file configuration > built-in
  precedence.
