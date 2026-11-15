# Workspace refactor Phase B-C handoff

This archived workspace-refactor record documents Workspace refactor Phase B-C handoff.

## Purpose

This index is the shared specification for the remaining workspace-refactor
work. It exists so a later session can pick one step without relying on prior
chat context.

Phase A is complete when PR #692 is merged. Do not start any Phase B or C step
until the maintainer explicitly releases that step.

## Global rules

- Work from current `origin/main` after Phase A.
- Use one branch, one worktree, and one pull request per numbered step.
- Use these branch names unless a later maintainer instruction changes them:
  `chore/tiff-optional-raster`, `chore/compression-id-audit`,
  `chore/codec-lzw-packbits`, `chore/multiband-note`, and
  `chore/publish-readiness`.
- Copy or link the selected step plan into the required
  `docs/plans/chore-<slug>.md` path on that branch.
- Keep `decoder-tracker/golden/` unchanged. Rerun the hash check for every PR.
- Never place raw-workflow types in `fornax-codec` or `fornax-tiff`.
- Report dependency trees for `fornax-codec` and `fornax-tiff` after every PR.
- Do not compile `compare-demosaic` locally.
- Benchmarks are allowed only for `fornax-dng`, `fornax-codec`, and
  `fornax-raw-pipeline`.
- Do not publish, change project versions, or change versions as part of these
  steps.
- Stop after the selected step PR opens. Do not continue automatically.

## Reserved decisions

A step must stop and ask the maintainer if it would decide any of:

- crate names;
- multi-band or `i16`/`f64` container shape;
- whether or when to publish;
- project version numbers;
- vendor codec placement;
- the intended `fornax-py` API;
- removing ARW's dependency on DNG;
- the workspace split.

The `fornax-raw` `zstd` feature is already decided and must remain.

## Step plans

| Step | Branch and worktree | Plan | Branch plan path | Deliverable |
| --- | --- | --- | --- | --- |
| B1 | `chore/tiff-optional-raster`; `.worktrees/chore-tiff-optional-raster` | [Optional TIFF raster](optional-tiff-raster-plan.md) | `docs/plans/chore-tiff-optional-raster.md` | Default `fornax-tiff` has no codec/image/Rayon dependency |
| B2 | `chore/compression-id-audit`; `.worktrees/chore-compression-id-audit` | [Compression ID audit](compression-id-audit-plan.md) | `docs/_archive/workspace-refactor/compression-id-audit-implementation.md` | Source-versioned ownership table, then implementation only if ownership is clear |
| B3 | `chore/codec-lzw-packbits`; `.worktrees/chore-codec-lzw-packbits` | [LZW and PackBits](lzw-packbits-plan.md) | `docs/_archive/plans/lzw-packbits-codecs.md` | Feature-gated LZW/PackBits codecs and legacy Deflate alias |
| B4 | `chore/multiband-note`; `.worktrees/chore-multiband-note` | [Multi-band design note](multiband-design-plan.md) | `docs/_archive/workspace-refactor/multiband-container-record.md` | Options and evidence, with no selected shape |
| C | `chore/publish-readiness`; `.worktrees/chore-publish-readiness` | [Publish readiness](publish-readiness-plan.md) | `docs/_archive/workspace-refactor/publish-readiness-implementation.md` | Read-only package/publish report, with no publishing |

## Required report

Keep the PR handoff under 60 lines and include:

- PR link and semantic title;
- files changed and non-blank LOC;
- commands and short results;
- both dependency trees;
- golden hash result;
- CI status;
- `[DONE]`, `[DEVIATION]`, `[DECIDED-BY-AGENT]`, and `[UNVERIFIED]` items;
- next step and whether it requires a maintainer decision.
