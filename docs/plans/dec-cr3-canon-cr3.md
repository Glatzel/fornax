# Canon CR3/CRM Lossless Slice Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Status:** Complete. The final 110-file remote corpus rerun was clean, produced
no baseline-update artifact, and the header audit uploaded separately for
review.

**Goal:** Decode provably lossless, single-sample, conventional 2x2 RGB Bayer
Canon CR3/CRM files into the existing Fornax `U16` mosaic and metadata
interfaces, while returning typed unsupported errors for C-RAW and every
ambiguous layout.

**Architecture:** Add a generic ISO-BMFF parser, a feature-gated Canon CRX
codec, and a vendor-only CR3 crate. The facade dispatches CR3/CRM to the vendor
crate. Lossless discrimination comes from audited CMP1 header values; the codec
keeps `enc_type` for future work but rejects types other than the validated
lossless value.

**Tech Stack:** Rust 2024, `fornax-core`, `fornax-tiff`, `fornax-codec`, Rayon,
`thiserror`, Cargo features, raw.pixls.us transient LFS fixtures.

**Spec:** The user-supplied DEC-CR3 request and `TODO.md`.

## Global Constraints

- Production dependencies remain `fornax-core`, `fornax-tiff`,
  `fornax-codec`, Rayon, and existing workspace dependencies; add no new
  third-party crates.
- `fornax-bmff` contains no Canon boxes or HEIF item parsing and depends only
  on `fornax-core`.
- Lossy C-RAW, preview/HEIF/dual-pixel data, non-2x2 layouts, unknown
  ColorData versions, and ambiguous raw tracks return typed errors before
  preprocessing.
- The selected raw sample is index 0.
- CR3 and CRM share the `crx` brand; the filename extension identifies the
  public `RawFormat::Crm`. `ftyp` is validated for the CRX brand but does not
  distinguish CR3 from CRM.
- The full 110-file corpus must never run on the local workstation. The audit
  tool and tracker commands are prepared for the remote corpus workflow.
- RAW payloads are transient, never committed, and never uploaded.

## Review Focus

- CMP1 `enc_type` values can separate lossless from lossy; an ambiguous or
  unobserved combination must not be guessed.
- IAD1 endpoints are sometimes inclusive and active-area endpoints can
  overshoot CMP1; conversion must happen exactly once and remain bounded.
- CRX plane placement and CFA origin parity must stay aligned after cropping.
- Malformed sample tables and box sizes must fail before large allocations or
  panics.
- CMT TIFF payload offsets and CTMD record bounds must not fabricate metadata.

---

### Task 1: Audit tool and validated header scope

**Files:**
- Create: `crates/fornax-devtool/src/bin/canon-crx-audit.rs`
- Modify: `crates/fornax-devtool/Cargo.toml`
- Create: `docs/references/canon-crx.md`

**Interfaces:**
- Consumes: raw file paths or manifest CSV rows.
- Produces: per-file CMP1 version, image/median precision, encoding type,
  wavelet levels, tile dimensions, track count, ColorData version, CFA layout,
  and classification CSV; documentation pins the accepted lossless tuple.

- [ ] Add a failing test for `classify_cmp1` using synthetic lossless, C-RAW,
  ambiguous, and unknown tuples.
- [ ] Add the audit binary, manifest filtering, and bounded box walk.
- [ ] Run the audit only over targeted local fixtures and record that the full
  corpus run is deferred to the remote tracker workflow.
- [ ] Commit the audit and reference note.

### Task 2: Generic `fornax-bmff`

**Files:**
- Create: `crates/fornax-bmff/Cargo.toml`
- Create: `crates/fornax-bmff/src/lib.rs`
- Create: `crates/fornax-bmff/src/box.rs`
- Create: `crates/fornax-bmff/src/sample_table.rs`
- Modify: root `Cargo.toml`

**Interfaces:**
- Produces: `FourCc`, `BoxHeader`, `BoxRef`, `Bmff<'a>`, `FullBox`,
  `SampleTable`, `SampleTable::parse`, and checked sample resolution by
  one-based sample number.

- [ ] Write failing tests for 32-bit, 64-bit, to-EOF, UUID, nested child, path,
  size cap, depth cap, and truncation parsing.
