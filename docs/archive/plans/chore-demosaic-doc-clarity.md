> Status: archived completed plan.
> Merged: #769 on 2026-11-21.
> Current guidance: [RCD reference](../../reference/demosaic/rcd.md) and [LMMSE reference](../../reference/demosaic/lmmse.md).

# Demosaic Equation Documentation Clarity Plan

**Goal:** Rewrite the RCD and LMMSE demosaic research notes so every equation
uses GitHub Markdown dollar delimiters and every symbol, index, direction, and
implementation parameter is defined before or alongside its first use.

**Architecture:** This is a documentation-only change. The notes remain
research references; production algorithms, tests, APIs, and dependencies do
not change.

**Spec:** User request dated 2026-11-20.

## Global Constraints

- Do not change Rust code, tests, dependencies, or lockfiles.
- Keep conventional 2x2 RGB Bayer scope.
- Preserve the current RCD and LMMSE classifications and Fornax behavior.
- Use GitHub Markdown inline math with `$...$` and display math with `$$`.
- Do not use fenced `math` blocks or backticks as substitutes for equations.
- Preserve pinned source provenance and licensing statements.

## Review Focus

- Define the CFA sample field, coordinates, directions, and color planes.
- Explain RCD low-pass fields, discrimination matrices, ratio candidates,
  gradients, interpolation weights, and red/blue correction symbols.
- Explain LMMSE observations, directional estimates, low-pass taps, local
  means, variances, error variances, fusion weights, and neighborhood sets.
- Explain implementation-specific tile geometry, transfer constants, and
  variance floors where the notes describe current Fornax behavior.
- Make equation flow readable without requiring the reader to reverse-engineer
  the implementation.

## Tasks

### Task 1: Create the isolated worktree

**Files:** None.

- [x] Create `.worktrees/chore-demosaic-doc-clarity` from `origin/main`.
- [x] Link `external/` while excluding nested `.git` metadata.

### Task 2: Rewrite RCD notation and equations

**Files:**
- Modify: `docs/reference/demosaic/rcd.md`

- [x] Add a notation table for coordinates, CFA samples, low-pass fields,
  directional energies, discrimination weights, gradients, interpolation
  candidates, tolerances, and color-difference estimates.
- [x] Replace all fenced or code-style equations with GitHub dollar math.
- [x] Explain vertical/horizontal and P/Q discrimination stages in prose.
- [x] Explain the ratio-corrected green candidates and inverse-gradient
  combination.
- [x] Explain current tile parameters and the signed denominator limitation.

### Task 3: Rewrite LMMSE notation and equations

**Files:**
- Modify: `docs/reference/demosaic/lmmse.md`

- [x] Add a notation table for directional observations, Laplacian estimates,
  color differences, filter taps, local statistics, directional errors,
  fusion, and red/blue neighborhoods.
- [x] Replace fenced `math` blocks and code-style symbols with GitHub dollar
  math.
- [x] Distinguish filter taps from directional estimate values.
- [x] Explain current tile geometry, transfer parameters, variance floor, and
  LMMSE mode behavior.
- [x] Keep canonical and darktable-compatible formulations clearly separated.

### Task 4: Validate documentation

**Files:**
- Modify: `docs/plans/chore-demosaic-doc-clarity.md`

- [x] Run `scripts/agent-validation.sh --docs-only`.
- [x] Run `git diff --check`.
- [x] Inspect the diff and confirm no Rust, test, or dependency files changed.
- [ ] Commit, push, and open one semantic PR.

## Files Expected To Change

- `docs/plans/chore-demosaic-doc-clarity.md`
- `docs/reference/demosaic/lmmse.md`
- `docs/reference/demosaic/rcd.md`

## Validation

```bash
scripts/agent-validation.sh --docs-only
git diff --check
git diff --name-only
```

## Decisions And Notes

- Dollar-delimited equations are the documented authoring target for this
  change because GitHub renders them directly.
- Readability takes priority over compressing equations into one-line matrix
  notation.
- Implementation constants are documented only where they explain current
  Fornax behavior; they do not become new algorithm requirements.
