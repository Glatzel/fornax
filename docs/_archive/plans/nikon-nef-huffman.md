# Nikon NEF Huffman Lossless Implementation Plan

This archived plan documents the completed Nikon NEF Huffman Lossless work.

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> `superpowers:subagent-driven-development` or
> `superpowers:executing-plans` to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add native Fornax decoding for Nikon NEF mode 3 (lossless
compressed Huffman difference coding, TIFF compression 34713) at 12 and 14
bits for standard 2x2 Bayer layouts.

**Architecture:** Add reusable canonical Huffman primitives to
`fornax-codec`, then add a Nikon-specific difference decoder behind the
existing `nef` feature. Extend `fornax-nef` to parse the Nikon metadata and
linearization curve, select the lossless tree by bit depth, construct the
codec parameters, and dispatch mode 3.

**Tech Stack:** Rust 2024, `fornax-codec`, `fornax-nef`, `fornax-decoder`,
`fornax-tiff`, `fornax-raw-core`, and existing test tooling.

**Spec:** `docs/decoder/formats/nikon-nef.md`

## Global Constraints

- Accept only Nikon mode 3 for Huffman decoding. Modes 1 and 4 are lossy and
  return `DecodeError::Unsupported` with a reason that identifies the policy
  deferral. Modes 8, 13, 14, unknown modes, multi-sample, tiled, and
  reduced-linear images remain unsupported.
- Do not add Rawler, LibRaw, dcraw, or other copyleft implementation code to
  production dependencies. Do not copy implementation code, comments, or
  naming.
- Numeric Huffman counts and symbols for the 12-bit and 14-bit lossless trees
  are transcribed as format data only. The owner approved this approach on
  2026-11-11. Source revisions, licenses, source agreement, and behavioral
  verification are recorded in `docs/reference/decoders/nikon-nef.md`. Lossy and split
  trees are explicitly out of scope.
- Production codec variants remain decode-only. Test-only encoders may live
  inside test modules.
- Malformed input must never panic or over-allocate. Validate dimensions,
  strip counts, metadata lengths, curve indices, and checked sample counts
  before allocating output. Reject invalid or over-subscribed codes and
  exhausted streams with typed errors.
- Decoding is serial because predictors depend on earlier rows. Do not add
  Rayon to the codec. Use table decoding and a 64-bit bit buffer.
- Preserve DNG, ARW, packed-NEF, and existing golden behavior.
- Keep `DEC-NEF` open. Mark only lossless Huffman at 12 and 14 bits complete;
  record lossy Huffman as deferred by policy.
- Never commit or upload RAW payloads. Corpus and golden files remain
  pointer-only.
- Branch: `task/dec-nef-huffman`.
- Worktree: `.worktrees/dec-nef-huffman`.

## Scope

In scope:

- Canonical Huffman construction from 16 code-length counts plus symbols.
- MSB-first bit reading and fast table decoding with a long-code fallback.
- Nikon 12-bit and 14-bit lossless Huffman difference decoding.
- Nikon metadata/curve parsing, mode-3 dispatch, synthetic coverage, fixture
  parity where corpus samples exist, golden selection, and corpus tracking.
- Documentation of table provenance, licensing, behavior, supported surface,
  and unsupported reasons.

Out of scope:

- Nikon lossy modes 1 and 4, HE/HE* modes 13 and 14, small raw mode 8,
  reduced-linear output, Coolpix-specific layouts, and multi-sample data.
- Lossy/split Huffman tables or split-point decoding.
- Camera-specific geometry overrides and headerless maker notes.

## Task Checklist

- [x] Task 0: Pin sources, confirm algorithm behavior, survey the corpus, and
  record the owner-approved numeric-tree provenance.
- [x] Task 1: Add canonical Huffman construction, bit reading, fast decoding,
  completeness validation, and randomized equivalence tests.
- [x] Task 2: Add Nikon `NikonTreeId`, `NikonHuffman`, difference decoding,
  test-only encoding, prediction tests, curve tests, and malformed-input tests.
- [x] Task 3: Parse the Nikon curve metadata, add mode-3 dispatch, preserve
  typed unsupported reasons, and apply the confirmed WhiteLevel behavior.
- [x] Task 4: Add fixture parity, golden rows and hashes, run workspace
  validation, open the pull request, and review the NEF tracker corpus rows.
- [x] Task 5: Update crate READMEs, format status, reference notes, and
  `TODO.md`; report performance measurements only if fixtures are available.

## Files Expected to Change

