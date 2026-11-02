# Move Demosaic Protocol Traits Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Move `IDemosaic` and `LazyDemosaic` from `fornax-core` to `fornax-postprocessor`, while keeping `DemosaicError` in `fornax-core` to avoid a dependency cycle.

**Architecture:** The postprocessor will own a `protocol` module containing the demosaic traits. `fornax-postprocessor` already depends on `fornax-core`, so the traits can continue returning `fornax_core::error::DemosaicError`. The facade will re-export the new postprocessor protocol module, and all algorithm, benchmark, comparison, and documentation imports will use the new path.

**Tech Stack:** Rust 2024, Cargo workspace, `rayon`, `fornax-core`, `fornax-postprocessor`.

**Spec:** The user requested moving the three demosaic protocol types, then selected option 1 after the dependency-cycle check: move only `IDemosaic` and `LazyDemosaic`, leaving `DemosaicError` in `fornax-core`.

## Scope

In scope: move `IDemosaic` and `LazyDemosaic`, their implementations and
rustdoc, the shared row-collection test coverage, all direct imports and
re-exports, the facade path, and ownership documentation.

Out of scope: moving `DemosaicError`, changing demosaic algorithms or CLI
behavior, adding compatibility re-exports at the old trait paths, changing
`TODO.md`, and modifying the comparison baseline.

## Global Constraints

- Keep decoding, preprocessing, and demosaicing as separate stages.
- Do not change behavior or public type and method signatures beyond module paths.
- Do not add a `fornax-core -> fornax-postprocessor` dependency.
- Keep `DemosaicError` at `fornax_core::error::DemosaicError`.
- Do not leave compatibility re-exports at the old trait paths.
- Do not change `TODO.md`.
- Run the required workspace checks and the comparison baseline command from the repository root.

## Review Focus

- A caller importing `IDemosaic` or `LazyDemosaic` from the old core path must fail to compile rather than silently use a stale duplicate.
- A caller using the new postprocessor protocol path must retain the row collection and demosaicing behavior.
- A dependent crate that misses an import update must fail compilation.
- The facade must expose the new protocol module without taking ownership of `DemosaicError`.
- The comparison baseline must remain byte-for-byte unchanged.

---

### Task 1: Lock the new public protocol path

**Files:**
- Create: `crates/fornax-postprocessor/tests/protocol.rs`

**Interfaces:**
- Consumes: `fornax_core::{BayerPattern, MonoImage, Primitive}`, `fornax_core::error::DemosaicError`
- Produces: a regression test importing `IDemosaic` and `LazyDemosaic` from `fornax_postprocessor::protocol`

- [x] **Step 1: Write the failing public-path test**

```rust
use fornax_core::error::DemosaicError;
use fornax_core::{BayerPattern, MonoImage, Primitive};
use fornax_postprocessor::protocol::{IDemosaic, LazyDemosaic};

struct IdentityLazy;

impl<T: Primitive> LazyDemosaic<T> for IdentityLazy {
    fn dimensions(&self) -> (u32, u32) { (1, 1) }

    fn fill_rows(&self, _y0: usize, out: &mut [T]) {
        out.fill(T::one());
    }
}

struct IdentityDemosaic;

impl<T: Primitive> IDemosaic<T> for IdentityDemosaic {
    fn demosaic<'a>(
        &self,
        _bayer: &'a MonoImage<T>,
        _pattern: BayerPattern,
    ) -> Result<impl LazyDemosaic<T> + 'a, DemosaicError> {
        Ok(IdentityLazy)
    }
}

#[test]
fn protocol_paths_expose_lazy_and_demosaic_traits() -> mischief::Result<()> {
    let mosaic = MonoImage::from_raw(1, 1, vec![0.5_f32]).expect("valid 1x1 mosaic");
    let rgb = IdentityDemosaic
        .demosaic(&mosaic, BayerPattern::RGGB)?
        .collect();

    assert_eq!(rgb.as_raw(), &[1.0, 1.0, 1.0]);
    Ok(())
}
```

- [x] **Step 2: Run the test to verify it fails**

Run: `cargo test -p fornax-postprocessor --test protocol`

Expected: compilation fails because `fornax_postprocessor::protocol` does not exist.

### Task 2: Move the traits and update all consumers

**Files:**
- Create: `crates/fornax-postprocessor/src/protocol.rs`
- Modify: `crates/fornax-core/Cargo.toml`
- Modify: `crates/fornax-core/src/protocol.rs`
- Modify: `crates/fornax-postprocessor/Cargo.toml`
- Modify: `crates/fornax-postprocessor/src/lib.rs`
- Modify: `crates/fornax-postprocessor/src/demosaic.rs`
- Modify: `crates/fornax-postprocessor/src/demosaic/{ahd,buffer,linear,ppg,rcd}.rs`
- Modify: `crates/bench-demosaic/benches/demosaic.rs`
- Modify: `crates/compare-demosaic/src/cases.rs`
- Modify: `crates/fornax/src/postprocessor.rs`
- Modify: `crates/fornax-core/README.md`
- Modify: `crates/fornax-postprocessor/README.md`
- Modify: `docs/architecture.md`

**Interfaces:**
- Consumes: the failing test and existing `DemosaicError`
- Produces:
  - `fornax_postprocessor::protocol::LazyDemosaic<T>`
  - `fornax_postprocessor::protocol::IDemosaic<T>`
  - `fornax::postprocessor::protocol::{IDemosaic, LazyDemosaic}`

