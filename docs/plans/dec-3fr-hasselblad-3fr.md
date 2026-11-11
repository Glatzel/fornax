# Hasselblad 3FR Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Implement pure-Rust decoding and DNG export for the accepted conventional-Bayer Hasselblad `3FR` corpus.

**Architecture:** Add a feature-gated predictor-8 lossless-JPEG codec to `fornax-codec`, a `fornax-hasselblad` container/metadata crate, and `.3FR` dispatch in the default decoder facade. The decoder returns storage-native 16-bit mosaics, maps source and camera-profile metadata into `fornax_tiff::Directory`, and uses the existing DNG writer.

**Tech Stack:** Rust 2024, existing Fornax TIFF/codec/DNG/raw-core crates, Rawler as a development-only fixture oracle, decoder-tracker CSV baseline.

**Spec:** `docs/decoder/formats/hasselblad-3fr.md`

## Global Constraints

- Work only in `.worktrees/dec-3fr-hasselblad-3fr` on `task/dec-3fr-hasselblad-3fr`.
- Keep decoding, metadata mapping, DNG writing, and preprocessing boundaries unchanged.
- The preprocessing handoff accepts conventional 2x2 RGB Bayer only.
- Support `.3FR` only; leave `.FFF`, merged/multi-shot files, and unknown variants typed unsupported.
- Keep Rawler and LibRaw development-only; do not add either to production dependencies.
- Never commit or upload RAW payloads.
- Do not hand-edit `manifest.csv`, `baseline.csv`, or generated README marker regions.
- Remove `DEC-3FR` from `TODO.md` only after implementation, tests, and corpus validation complete.
- Open one pull request and report its URL; do not watch, merge, or close it.

## Review Focus

- Compression-7 `StripByteCounts` can exceed the file size; decoding must use the payload through EOF and still reject an out-of-range offset.
- Predictor 8 reads two Huffman lengths before either attached difference and resets both predictors at every row; standard LJPEG ordering must not be reused accidentally.
- X1D and X1D II active borders remove pixels, so `DefaultCropOrigin` must be rebased relative to `ActiveArea`.
- Unknown tagless models and non-2x2 profiles must return typed unsupported errors rather than being guessed into a Bayer handoff.
- Compressed and uncompressed paths must both round-trip their exact storage samples through the generic DNG writer.

---

## Task 1: Predictor-8 lossless JPEG codec

**Files:**

- Modify: `crates/fornax-codec/Cargo.toml`
- Modify: `crates/fornax-codec/src/request.rs`
- Modify: `crates/fornax-codec/src/lib.rs`
- Modify: `crates/fornax-codec/src/ljpeg.rs`
- Test: `crates/fornax-codec/tests/decode_contract.rs`

**Interfaces:**

- Produce: `DecodeCompression::HasselbladLosslessJpeg` behind feature `hasselblad`.
- Require: `fornax-codec/hasselblad` implies `lossless-jpeg`.
- Preserve: `DecodeRequest` and `SampleBuffer` signatures.

- [x] Write a failing contract test that constructs a one-component 16-bit SOF3 stream with predictor 8 and asserts the exact two-column output.
- [x] Run `cargo test -p fornax-codec --features hasselblad --test decode_contract hasselblad`; expect failure because the feature/variant is absent.
- [x] Add the feature, enum variant, dispatch arm, header validation, and predictor-8 decoder.
- [x] Add failing malformed-stream tests for nonzero restart interval, point transform, unsupported predictor, component mismatch, missing Huffman table, and truncated entropy.
- [x] Run the codec feature test suite until all cases pass.

## Task 2: 3FR container, profiles, and metadata

**Files:**

- Create: `crates/fornax-hasselblad/Cargo.toml`
- Create: `crates/fornax-hasselblad/README.md`
- Create: `crates/fornax-hasselblad/src/lib.rs`
- Create: `crates/fornax-hasselblad/src/container.rs`
- Create: `crates/fornax-hasselblad/src/metadata.rs`
- Create: `crates/fornax-hasselblad/src/profiles.rs`
- Create: `crates/fornax-hasselblad/src/tests.rs`
- Modify: `Cargo.toml`

**Interfaces:**

- Produce: `fornax_hasselblad::HasselbladImage`.
- Consume: `fornax_tiff::parse`, `fornax_codec::decode`, `fornax_dng::dng_decoder::process_metadata_from_dng_metadata`, and `fornax_dng::metadata::IDecoderDng`.

