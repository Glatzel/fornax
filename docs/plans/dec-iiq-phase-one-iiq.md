# Phase One IIQ Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [x]`) syntax for tracking.

**Goal:** Implement native Phase One/Leaf IIQ decoding for modes 0, 3, 5, 6,
and 8, including sensor corrections, DNG-compatible metadata, facade routing,
corpus coverage, and Bayer pipeline handoff.

**Architecture:** Add feature-gated IIQ sample decoders to `fornax-codec` and a
native `fornax-iiq` format crate for the TIFF/private-maker-note container,
camera profiles, metadata mapping, and post-decode sensor correction. Route
`.IIQ` through `fornax-decoder` without adding a reference implementation to
production dependencies.

**Tech Stack:** Rust 2024, `fornax-codec`, `fornax-iiq`, `fornax-decoder`,
`fornax-tiff`, `fornax-dng`, `fornax-raw-core`, `image`, `rayon`, and the
existing decoder tracker.

**Spec:** `docs/decoder/formats/phase-one-iiq.md`

## Global Constraints

- Support only conventional 2x2 RGB Bayer images in the preprocessing and
  demosaic handoff.
- Accept IIQ modes 0, 3, 5, 6, and 8. Reject legacy modes 1 and 2 and unknown
  modes with typed errors.
- Scope is Phase One/Leaf `.IIQ`; `.MOS`, Phase One `.TIF`, previews, and
  legacy Raw1/Raw2 remain outside this task.
- Do not add Rawler, LibRaw, darktable, or RawTherapee to production
  dependencies and do not copy or adapt implementation source.
- Never commit or upload RAW payloads. Corpus and golden files remain
  pointer-only.
- Preserve existing DNG, ARW, NEF, RAF, codec, tracker, and golden behavior.
- Use checked arithmetic and bounds validation before allocation or pixel
  access.
- Branch: `task/dec-iiq-phase-one-iiq`.
- Worktree: `.worktrees/dec-iiq-phase-one-iiq`.

## Scope

In scope:

- TIFF header and Phase One private maker-note parsing.
- IIQ uncompressed, IIQ-L/L16, IIQ-S, and IIQ-Sv2 sample decoding.
- Accepted Phase One and Leaf camera profiles from the pinned corpus.
- Base/split black levels, polynomial curves, quadrant calibration, bad-column
  repair, and flat-field correction.
- DNG-compatible geometry, CFA, levels, white balance, identity, and EXIF
  metadata.
- Facade routing, DNG round-trip tests, selected Rawler parity, golden hashes,
  full IIQ corpus tracking, documentation, and reference notes.

Out of scope:

- IIQ modes 1 and 2.
- Leaf `.MOS` and Phase One `.TIF` containers.
- Preview extraction.
- Guessed CFA profiles or synthesized camera color matrices for unknown models.
- Changes that admit non-2x2 CFA data to preprocessing or demosaicing.

## Review Focus

- Private offsets relative to file offset 8: wrong bases corrupt every later
  table or pixel read.
- Unsorted or duplicate row offsets: wrong row assembly can silently
  transpose or duplicate image rows.
- Correction ordering and signed split blacks: values can remain plausible
  while differing from the reference output.
- Truncated correction payloads: malformed floating-point or table data must
  fail instead of panicking or reading stale bytes.
- Unknown camera/model combinations: an incorrectly guessed CFA can produce a
  full-looking image with wrong colors.

## Task Checklist

- [x] Add the IIQ codec API and table-driven malformed-input tests.
- [x] Implement uncompressed, IIQ-L/L16, IIQ-S, and IIQ-Sv2 decoding.
- [x] Add the `fornax-iiq` crate, container parser, profiles, and metadata.
- [x] Implement sensor correction and DNG round-trip coverage.
- [x] Route `.IIQ` through the default decoder and document the format.
- [x] Add fixture parity and golden rows; run the complete IIQ corpus.
- [x] Run workspace validation, review the diff, update `TODO.md`, commit, push,
  and open one pull request.

