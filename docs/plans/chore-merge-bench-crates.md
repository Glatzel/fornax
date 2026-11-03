# Merge Benchmark Crates Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [x]`) syntax for tracking.

**Goal:** Consolidate the demosaic and preprocessor benchmark crates into their corresponding production-stage crates while preserving benchmark behavior and explicit `cargo bench -p` entry points.

**Architecture:** Criterion targets move into `fornax-postprocessor/benches` and `fornax-preprocessor/benches`. The demosaic registry helper stays in a benchmark-only support module shared with a focused integration test, so it is testable without expanding the postprocessor library API. Workspace membership, CI matrix packages, and architecture documentation then reference only the production-stage crates.

**Tech Stack:** Rust 2024, Cargo benchmark targets, Criterion compatibility crate, `fornax-devtool`, `serde_json`, GitHub Actions.

**Spec:** The user confirmed full consolidation of `bench-demosaic` into `fornax-postprocessor` and `bench-preprocessor` into `fornax-preprocessor`, including removal of the standalone benchmark crates.

## Global Constraints

- Keep decoding, preprocessing, and demosaicing as separate pipeline stages.
- Preserve the active conventional 2x2 RGB Bayer scope; do not add CFA support.
- Preserve benchmark case names, environment variables, sample types, thread sweeps, and timed work.
- Expose benchmarks through `cargo bench -p fornax-postprocessor` and `cargo bench -p fornax-preprocessor`.
- Do not add benchmark registry helpers to the public production API.
- Delete `crates/bench-demosaic` and `crates/bench-preprocessor`.
- Do not modify `TODO.md`.
- Run the repository-required Rust validation commands and report any failures explicitly.

## Review Focus

- A benchmark output row must keep its existing name and measured stage boundary after moving into the stage crate.
- An unknown `FORNAX_BENCH_ALGORITHM` value must still fail instead of silently benchmarking everything.
- Every registry ID must still map to its production `Demosaic` variant.
- Running `cargo bench -p fornax-postprocessor` or `cargo bench -p fornax-preprocessor` must compile the moved target without requiring a separate benchmark package.
- CI changed-path filters must trigger when either a stage crate or its integrated benchmark changes.

---

### Task 1: Integrate the demosaic benchmark

**Files:**

- Create: `crates/fornax-postprocessor/benches/demosaic.rs`
- Create: `crates/fornax-postprocessor/benches/algorithms.json`
- Create: `crates/fornax-postprocessor/benches/support/demosaic_registry.rs`
- Create: `crates/fornax-postprocessor/tests/bench_demosaic_registry.rs`
- Modify: `crates/fornax-postprocessor/Cargo.toml`
- Modify: `crates/fornax-postprocessor/README.md`

**Interfaces:**

- Consumes: `fornax_postprocessor::Demosaic`, `fornax_postprocessor::protocol::{IDemosaic, LazyDemosaic}`, `fornax_core::{BayerPattern, MonoImage, Primitive}`, and `fornax_devtool::{BenchParams, BenchSampleBits, synthetic_mono_image}`.
- Produces: a `demosaic` Criterion target in `fornax-postprocessor`; benchmark-local `algorithm_ids() -> &'static [String]`, `demosaic(&str) -> Demosaic`, and `selected_algorithm() -> Option<String>`.

- [x] **Step 1: Write the failing registry integration test**

Create `crates/fornax-postprocessor/tests/bench_demosaic_registry.rs` with:

```rust
#[path = "../benches/support/demosaic_registry.rs"]
mod demosaic_registry;
```

- [x] **Step 2: Run the test to verify it fails**

Run: `cargo test -p fornax-postprocessor --test bench_demosaic_registry`

Expected: compilation fails because `benches/support/demosaic_registry.rs` does not exist.

- [x] **Step 3: Move the benchmark implementation and registry support**

Move `crates/bench-demosaic/benches/demosaic.rs` to
`crates/fornax-postprocessor/benches/demosaic.rs`. Move
`crates/bench-demosaic/algorithms.json` beside it. Move
`crates/bench-demosaic/src/lib.rs` to
`crates/fornax-postprocessor/benches/support/demosaic_registry.rs`, change its
registry include to `include_str!("../algorithms.json")`, keep its unit tests,
and keep helper functions crate-visible for the benchmark target.

- [x] **Step 4: Wire the benchmark target and dependencies**

Add:

```toml
[[bench]]
harness = false
name = "demosaic"
```

Add `criterion`, `fornax-devtool`, and `serde_json` to the postprocessor's
`[dev-dependencies]`. In `benches/demosaic.rs`, load the support module with
`#[path = "support/demosaic_registry.rs"] mod demosaic_registry;` and import
the same three helpers from it instead of `bench_demosaic`.

- [x] **Step 5: Move the benchmark documentation into the crate README**

