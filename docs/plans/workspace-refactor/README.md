# Workspace refactor step triggers

## Purpose

This directory contains one self-contained task prompt per approved refactor
step. Give an implementing agent the relevant step file as its task prompt.
The agent must execute only that step, validate it, open one PR, and stop.

## Original step sequence

| Step | Trigger | Status at baseline |
| --- | --- | --- |
| 1 | [Decode baseline](step-01-decode-baseline.md) | Complete in PR #681 |
| 2 | [Merge raw processing](step-02-raw-pipeline.md) | In progress in a separate worktree |
| 3 | [Move generic TIFF accessors](step-03-tiff-accessors.md) | Ready after Step 2 |
| 4 | [Add codec sample buffer](step-04-sample-buffer.md) | Blocked until buffer shape is decided |
| 5 | [Move TIFF raster handling](step-05-tiff-raster.md) | Requires compression-ownership review |
| 6 | [Move DNG constants](step-06-dng-constants.md) | Requires compression-ownership review |

Phase A repaired the #690 leftovers after Steps 1-6 and is recorded in
[chore-workspace-refactor-phase-a.md](../chore-workspace-refactor-phase-a.md).

## Future work

The remaining Phase B and Phase C requests are split into independently
pickable plans. Start only the step the maintainer releases.

- [Phase B-C handoff](phase-b-c-handoff.md)
- [B1: optional TIFF raster](b1-optional-tiff-raster.md)
- [B2: compression ID ownership audit](b2-compression-id-audit.md)
- [B3: LZW and PackBits codecs](b3-lzw-packbits.md)
- [B4: multi-band design note](b4-multiband-note.md)
- [B4 evidence: multi-band container options](multiband-note.md)
- [C: publish readiness report](c-publish-readiness.md)

## Global execution rules

- Work from current `origin/main`.
- Confirm that every earlier step is merged before starting.
- Read `AGENTS.md`, the step plan, and
  [workspace-refactor-baseline.md](../../references/workspace-refactor-baseline.md).
- Use one branch, one worktree, one pull request, and one semantic commit series
  per step.
- Create a plan under `docs/plans/` before implementation.
- Use TDD for behavior or API changes.
- Keep the workspace green after every commit.
- Run `scripts/agent-validation.sh`.
- Run the golden decoder check when the step is present on the branch:

```bash
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

- Do not modify `TODO.md`; these are explicit maintenance requests.
- Do not merge, poll, or close the pull request.
- Stop and report after the step PR opens.

## Decisions that remain open

- Neutral buffer shape.
- Vendor codec placement.
- Crate names.

Where a trigger reaches one of these decisions, it must stop and request a
maintainer ruling rather than selecting an answer.

## Stop boundary

The Steps 1-6 scope ended with Phase A. Phase B and Phase C remain explicitly
gated by the maintainer. Do not merge ARW and decoder into one format crate,
split Cargo workspaces, rename the facade, move vendor codecs, or publish
crates unless the maintainer explicitly starts a new task.
