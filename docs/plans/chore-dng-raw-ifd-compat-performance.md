# DNG Raw-IFD Compatibility and Performance Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Decode every valid conventional 2x2 RGB Bayer DNG raw IFD without an
unsupported result, accelerate the existing DNG path to Rawler-class
performance, and implement lossless JPEG-92 DNG writing without changing the
public API.

**Architecture:** Keep the current crates and public traits. Replace repeated
directory discovery with an internal parsed raw-image plan, decode independent
strips and tiles with Rayon, use focused in-tree codec/transform modules, and
keep auxiliary TIFF directories deferred behind an explicit documentation
record. `fornax-core::writer` gains JPEG-92 output and a lossless Deflate
fallback while retaining the existing signatures and enums.

**Tech Stack:** Rust 2024, `tiff`, `rayon`, `flate2`, `zune-jpeg`,
`jxl-oxide`, local Criterion benchmarks, and synthetic in-memory DNG
fixtures.

**Spec:** The approved implementation plan supplied by the user and the
repository constraints in `AGENTS.md`, `docs/architecture.md`, and
`docs/decoder/formats/dng.md`.

## Global Constraints

- Do not change public types, trait methods, function signatures, enum
  variants, or public fields.
- The active pipeline supports conventional RGB 2x2 Bayer only. Non-Bayer,
  monochrome, four-color, X-Trans, and arbitrary-CFA inputs remain outside the
  handoff.
- Do not add trivial production helper functions. Private helpers must carry a
  distinct codec, validation, or transformation responsibility.
- Keep `fornax-decoder` free of Rawler, LibRaw, darktable, RawTherapee, and
  C/C++ codec dependencies.
- Raw-IFD completeness is the current task. Preview, thumbnail, mask, depth,
  nested Exif/GPS, source-byte-preserving export, and full OpcodeList2/3
  execution are deferred and documented.
- Apply `LinearizationTable` and `OpcodeList1` to stored raw samples; preserve
  `OpcodeList2` and `OpcodeList3` as metadata.
- `DngCompression::LosslessJpeg` writes JPEG-92 for exact U8/U16 samples and
  lossless Deflate for wider integer or floating-point samples.
- Do not modify `TODO.md`.
- One branch and pull request carry all changes:
  `chore/dng-raw-ifd-compat-performance`.

## Task Checklist

- [ ] Establish the worktree, record the baseline, and document the selected
  raw-IFD support boundary.
- [ ] Refactor TIFF parsing into a validated raw decode plan, cache metadata,
  and make `process_metadata` independent of pixel decoding.
- [ ] Add the Rayon-backed strip/tile execution path and complete uncompressed
  sample decoding, prediction, and pixel layout handling.
- [ ] Optimize in-tree lossless JPEG-92 and cover its DNG profiles.
- [ ] Add Deflate, lossy JPEG, and JPEG XL raw payload decoding.
- [ ] Add linearization, interleave, sub-tile, and raw opcode transforms.
- [ ] Implement JPEG-92 writing and lossless fallback writing in
  `fornax-core`.
- [ ] Add regression, malformed-input, fixture, round-trip, and benchmark
  coverage.
- [ ] Update architecture/format/reference documentation, including deferred
  auxiliary TIFF work.
- [ ] Run all required validation, inspect the diff, commit, push, and open the
  pull request.

## Files Expected to Change

- `Cargo.toml`, `Cargo.lock`, `crates/fornax-decoder/Cargo.toml`, and
  `crates/fornax-core/Cargo.toml`
- `crates/fornax-decoder/src/lib.rs`
- `crates/fornax-decoder/src/dng_decoder/mod.rs`
- `crates/fornax-decoder/src/dng_decoder/tiff.rs`
- `crates/fornax-decoder/src/dng_decoder/ljpeg.rs`
- Focused decoder modules for compression, transforms, and decode planning
- `crates/fornax-decoder/src/dng_decoder/process_metadata.rs`
- `crates/fornax-core/src/writer.rs`
- Focused writer JPEG-92/Deflate modules
- `crates/bench-decoder/benches/decoder.rs` if additional benchmark coverage
  is required
- `docs/architecture.md`, `docs/decoder/architecture.md`,
  `docs/decoder/formats/dng.md`, `docs/references/dng-lossless-jpeg.md`, and
  this plan

## Validation