Add a “Demosaic benchmarks” section preserving the old benchmark description,
environment variables, default sizes, bit selection, algorithm selection,
thread sweep behavior, and Criterion argument note. Replace every
`cargo bench -p bench-demosaic` command with
`cargo bench -p fornax-postprocessor`. Point the algorithm registry path to
`crates/fornax-postprocessor/benches/algorithms.json` and the mapping to
`crates/fornax-postprocessor/benches/support/demosaic_registry.rs`.

- [x] **Step 6: Run the focused registry and package tests**

Run: `cargo test -p fornax-postprocessor --test bench_demosaic_registry`

Run: `cargo test -p fornax-postprocessor`

Expected: both commands pass.

- [x] **Step 7: Run the demosaic benchmark without executing timings**

Run: `cargo bench -p fornax-postprocessor --bench demosaic --no-run`

Expected: exit 0 and a compiled `demosaic` benchmark target.

- [x] **Step 8: Commit the demosaic integration**

```bash
git add crates/fornax-postprocessor
git commit -m "refactor: integrate demosaic benchmark"
```

### Task 2: Integrate the preprocessor benchmark

**Files:**

- Create: `crates/fornax-preprocessor/benches/preprocessor.rs`
- Modify: `crates/fornax-preprocessor/Cargo.toml`
- Modify: `crates/fornax-preprocessor/README.md`

**Interfaces:**

- Consumes: `fornax_preprocessor::Preprocessor`, `fornax_core::protocol::IPreprocessor`, and `fornax_devtool::{BenchParams, BenchSampleBits, synthetic_mono_image}`.
- Produces: a `preprocessor` Criterion target in `fornax-preprocessor`.

- [x] **Step 1: Move the benchmark implementation**

Move `crates/bench-preprocessor/benches/preprocessor.rs` unchanged to
`crates/fornax-preprocessor/benches/preprocessor.rs`.

- [x] **Step 2: Wire the benchmark target and dependencies**

Add:

```toml
[[bench]]
harness = false
name = "preprocessor"
```

Add `criterion` and `fornax-devtool` to `[dev-dependencies]`.

- [x] **Step 3: Move the benchmark documentation into the crate README**

Add a “Preprocessor benchmarks” section preserving the benchmark description,
environment variables, default sizes, bit selection, single-threaded note,
case naming, and Criterion argument note. Replace every
`cargo bench -p bench-preprocessor` command with
`cargo bench -p fornax-preprocessor`.

- [x] **Step 4: Run focused tests and compile the benchmark**

Run: `cargo test -p fornax-preprocessor`

Run: `cargo bench -p fornax-preprocessor --bench preprocessor --no-run`

Expected: both commands pass.

- [x] **Step 5: Commit the preprocessor integration**

```bash
git add crates/fornax-preprocessor
git commit -m "refactor: integrate preprocessor benchmark"
```

### Task 3: Remove standalone crates and update integration points

**Files:**

- Delete: `crates/bench-demosaic/Cargo.toml`
- Delete: `crates/bench-demosaic/README.md`
- Delete: `crates/bench-demosaic/algorithms.json`
- Delete: `crates/bench-demosaic/benches/demosaic.rs`
- Delete: `crates/bench-demosaic/src/lib.rs`
- Delete: `crates/bench-preprocessor/Cargo.toml`
- Delete: `crates/bench-preprocessor/README.md`
- Delete: `crates/bench-preprocessor/benches/preprocessor.rs`
- Modify: `.github/workflows/ci.yml`
- Modify: `docs/architecture.md`
- Modify: `Cargo.lock`

**Interfaces:**

- Consumes: the integrated benchmark targets from Tasks 1 and 2.
- Produces: workspace, CI, lockfile, and architecture references that use only `fornax-postprocessor` and `fornax-preprocessor`.

- [x] **Step 1: Delete the standalone benchmark crates**

Remove the files listed above and their now-empty parent directories.

- [x] **Step 2: Update the CI benchmark matrix**

Change the demosaic package to `fornax-postprocessor` and keep its watched
paths limited to `Cargo.toml`, `Cargo.lock`, the postprocessor crate,
`fornax-core`, and `fornax-devtool`. Change the preprocessor package to
`fornax-preprocessor` with the equivalent preprocessor paths.

- [x] **Step 3: Update architecture ownership**

State that decoder benchmarks remain in `bench-decoder`, while demosaic and
preprocessor benchmarks live in their production-stage crates. Point the
pipeline implementation references at the integrated benchmark directories.

- [x] **Step 4: Verify package removal and stale references**

Run: `cargo metadata --no-deps --format-version 1`

Expected: exit 0 and no `bench-demosaic` or `bench-preprocessor` package.

Run: `git grep -n -E 'bench-demosaic|bench-preprocessor'`

Expected: only this plan may mention the removed crate names; no active
workspace, CI, README, or architecture reference remains.

