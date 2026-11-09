# Trigger: Step 1 — decode baseline

This step is complete. It is retained so every trigger has the same format.

## Required state

- PR #681 is merged.
- `docs/references/workspace-refactor-baseline.md` exists.
- `crates/decoder-tracker/golden/selected-manifest.csv` exists.
- `crates/decoder-tracker/golden/decode-hashes.csv` exists.

## Completed deliverable

- Reproducible pre-refactor build and test measurements.
- Canonical hashes for raw samples, cropped samples, process metadata, and DNG
  metadata.
- A nine-case pointer-only golden corpus set covering representative DNG and
  ARW decoding paths.
- Verification and correction of the advisory feasibility analysis.

Do not redo this step. See
[workspace-refactor-baseline.md](../../references/workspace-refactor-baseline.md).
