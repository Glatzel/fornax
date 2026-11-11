# Fujifilm RAF Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement native Fujifilm RAF decoding for uncompressed and
Fujifilm-lossless standard Bayer and X-Trans images, preserving X-Trans through
DNG export while rejecting it at the Bayer-only pipeline handoff.

**Architecture:** Add a feature-gated Fujifilm codec to `fornax-codec` and a
native `fornax-raf` container/metadata crate. Wire the format through the
default decoder facade without changing the 2x2 Bayer preprocess/demosaic
contract.

**Tech Stack:** Rust 2024, `fornax-codec`, `fornax-raf`, `fornax-decoder`,
`fornax-tiff`, `fornax-dng`, `fornax-raw-core`, and the existing decoder
tracker.

**Spec:** `docs/decoder/formats/fuji-raf.md`

## Global Constraints

- Support uncompressed RAF and Fujifilm block-based lossless compression only.
- Lossy block compression, SuperCCD rotation/staggering, FinePix DBP, and
  unknown compression families return `DecodeError::Unsupported`.
- Standard 2x2 Bayer RAFs reach the existing preprocessing handoff.
- X-Trans RAFs expose storage-native samples and DNG metadata, but
  `process_metadata` and `cropped_mosaic` return a typed unsupported-CFA error.
- Do not add Rawler, LibRaw, or other reference implementations to production
  dependencies. Do not copy reference code, comments, or naming.
- Never commit or upload RAW payloads. Corpus and golden files stay pointer-only.
- Preserve all existing DNG, ARW, NEF, codec, and golden behavior.
- Branch: `task/dec-raf-fujifilm-raf`.
- Worktree: `.worktrees/dec-raf-fujifilm-raf`.

## Scope

In scope:

- RAF header, embedded TIFF, Fuji IFD, proprietary tag block, and raw payload
  parsing with strict bounds.
- Uncompressed 12-bit little-endian packed and 14/16-bit word samples.
- Fujifilm lossless block decoding for 2x2 Bayer and 6x6 X-Trans CFA layouts.
- Camera profiles for supported GFX 50/100 Bayer models.
- DNG metadata mapping and X-Trans DNG round-trip tests.
- Facade dispatch, crate documentation, reference notes, full RAF corpus
  tracking, passing Bayer golden rows, and removal of `DEC-RAF` from `TODO.md`.

Out of scope:

- Lossy Fujifilm block compression.
- SuperCCD 45-degree rotation, staggered/non-rectangular layouts, and DBP.
- X-Trans preprocess, demosaic, or color-transform pipeline support.
- Embedded JPEG extraction or raw fallback through the JPEG preview.

## Task Checklist

- [x] Add the codec API and malformed-input tests; implement RAF packed and
  Fujifilm-lossless sample decoding for 2x2 and 6x6 CFA layouts.
- [x] Add `fornax-raf` container, tag, camera-profile, metadata, and decoder
  tests; implement parsing, metadata mapping, and typed unsupported paths.
- [x] Add facade dispatch, DNG round-trip coverage, fixture parity, and docs.
- [ ] Add RAF golden rows, run the full RAF corpus workflow, review the
  baseline, regenerate the tracker README, and rerun until unchanged.
- [ ] Run workspace validation, review the complete diff, update `TODO.md`,
  commit, push, and open one semantic pull request.

## Files Expected to Change

- `crates/fornax-codec/{Cargo.toml,README.md,src/lib.rs,src/request.rs}`
- `crates/fornax-codec/src/raf.rs`
- `crates/fornax-codec/tests/raf.rs`
- `crates/fornax-raf/**`
- `crates/fornax-decoder/{Cargo.toml,README.md,src/lib.rs,tests/raf*.rs}`
- `crates/decoder-tracker/{golden/**,baseline.csv,README.md}`
- `docs/{architecture.md,decoder/architecture.md,decoder/formats/fuji-raf.md}`
- `docs/references/fuji-raf.md`
- `docs/plans/dec-raf-fujifilm-raf.md`
- `TODO.md`

## Validation

```bash
cargo test -p fornax-codec --features raf
cargo test -p fornax-codec -p fornax-raf -p fornax-decoder
cargo test -p decoder-tracker
pixi run -e tracker cargo test -q -p decoder-tracker duckdb_
pixi run -e dev cargo run -q -p decoder-tracker -- manifest --check
pixi run -e dev cargo run --release -q -p decoder-tracker -- hash --check \
  crates/decoder-tracker/golden/decode-hashes.csv
scripts/agent-validation.sh
git diff --check
```

Fixture parity uses `FORNAX_RAW_SAMPLES` and skips when unset or when files are
unmaterialized LFS pointers. Golden hashes are generated and verified in release
mode. The manual corpus filter is `**/*.[rR][aA][fF]` with
`run_duckdb_tests=false`.

## Decisions and Notes

- The owner approved lossless-only compression scope and selected X-Trans
  decode plus DNG export as the boundary, with no X-Trans pipeline expansion.
- The full RAF corpus is recorded so supported Bayer rows and expected X-Trans,
  lossy, and legacy failures define the support boundary.
- Bayer support is gated by explicit profiles for `GFX 50S`, `GFX 50R`,
  `GFX50S II`, `GFX 100`, `GFX100S`, `GFX100 II`, `GFX100S II`, and
  `GFX100RF`. Their profile borders are respectively
  `[0,0,936,2]`, `[0,0,936,2]`, `[0,0,192,2]`, `[0,4,144,2]`,
  `[0,4,144,2]`, `[0,2,144,4]`, `[0,0,144,4]`, and `[0,0,144,4]`.
- Rawler `0.8.0` (LGPL-2.1) and LibRaw commit
  `cf02d66eb5c19d4e46df7aee0582e47a4918509a` are behavioral references only.
  The implementation is independently written from format behavior.
- Color matrices are not synthesized when the RAF metadata does not provide
  them, matching the existing native decoder policy.
- Materialized fixture tests match Rawler exactly for GFX 50S lossless,
  GFX100S 14/16-bit uncompressed, X-H2 14-bit lossless, and X-H2 uncompressed.
  Lossy X-H2 compression returns typed unsupported.
- X-Trans DNG round-trip tests preserve storage-native samples and the 6x6,
  36-plane CFA, while process metadata and cropped handoff reject the layout.
- Local validation passed `scripts/agent-validation.sh` after adding the RAF
  golden rows and generated hashes. The full RAF corpus remains a manual CI
  stage because that is the repository's designated full-corpus environment.
