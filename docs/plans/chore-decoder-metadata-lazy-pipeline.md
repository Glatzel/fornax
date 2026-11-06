# Decoder Metadata and Lazy Pipeline Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task.

**Goal:** Remove repeated DNG metadata work, accelerate packed sample decode,
and establish a deferred preprocessing handoff for postprocessing.

**Architecture:** `DngImage` caches its merged metadata so accessor calls no
longer rebuild maps. Packed integer samples decode directly into their final
storage type. `LazyNormalizedMosaic` defers preprocessing until a consuming
stage requests the normalized image, while existing eager APIs remain.

**Base branch:** `chore/tiff-dng-crates`.

## Scope

In scope:

- cache merged DNG metadata and process geometry;
- avoid redundant metadata construction in decode paths;
- direct u16 decoding for packed samples and 16-bit samples;
- crop before pipeline conversion;
- add a lazy normalized-mosaic handoff;
- preserve existing stage APIs and numerical behavior;
- validate against decoder benchmarks when fixtures are available.

Out of scope:

- making every demosaic algorithm row-streaming;
- camera color rendering;
- Zstandard;
- CLI changes;
- changing the conventional 2x2 Bayer boundary.

## Task Checklist

- [x] Add failing tests for cheap repeated metadata and direct packed decode.
- [x] Cache merged `DngMetadata` in `DngImage`.
- [x] Decode packed 1-16-bit samples directly into `u8`/`u16`.
- [x] Decode 16-bit samples without an intermediate byte image.
- [x] Crop before conversion in `cropped_mosaic`.
- [x] Add lazy normalized preprocessing and a postprocessor handoff.
- [x] Run full validation and benchmark compilation.
- [x] Record performance evidence available in this environment.

## Validation Results

- `cargo test --workspace --all-features --exclude compare-demosaic`: 135
  passed across 21 suites.
- `cargo +nightly fmt --check`: passed.
- Repository rustdoc command: passed.
- Root clippy flag set: no issues.
- `cargo bench -p bench-decoder --no-run`: passed.
- `git diff --check`: passed.
- Fixture-backed timing could not run because the ignored colorchart fixture
  is absent from this worktree. The code paths now avoid the full metadata
  rebuild, packed `u32` intermediate, full 16-bit byte-image intermediate, and
  conversion of inactive-area samples.

## Public Interfaces

- Existing `IDecoder` methods retain their signatures.
- `DngImage::dng_metadata()` returns a cheap clone of cached metadata.
- `fornax_preprocessor::LazyNormalizedMosaic<'a, T>` exposes dimensions,
  row normalization, and `collect()`.
- `fornax_core::protocol::IPreprocessor<T>` gains a default `lazy` entry that
  materializing implementations can use without breaking existing callers.
- `PostProcessor` gains a `process_lazy` path that collects when an algorithm
  requires a dense working image.

## Files Expected to Change

- `crates/fornax-dng/src/dng_decoder/mod.rs`
- `crates/fornax-dng/src/dng_decoder/process_metadata.rs`
- `crates/fornax-core/src/protocol.rs`
- `crates/fornax-core/src/primitive.rs`
- `crates/fornax-preprocessor/src/lib.rs`
- `crates/fornax-postprocessor/src/lib.rs`
- `crates/fornax-postprocessor/src/demosaic/protocol.rs`
- decoder/preprocessor/postprocessor tests and READMEs
- `docs/architecture.md`, `docs/decoder/architecture.md`

## Validation

```bash
cargo test --workspace --all-features --exclude compare-demosaic
cargo +nightly fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --exclude compare-demosaic
cargo clippy --workspace --all-features --all-targets --exclude compare-demosaic -- <root AGENTS.md flags>
cargo bench -p bench-decoder --no-run
git diff --check
```

Fixture-backed timing runs are executed when `FORNAX_BENCH_RAW` points to a
valid image; otherwise only compile checks and synthetic tests are reported.

## Decisions and Notes

- Dense demosaic implementations may still materialize the normalized mosaic;
  the lazy API removes a mandatory eager boundary rather than claiming every
  algorithm is streaming.
- Existing eager `process` paths remain supported for compatibility and for
  callers that explicitly need an intermediate image.
