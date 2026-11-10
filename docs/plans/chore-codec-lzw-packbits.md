# TIFF LZW, PackBits, and F64 Codec Support

## Goal

Add feature-gated, format-neutral TIFF LZW and PackBits decode/encode support
to `fornax-codec`, then forward those features through `fornax-tiff` so
compression IDs 5 and 32773 resolve only when their codec is enabled. Add F64
to `SampleBuffer` and the codec paths. Preserve the B2 typed resolver contract,
existing predictors, chunk behavior, DNG defaults, and the legacy Deflate
identifier.

## Scope

### In

- Add `lzw` and `packbits` features to `fornax-codec`, enabled by default as
  the standard TIFF compression pair.
- Implement TIFF-compatible MSB-first LZW with TIFF code-width transitions.
- Implement the TIFF PackBits literal, repeat, and no-op control-byte rules.
- Add F64 to `SampleBuffer`, raw assembly, supported encoders, and
  floating-point prediction while keeping `RawMosaic`/DNG unchanged.
- Add chunk-local predictors, row-major chunk ordering, partial-edge coverage,
  and chunk-indexed malformed-input errors.
- Add `lzw` and `packbits` features to `fornax-tiff` and gate resolver arms.
- Add fixed interoperability vectors, round trips, feature matrices, and
  dependency-surface checks.
- Record implementation and validation evidence in this plan.

### Out

- Enabling either codec in `fornax-dng` or changing DNG/ARW behavior.
- LZMA, JPEG, WebP, vendor codecs, or additional TIFF identifiers.
- TIFF/raw workflow types in `fornax-codec`.
- Version, crate-name, workspace, golden-file, or Python API changes.
- Local `compare-demosaic` compilation, lefthook, or unrequested benchmarks.

## Task Checklist

- [x] Verify the locked `weezl` API, license, maintenance metadata, and
  dependency surface.
- [x] Add failing LZW tests for the fixed TIFF vector, round trip, predictor
  interaction, partial chunks, and chunk-indexed malformed input.
- [x] Implement and pass feature-gated LZW decode/encode.
- [x] Add failing PackBits tests for repeat, literal-only, no-op, round trip,
  partial chunks, and chunk-indexed malformed input.
- [x] Implement and pass feature-gated PackBits decode/encode.
- [x] Add and pass F64 construction, extraction, raw round-trip, and predictor
  tests; verify DNG rejects F64 at its `RawMosaic` boundary.
- [x] Gate the new compression variants and dispatch arms.
- [x] Run the codec feature matrix and fix any pre-existing test-feature gating
  needed for the required commands to be meaningful.
- [x] Add failing TIFF resolver tests for enabled and disabled IDs.
- [x] Add TIFF feature forwards and resolver arms.
- [x] Run the TIFF feature matrix.
- [x] Run focused DNG/ARW tests, workspace tests excluding `compare-demosaic`,
  formatting, docs, and CI Clippy flags.
- [x] Run dependency trees and inspect the final diff; local golden hashing was
  attempted and stopped at missing `git-lfs`, so CI remains authoritative.
- [ ] Open one
  semantic pull request.

## Files Expected To Change

- `Cargo.toml`
- `Cargo.lock` only if Cargo requires a lock update
- `crates/fornax-codec/Cargo.toml`
- `crates/fornax-codec/README.md`
- `crates/fornax-codec/src/{lib.rs,request.rs,lzw.rs,packbits.rs}`
- `crates/fornax-codec/src/{sample_buffer.rs,samples.rs,encode.rs,uncompressed.rs,predictor.rs}`
- `crates/fornax-codec/tests/{decode_contract.rs,encode_contract.rs}` for
  feature-gating required by the mandated matrix
- `crates/fornax-tiff/Cargo.toml`
- `crates/fornax-tiff/README.md`
- `crates/fornax-tiff/src/raster.rs`
- `crates/fornax-tiff/tests/raster.rs`
- `crates/fornax-dng/src/sample_buffer.rs` for the F64 boundary
- `docs/plans/chore-codec-lzw-packbits.md`
- `docs/references/compression-ids.md` if availability wording must change

