# Experimental DNG Zstandard and Writer Benchmarks

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Add opt-in pure-Rust Zstandard DNG read/write support and benchmark
its encode speed and size against uncompressed, JPEG-92, and Deflate output.

**Architecture:** TIFF compression 50000 is recognized as Zstandard behind a
`fornax-dng/zstd` feature. The writer emits independent 256x256 Zstandard
tiles, and `fornax-dng` owns Criterion writer measurements. The default
interoperable DNG compression remains LosslessJPEG.

**Base branch:** `chore/pipeline-preview-color`.

## Scope

In scope:

- pure-Rust Zstandard decode and encode behind a feature;
- TIFF compression 50000 support;
- predictor-compatible Zstandard tiles;
- writer benchmarks for compression speed and output size;
- U8/U16/U32/F32 round-trip tests;
- feature-off typed errors and interoperability notes.

Out of scope:

- making Zstandard the default;
- claims of Adobe DNG standardization;
- C-backed `zstd-sys`;
- host-dependent committed timing baselines.

## Task Checklist

- [x] Add the pure-Rust codec dependency and failing feature-gated tests.
- [x] Implement Zstandard tile decode.
- [x] Implement Zstandard tile encode with a level option.
- [x] Add writer benchmark diagnostics and environment controls.
- [x] Update DNG and reference support tables.
- [x] Run full validation, including all features and feature-off builds.

## Validation Results

- `cargo test -p fornax-dng`: 32 passed with the feature disabled.
- `cargo test -p fornax-dng --all-features`: 33 passed.
- A 0.38MP U16 synthetic writer run measured Zstandard at about 1.19 ms and
  751,321 bytes versus Deflate at about 3.85 ms and 752,866 bytes. This input
  is deliberately noisy, so the size result is useful only as a harness check;
  representative RAW sizes remain a local fixture task.
- `cargo test --workspace --all-features --exclude compare-demosaic`: 142
  passed.
- Rustdoc, nightly formatting, the root clippy flag set, writer benchmark
  compilation, and `git diff --check` all passed.

## Public Interfaces

- `CompressionMethod::Zstd` maps to TIFF compression 50000.
- `DngCompression::Zstd` is always representable.
- `DngWriter::with_zstd_level(level: i32)` selects the writer level; default
  level is 3.
- Without feature `zstd`, reading or writing compression 50000 returns
  `DecodeError::Unsupported` or `DngWriteError::Unsupported`.

## Benchmark Controls

- `FORNAX_BENCH_DNG_COMPRESSION=uncompressed,lossless-jpeg,deflate,zstd`
- `FORNAX_BENCH_ZSTD_LEVEL=3`
- `FORNAX_BENCH_SIZE` reuses `fornax-devtool`.
- Output size and ratio are printed once per case, outside timed iterations.

## Validation

```bash
cargo test -p fornax-dng
cargo test -p fornax-dng --features zstd
cargo test --workspace --all-features --exclude compare-demosaic
cargo +nightly fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --exclude compare-demosaic
cargo clippy --workspace --all-features --all-targets --exclude compare-demosaic -- <root AGENTS.md flags>
cargo bench -p fornax-dng --features zstd --bench writer --no-run
git diff --check
```

## Decisions and Notes

- Compression 50000 is the libtiff/GDAL de-facto assignment, not an
  Adobe-registered DNG compression code.
- The implementation must not introduce `zstd-sys` or a system library.
- Zstandard uses the same predictor rules as Deflate; each tile is an
  independent frame so tiles remain parallel and independently recoverable.
