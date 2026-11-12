# Optional TIFF Raster Feature Plan

## Goal

Make `fornax-tiff`'s raster integration optional so a consumer that only needs
TIFF values, tags, parsing, and writing does not pull in `fornax-codec` or its
codec/image/Rayon dependency stack. Keep the existing raster API and behavior
unchanged when the new feature is enabled.

## Scope

In scope:

- Add an empty default feature set and a `raster` feature backed by an optional
  `fornax-codec` dependency, preserving the current codec features.
- Gate the raster module, raster re-exports, and raster-only integration tests.
- Enable `raster` on `fornax-dng`'s TIFF dependency, since the DNG decoder uses
  `RasterRequest` and `resolve_tiff_compression`.
- Document the feature boundary in the `fornax-tiff` README and workspace
  architecture.
- Run focused feature, workspace, CI-equivalent, and golden-hash validation.

Out of scope:

- Compression ID or constants changes.
- Moving codecs or codec functionality.
- Changes under `crates/fornax-codec`.
- Version, publication, benchmark, or local `compare-demosaic` work.
- Golden-file edits or decoder behavior changes.

## Task Checklist

- [x] Gate the raster integration test file with the `raster` feature.
- [x] Make `fornax-codec` optional with its existing feature list unchanged.
- [x] Add `default = []` and `raster = ["dep:fornax-codec"]` to
  `fornax-tiff`; enable all features for docs.rs builds.
- [x] Gate `mod raster` and the raster re-exports in `fornax-tiff`.
- [x] Enable `fornax-tiff/raster` only for dependents that use raster symbols;
  inventory confirms this is `fornax-dng`.
- [x] Document what the feature adds and its dependency cost.
- [x] Run all focused, workspace, formatting, clippy, and golden-hash checks.
- [x] Leave lefthook execution to CI per the user's explicit override and
  report the local lefthook check as unverified.
- [x] Review the final diff and open one pull request.

## Inventory

- `fornax-codec` is imported by `fornax-dng` normally and by `fornax-tiff` only
  in `src/raster.rs` and `tests/raster.rs`.
- The TIFF raster API is used by `fornax-tiff` tests and by `fornax-dng` in
  `src/dng_decoder/mod.rs`.
- Direct `fornax-tiff` dependents `fornax-arw` and `decoder-tracker` use parser,
  constants, values, and writer types but no raster symbols.
- No non-raster `fornax-tiff` module imports a `fornax-codec` type.

## Files Expected To Change

- `crates/fornax-tiff/Cargo.toml`
- `crates/fornax-tiff/src/lib.rs`
- `crates/fornax-tiff/tests/raster.rs`
- `crates/fornax-tiff/README.md`
- `crates/fornax-dng/Cargo.toml`
- `docs/architecture.md`
- `docs/plans/chore-tiff-raster-feature.md`

## Validation

- Confirm default, raster, DNG, and codec dependency trees have the required
  edges and no undesired default dependencies.
- Run `cargo check -p fornax-tiff --no-default-features`.
- Run `cargo test -p fornax-tiff --no-default-features` and
  `cargo test -p fornax-tiff --all-features`; compare their counts with the
  54-test pre-change baseline.
- Run `cargo test --workspace --exclude compare-demosaic`.
- Run repository formatting, clippy, and lefthook checks as CI does.
- Run the golden decoder hash check and require all 9 rows unchanged.
- Run `git diff --check` and inspect non-code dependency changes.

## Decisions and Notes

- Branch from current `origin/main` (`5ecf385`) in
  `.worktrees/chore-tiff-raster-feature`; PR #690's renamed crates are already
  present, so no A1 rebase is expected.
- Configure the feature only on the `fornax-dng` dependency edge so the
  workspace-level `fornax-tiff` dependency remains lightweight by default.
- Keep `fornax-codec`'s TIFF-requested `deflate` and `lossless-jpeg` features on
  the optional dependency exactly as before.

## Validation Results

- Pre-change `fornax-tiff` baseline: 54 passed.
- `--no-default-features`: 22 passed; `--all-features`: 54 passed. The 32
  feature-bound tests are 12 raster integration tests and 20 raster doctests.
- Workspace tests excluding `compare-demosaic`: 332 passed with both default
  and all features.
- `cargo +nightly fmt --check -q`, workspace docs with warnings denied, and
  Clippy with the CI flags all passed.
- Golden decode hashes: `golden: unchanged (9 rows)`.
- Lefthook was not run locally because the user explicitly prohibited it; its
  result remains `[UNVERIFIED]` pending PR CI.
