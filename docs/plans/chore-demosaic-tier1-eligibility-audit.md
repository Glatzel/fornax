# Demosaic Tier 1 Eligibility Audit Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Add a research-only eligibility audit for MHC, PPG, RCD, and LMMSE,
and repair the documentation-link failures on PR 764 without changing
production behavior.

**Architecture:** The audit is split into one report per algorithm plus an
index that owns the cross-algorithm matrix and retain/remove/variant decisions.
Each report separates published mathematics, reference implementation behavior,
current Fornax behavior, and proposed mathematical changes. Link repairs update
current documentation and comment-only source links while leaving archived
history intact.

**Tech Stack:** Markdown, Rustdoc comments, Python documentation-link checker,
project validation scripts.

**Spec:** User-approved research audit scope from 2026-11-19.

## Global Constraints

- Do not change production logic, tests, dependencies, lockfiles, or `TODO.md`.
- Support only conventional 2x2 RGB Bayer processing.
- Distinguish original formulation, reference implementation, current Fornax
  behavior, and proposed modifications.
- Treat integer arithmetic and LUT use as implementation details when the
  underlying mathematics is linear or convertible.
- Mark insufficient evidence unresolved rather than inferring missing claims.
- Do not commit binary files.
- Keep archived documents historical and do not rewrite them as current.

## Review Focus

- A reader must not confuse modern LibRaw/darktable PPG with Chuan-kai Lin's
  original Pixel Grouping formulation.
- Signed samples crossing zero must not be mistaken for an RCD or LMMSE
  singularity without an explicit denominator policy.
- Positive scale equivariance must not be claimed for fixed-range constants.
- Reversible gamma processing must still be identified as a nonlinear-domain
  variant even when it decodes back to linear output.
- MHC has no current Fornax implementation, so the report must not imply one
  exists.

## Tasks

### Task 1: Repair current documentation links

**Files:**
- Delete: `.agents/skills/demosaic-comparison/SKILL.md`
- Create: `docs/reference/demosaic/README.md`
- Modify: `book/README.md`
- Modify: `book/src/bibliography.md`
- Modify: `book/src/notation.md`
- Modify: `book/template.md`
- Modify: `crates/fornax-raw-pipeline/src/demosaic/amaze.rs`
- Modify: `crates/fornax-raw-pipeline/src/demosaic/lmmse.rs`
- Modify: `crates/fornax-raw-pipeline/src/demosaic/vng.rs`
- Modify: `docs/README.md`
- Modify: `docs/CONVENTIONS.md`
- Modify: `docs/architecture/overview.md`

- [ ] Confirm `python3 scripts/check-doc-links.py` fails on the PR branch.
- [ ] Replace active links to deleted comparison or old algorithm paths with
  current `docs/reference/demosaic/` or archived-plan targets.
- [ ] Remove the obsolete comparison skill and the comparison-guide prose that
  presents the removed crate as current.
- [ ] Change only URL-bearing module-doc lines in the three Rust modules.
- [ ] Run `python3 scripts/check-doc-links.py`; expect
  `documentation links: ok`.
- [ ] Run `git diff --check`; expect no output.

### Task 2: Add the MHC research report

**Files:**
- Create: `docs/reference/demosaic/mhc.md`

- [ ] Confirm `test -e docs/reference/demosaic/mhc.md` fails.
- [ ] Add the Malvar-He-Cutler formulation, its 5x5 equations, boundary policy,
  absence of gamma/LUT/range dependencies, signed and scale analysis, generic
  float analysis, licensing/patent constraints, and the `Eligible` verdict.
- [ ] Run `rg -n '^## |^### ' docs/reference/demosaic/mhc.md`; expect sections
  for sources, equations, numeric properties, Fornax status, modifications,
  and unresolved questions.

### Task 3: Add the PPG research report

**Files:**
- Create: `docs/reference/demosaic/ppg.md`

- [ ] Confirm `test -e docs/reference/demosaic/ppg.md` fails.
- [ ] Document the original 2003 Pixel Grouping formulation, the 2007
  Desbiolles/dcraw lineage, and the modern LibRaw/darktable equations
  separately.
- [ ] Record the `Eligible` verdict for the original and modern formulations,
  the min/max and border behavior, and the separate-name rule where exact
  lineage matters.
- [ ] Run `rg -n '^## |^### ' docs/reference/demosaic/ppg.md`; expect sections
  for sources, equations, numeric properties, Fornax status, modifications,
  and unresolved questions.

### Task 4: Add the RCD research report

**Files:**
- Create: `docs/reference/demosaic/rcd.md`