## Files Expected to Change

- `Cargo.toml`, `Cargo.lock`, `TODO.md`
- `crates/fornax-codec/{Cargo.toml,README.md,src/lib.rs,src/request.rs}`
- `crates/fornax-codec/src/iiq.rs`
- `crates/fornax-iiq/**`
- `crates/fornax-decoder/{Cargo.toml,README.md,src/lib.rs,tests/iiq*.rs}`
- `crates/decoder-tracker/{golden/**,baseline.csv,README.md}`
- `docs/{architecture.md,decoder/architecture.md,decoder/formats/phase-one-iiq.md}`
- `docs/references/phase-one-iiq.md`
- `docs/plans/dec-iiq-phase-one-iiq.md`

## Validation

```bash
cargo test -p fornax-codec --features iiq
cargo test -p fornax-iiq -p fornax-decoder
cargo test -p decoder-tracker
pixi run -e tracker cargo test -q -p decoder-tracker duckdb_
pixi run -e dev cargo run -q -p decoder-tracker -- manifest --check
pixi run -e dev cargo run --release -q -p decoder-tracker -- hash --check \
  crates/decoder-tracker/golden/decode-hashes.csv
cargo bench -p fornax-decoder --no-run
scripts/agent-validation.sh
git diff --check
```

Fixture parity uses `FORNAX_RAW_SAMPLES` and skips missing or pointer-only
payloads. The manual corpus filter is `**/*.[iI][iI][qQ]` with
`run_duckdb_tests=false`.

## Tasks

### Task 1: Add the IIQ codec API

**Files:**

- Modify: `crates/fornax-codec/Cargo.toml`
- Modify: `crates/fornax-codec/src/request.rs`
- Modify: `crates/fornax-codec/src/lib.rs`
- Create: `crates/fornax-codec/src/iiq.rs`

**Interfaces:**

- Produces: `DecodeCompression::IiqUncompressed`,
  `DecodeCompression::IiqLossless`, `DecodeCompression::IiqNonlinear`, and
  `DecodeCompression::IiqSv2` behind feature `iiq`.

- [x] Add failing tests for unsupported sample type, geometry, chunk count, and
  truncated row streams.
- [x] Run `cargo test -p fornax-codec --features iiq` and confirm the expected
  compile/feature failure.
- [x] Add the feature, enum variants, dispatch, and strict request validation.
- [x] Run the focused tests and commit.

### Task 2: Implement IIQ sample decoders

**Files:**

- Modify: `crates/fornax-codec/src/iiq.rs`

**Interfaces:**

- Produces: U16 `SampleBuffer` output for all four IIQ decode variants.

- [x] Add hand-derived tests for uncompressed 14-to-16-bit scaling, IIQ-L
  alternating predictors, IIQ-S low-value linearization, and IIQ-Sv2 clusters.
- [x] Run the tests and confirm local decoding assertions fail.
- [x] Implement the four row decoders with checked geometry and fixed output
  allocation.
- [x] Run codec tests and commit.

### Task 3: Add the IIQ container and metadata crate

**Files:**

- Create: `crates/fornax-iiq/**`
- Modify: `Cargo.toml`, `Cargo.lock`

**Interfaces:**

- Produces: `fornax_iiq::IiqImage::{parse,raw_mosaic,dng_metadata,process_metadata,cropped_mosaic}`.

- [x] Add synthetic parser tests for file-relative and private-relative
  offsets, mode classification, dimensions, row tables, CFA, white balance,
  and malformed bounds.
- [x] Run `cargo test -p fornax-iiq` and confirm the crate/API is missing.
- [x] Implement profiles, parser, metadata mapping, and decoder interfaces.
- [x] Run crate tests and commit.

### Task 4: Implement sensor correction

**Files:**

- Modify: `crates/fornax-iiq/src/**`

**Interfaces:**

- Produces: corrected U16 `RawMosaic` samples and zero corrected black level.

