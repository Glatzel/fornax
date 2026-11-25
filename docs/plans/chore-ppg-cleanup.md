# PPG Cleanup and Documentation Plan

## Goal

Fix the implementation and documentation issues found during the PPG review:
1. Eliminate the 10x redundant `green_at` evaluation in `PpgLazy::fill_rows` via a per-chunk green strip cache (bit-identical output, preserving laziness and Rayon row-chunk parallelism).
2. Dispatch horizontal color directly from `BayerChannel::GRedRow` / `BayerChannel::GBlueRow` instead of `channel_at(x + 1, y)`.
3. Add signed, zero-crossing, HDR, positive-scale equivariance, shift invariance, and flip-symmetry tests in `f32` and `f64`.
4. Rewrite `docs/reference/demosaic/ppg.md` to fix the 2-axis equation domain, 3px green / 1px chroma border description, `RawFloat` trait name, test coverage claims, and GitHub-safe LaTeX.

## Scope

In scope:
- `crates/fornax-raw/src/demosaic/ppg.rs`
- `docs/reference/demosaic/ppg.md`
- `docs/plans/chore-ppg-cleanup.md`

Out of scope:
- Changing PPG's mathematical formulation or tie-breaking behavior (the existing `fixed_9x9_rggb_mosaic_matches_reference` snapshot must pass unchanged).
- Modifying RCD, LMMSE, `compare-demosaic`, or `TODO.md`.

## Task Checklist

- [x] Create `.worktrees/chore-ppg-cleanup` from `origin/main` and link external fixtures
- [x] Add green strip cache and direct `GRedRow`/`GBlueRow` dispatch in `ppg.rs`
- [x] Add signed, HDR, scale, shift, flip, and `f64` tests in `ppg.rs`
- [x] Rewrite `docs/reference/demosaic/ppg.md` with accurate 2-axis equations, border policy, and GitHub-safe LaTeX
- [x] Run independent reviewer agent on the diff and address findings
- [x] Run `INSTA_UPDATE=no cargo test -p fornax-raw --lib demosaic::ppg` and `scripts/agent-validation.sh`
- [x] Commit, push, and open PR

## Files Expected to Change

- `crates/fornax-raw/src/demosaic/ppg.rs`
- `docs/reference/demosaic/ppg.md`
- `docs/plans/chore-ppg-cleanup.md`

## Validation

- `INSTA_UPDATE=no cargo test -p fornax-raw --lib demosaic::ppg`
- `scripts/agent-validation.sh`
- `git diff --check`

## Decisions and Notes

- Option A selected: keep modern PPG's horizontal-first green tie-break so existing snapshot compatibility is preserved while removing the 10x redundant green stencil work.
- Also notice `scripts/agent-validation.sh` initializes `external/raw.pixls.us` with `--remote` if missing, and `external/libtiff-pics` if missing.