- [ ] Confirm `test -e docs/reference/demosaic/rcd.md` fails.
- [ ] Add the RCD 2.3 discrimination, low-pass, ratio-correction, and
  red/blue equations from the pinned author source.
- [ ] Analyze fixed epsilon, denominator crossing, reference-domain clamping,
  signed values, scale homogeneity, and current Fornax behavior.
- [ ] Record the `Convertible` verdict and mandatory scale-relative
  denominator policy.
- [ ] Run `rg -n '^## |^### ' docs/reference/demosaic/rcd.md`; expect sections
  for sources, equations, numeric properties, Fornax status, modifications,
  and unresolved questions.

### Task 5: Rewrite the LMMSE research report

**Files:**
- Modify: `docs/reference/demosaic/lmmse.md`

- [ ] Confirm the current report contains the incorrect DOI fragment
  `TIP.2005.857267` and describes the removed comparison harness as current.
- [ ] Replace it with the Zhang-Wu primary formulation, correct DOI
  `10.1109/TIP.2005.857260`, IPOL equations, directional variance fusion, and
  color-difference reconstruction.
- [ ] Separate canonical linear-domain `LMMSE` (`Convertible`) from the current
  darktable-compatible gamma workspace (`Conditional` named variant).
- [ ] Run `rg -n '857260|Convertible|Conditional|gamma' docs/reference/demosaic/lmmse.md`;
  expect all four terms.

### Task 6: Complete the eligibility matrix and project index

**Files:**
- Modify: `docs/reference/demosaic/README.md`
- Modify: `docs/README.md`
- Modify: `docs/plans/chore-demosaic-tier1-eligibility-audit.md`

- [ ] Confirm the report links or required matrix headings are absent from the
  index.
- [ ] Add report links, eligibility matrix, removal list, named variants,
  mandatory changes, and unresolved questions.
- [ ] Add the index and reports to `docs/README.md`.
- [ ] Mark completed checklist items in this plan without changing `TODO.md`.
- [ ] Run `python3 scripts/check-doc-links.py`; expect
  `documentation links: ok`.

## Task Checklist

- [ ] Repair current documentation links and stale active references to the
  removed `compare-demosaic` crate.
- [ ] Add MHC, PPG, and RCD reports with the required equation and numeric
  analyses.
- [ ] Rewrite the LMMSE report so the correct paper DOI, linear-domain
  formulation, current gamma-domain implementation, and named variant status
  are explicit.
- [ ] Add the audit index with the eligibility matrix, removal list, variants,
  mandatory changes, and unresolved questions.
- [ ] Update the documentation index and planning record.
- [ ] Run documentation-link, docs-only validation, formatting, and diff-scope
  checks.

## Files Expected To Change

- `.agents/skills/demosaic-comparison/SKILL.md` (remove the obsolete skill)
- `book/README.md`
- `book/src/bibliography.md`
- `book/src/notation.md`
- `book/template.md`
- `crates/fornax-raw-pipeline/src/demosaic/{amaze,lmmse,vng}.rs`
  (module-doc link lines only)
- `docs/README.md`
- `docs/CONVENTIONS.md`
- `docs/architecture/overview.md`
- `docs/reference/demosaic/README.md`
- `docs/reference/demosaic/{mhc,ppg,rcd,lmmse}.md`
- `docs/plans/chore-demosaic-tier1-eligibility-audit.md`
- Current non-archived prose that directly identifies the removed harness as
  active

## Validation

```bash
python3 scripts/check-doc-links.py
scripts/agent-validation.sh --docs-only
cargo +nightly fmt --check -q
git diff --check
git diff --name-only
```

The PR branch already has an unrelated AMaZE scale-test failure. Record it and
do not modify code, tests, or waivers to address it.

## Decisions And Notes

- Verdicts describe the canonical mathematical family; current Fornax behavior
  is reported separately.
- `MHC`: eligible as a linear formulation, with no current Fornax kernel.
- Modern LibRaw/darktable `PPG`: eligible and already compatible with the
  current linear-light implementation.
- Original 2003 Pixel Grouping: eligible, but distinct from modern `PPG`; use
  a named variant if exact lineage matters.
- `RCD`: convertible through scale-relative denominator protection and removal
  of reference-domain assumptions.
- Zhang-Wu `LMMSE`: convertible to sensor-linear operation with a
  scale-relative variance floor.
- Darktable-compatible `LMMSE`: conditional, explicitly named variant because
  its gamma workspace is nonlinear.
- No Tier 1 mathematical family is fundamentally ineligible.
