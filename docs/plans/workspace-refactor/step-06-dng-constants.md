# Trigger: Step 6 — move DNG-owned constants

Implement only Step 6 of the approved Fornax refactor. Open one PR and stop.

## Required state

- Work from current `origin/main`.
- Confirm Steps 3, 4, and 5 are merged.
- Read `docs/references/workspace-refactor-baseline.md` and
  `docs/references/dng-compression.md`.

## Goal

Move DNG-defined tag/value constants out of `fornax-tiff` while preserving
unknown tag IDs and unknown field types.

## Mandatory ownership gate

Re-verify every moved compression and photometric value against the DNG
specification and relevant TIFF registry material. The baseline review found:

- 34892 is DNG lossy JPEG;
- 52546 is DNG 1.7 JPEG XL;
- 50000 is libtiff self-assigned Zstandard;
- 50002 is a non-DNG self-assigned JPEG XL value.

Treat the list as evidence to re-check, not permission to omit the gate. Stop if
an owner remains ambiguous.

## Required implementation after approval

- Move only DNG-owned constants to `fornax-dng`.
- Keep TIFF 6.0 and TIFF/EP constants grouped by specification in
  `fornax-tiff`.
- Ensure open `Tag` IDs survive parsing and rewriting.
- Preserve currently supported unknown-value behavior. Do not invent an open
  `Value` representation unless the maintainer separately requires it.
- Update DNG, ARW, tracker, CLI, and test imports.
- Add round-trip tests proving known DNG values resolve while unknown IDs and
  field types are preserved.

## Forbidden

- Do not change decode output or codec behavior.
- Do not move raster layout back into DNG.
- Do not merge format crates.
- Do not split workspaces.
- Do not decide vendor codec placement or crate names.

## Branch and plan

- Suggested branch: `chore/dng-owned-constants`
- Suggested worktree: `.worktrees/chore-dng-owned-constants`
- Plan: `docs/plans/chore-dng-owned-constants.md`

## Validation

```bash
cargo test -p fornax-tiff
cargo test -p fornax-dng
cargo test -p fornax-arw
cargo test -p decoder-tracker
cargo test -p fornax-cli
cargo test --workspace --all-features --exclude compare-demosaic
scripts/agent-validation.sh
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
git diff --check
```

## Completion

After this PR, stop. Do not move vendor codecs, consolidate format/facade
crates, split the workspace, or publish anything without a new maintainer task.
