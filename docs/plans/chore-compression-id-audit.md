# TIFF/DNG Compression ID Ownership Audit

**Goal:** Establish source-backed ownership for the requested TIFF-family
compression IDs, move non-DNG values into the TIFF vocabulary, and make the TIFF
resolver distinguish unknown IDs from known IDs whose codec feature is
unavailable.

**Spec:** `docs/plans/workspace-refactor/b2-compression-id-audit.md` and
`docs/plans/workspace-refactor/phase-b-c-handoff.md`.

## Scope

In scope:

- audit IDs `1`, `5`, `7`, `8`, `32773`, `32946`, `34892`, `34925`, `50000`,
  `50001`, `50002`, and `52546`;
- keep DNG-defined `34892` and `52546` in `fornax-dng`;
- move source-confirmed non-DNG IDs into `fornax-tiff::constants`;
- add `TiffCompressionError` and the fallible `resolve_tiff_compression`
  contract from B2;
- preserve existing supported DNG and ARW decode results;
- document feature availability without implementing new codecs.

Out of scope:

- LZW, PackBits, LZMA, JPEG, WebP, or vendor codec implementations;
- changes to crate names, project versions, workspace ownership, golden files,
  benchmark execution, or the Python API;
- local `compare-demosaic` compilation or `lefthook`.

## Task Checklist

- [x] Verify PR #694 is merged before starting from current `main`.
- [x] Create branch `chore/compression-id-audit` and worktree
  `.worktrees/chore-compression-id-audit`.
- [x] Inspect TIFF 6.0, TIFF Technical Note #2, TIFF/EP 1.0, DNG 1.7.1.0,
  pinned LibTIFF headers/release notes, and the GDAL JPEG XL registration
  discussion.
- [x] Create `docs/references/compression-ids.md` with one row per requested ID.
- [x] Apply the ambiguity gate. No contradictory or unestablished ownership was
  found.
- [x] Update `docs/references/dng-compression.md` to match the audit.
- [x] Add failing TIFF resolver tests for supported, feature-disabled, and
  unknown IDs.
- [x] Add all confirmed non-DNG IDs to TIFF vocabulary with conservative codec
  feature gates.
- [x] Reduce `DngCompressionMethod` to DNG-owned `34892` and `52546`.
- [x] Route DNG compression resolution through the TIFF resolver and preserve
  existing DNG error behavior.
- [x] Update TIFF/DNG constants, resolver, writer, README, and architecture
  coverage.
- [x] Run focused tests, workspace tests, formatting, docs, Clippy, and
  dependency trees.
- [x] Record the local golden-check limitation; the CI golden job is pending at
  handoff.
- [x] Open one semantic PR:
  <https://github.com/Glatzel/fornax/pull/695>.

## Files Expected to Change

- `docs/references/compression-ids.md`
- `docs/references/dng-compression.md`
- `docs/plans/chore-compression-id-audit.md`
- `crates/fornax-tiff/Cargo.toml`
- `crates/fornax-tiff/README.md`
- `crates/fornax-tiff/src/constants.rs`
- `crates/fornax-tiff/src/lib.rs`
- `crates/fornax-tiff/src/raster.rs`
- `crates/fornax-tiff/tests/constants.rs`
- `crates/fornax-tiff/tests/raster.rs`
- `crates/fornax-codec/README.md`
- `crates/fornax-dng/Cargo.toml`
- `crates/fornax-dng/README.md`
- `crates/fornax-dng/src/constants.rs`
- `crates/fornax-dng/src/dng_decoder/mod.rs`
- `crates/fornax-dng/src/writer.rs`
- `crates/fornax-dng/tests/constants.rs`
- `crates/fornax-dng/tests/writer_boundary.rs`
- `docs/architecture.md`
- `docs/decoder/architecture.md`
- `docs/decoder/corpus.md`
- `docs/decoder/formats/dng.md`

## Validation

```bash
cargo test -p fornax-tiff --no-default-features
cargo test -p fornax-tiff --all-features
cargo test -p fornax-dng --all-features
cargo test -p fornax-arw
scripts/agent-validation.sh
cargo tree -p fornax-codec -e normal --depth 1
cargo tree -p fornax-tiff -e normal --depth 1 --features raster,jpegxl,zstd
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

Workspace validation is run with only the prohibited benchmark step skipped.
The golden hash command must report `golden: unchanged (9 rows)`.

## Validation Results

- `cargo test -p fornax-tiff --no-default-features`: pass, 22 tests.
- `cargo test -p fornax-tiff --all-features`: pass, 58 tests.
- `cargo test -p fornax-dng --all-features`: pass, 40 tests.
- `cargo test -p fornax-dng --no-default-features --lib`: pass, 26 tests.
- `cargo test -p fornax-arw`: pass, 12 tests.
- Workspace tests excluding `compare-demosaic`: pass, 337 tests in 41 suites.
- Decoder manifest check: current, 2020 rows.
- Decoder tracker DuckDB tests: pass, 4 tests.
- `cargo +nightly fmt --check`: pass.
- Workspace rustdoc with warnings denied: pass.
- Workspace Clippy with the CI flag set: pass.
- `cargo tree -p fornax-codec -e normal --depth 1`: only bytemuck, image,
  rayon, and thiserror.
- `cargo tree -p fornax-tiff -e normal --depth 1 --features
  raster,jpegxl,zstd`: only fornax-codec, num_enum, and strum.
- Local golden hash materialization failed at `git-lfs install`; this is
  unverified locally. The check remains a required CI gate.
- The decoder benchmark compilation and all other prohibited local actions were
  skipped.

## Decisions and Notes

- The audit gate found no reason to stop before production changes.
- TIFF `7` remains the generic JPEG identifier. Fornax's current supported raw
  subset still maps it to lossless JPEG-92.
- TIFF `8` and LibTIFF's legacy `32946` both identify Deflate data. They remain
  distinct vocabulary values but resolve to the same existing codec.
- IDs owned by LibTIFF or GDAL enter generic TIFF vocabulary but do not become
  DNG-owned values.
- Known IDs without an implemented codec return `UnsupportedCompression` with
  the required feature name. Unknown IDs return `UnknownCompression`.
- No new codec implementation is part of B2.
- The resolver remains a `const fn`; this preserves the B2 signature while
  satisfying the repository's `missing_const_for_fn` Clippy gate.
- LZW, PackBits, LZMA, and WebP are vocabulary entries only. Their resolver
  results name future features and do not add codec implementations.
- The local golden hash command could not materialize LFS objects because
  `git-lfs install` failed in this environment. Per maintainer direction, the
  CI golden job is authoritative instead of retrying local materialization.
- Pull request #695 is open and unmerged. CI status is pending at handoff.
