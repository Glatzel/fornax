# TIFF Raster Step

This archived workspace-refactor record documents TIFF Raster Step.

Implement only Step 5 of the Fornax workspace refactor. Open one PR and stop.

## Required state

- Work from current `origin/main`.
- Confirm Steps 3 and 4 are merged.
- Read `docs/_archive/workspace-refactor/baseline-findings.md`.

## Goal

Move generic TIFF strip/tile layout and compression-tag mapping out of DNG and
into `fornax-tiff`.

## Mandatory compression-ownership gate

Compression-ID ownership is not fully settled. Before implementation, produce a
table of every mapped ID with its normative source and supported data type.

At minimum classify:

- TIFF 6.0 IDs;
- TIFF-extension IDs;
- DNG-defined IDs, including 34892 and 52546;
- non-DNG self-assigned IDs, including 50000 and 50002.

Do not move ownership based only on comments or repository recollection. If a
required ID remains ambiguous, stop and request a maintainer ruling.

## Required implementation after approval

- Define the approved raster request/layout model in `fornax-tiff`.
- Accept already-resolved codec compression where the ownership boundary
  requires it.
- Provide a default resolver for TIFF-registered IDs.
- Let DNG resolve DNG-specific IDs before falling back to TIFF defaults.
- Preserve strip/tile ordering, partial final chunks, predictor behavior, and
  error semantics.
- Add tests for strips, tiles, partial geometry, unknown compression, and
  missing byte counts.

## Forbidden

- Do not make TIFF depend on DNG.
- Do not move `active_area`, `default_crop`, or format policy into TIFF.
- Do not move DNG constants until Step 6.
- Do not move vendor codecs or split workspaces.

## Branch and plan

- Suggested branch: `chore/tiff-raster`
- Suggested worktree: `.worktrees/chore-tiff-raster`
- Plan: `docs/_archive/workspace-refactor/tiff-raster-implementation.md`

## Validation

```bash
cargo test -p fornax-tiff
cargo test -p fornax-dng
cargo test -p fornax-arw
cargo test -p decoder-tracker
cargo test --workspace --all-features --exclude compare-demosaic
scripts/agent-validation.sh
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
git diff --check
```

## Pull request

- Suggested title: `refactor: add TIFF raster layout support`
- Include the compression-ownership table and any maintainer ruling.
- Stop after reporting the PR URL and any deviations.
