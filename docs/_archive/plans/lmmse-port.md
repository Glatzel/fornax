# LMMSE faithful port

This archived plan documents the completed LMMSE faithful port work.

## Goal

Replace Fornax's generic directional LMMSE approximation with a dedicated
implementation of darktable 5.6.2's mode-1 LMMSE path. The native GBRG
comparison row must meet MAE <= 2, P99 <= 64, and a high-error fraction <=
0.001 above 1024 codes.

## Scope

In scope:

- PPG-seeded borders and darktable-compatible LMMSE reconstruction.
- Gamma encode/decode lookup tables with interpolated access.
- 136-pixel tiles, four-pixel borders, eight-pixel overlap, and valid stride 112.
- Directional color differences, nine-tap low-pass filtering, directional
  variance weighting, red/blue interpolation, and one median pass.
- Unit and comparison regressions for all Bayer phases.
- Reference documentation, snapshots, baseline, generated README, and removal
  of `[LMMSE]` from `TODO.md`.

Out of scope:

- Other darktable LMMSE modes or refinement passes.
- White balance, camera matrices, output crops, and pipeline module behavior.

## Task checklist

- [x] Add a failing native comparison-limit regression.
- [x] Add focused tests for gamma lookup and small-image handling.
- [x] Implement the dedicated code-domain LMMSE kernel.
- [x] Remove the obsolete generic `Lmmse` directional profile.
- [x] Run all-pattern tests and the native GBRG comparison.
- [x] Refresh LMMSE snapshots, canonical baseline, and generated README.
- [x] Run workspace validation and inspect the final diff.

## Files expected to change

- `crates/compare-demosaic/src/lib.rs`
- `crates/compare-demosaic/src/snapshots/*-LMMSE-darktable.snap`
- `crates/compare-demosaic/README.md`
- `crates/compare-demosaic/baseline/canonical.json`
- `crates/fornax-raw-pipeline/src/demosaic.rs`
- `crates/fornax-raw-pipeline/src/demosaic/directional.rs`
- `crates/fornax-raw-pipeline/src/demosaic/lmmse.rs`
- `docs/reference/algorithms/lmmse.md`
- `TODO.md`

## Validation

```bash
cargo test -p fornax-raw-pipeline lmmse
cargo test -p compare-demosaic native_lmmse_meets_reference_limits
cargo run --release -p compare-demosaic -- run --algorithm lmmse
cargo run --release -p compare-demosaic -- readme --check
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-features --all-targets -- -D warnings -A clippy::too_many_arguments
```

Also run `git diff --check` and the repository Markdown lint on changed
Markdown.

## Decisions and notes

- Use 16-bit codes internally because the comparison reference and existing
  exact PPG path operate in the same normalized code domain.
- Keep `DemosaicLmmse` and `DenseLazy` as the public interfaces.
- Match the comparison bridge's `mode = 1`, `scaler = 1.0`, and
  `threshold = 0.0` PPG seed.
- Preserve the reference's single scratch-buffer initialization across tiles;
  edge decisions depend on padding retained from earlier tiles.
- The pinned darktable `lmmse.c` is GPLv3. Record it as behavioral evidence
  and document the license constraint; direct source reuse requires project
  licensing review.
