# Demosaic Readiness Review Implementation Plan

**Goal:** Record the independent readiness review for Linear and modern Pixel
Grouping (PPG), make modern PPG the supported PPG contract, classify the other
demosaic selectors as not ready, move Malvar-He-Cutler (MHC) to Tier -1 as a
reference-only method, and add a fixed-array PPG regression oracle.

**Architecture:** Production demosaicing remains unchanged. Readiness decisions
live in the demosaic research notes, while test-only code in the PPG module
compares a fixed mosaic against a fixed RGB result with indexed failure output.
MHC remains mathematical research evidence but is excluded from implementation
planning because of patent and source-license risk.

**Spec:** User request dated 2026-11-20.

## Global Constraints

- Do not change production demosaic logic, public selectors, dependencies, or
  production test behavior.
- Support only conventional 2x2 RGB Bayer input.
- Treat modern LibRaw/darktable PPG as sufficient; original 2003 Pixel Grouping
  is out of readiness scope.
- Do not implement, vendor, or copy MHC code or source.
- If the independent review reports a correctness problem, record it in the
  corresponding Linear or PPG research note instead of fixing production code.
- Test changes are limited to fixed expected outputs and indexed diagnostics.

## Review Focus

- Confirm the PPG equations describe diagonal direction selection at red and
  blue centers, while cardinal-direction reconstruction is reserved for green
  centers.
- Check Bayer phase handling, border behavior, direction ties, min/max
  envelope, measured-sample preservation, and small-image behavior.
- Check Linear interpolation for all four Bayer phases, borders, row chunking,
  and deterministic expected values.
- Separate mathematical eligibility, implementation availability, and
  production readiness.

## Tasks

### Task 1: Create and validate the isolated worktree

**Files:** None.

- [x] Create `.worktrees/chore-demosaic-readiness-review` from current
  `origin/main`.
- [x] Link the main checkout's populated `external/` tree.
- [x] Run the targeted demosaic baseline.

### Task 2: Independently review Linear and modern PPG

**Files:** None; the reviewer is read-only.

- [x] Dispatch an independent reviewer with the modern-PPG scope and
  no-production-change constraint.
- [x] Record whether either implementation is ready.
- [x] Convert every reported problem into evidence in the corresponding
  research note.

### Task 3: Update PPG research and add a fixed snapshot

**Files:**
- Modify: `docs/reference/demosaic/ppg.md`
- Modify: `crates/fornax-raw-pipeline/src/demosaic/ppg.rs` (test module only)

- [ ] Update the modern equations to name diagonal directions accurately.
- [ ] State that modern PPG is the supported formulation and remove original
  Pixel Grouping from the readiness backlog.
- [x] Add a fixed 9x9 expected-output snapshot with three decimal places. Use
  `insta` directly rather than a custom assertion macro.
- [x] Add review findings or the completed readiness decision.

### Task 4: Add the Linear readiness note

**Files:**
- Create: `docs/reference/demosaic/linear.md`

- [x] Document the bilinear formulation, Bayer-phase mapping, border policy,
  numeric contract, fixed three-decimal snapshots, and review result.
- [x] Add review findings or the completed readiness decision.

### Task 5: Classify other algorithms and MHC tiering

**Files:**
- Modify: `docs/reference/demosaic/README.md`
- Modify: `docs/reference/demosaic/mhc.md`
- Modify: `crates/fornax-raw-pipeline/README.md`
- Modify: `TODO.md`

- [x] Mark Linear and modern PPG as ready only after the independent review
  finds no blocking problem.
- [x] Mark AMaZE, RCD, LMMSE, VNG, and VNG4 as implemented but not production
  ready.
- [x] Move MHC from Tier 1 to Tier -1, reference-only, and prohibit future
  implementation because of patent/source-license constraints. Moving the
  existing human-owned entry responds to the explicit user request and does
  not add a new TODO item.

### Task 6: Validate and inspect scope

**Files:**
- Modify: `docs/plans/chore-demosaic-readiness-review.md`

- [x] Run the PPG and Linear tests.
- [x] Run `scripts/agent-validation.sh --docs-only` if the final diff is
  documentation-only.
- [x] Otherwise run `scripts/agent-validation.sh` because the diff includes a
  test module change.
- [x] Run `git diff --check`, inspect `git diff --name-only`, and confirm no
  production logic changed.
- [ ] Commit and push the branch, then open one semantic PR.

## Files Expected To Change

- `Cargo.lock`
- `Cargo.toml`
- `crates/fornax-raw-pipeline/Cargo.toml`
- `crates/fornax-raw-pipeline/README.md`
- `crates/fornax-raw-pipeline/src/demosaic/test_support.rs` (test-only
  three-decimal snapshot formatter)
- `crates/fornax-raw-pipeline/src/demosaic/ppg.rs` (snapshot test only)
- `crates/fornax-raw-pipeline/src/demosaic/linear.rs` (snapshot tests only)
- `crates/fornax-raw-pipeline/src/demosaic/snapshots/*.snap`
- `docs/plans/chore-demosaic-readiness-review.md`
- `docs/README.md`
- `docs/reference/demosaic/README.md`
- `docs/reference/demosaic/linear.md`
- `docs/reference/demosaic/mhc.md`
- `docs/reference/demosaic/ppg.md`
- `TODO.md`

## Validation

```bash
cargo test -p fornax-raw-pipeline --lib demosaic
scripts/agent-validation.sh
git diff --check
git diff --name-only
```

The initial targeted baseline is `cargo test -p fornax-raw-pipeline --lib
demosaic`: 41 passed, 0 failed.

Completed checks:

- `cargo +nightly fmt --all -- --check`
- `cargo test -p fornax-raw-pipeline --lib demosaic::` with `INSTA_UPDATE=no`:
  41 passed, 0 failed
- `cargo test --workspace --all-features --exclude compare-demosaic`: passed
- `cargo clippy --workspace --all-features --all-targets --exclude
  compare-demosaic`: passed
- `RUSTDOCFLAGS="-D warnings" cargo doc --workspace --all-features --exclude
  compare-demosaic --no-deps`: passed
- `cargo check -p fornax-codec --no-default-features`: passed
- `cargo test -p fornax-codec --features lossless-jpeg`: passed
- `git diff --check`: passed

Failed or interrupted checks:

- `scripts/agent-validation.sh` stops at the existing
  `docs/reference/demosaic/lmmse.md:229` link to the absent
  `crates/fornax-raw-pipeline/tests/demosaic_invariants.rs`. The same link is
  absent from `origin/main`, so this branch does not introduce it.
- `cargo bench -p fornax-raw-decoder --no-run` was intentionally interrupted
  when the user asked to stop checks.

## Decisions And Notes

- The user explicitly allows the fixed-array snapshot tests and dev-dependency
  update even though production code changes are forbidden.
- Final test snapshots use `insta`, format every channel to three decimal
  places, and only run on Linux x86_64.
- Eligibility does not imply production readiness.
- Hybrid CFA support remains out of scope.
- Independent review found no production correctness problem in Linear or
  modern PPG.
- The PPG direction `d` is cardinal; diagonal reconstruction applies only to
  opposite-color estimates.