- [ ] Implement bounded box traversal and full-box parsing.
- [ ] Write failing tests for `stsc`/`stsz`/`stco`/`co64`, entry caps,
  malformed mappings, constant sample size, chunk offsets, and truncation.
- [ ] Implement the checked sample-table resolver.
- [ ] Run `cargo test -p fornax-bmff` and commit.

### Task 3: Lossless CRX codec

**Files:**
- Create: `crates/fornax-codec/src/crx/mod.rs`
- Create: `crates/fornax-codec/src/crx/bitstream.rs`
- Create: `crates/fornax-codec/src/crx/entropy.rs`
- Create: `crates/fornax-codec/src/crx/header.rs`
- Create: `crates/fornax-codec/src/crx/wavelet.rs`
- Modify: `crates/fornax-codec/Cargo.toml`
- Modify: `crates/fornax-codec/src/lib.rs`
- Modify: `crates/fornax-codec/src/request.rs`

**Interfaces:**
- Produces: `CanonCrxCfa`, `CanonCrxHeader`, its validating constructor, and
  `DecodeCompression::CanonCrx { header, cfa }`.
- `fornax_codec::decode` returns one `SampleBuffer::U16` mosaic.

- [ ] Write failing tests for MSB bit trimming, adaptive Rice, escapes, and
  checked shifts.
- [ ] Write failing tests for JPEG-LS run mode, MED prediction, line edges, and
  signed error mapping.
- [ ] Write failing tests for CMP1/CRX header validation, tile/subband bounds,
  5/3 wavelet odd/boundary cases, all four CFA placements, and lossy rejection.
- [ ] Implement the bounded lossless path, QP/Q-step checks required by
  lossless headers, disjoint Rayon tile/plane decoding, and payload
  plausibility before allocation.
- [ ] Run `cargo test -p fornax-codec --features cr3` and
  `cargo check -p fornax-codec --no-default-features --features cr3`.
- [ ] Commit.

### Task 4: `fornax-cr3` vendor layer

**Files:**
- Create: `crates/fornax-cr3/Cargo.toml`
- Create: `crates/fornax-cr3/src/lib.rs`
- Create: `crates/fornax-cr3/src/container.rs`
- Create: `crates/fornax-cr3/src/metadata.rs`
- Create: `crates/fornax-cr3/src/color_data.rs`
- Create: `crates/fornax-cr3/README.md`
- Modify: root `Cargo.toml`

**Interfaces:**
- Produces: `Cr3Image::parse(&[u8])`, `Cr3Image::parse_sample(&[u8], usize)`,
  `raw_mosaic`, `cropped_mosaic<T>`, `dng_metadata`, and `process_metadata`.

- [ ] Write failing tests for CRX brand, CCTP image type 0 selection, unique
  CRAW fallback, preview/JPEG exclusion, ambiguous tracks, and sample bounds.
- [ ] Write failing tests for CMT TIFF payload offsets, inclusive IAD1 crop,
  active-area overshoot clamping, crop-as-active fallback, and odd CFA origin.
- [ ] Write failing tests for ColorData versions observed in the audit,
  unknown-version process-metadata errors, CTMD allowlists, and Light Raw mode
  130 (`AsShotNeutral = 1,1,1`).
- [ ] Implement BMFF selection, CMT parsing through `fornax-tiff`, metadata
  allowlists, CMP1/IAD1 parsing, and codec dispatch.
- [ ] Write synthetic CR3/CRM sample-selection tests.
- [ ] Run `cargo test -p fornax-cr3` and commit.

### Task 5: Facade and real-fixture validation

**Files:**
- Modify: `crates/fornax-decoder/Cargo.toml`
- Modify: `crates/fornax-decoder/src/lib.rs`
- Modify: `crates/fornax-raw-core/src/format.rs`
- Create: `crates/fornax-decoder/tests/cr3.rs`

**Interfaces:**
- Produces: `RawFormat::Crm`, `Decoder::Cr3(Cr3Image)`, and unchanged
  `IDecoder`/`IDecoderDng` behavior.

