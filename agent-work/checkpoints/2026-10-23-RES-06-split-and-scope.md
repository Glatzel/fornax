# RES-06 Split and Initial Scope Checkpoint — 2026-10-23

## Completed

- Split the broad Rust ecosystem research into:
  - `RES-06`: Rust RAW decoder crates;
  - `RES-07`: Rust TIFF/DNG building blocks;
  - `RES-08`: Rust demosaicing crates; and
  - `RES-09`: cross-report comparison.
- Preserved the inspected evidence in `rust-raw-decoders.md`,
  `rust-dng-tiff.md`, and `rust-demosaicing.md`.
- Reset `rust-ecosystem.md` to the blocked `RES-09` comparison deliverable.
- Added a task dashboard and Mermaid dependency overview to the top of the live
  tracker.
- Defined the initial format and sensor scope as conventional 2x2 RGB Bayer
  DNG, Sony ARW, Nikon NEF, and Canon `CR*` (`CR2` and `CR3` expected in tests).
- Recorded Fujifilm X-Trans, Sigma Foveon/X3, Quad Bayer, four-color,
  monochrome, non-mosaic payloads, and all other RAW formats as deferred.
- Converted research references from local-only clones to pinned Git
  submodules.

## Status

`RES-01`–`RES-08` remain `paused`. `RES-09` is `blocked` on the three split
reports. No synthesis, design, implementation, or QA task has started.

## Next steps

1. Initialize and verify the pinned reference submodules.
2. Review the split reports against their updated acceptance criteria.
3. Resolve or accept the remaining evidence gaps.
4. Start `RES-09` only after `RES-06`–`RES-08` are in `review` or `done`.

## Unresolved questions

- Which target-format fixtures are available and valid for verification?
- Should the DNG adapter use image-tiff, rawler, or a minimal parser?
- What exact Bayer crop/CFA phase and black-level behavior should synthesis
  accept?
- Which licenses are acceptable for optional decoder dependencies?
