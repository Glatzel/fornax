# Nikon NEF Uncompressed and Packed Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development or superpowers:executing-plans to
> implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for
> tracking.

**Goal:** Add native Fornax decoding for standard Nikon NEF compression modes
2, 5, 6, 7, 9, and 10, including Nikon white-balance mapping for WB0 and WB1
versions `0x100`, `0x102`, `0x103`, `0x204`, `0x205`, and `0x206`.

**Architecture:** Add a `fornax-nef` vendor crate for TIFF and maker-note
parsing, sample-layout selection, NEF-to-DNG metadata mapping, and Bayer
handoff. Add Nikon bit unpackers to `fornax-codec` behind a `nef` feature,
then route `.NEF` through the default `fornax-decoder` facade.

**Tech Stack:** Rust 2024, `fornax-tiff`, `fornax-codec`,
`fornax-raw-core`, `fornax-dng`, `image`, `rayon`, and `generic-num`.

**Spec:** `docs/decoder/formats/nikon-nef.md`

## Global Constraints

- Support conventional 2x2 RGB Bayer data only. Reject non-Bayer,
  multi-sample, tiled, and reduced-linear layouts with
  `DecodeError::Unsupported`.
- Accepted modes are exactly `2`, `5`, `6`, `7`, `9`, and `10`. Reject modes
  `1`, `3`, `4`, `8`, `13`, `14`, unknown modes, and unsupported byte layouts.
- Standard Nikon layouts only. Coolpix interlacing, per-model geometry
  overrides, and non-type-2 maker notes remain unsupported.
- Do not add Rawler or LibRaw to production dependencies or copy their
  implementation source. Rawler parity coverage belongs in
  `fornax-decoder` tests, which already have a Rawler dev-dependency.
- Keep `DEC-NEF` in `TODO.md`. Add separate TODO items for deferred Huffman
  lossless/lossy, HE/HE*, small raw mode 8, and Coolpix-specific layouts.
- Branch: `task/dec-nef-packed`. Worktree: `.worktrees/dec-nef-packed`.
- Never commit or upload RAW payloads. Corpus and golden files remain
  pointer-only.
- Preserve existing DNG and ARW behavior and golden hashes.
- Validate dimensions and payload sizes with checked arithmetic before
  allocating output. Bounds-check every maker-note entry and value offset.

## Scope

In scope:

- Type-2 Nikon maker notes, including makers' embedded byte order.
- Standard uncompressed and packed CFA layouts for modes `2`, `5`, `6`, `7`,
  `9`, and `10`.
- CFA, active area, default crop, black/white levels, camera identity,
  orientation, serial, and Nikon WB metadata.
- Typed provenance for white balance and black-level sources in
  `ProcessMetadata`.
- Facade routing, synthetic tests, selected fixture parity, golden hashes, and
  manual corpus tracking.

Out of scope:

- Huffman lossless/lossy NEF, HE/HE*, small raw mode 8, reduced linear raw,
  Coolpix interlacing/MSB32 layouts, per-model width overrides, and headerless
  maker notes.
- Tracker baseline schema changes for provenance diagnostics.
- Lens resolution and camera color matrices not present in the source file.

## Task Checklist

- [x] Confirm source semantics and survey real corpus modes and WB versions.
- [x] Add the `fornax-nef` crate, TIFF constants, maker-note parser, and
  synthetic NEF fixture builder.
- [x] Add Nikon packed, unpacked-word, and striped mode-5 codecs.
- [x] Add raw-IFD selection, layout dispatch, `NefImage`, and the `IDecoder`
  raw path.
- [x] Add DNG metadata mapping, WB decryption, and typed provenance.
- [x] Route `.NEF` through `fornax-decoder` and document the accepted surface.
- [x] Add fixture parity, golden hashes, manual corpus validation, and review.
- [x] Update this plan with decisions, validation results, and any deviations.

## Files Expected to Change

- `Cargo.toml`, `Cargo.lock`, `TODO.md`
- `crates/fornax-codec/{Cargo.toml,README.md,src/*}`
- `crates/fornax-tiff/src/constants.rs`
- `crates/fornax-nef/**`
- `crates/fornax-decoder/{Cargo.toml,README.md,src/lib.rs,tests/**}`
- `crates/fornax-raw-core/src/metadata.rs`
- DNG, ARW, pipeline, tracker, and postprocessor constructors/tests affected by
  the new `ProcessMetadata` provenance fields
- `crates/decoder-tracker/{golden/**,baseline.csv,README.md}`
- `docs/architecture.md`, `docs/decoder/architecture.md`
- `docs/decoder/formats/nikon-nef.md`, `docs/references/nikon-nef.md`

## Validation

```bash
cargo test -p fornax-tiff -p fornax-codec -p fornax-nef -p fornax-decoder
cargo test --workspace --exclude compare-demosaic
scripts/agent-validation.sh
git diff --check
```

Fixture-backed parity is driven by `FORNAX_RAW_SAMPLES`, pointing at the
raw.pixls.us corpus root. The manual tracker workflow uses
`**/*.[nN][eE][fF]` and leaves `run_duckdb_tests=false`.

## Decisions and Notes

- Mode 5 uses 32-bit little-endian word reads whose extracted sample bits are
  MSB-first. The plan's width-8 vector
  `21 43 65 87 ba dc fe 10 32 54 76 98` therefore decodes to
  `0876 0543 0211 00fe 0dcb 0a98 0765 0432`.
- The 15-byte row-padding bound is Rawler's conservative inference limit for
  uncompressed data, not a Nikon mode-9 specification. Mode-specific dispatch
  derives row strides from validated strip byte counts; mode 6 requires zero
  padding.
- The corpus survey found real files for modes 2, 6, and 10. Modes 5, 7, and 9
  are synthetic-only for this snapshot. All requested WB1 versions have real
  files. Encrypted WB uses serial and Nikon key tag `0xa7`; shutter count is
  not a key input.
- Provenance is exposed as typed `ProcessMetadata` API only. It is excluded
  from golden process hashes and does not change the tracker CSV schema.
- The Task 0 corpus survey runs through a test-only survey helper after the
  maker-note parser exists. It writes a summary to the SDD workspace and
  commits only aggregate text to `docs/references/nikon-nef.md`.
- Rawler parity tests live in `crates/fornax-decoder/tests`, using the
  existing dev-dependency and the public `Decoder` facade.
- Validation completed with `scripts/agent-validation.sh`. The selected D850
  12-bit, D850 14-bit, and D500 12-bit fixtures match Rawler sample-for-sample.
- The first manual NEF corpus run evaluated 277 rows after all 20 shards
  succeeded: 30 rows pass and 247 return typed failures. Modes 6 and 10 and the
  legacy mode-2 D100 row have passing corpus coverage; modes 5, 7, and 9 remain
  synthetic-only in this snapshot.