```bash
cargo test --workspace --all-features --exclude compare-demosaic
cargo fmt --check
RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --exclude compare-demosaic
cargo clippy --workspace --all-features --all-targets --exclude compare-demosaic -- \
    -A clippy::too_many_arguments \
    -D warnings
cargo bench -p bench-decoder --no-run
git diff --check
```

Run the repository's complete clippy flag list from the root `AGENTS.md`.
Run the decoder benchmark with the colorchart fixture and compare
`raw_mosaic`, `cropped_mosaic`, and `process_metadata` against Rawler.

## Decisions and Notes

- On local `main`, the 345-tile colorchart fixture measures 495 ms for
  `raw_mosaic`, 519 ms for `cropped_mosaic`, and 497 ms for
  `process_metadata`. Parallel tile decode, avoiding pixel work in metadata
  access, and a faster entropy decoder are the primary performance changes.
- The branch measures 51.4 ms for `raw_mosaic`, 78.9 ms for
  `cropped_mosaic`, and 190 us for `process_metadata` with the same local
  command.
- The in-tree JPEG codec is retained to preserve checked malformed-input
  handling and avoid the LGPL obligation of the standalone `ljpeg` crate.
- `flate2`, `zune-jpeg`, and `jxl-oxide` are already resolved transitively at
  compatible versions; production crates will declare direct dependencies
  rather than relying on transitive imports.
- Auxiliary TIFF directories are intentionally not represented by the flat
  `DngMetadata` map. A future API design must model directory identity and
  source-byte retention before this scope can expand.
- Acceptance is measured with the same local Criterion command against
  `origin/main`: `raw_mosaic` and `cropped_mosaic` must improve materially,
  and `process_metadata` must perform no pixel decode.

### Task 1: Workspace and Baseline

**Files:**
- Create: `docs/plans/chore-dng-raw-ifd-compat-performance.md`

**Interfaces:**
- Consumes: current `origin/main`.
- Produces: branch/worktree and the implementation contract.

- [x] Create the required branch and worktree from `origin/main`.
- [x] Record the local `main` benchmark baseline and supported raw-IFD boundary
  in this plan.
- [x] Run `cargo test -p fornax-decoder --lib` and confirm the pre-change suite.
- [x] Commit the plan.

### Task 2: Parsed Raw Decode Plan

**Files:**
- Modify: `crates/fornax-decoder/src/dng_decoder/tiff.rs`
- Modify: `crates/fornax-decoder/src/dng_decoder/mod.rs`
- Modify: `crates/fornax-decoder/src/lib.rs`

**Interfaces:**
- Consumes: current TIFF parser and `DngMetadata`.
- Produces: private parsed raw-directory state cached in `ParsedTiff`.

- [x] Add a failing test proving `process_metadata` does not decode pixel data.
- [x] Add parser tests for classic/BigTIFF, both endiannesses, field types,
  malformed offsets, and raw-IFD selection.
- [x] Cache selected raw metadata and geometry once during parsing.
- [x] Replace decoder-path unwraps with existing typed errors.
- [x] Run decoder tests and commit.

### Task 3: Parallel Chunk Decode and Uncompressed Samples

**Files:**
- Modify: `crates/fornax-decoder/src/dng_decoder/mod.rs`
- Create: focused compression/decode modules under
  `crates/fornax-decoder/src/dng_decoder/`

**Interfaces:**
- Consumes: parsed raw decode plan from Task 2.
- Produces: strip/tile execution and `RawMosaic` sample decoding.

- [x] Add failing uncompressed tests for packed depths, floats, strips, tiles,
  predictors, and final partial chunks.
- [x] Parallelize disjoint destination bands with Rayon while reusing bounded
  scratch storage.
- [x] Implement uncompressed integer/float decoding and validation.
- [x] Run decoder tests and commit.

### Task 4: Lossless JPEG-92 Decoder

**Files:**
- Modify: `crates/fornax-decoder/src/dng_decoder/ljpeg.rs`
- Modify: tile dispatch in `crates/fornax-decoder/src/dng_decoder/mod.rs`

**Interfaces:**
- Consumes: encoded tile/strip slices from Task 3.
- Produces: caller-buffer `u16` JPEG-92 samples.

- [x] Add tests for 12-bit precision in a 16-bit container, predictors 1-7,
  14-bit difference categories, point transforms, parallel padded tiles, and
  two-component tiles.
- [ ] Add subsampled-component and restart-marker coverage.
- [x] Replace the per-sample bit path with a fused lookahead entropy decoder.
- [x] Decode directly into caller-owned tile scratch buffers.
- [x] Run decoder tests and commit.