- [x] Write failing synthetic TIFF tests for preview/raw SubIFD selection, uncompressed byte-count validation, compression-7 EOF handling, profile CFA/active-area selection, and unknown-model rejection.
- [x] Run `cargo test -p fornax-hasselblad`; expect the crate/tests to fail because the decoder is absent.
- [x] Implement container selection and validate checked sample-count and source-offset arithmetic, single sample, 16-bit integer storage, strip/tile absence, and source offsets.
- [x] Implement explicit RGGB profiles:
  - full active area: `Hasselblad H3D`, `Hasselblad H4D`, `Hasselblad CFV`, `Hasselblad96`, `Hasselblad ixpressCF132`, `Hasselblad X2D 100C`, `Hasselblad CFV 100C/Electronic Shutter`;
  - borders `[left 46, top 96, right 58, bottom 0]`: `Hasselblad X1D`, `Hasselblad X1D II 50C`.
- [x] Implement source CFA parsing; when tags are absent use the profile pattern. Keep arbitrary well-formed CFA metadata available to raw/DNG access while rejecting non-2x2 layouts at pipeline access.
- [x] Map geometry, active/default crop, levels, WB, matrix, identity, orientation, raw unique ID, and allowlisted EXIF into `Directory`.
- [x] Add failing metadata tests and make them pass, including X1D crop rebasing and unknown/non-Bayer unsupported behavior.

## Task 3: Default decoder and exact fixture parity

**Files:**

- Modify: `crates/fornax-decoder/Cargo.toml`
- Modify: `crates/fornax-decoder/src/lib.rs`
- Create: `crates/fornax-decoder/tests/hasselblad.rs`
- Modify: `crates/fornax-hasselblad/src/tests.rs`
- Modify: `crates/fornax-decoder/README.md`

**Interfaces:**

- Produce: `Decoder::Hasselblad(HasselbladImage)`.
- Preserve: every `IDecoder` and `IDecoderDng` accessor.

- [x] Write a failing extension-dispatch test proving `.3FR` routes to the native reader.
- [x] Run the focused decoder test; expect the current unsupported/DNC path rather than a native read error.
- [x] Add the facade dependency, enum variant, dispatch arm, and accessor forwarding.
- [x] Write fixture tests that compare all 11 accepted samples against Rawler raw samples and dimensions, then assert `f32` handoff and metadata dimensions.
- [x] Write DNG round-trip tests for one compressed and one uncompressed sample; assert the storage samples survive exactly.
- [x] Run `FORNAX_RAW_SAMPLES=external/raw.pixls.us cargo test --release -p fornax-hasselblad` and the decoder fixture test.

## Task 4: Documentation, validation, and corpus baseline

**Files:**

- Modify: `docs/decoder/formats/hasselblad-3fr.md`
- Modify: `docs/decoder/architecture.md`
- Modify: `docs/architecture.md`
- Create: `docs/references/hasselblad-3fr.md`
- Modify: `TODO.md`
- Generated only through required workflow: `crates/decoder-tracker/baseline.csv`, `crates/decoder-tracker/README.md`

- [x] Document the observed TIFF/SubIFD shape, compression paths, predictor-8 ordering, profiles, metadata mapping, exclusions, pinned source links, and licensing constraints.
- [x] Run `scripts/agent-validation.sh`.
- [x] Run `cargo test -p decoder-tracker`.
- [x] Run the local tracker selection for all 11 accepted `3FR` rows and require every stage to pass.
- [x] Commit and push the branch, then run the manual decoder corpus workflow with `filter='Hasselblad/**/*.3FR'` and `run_duckdb_tests=false`.
- [x] Review the complete update artifact, copy only the complete baseline, and regenerate the README.
- [ ] Commit and push the reviewed baseline, then rerun to an unchanged result.
- [ ] Remove `DEC-3FR` from `TODO.md`, inspect the final diff, create the pull request, and report its URL.

## Files Expected to Change

- Codec: `crates/fornax-codec/`
- Format and facade: `crates/fornax-hasselblad/`, `crates/fornax-decoder/`
- Workspace manifests and lockfile as required by Cargo
- Decoder architecture, reference, and plan documents
- `TODO.md`
- Reviewed decoder-tracker baseline and generated README region

## Validation

```bash
cargo test -p fornax-codec --features hasselblad
cargo test -p fornax-hasselblad
FORNAX_RAW_SAMPLES=external/raw.pixls.us cargo test --release -p fornax-hasselblad
cargo test -p fornax-decoder
cargo test -p decoder-tracker
pixi run -e tracker cargo test -q -p decoder-tracker duckdb_
scripts/agent-validation.sh
cargo bench -p fornax-decoder --no-run
git diff --check
```

## Decisions and notes

- Six accepted samples use compression 1 with exact 16-bit little-endian payloads; five use compression 7 with SOF3 16-bit data, one component, predictor 8.
- Compression-7 strip byte counts are advisory for this corpus because several exceed the file size; the offset must be bounded and entropy decoding runs through EOF.
- Camera profiles supply RGGB and active borders because accepted raw IFDs do not carry CFA tags.
- Embedded `ColorMatrix1`, `AsShotNeutral`, levels, crop geometry, orientation, and safe EXIF take precedence over profile defaults.
- Non-Bayer CFA data remains exportable but never enters `process_metadata` or `cropped_mosaic`.