- [ ] Write failing dispatch tests for `.cr3`, `.crm`, lossless handoff,
  non-2x2 rejection, and C-RAW rejection.
- [ ] Add dispatch and typed error propagation.
- [ ] Use only targeted cached fixtures: R5 lossless RAW, R5 C-RAW, an R6 Mark
  II lossless RAW, and R6 Mark II C-RAW. Do not run the full corpus.
- [ ] Verify one lossless fixture reaches `RawMosaic::U16`, cropped `f32`, DNG
  metadata, process metadata, and DNG writer round-trip.
- [ ] Run facade tests and commit.

### Task 6: Golden entries, tracker baseline, and documentation

**Files:**
- Modify: `crates/decoder-tracker/golden/selected-manifest.csv`
- Modify: `crates/decoder-tracker/golden/decode-hashes.csv`
- Modify: `crates/decoder-tracker/baseline.csv`
- Modify: `crates/decoder-tracker/README.md`
- Modify: `docs/architecture.md`
- Modify: `docs/decoder/architecture.md`
- Modify: `docs/decoder/formats/canon-cr3.md`
- Modify: `docs/references/canon-crx.md`
- Modify: `TODO.md`

**Interfaces:**
- Consumes: targeted release-mode decoder hashes and tracker classifications.
- Produces: additive golden rows, reviewed typed-unsupported tracker rows, and
  current architecture/format documentation.

- [ ] Add two lossless golden rows from different camera families without
  changing existing rows.
- [ ] Prepare lossy expected-unsupported tracker rows and record reasons.
- [ ] Dispatch the remote corpus workflow after decoder commits are pushed;
  review its complete diff output before applying the baseline.
- [ ] Regenerate the tracker README with `update-badge.sh` and repeat until the
  remote run is clean.
- [ ] Remove only DEC-CR3 from `TODO.md` after every gate passes.
- [ ] Run the full validation commands listed below and commit.

## Validation

```bash
cargo test -p fornax-bmff -p fornax-cr3
cargo test -p fornax-codec --features cr3
cargo check -p fornax-codec --no-default-features --features cr3
cargo test -p fornax-bmff -p fornax-cr3 -p fornax-codec -p fornax-decoder --all-features
cargo test -p fornax-bmff -p fornax-cr3 -p fornax-codec -p fornax-decoder --all-features --doc
cargo test -p decoder-tracker
cargo clippy --all-targets --all-features -- -D warnings
cargo fmt --check
scripts/agent-validation.sh
git diff --check
```

Tracker DuckDB tests, golden hash verification, and corpus-fixture tests are
run through `scripts/agent-validation.sh`. The 110-file corpus classification
is remote-only.

## Decisions and notes

- Losslessness is accepted only for the exact CMP1 tuple pinned by the audit;
  all other values are `Unsupported`.
- CCTP image type 0 wins; otherwise exactly one CRAW track is required.
  Preview, JPEG, HEIF, and auxiliary dual-pixel tracks are never selected.
- The first decoded sample is index 0, as requested.
- CR3 and CRM both require the `crx` compatible brand. The extension selects
  `RawFormat::Cr3` versus `RawFormat::Crm`.
- IAD1 right/bottom endpoints are made half-open once. CMP1 overshoot is
  clamped; an otherwise valid crop outside active area replaces active area.
- CTMD metadata is allowlisted. No missing black or white level is replaced
  with fabricated zero/default values.
- Full local corpus execution is prohibited by the operator. The tooling and
  remote workflow remain the authority for complete classification.

### Final results

- The remote corpus classified 110/110 CR3 rows without a local full-corpus
  run.
- 28 files pass all decoder, metadata, and handoff stages.
- 82 files return typed unsupported errors: 51 are lossy with three wavelet
  levels and 31 are ambiguous zero-level layouts outside the validated tuple.
- ColorData versions 16, 17, 18, 19, 32, 33, 34, 48, 64, 65, and 66 were
  observed by the audit.
- Targeted R5 and R6 Mark II mosaics match Rawler 0.8.0 digests exactly.
- `scripts/agent-validation.sh`, workspace tests, docs, strict clippy, nightly
  formatting, golden hashes, and decoder benchmark compilation pass.
