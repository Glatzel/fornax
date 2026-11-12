# Trigger: Step 3 — move generic TIFF accessors

Implement only Step 3 of the Fornax workspace refactor. Open one PR and stop.

## Required state

- Work from current `origin/main`.
- Confirm Step 2 is merged and `fornax-raw-pipeline` exists.
- Confirm no later refactor step is already on `main`.

## Goal

Move only generic TIFF value/accessor helpers into `fornax-tiff`.

## Required code review before editing

- Compare `fornax-dng` and `fornax-arw` implementations of `unsigned_scalar`,
  `unsigned_vec`, and directly related list/byte access helpers.
- Identify behavior differences and preserve each caller’s accepted TIFF value
  types and error semantics.
- Do not move DNG or Sony semantics into TIFF.

## Required change

- Add the smallest generic accessor API needed by both callers to
  `fornax-tiff`.
- Replace duplicated DNG and ARW helpers with the TIFF API.
- Add focused tests for scalar/list conversion, list handling, overflow, and
  rejected value types.
- Keep `active_area`, `default_crop`, `crop_and_cast`, Sony model fallbacks, and
  other format policy in their existing owners.

## Workflow

- Branch: `chore/tiff-generic-accessors`
- Worktree: `.worktrees/chore-tiff-generic-accessors`
- Plan: `docs/plans/chore-tiff-generic-accessors.md`
- Do not modify `TODO.md`.

## Forbidden

- Do not move DNG tag policy or active-area/default-crop interpretation.
- Do not move strip/tile raster layout yet.
- Do not move DNG-owned constants.
- Do not change codec output or sample-buffer types.
- Do not resolve neutral buffer, vendor placement, or crate naming.

## Validation

```bash
cargo test -p fornax-tiff
cargo test -p fornax-dng
cargo test -p fornax-arw
cargo test --workspace --all-features --exclude compare-demosaic
scripts/agent-validation.sh
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
git diff --check
```

## Pull request

- Suggested title: `refactor: share generic TIFF accessors`
- Record the exact helper behavior moved and any intentional differences.
- Stop after reporting the PR URL and any deviations.
