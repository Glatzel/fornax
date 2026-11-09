# Trigger: Step 4 — add codec sample buffer

Implement only Step 4 of the Fornax workspace refactor. Open one PR and stop.

## Required state

- Work from current `origin/main`.
- Confirm Step 3 is merged.
- Read `docs/references/workspace-refactor-baseline.md`.

## Mandatory design gate

The neutral buffer shape is an open maintainer decision. Do not choose it.

Before implementation, report:

- the required sample types;
- band representation;
- interleaving representation;
- whether bands are compile-time or runtime;
- how the existing mono `RawMosaic` adapts to and from the buffer;
- effects on integer rasters versus `Primitive: Float`.

Stop and request a ruling before writing production code if the maintainer has
not supplied the exact shape.

## Goal after the shape is decided

Move neutral codec output into a codec-owned sample buffer so `fornax-codec`
does not need raw workflow types, while keeping `RawMosaic` source-compatible
through adapters.

## Required change after approval

- Add the approved sample buffer to `fornax-codec`.
- Make generic codec decode/encode APIs use it.
- Add conversion between a single-band buffer and `RawMosaic`.
- Keep `RawMosaic` ownership stable unless the maintainer explicitly changes
  it.
- Add round-trip tests for every approved sample type and band layout.
- Cover interleaved and planar invalid cases if the approved shape represents
  both.

## Forbidden

- Do not move DNG constants.
- Do not move TIFF raster policy.
- Do not move vendor codecs.
- Do not rename or split workspaces.
- Do not decide vendor codec placement.

## Branch and plan

- Suggested branch: `chore/codec-sample-buffer`
- Suggested worktree: `.worktrees/chore-codec-sample-buffer`
- Plan: `docs/plans/chore-codec-sample-buffer.md`

## Validation

```bash
cargo test -p fornax-codec
cargo test -p fornax-dng
cargo test -p decoder-tracker
cargo test --workspace --all-features --exclude compare-demosaic
scripts/agent-validation.sh
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
git diff --check
```

## Pull request

- Suggested title: `refactor: add neutral codec sample buffer`
- Include the approved shape, adapter boundary, and golden-hash result.
- Stop after reporting the PR URL and any deviations.