- [x] Add small hand-derived correction tests for split black, curves,
  quadrant gain, defect repair, and flat fields.
- [x] Run the correction tests and confirm the transform assertions fail.
- [x] Implement the reference correction order and malformed-payload errors.
- [x] Add fixture parity and DNG round-trip tests, run the focused suite, and
  commit.

### Task 5: Route and document IIQ

**Files:**

- Modify: `crates/fornax-decoder/{Cargo.toml,README.md,src/lib.rs}`
- Modify: `crates/fornax-codec/README.md`
- Modify: `docs/{architecture.md,decoder/architecture.md}`
- Modify: `docs/decoder/formats/phase-one-iiq.md`
- Create: `docs/references/phase-one-iiq.md`

**Interfaces:**

- Produces: `Decoder::Iiq(IiqImage)` for case-insensitive `.iiq`.

- [x] Add a failing facade routing test.
- [x] Route `.IIQ` and update all format documentation and provenance notes.
- [x] Run decoder and documentation checks and commit.

### Task 6: Add golden and corpus coverage

**Files:**

- Modify: `crates/decoder-tracker/{golden/**,baseline.csv,README.md}`

**Interfaces:**

- Produces: accepted IIQ golden hashes and the reviewed full IIQ baseline.

- [x] Select representative IIQ mode 3, 5, 6, and 8 fixtures for golden hashes.
- [x] Run the full pinned IIQ corpus through the manual CI workflow, review
  failures, and update the complete baseline.
- [x] Generate golden hashes and the tracker README, verify unchanged reruns,
  and commit.

### Task 7: Validate and publish

**Files:**

- Modify: `TODO.md`
- Modify: `docs/plans/dec-iiq-phase-one-iiq.md`

- [x] Run all focused tests, fixture parity, golden verification,
  `scripts/agent-validation.sh`, and `git diff --check`.
- [x] Review the full branch diff, remove `DEC-IIQ` from `TODO.md`, and record
  final validation results in this plan.
- [x] Commit, push, and open one semantic pull request.

## Decisions and Notes

- Correction output is the `raw_mosaic` stage, matching Rawler. Corrected
  samples therefore export `BlackLevel=0` and `WhiteLevel=65535`.
- Supported correction tags are `0x0400`, `0x0401`, `0x040b`, `0x0410`,
  `0x0416`, `0x0419`, `0x041a`, `0x041e`, `0x041f`, and `0x0431`.
  Unrelated payload tags observed in valid correction directories are ignored;
  malformed supported payloads are errors.
- Unknown camera models receive no guessed CFA and return
  `DecodeError::Unsupported`.
- Mode 0 is covered synthetically because the current `.IIQ` corpus snapshot
  has no materialized mode-0 sample.
- P45+ incomplete split-black tables follow Rawler by being ignored when only
  one signed row/column table is present; both P45+ corpus files match Rawler.
- P65+ corpus files use a 260-byte quadrant-linearization payload rather than
  the supported 1028-byte table. They return `DecodeError::Unsupported`, making
  five P65+ rows intentional failures in the accepted baseline.
- P40+ remains an unknown camera profile and returns
  `DecodeError::Unsupported`; two P40+ rows are intentional failures.
- CI run `43047950209` completed all four IIQ shards successfully. The
  aggregate job intentionally failed after uploading the changed baseline.
  Review accepted 28/35 rows: 23/28 Phase One, 1/3 Phase One A/S, and 4/4 Leaf.
- The complete reviewed artifact updated `baseline.csv`; the regenerated
  tracker badge reports 567/2020 corpus rows passing across 117 pairs.
- `scripts/agent-validation.sh` passed after the final baseline and formatting
  updates.
- Final self-review found a malformed zero-length IIQ-L selector could panic
  before validation. The new regression test first failed with the underflow,
  then passed after explicit zero-length rejection. Compressed rows now also
  require complete 32-bit words.
- The maker-note parser now shares the TIFF parser's source `Arc` instead of
  copying the full RAW payload a second time.