### Task 5: Other DNG Compression Backends

**Files:**
- Modify: `Cargo.toml`, decoder manifest/lockfile
- Create: Deflate, lossy JPEG, and JPEG XL decoder modules

**Interfaces:**
- Consumes: parsed chunk geometry and compression metadata.
- Produces: `RawMosaic` outputs for compression 8, 34892, and 52546.

- [x] Add backend tests using minimal valid Deflate and lossy-JPEG payloads.
- [x] Implement Deflate with supported predictors and sample formats.
- [x] Implement lossy JPEG and JPEG XL decoding with checked conversions.
- [x] Run decoder tests and commit.

### Task 6: Raw-Domain Transforms

**Files:**
- Create: focused raw transform/opcode module
- Modify: raw sample completion in `dng_decoder/mod.rs`

**Interfaces:**
- Consumes: decoded stored samples.
- Produces: transformed `RawMosaic` and preserved later-domain metadata.

- [x] Add failing tests for linearization, row/column interleave, and
  `OpcodeList1`.
- [ ] Add `SubTileBlockSize` coverage and explicit Lists 2/3 regression tests.
- [x] Apply transforms in DNG order with checked bounds.
- [x] Preserve unconsumed tags in `DngMetadata`.
- [x] Run decoder tests and commit.

### Task 7: Lossless DNG Writer

**Files:**
- Modify: `crates/fornax-core/src/writer.rs`
- Modify: `crates/fornax-core/Cargo.toml`, root manifest/lockfile
- Create: focused writer JPEG-92/Deflate module

**Interfaces:**
- Consumes: existing `DngWriter::write_dng_with` signature.
- Produces: lossless tiled JPEG-92 or Deflate output without API changes.

- [x] Add failing round-trip tests for U8/U16 JPEG-92 and U32/F32 Deflate.
- [x] Implement deterministic tiled JPEG-92 encoding.
- [x] Implement the lossless Deflate fallback.
- [ ] Add automatic classic/BigTIFF selection for outputs that exceed classic
  TIFF limits.
- [x] Filter consumed/layout-dependent metadata and run core tests.
- [ ] Commit.

### Task 8: Fixture and Regression Coverage

**Files:**
- Modify: decoder/core test modules
- Modify: `crates/bench-decoder/benches/decoder.rs` only if needed

**Interfaces:**
- Consumes: Tasks 2-7.
- Produces: end-to-end correctness and malformed-input evidence.

- [x] Decode native DNGs from Canon, Nikon, Sigma, and Ricoh end to end.
- [x] Decode generated Sony ILCE-1M2 and Nikon Z 7 II DNGs end to end.
- [x] Compare decoded and preprocessed samples with Rawler where supported;
  Apple, Nikon, Sigma, Ricoh, Sony, and Canon CFA samples match exactly after
  crop normalization.
- [x] Add no-panic malformed-input tests for every supported codec.
- [x] Verify writer output decodes through Fornax for every mosaic storage
  type.
- [x] Verify generated JPEG-92 DNG output through the Rawler adapter.
- [x] Run tests and commit.

### Task 9: Documentation and Deferred Scope

**Files:**
- Modify: `docs/architecture.md`
- Modify: `docs/decoder/architecture.md`
- Modify: `docs/decoder/formats/dng.md`
- Modify: `docs/references/dng-lossless-jpeg.md`

**Interfaces:**
- Consumes: final implementation behavior.
- Produces: accurate support matrix and explicit deferred auxiliary TIFF work.

- [x] Document the supported compression/layout/bit-depth matrix.
- [x] Document raw-domain opcode order and lossless writer fallback.
- [x] Record auxiliary TIFF directories, source-byte export, and OpcodeList2/3
  execution as deferred.
- [x] Run `git diff --check` and commit; a Markdown-lint executable is not
  installed in the active environment.

### Task 10: Full Validation and Pull Request

**Files:**
- No expected source changes unless validation finds a defect.

**Interfaces:**
- Consumes: all prior tasks.
- Produces: committed, pushed branch and one pull request.

- [x] Run all repository-required Rust checks and compile the decoder benchmark.
- [x] Run the colorchart benchmark locally against `main`.
- [ ] Inspect the full diff and dependency tree.
- [ ] Commit any fixes, push the branch, and open one semantic pull request.
  Add the `perf` label to the pull request.