- [x] **Step 5: Commit the workspace cleanup**

```bash
git add Cargo.lock .github/workflows/ci.yml docs/architecture.md \
  crates/bench-demosaic crates/bench-preprocessor
git commit -m "chore: remove standalone benchmark crates"
```

### Task 4: Run full validation and open the pull request

**Files:**

- Modify: `docs/plans/chore-merge-bench-crates.md`

**Interfaces:**

- Consumes: all completed consolidation changes.
- Produces: recorded validation evidence for the pull request.

- [x] **Step 1: Run the required Rust validation**

Run: `cargo test --workspace --all-features`

Run: `cargo fmt --check`

Run: `cargo clippy --workspace --all-features --all-targets -- -D warnings`

- [x] **Step 2: Run task-specific benchmark compilation and documentation checks**

Run: `cargo bench -p fornax-postprocessor --bench demosaic --no-run`

Run: `cargo bench -p fornax-preprocessor --bench preprocessor --no-run`

Run: `git diff --check`

Run the repository-configured Markdown lint on changed Markdown files.

- [x] **Step 3: Record outcomes in this plan**

Mark completed checklist items, add the exact commands and pass/fail outcomes
under “Validation,” and note any pre-existing failures or skipped checks.

- [x] **Step 4: Inspect and commit**

Run: `git status --short` and `git diff origin/main...HEAD`.

Expected: only planned files changed, `TODO.md` is untouched, and both removed
benchmark packages are absent.

```bash
git add docs/plans/chore-merge-bench-crates.md
git commit -m "docs: record benchmark consolidation validation"
```

- [x] **Step 5: Push and open one pull request**

```bash
git push -u origin chore/merge-bench-crates
gh pr create --base main --title "refactor: merge benchmark crates into stage crates" \
  --body "Consolidates bench-demosaic into fornax-postprocessor and bench-preprocessor into fornax-preprocessor, removes the standalone crates, and updates CI and architecture documentation."
```

## Validation

The repository setup script materialized the pinned dev environment, LibRaw,
and darktable references. Validation used `LIBRAW_ROOT` and `LD_LIBRARY_PATH`
from that worktree-local Pixi environment.

```bash
cargo test -p fornax-postprocessor --test bench_demosaic_registry
cargo test -p fornax-postprocessor
cargo test -p fornax-preprocessor
cargo bench -p fornax-postprocessor --bench demosaic --no-run
cargo bench -p fornax-preprocessor --bench preprocessor --no-run
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
git diff --check
```

- `cargo test -p fornax-postprocessor --test bench_demosaic_registry`: PASS,
  2 tests.
- `cargo test -p fornax-postprocessor`: PASS, 32 tests.
- `cargo test -p fornax-preprocessor`: PASS, 4 tests.
- `cargo test --workspace --all-features`: PASS, all unit, integration, and
  doc tests.
- `cargo fmt --check`: FAIL with the default stable toolchain because the
  repository enables nightly-only rustfmt options and reports workspace-wide
  baseline formatting differences. `cargo +nightly fmt --check`: PASS.
- `cargo clippy --workspace --all-features --all-targets -- -D warnings`:
  FAIL on the pre-existing `clippy::too_many_arguments` finding in
  `crates/fornax-postprocessor/src/demosaic/ahd.rs`.
  `cargo clippy -p fornax-postprocessor -p fornax-preprocessor --all-features
  --all-targets -- -D warnings -A clippy::too_many_arguments`: PASS with no
  issues in the changed packages.
- `cargo bench -p fornax-postprocessor --bench demosaic --no-run`: PASS.
- `cargo bench -p fornax-preprocessor --bench preprocessor --no-run`: PASS.
- `git diff --check`: PASS.
- `pixi exec markdownlint-cli2 ...`: PASS, 4 files with 0 issues.
- `cargo metadata --no-deps --format-version 1`: PASS; neither removed
  benchmark package is present.

## Decisions and notes

- The demosaic helpers are benchmark-local and are compiled into an
  integration test through the same support module; they are not part of the
  production postprocessor API.
- The registry assertions live in the integration-test wrapper because
  Criterion compiles `cfg(test)` code without running its harness, which would
  otherwise produce warnings in benchmark builds.
- The existing preprocessor manifest and benchmark source used CRLF line
  endings. The moved manifest and benchmark are normalized to LF.
- The decoder benchmark remains a standalone crate because the request covers
  only the demosaic and preprocessing benchmarks.
- The CI matrix continues to benchmark a whole production-stage package, but
  changed-path filters remain stage-specific so unrelated demosaic work does
  not trigger preprocessing benchmarks.
- Historical plans may still mention the removed benchmark crate names as a
  record of completed work; active workspace, CI, and current documentation
  references are removed.
- `TODO.md` is untouched because this is an explicit maintenance request, not
  a selected bracketed backlog item.
