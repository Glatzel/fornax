# Workspace refactor Phase A

## Goal

Fix the stale names, failing checks, and record-keeping gaps left by PR #690,
then record comparable measurements without changing decoder output or making
reserved design decisions.

## Scope

In scope:

- mechanically propagate current crate and facade names;
- diagnose and narrowly fix the `fornax-dng` benchmark failure;
- report the `check-point` and lefthook failures;
- document the excluded, currently unbuildable `fornax-py` crate;
- record the implemented `SampleBuffer` contract;
- correct the Step 6 photometric-constant history;
- record baseline and current-main benchmark and build timings.

Out of scope:

- crate or facade renames;
- multi-band sample containers;
- publishing, versions, vendor codecs, or workspace restructuring;
- changing decoder behavior or golden data;
- compiling `compare-demosaic` locally;
- adding or removing the maintained `fornax-raw` `zstd` feature.

## Task checklist

- [x] Create an isolated branch/worktree from current `origin/main`.
- [x] Inventory stale names while excluding `external/` and `target/`.
- [x] Update source-controlled docs, CI, scripts, and crate documentation.
- [x] Remove the deleted compatibility-crate rows and document the `zstd`
      feature.
- [x] Identify the #690 `benchmark (fornax-dng)` failure from its CI log.
- [x] Identify what `check-point` does and why the listed PRs failed.
- [x] Fix the DNG manifest configuration that CodSpeed could not parse.
- [x] Document `fornax-py` unresolved dependencies and imports.
- [x] Record the approved `SampleBuffer` shape and `image` 0.x compatibility
      constraint.
- [x] Correct the Step 6 `LinearRaw`, `Depth`, and `PhotometricMask` history.
- [x] Run baseline and current-main measurements and build timings.
- [x] Re-run the golden hash check.
- [x] Run workspace validation and dependency-tree checks.
- [x] Inspect the final diff, commit, push, and open one Phase A pull request.

## Files expected to change

- `.github/workflows/ci.yml`
- `AGENTS.md`
- `README.md`
- `book/src/glossary.md`
- `book/src/notation.md`
- `crates/fornax-dng/Cargo.toml`
- crate README files under `crates/`
- `docs/architecture.md`
- `docs/decoder/architecture.md`
- `docs/decoder/benchmark.md`
- `docs/plans/**`
- `docs/proposal/**`
- `docs/references/**`
- `docs/superpowers/specs/**`

No decoder source, submodule pointer, or golden file should change.

## Validation

```bash
rg -n --hidden -g '!external/**' -g '!target/**' -g '!.git/**' \
  'fornax-core|fornax_core|fornax-preprocessor|fornax-postprocessor|fornax_preprocessor|fornax_postprocessor|crates/fornax-core' .
cargo tree -p fornax-codec -e normal --depth 1
cargo tree -p fornax-tiff -e normal --depth 1
cargo bench -p fornax-dng --all-features --bench writer --no-run
pixi run -e dev cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
scripts/agent-validation.sh
git diff --check
```

## Decisions and notes

- PR #690 is authoritative: `fornax-core` is `fornax-raw-core`, the facade
  package is `fornax-raw`, and the compatibility processor crates are deleted.
- The `fornax-raw` `zstd` feature is a maintainer decision and remains
  unchanged. It forwards to `fornax-decoder/zstd`.
- The `fornax-py` dependency and imports are intentionally unchanged. A3 is
  documentation-only and the Python API is a reserved decision.
- `benchmark (fornax-dng)` failed after a successful build because CodSpeed
  could not parse the multiline inline dependency table introduced in #690.
  The dependency remains feature-complete but is written on one TOML line.
- `check-point` is an aggregate sentinel that exits 1 when a needed CI job
  fails or is cancelled. It is a required status check under the active main
  ruleset.
- The #684, #686, and #687 lefthook failures included markdownlint MD036 in
  `docs/plans/chore-tiff-generic-accessors.md`. #687 also found a TOML order
  change. #688 also found a typos warning for `catch-alls` in
  `docs/plans/chore-dng-owned-constants.md`. They are content lint findings,
  not configuration or naming failures, so this PR reports them without
  changing those documents.
- No `fornax-codec` benchmark target exists at the baseline or main. The DNG
  target is a writer benchmark, and the baseline stage benchmarks lived in
  the preprocessor/postprocessor crates before moving to
  `fornax-raw-pipeline`. Measurements record that limitation explicitly.
- Pull request: [#692](https://github.com/Glatzel/fornax/pull/692).