- `crates/fornax-codec/{Cargo.toml,README.md,src/lib.rs,src/request.rs}`
- `crates/fornax-codec/src/huffman.rs`
- `crates/fornax-codec/src/nef.rs`
- `crates/fornax-codec/tests/nef.rs`
- `crates/fornax-nef/{README.md,src/container.rs,src/decode.rs,src/lib.rs}`
- `crates/fornax-nef/src/curve.rs`
- `crates/fornax-nef/src/tests.rs`
- `crates/fornax-nef/tests/**`
- `crates/fornax-decoder/tests/**` for fixture parity where needed
- `crates/decoder-tracker/{golden/**,baseline.csv,README.md}`
- `docs/decoder/formats/nikon-nef.md`
- `docs/reference/decoders/nikon-nef.md`
- `TODO.md`

## Validation

```bash
cargo test -p fornax-codec
cargo test -p fornax-codec --features nef
cargo test -p fornax-codec -p fornax-nef -p fornax-decoder
cargo run --release -q -p decoder-tracker -- hash --check \
  crates/decoder-tracker/golden/decode-hashes.csv
scripts/agent-validation.sh
git diff --check
```

Fixture parity uses `FORNAX_RAW_SAMPLES` and must skip cleanly when unset.
Golden corpus hash generation and verification must use the release profile.
The decoder tracker corpus filter is `**/*.[nN][eE][fF]` with
`run_duckdb_tests=false`. Only pointer metadata and generated text artifacts
may be committed.

## Decisions and Notes

- 2026-11-11 owner approval: transcribe only the 12-bit lossless tree at
  index 2 and 14-bit lossless tree at index 5 as numeric format data into an
  independently written Rust representation. Do not transcribe lossy or split
  trees and do not copy source code, comments, or naming.
- Pinned Rawler revision `62b77de1a206d467e910d55199191068b45a9cc1` prefers
  maker tag `0x0096` and falls back to `0x008c`. Its mode-3 path uses tree
  index 2 for 12 bits and index 5 for 14 bits and does not activate a split
  tree.
- Pinned LibRaw revision `cf02d66eb5c19d4e46df7aee0582e47a4918509a` reads the
  same metadata header, seeds four predictor values, applies the same
  difference sign extension, and uses the same decoder for modes 1, 3, and 4.
  LibRaw activates the after-split tree for lossy modes when `0x44` metadata
  exposes a split row.
- The two implementations agree on the lossless counts and symbols. Each row
  forms a complete canonical prefix code.
- The curve table transforms decoded predictor values. The effective WhiteLevel
  is determined during the corpus/source verification and recorded in
  `docs/reference/decoders/nikon-nef.md`.
- Mode 3 accepts exactly one strip until corpus evidence proves that Nikon
  lossless strips have independent state. Multi-strip mode 3 remains
  unsupported otherwise.
- A single mode-3 strip may declare `RowsPerStrip` above the image height.
  The D7100 corpus files require accepting 4022 declared rows for 4020 output
  rows.
- Rawler's identity curve applies a lossy dithering step, so real-file parity
  is exact for Fornax predictors and accepts Rawler's value or one code below
  it. Fornax does not adopt that reference-only dithering.
- Golden corpus hashing uses the release profile. The manual corpus workflow
  builds one release tracker binary and every shard downloads that artifact.
- Corpus run `43008760213` validated the release build and artifact-consuming
  shards, then exposed two D7100 `RowsPerStrip` edge cases. Run `43009807548`
  passed all 88 changed mode-3 rows after the fix. Applying the complete
  reviewed baseline raised the tracker total to 514/2020. Run `43010446775`
  completed all shards and aggregation with an unchanged baseline.

## Implementation Tasks

### Task 0: Verification gates

- [x] Pin Rawler/LibRaw revisions in `docs/reference/decoders/nikon-nef.md` and
  confirm the curve tag, metadata header, lossless trees, difference rule,
  predictor scheme, bit-depth rule, curve behavior, and effective WhiteLevel.
- [x] Confirm lossy modes 1 and 4 share entropy decoding but use a split row
  and after-split trees. Record the behavior without implementing it.
- [x] Confirm whether corpus mode-3 images have multiple strips with
  independent state. If not, multi-strip mode 3 is unsupported.
- [x] Survey the corpus with the decoder tracker filtered to
  `**/*.[nN][eE][fF]`; list mode-3 files by camera, bit depth, and metadata
  version. Synthetic-only bit depths or cameras must be identified.
- [x] Record the owner decision approving transcription of only the two
  lossless trees as numeric format data.

Expected: reference notes contain pinned revisions, source licenses, the
two-source agreement, the owner approval, the behavioral conclusions, and a
corpus table. No production code changes in this task.

### Task 1: Generic Huffman primitives

- [x] Add `fornax-codec/src/huffman.rs` with
  `HuffmanTable::from_counts(counts: &[u8; 16], symbols: &[u8]) ->
  Result<Self, DecodeError>`, a `BitReader` over `&[u8]`, and
  `decode_symbol` with a lookup fast path and slow fallback for long codes.