- [x] **Step 1: Move the trait definitions and implementation unchanged into `crates/fornax-postprocessor/src/protocol.rs`**

Use `rayon`, `fornax_core::error::DemosaicError`, and `fornax_core::{BayerPattern, MonoImage, Primitive, RgbImage}`. Preserve `dimensions`, `fill_rows`, `collect_with`, `collect`, and `demosaic` signatures and bodies.

- [x] **Step 2: Remove only the moved traits from `crates/fornax-core/src/protocol.rs`**

Keep `IDecoder`, `IPreprocessor`, `IPostProcessMetadata`, and `IPostProcessor` in core. Remove imports that become unused, but leave `DemosaicError` in `crates/fornax-core/src/error.rs`.

- [x] **Step 3: Update postprocessor imports to `crate::protocol`**

Add `pub mod protocol;` to the crate root. Keep `DemosaicError` imports from `fornax_core::error`.

- [x] **Step 4: Update benchmarks, comparison code, and the facade path**

Use `fornax_postprocessor::protocol::{IDemosaic, LazyDemosaic}` in dependent crates. Re-export `fornax_postprocessor::protocol` from `crates/fornax/src/postprocessor.rs`.

- [x] **Step 5: Update ownership documentation**

State that core retains the cross-stage traits and `DemosaicError`, while postprocessor owns the demosaic protocol traits. Update architecture paths and crate-map descriptions.

- [x] **Step 6: Run the focused test and package tests**

Run: `cargo test -p fornax-postprocessor --test protocol`

Expected: PASS.

Run: `cargo test -p fornax-postprocessor`

Expected: PASS with all postprocessor unit and integration tests green.

- [x] **Step 7: Confirm the old paths are absent**

Run: `rg -n 'fornax_core::protocol::(IDemosaic|LazyDemosaic)|fornax::protocol::(IDemosaic|LazyDemosaic)' crates docs`

Expected: no matches.

### Task 3: Validate the workspace and comparison behavior

**Files:**
- No source-file changes expected.

**Interfaces:**
- Consumes: the completed move
- Produces: recorded validation evidence and PR-ready commit

- [x] **Step 1: Run workspace tests**

Run: `cargo test --workspace --all-features`

Expected: exit 0.

- [x] **Step 2: Check formatting**

Run: `cargo +nightly fmt --check`

Expected: exit 0.

- [x] **Step 3: Run clippy**

Run: `cargo clippy --workspace --all-features --all-targets -- -D warnings`

Expected: exit 0.

- [x] **Step 4: Run the comparison baseline**

Run: `cargo run --release -p compare-demosaic -- run --check`

Expected: exit 0 with no changed rows.

- [x] **Step 5: Run documentation checks for moved public traits**

Run: `cargo test --doc -p fornax-postprocessor`

Expected: exit 0.

Run: `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p fornax-postprocessor -p fornax-core -p fornax`

Expected: exit 0 with no broken intra-doc links.

- [x] **Step 6: Inspect, commit, push, and open the PR**

Run: `git diff --check` and inspect `git diff`.

Expected: only the planned files change and `TODO.md` is untouched.

Commit with `chore: move demosaic protocol traits to postprocessor`, push the branch, and open one PR to `main` summarizing option 1, validation results, and the absence of old compatibility re-exports.

## Validation

- `cargo test --workspace --all-features`, with `LIBRAW_ROOT` and
  `LD_LIBRARY_PATH` pointing at the pinned Pixi environment: PASS, 166 tests
  across 24 suites.
- `cargo +nightly fmt --check`: PASS. The literal `cargo fmt --check` is not
  usable in this shell because the default stable rustfmt ignores the
  repository's nightly-only settings and reports workspace-wide false
  differences.
- `cargo clippy --workspace --all-features --all-targets -- -D warnings`:
  FAIL on pre-existing `clippy::too_many_arguments` findings in
  `fornax-postprocessor/src/demosaic/ahd.rs` and
  `libraw/src/c_api/data_loading.rs`. The same CI-matching
  `cargo +nightly clippy` command reports the same two findings.
- The same nightly clippy command with
  `-A clippy::too_many_arguments`: PASS.
- `cargo run --release -p compare-demosaic -- run --check`: FAIL before
  execution because this revision's `run` subcommand has no `--check`
  argument.
- `cargo run --release -p compare-demosaic -- run`, followed by `diff` against
  `crates/compare-demosaic/baseline/canonical.json`: PASS with no differences.
- `cargo test --doc -p fornax-postprocessor`: PASS, 2 doctests.
- `RUSTDOCFLAGS="-D warnings" cargo doc --no-deps -p fornax-postprocessor -p
  fornax-core -p fornax`: PASS with no broken intra-doc links.
- `git diff --check`: PASS.

## Decisions and notes

- `DemosaicError` remains in core because `PostProcessError::Demosaic` and core postprocessor signatures use it; moving it into postprocessor would create a core/postprocessor cycle.
- `IDemosaic::demosaic` continues returning `fornax_core::error::DemosaicError`.
- The old trait paths will not be re-exported.
- The direct `rayon` dependency moves from core to postprocessor with the trait implementation.
- The comparison baseline must not be regenerated as part of this move.