## Validation

- Every new behavior starts with a failing test whose failure is inspected.
- Codec:
  - `cargo test -p fornax-codec`
  - `cargo test -p fornax-codec --no-default-features`
  - `cargo test -p fornax-codec --features lzw`
  - `cargo test -p fornax-codec --features packbits`
  - `cargo test -p fornax-codec --all-features`
- TIFF:
  - `cargo test -p fornax-tiff`
  - `cargo test -p fornax-tiff --no-default-features`
  - `cargo test -p fornax-tiff --no-default-features --features raster`
  - `cargo test -p fornax-tiff --all-features`
- Focused DNG/ARW tests and workspace tests excluding `compare-demosaic`.
- Nightly formatting, documentation, and CI-equivalent Clippy flags.
- Dependency trees for default and enabled `fornax-codec` / `fornax-tiff`.
- The decoder-tracker golden hash check when local corpus material is
  available; otherwise report CI as the authoritative golden check.

## Decisions And Notes

- Fixed interoperability vector:
  - LZW bytes: `80 0b 60 50 22 0c 0c 85 01`
  - Decoded bytes: `-----A---B`
  - Geometry: 10x1 U8.
  - Deviation: the source B3 plan states 9x1, but the fixed decoded
    literal contains 10 bytes. The tests use the literal's actual length.
- PackBits repeat vector: `fe aa 02 01 02 03` decodes to
  `aa aa aa 01 02 03`.
- LZW uses `weezl 0.1.12` (`MIT OR Apache-2.0`) because it has no direct
  dependencies, is pure safe Rust, and exposes both `BitOrder::Msb` and
  `with_tiff_size_switch` for TIFF's one-code-early width transition.
- The codecs are feature-gated but on by default in `fornax-codec` and
  `fornax-tiff`. `fornax-tiff --no-default-features --features raster`
  remains codec-minimal, and DNG/ARW opt out of the new defaults.
- `32946` remains an alias for `DecodeCompression::Deflate`.
- Implemented optional codecs use
  `TiffCompressionError::FeatureDisabled { required_feature, .. }` when their
  Cargo feature is off. `UnsupportedCompression` remains for documented IDs
  without a Fornax implementation, such as LZMA2 and WebP.
- Baseline observation from `origin/main`: `cargo test -p fornax-codec`
  already fails because codec-specific integration tests are not feature-gated.
  The feature matrix requires correcting that test-only gating.
- A maintainer correction superseded the original optional-only default:
  standard TIFF LZW and PackBits are enabled by default in `fornax-codec` and
  `fornax-tiff`; DNG and ARW opt out explicitly.
- F64 is codec-level support only. `fornax-raw-core::RawMosaic` and the DNG
  decoder/writer retain their existing U8/U16/U32/F32 surface.

## Validation Results

- `cargo test -p fornax-codec`: 62 passed.
- `cargo test -p fornax-codec --no-default-features`: 46 passed.
- `cargo test -p fornax-codec --features lzw`: 62 passed.
- `cargo test -p fornax-codec --features packbits`: 62 passed.
- `cargo test -p fornax-codec --all-features`: 77 passed.
- `cargo test -p fornax-tiff`: 61 passed.
- `cargo test -p fornax-tiff --no-default-features`: 22 passed.
- `cargo test -p fornax-tiff --no-default-features --features raster`: 60
  passed.
- `cargo test -p fornax-tiff --all-features`: 61 passed.
- `cargo test -p fornax-dng`: 36 passed.
- `cargo test -p fornax-dng --features zstd`: 39 passed.
- `cargo test -p fornax-arw`: 12 passed.
- `scripts/agent-validation.sh`: passed after initializing the raw-image
  fixture and pointer-only corpus submodule; includes manifest checks, the
  workspace all-feature suite excluding `compare-demosaic`, nightly formatting,
  rustdoc with warnings denied, CI Clippy flags, and decoder benchmark
  compilation.
- Local golden hash check materialized the nine-row manifest but failed because
  `git-lfs` is not installed. CI remains the authoritative golden check.