- [x] First write failing tests for canonical code assignment, rejection of
  over-subscribed and incomplete tables, rejection of symbol lengths above
  16, MSB-first reads across byte boundaries, end-of-data handling, and
  equivalence with a naive bit-by-bit decoder on randomized streams.
- [x] Include a canonical-prefix-code completeness test that proves each
  supported lossless tree is complete.
- [x] Run `cargo test -p fornax-codec` and commit
  `feat: add generic Huffman decoding primitives`.

Expected: the new tests fail because the module/API is absent, then pass; the
default feature build remains green.

### Task 2: Nikon difference decoder

- [x] Add `NikonTreeId` and
  `DecodeCompression::NikonHuffman { bits, tree, init, curve }` behind the
  `nef` feature. Do not add a split field.
- [x] Add only the 12-bit lossless (index 2) and 14-bit lossless (index 5)
  numeric count and symbol arrays with provenance comments and
  `docs/reference/decoders/nikon-nef.md` links. Do not transcribe lossy or split
  arrays.
- [x] First write failing tests for the 2x2 initial predictors, vertical
  propagation, horizontal prediction, odd widths, heights one and two,
  identity and non-trivial curves, curve index overflow, invalid codes,
  truncated streams, trailing bytes, zero dimensions, and allocation after
  validation.
- [x] Add a test-only encoder for 12/14-bit randomized round trips.
- [x] Run `cargo test -p fornax-codec --features nef` and default-feature
  tests; commit `feat: add Nikon lossless Huffman codec`.

Expected: decoding writes directly into validated `u16` output, uses no
per-pixel allocation, and rejects malformed streams with `CodecError`.

### Task 3: NEF integration

- [x] Add `fornax-nef/src/curve.rs` to read maker tags `0x0096` then
  `0x008c`, validate endian/version/header lengths, extract the four initial
  predictors and curve, and scale the curve by bit depth as established in
  Task 0.
- [x] First write failing synthetic NEF tests for mode 3 at 12/14 bits,
  missing/truncated curve metadata, inconsistent curve length, TIFF/maker
  mode conflicts, modes 1/4 policy errors, unchanged modes 8/13/14, and the
  multi-strip ruling.
- [x] Dispatch mode 3 through `LayoutKind`, `DecodeCompression`, and
  `fornax-nef::decode`; preserve existing packed modes.
- [x] Apply the confirmed WhiteLevel behavior and leave black-level scaling
  unchanged.
- [x] Run `cargo test -p fornax-codec -p fornax-nef -p fornax-decoder` and
  commit `feat: decode Nikon lossless NEF`.

Expected: synthetic images decode to hand-derived mosaics; unsupported
reasons distinguish lossy policy, HE/HE*, unknown variants, and malformed
mode-3 metadata.

### Task 4: Fixtures, golden, and corpus

- [x] Add skip-when-unset fixture tests for surveyed mode-3 files, including
  at least one accepted sample per available bit depth and a Z-series sample
  if one exists.
- [x] Compare raw samples, dimensions, and metadata handoff against Rawler
  using decoder test tooling, not a `fornax-nef` dependency.
- [x] Add passing selected-manifest rows and regenerate decode hashes without
  changing existing DNG, ARW, or packed-NEF hashes.
- [x] Run `scripts/agent-validation.sh`, inspect the diff, commit, push the
  branch, and open a semantic pull request with scope, survey table, and
  validation outcomes.
- [x] Dispatch `decoder-tracker-corpus.yml` with filter
  `**/*.[nN][eE][fF]` and `run_duckdb_tests=false`; review every changed row.
  Require at least one pass per sampled bit depth.
- [x] Run the corpus workflow with one release-mode build job and shards that
  download and execute the uploaded tracker binary.
- [x] Apply only the complete reviewed baseline artifact, regenerate the
  tracker README, commit and push, then rerun until unchanged.
- [x] Run final `scripts/agent-validation.sh` and `git diff --check`; report
  skipped or failed checks explicitly.

Expected: fixtures and corpus prove real-file parity where samples exist;
lossy, HE/HE*, unsupported, and malformed rows retain typed reasons.

### Task 5: Documentation and tracking

- [x] Update `fornax-codec` and `fornax-nef` READMEs,
  `docs/decoder/formats/nikon-nef.md`, and
  `docs/reference/decoders/nikon-nef.md` with the supported surface, dispatch table,
  tree provenance, curve layout, WhiteLevel behavior, and unsupported reasons.
- [x] Update `TODO.md`: mark lossless Huffman complete, keep `DEC-NEF` open,
  and record lossy modes as deferred by policy.
- [x] Optional benchmark deferred; correctness and corpus coverage are the
  merge gates for this slice.

Expected: public documentation matches the tested implementation and no
unsupported mode is described as implemented.
