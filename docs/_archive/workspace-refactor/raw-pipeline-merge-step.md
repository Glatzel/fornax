# Raw Pipeline Merge Step

This archived workspace-refactor record documents Raw Pipeline Merge Step.

Implement only Step 2 of the Fornax workspace refactor. Open one PR and stop.

## Required state

- Work from current `origin/main`.
- Confirm PR #681 and the decode baseline are merged.
- Confirm Steps 3 through 6 have not been started.
- Do not interfere with another active worktree implementing this same step.

## Goal

Merge the preprocessing and demosaicing stages into `fornax-raw-pipeline`.

## Read first

- `AGENTS.md`
- `docs/architecture/overview.md`
- `docs/_archive/workspace-refactor/baseline-findings.md`
- `crates/fornax-raw-pipeline`
- `crates/fornax`
- `crates/fornax-cli`
- `crates/compare-demosaic`

## Workflow

- Create branch `chore/merge-pre-post-pipeline`.
- Create worktree `.worktrees/chore-merge-pre-post-pipeline` from `origin/main`.
- Create a plan at `docs/_archive/workspace-refactor/raw-pipeline-merge.md`.
- Do not modify `TODO.md`.
- Keep every commit independently green.

## Required change

- Create `fornax-raw-pipeline`.
- Move normalization into `fornax-raw-pipeline`.
- Move demosaic and color processing into `fornax-raw-pipeline`.
- Preserve behavior and existing public names where practical:
  `Preprocessor`, `Demosaic`, `LmmseMode`, `IDemosaic`, `LazyDemosaic`,
  `OutputColorSpace`, and `PostProcessor`.
- Update facade, CLI, and comparison imports/reexports.
- Retire or reduce the old crates only when the workspace remains green and
  the change remains limited to this merge.
- Use TDD for behavior-affecting changes.

## Forbidden

- Do not change codec output.
- Do not implement a neutral buffer.
- Do not move TIFF raster logic.
- Do not move DNG constants.
- Do not move vendor codecs.
- Do not rename crates outside the required pipeline crate.
- Do not split or publish workspaces.

## Validation

```bash
cargo test -p fornax-raw-pipeline
cargo test -p fornax-cli
cargo test --workspace --all-features --exclude compare-demosaic
scripts/agent-validation.sh
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
git diff --check
```

Compile benchmark targets through repository validation. Do not compile or run
`compare-demosaic` unless its native environment is available; report any skip.

## Pull request

- Base: `main`
- Suggested title: `refactor: merge preprocessing and postprocessing`
- Include moved symbols, validation results, golden-hash result, and skipped
  checks.
- Stop after reporting the PR URL and any deviations.
