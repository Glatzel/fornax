# Optional TIFF Raster Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use superpowers:subagent-driven-development (recommended) or superpowers:executing-plans to implement this plan task-by-task. Steps use checkbox (`- [ ]`) syntax for tracking.

**Goal:** Let `fornax-tiff` provide parser/writer-only functionality without pulling the codec, image, or Rayon stack.

**Architecture:** Make `fornax-codec` optional and place `raster`, all raster re-exports, and `resolve_tiff_compression` behind a new `raster` feature. `fornax-dng` enables that feature because its decoder consumes raster requests; TIFF-only consumers use the empty default feature set.

**Tech Stack:** Rust 2024, Cargo optional dependencies and features.

**Spec:** `docs/plans/workspace-refactor/phase-b-c-handoff.md`

## Global Constraints

- Use branch `chore/tiff-optional-raster` and worktree
  `.worktrees/chore-tiff-optional-raster`.
- Use one worktree and one pull request.
- Keep the branch plan at `docs/plans/chore-tiff-optional-raster.md`.
- Keep the default feature set empty.
- Do not move raw workflow types into `fornax-tiff`.
- Do not change codec behavior or decoder output.
- Do not compile `compare-demosaic`.
- Do not edit golden files.

## Review Focus

- A TIFF-only consumer must not see `fornax-codec`, `image`, or `rayon` in the depth-one normal tree.
- Enabling only `raster` must restore the complete existing raster API.
- `fornax-dng` must continue to compile against the raster API.
- Raster doctests must not compile in a default-feature build.
- Unknown compression behavior must remain unchanged.

---

### Task 1: Gate the raster module and dependency

**Files:**
- Modify: `crates/fornax-tiff/Cargo.toml`
- Modify: `crates/fornax-tiff/src/lib.rs`
- Modify: `crates/fornax-dng/Cargo.toml`
- Modify: `crates/fornax-tiff/tests/raster.rs`
- Modify: `crates/fornax-tiff/README.md`

**Interfaces:**
- Consumes: the current `fornax_codec::DecodeCompression`-based raster API.
- Produces: Cargo feature `raster`; no API name changes when the feature is enabled.

- [ ] **Step 1: Make the raster integration test feature-bound**

Add this first line to `crates/fornax-tiff/tests/raster.rs`:

```rust
#![cfg(feature = "raster")]
```

- [ ] **Step 2: Make the TIFF codec dependency optional**

In `crates/fornax-tiff/Cargo.toml`, retain the existing codec features but make
the dependency optional:

```toml
[dependencies]
fornax-codec = { workspace = true, optional = true, features = ["deflate", "lossless-jpeg"] }
num_enum.workspace = true
strum.workspace = true

[features]
default = []
raster = ["dep:fornax-codec"]
```

- [ ] **Step 3: Gate raster declarations and re-exports**

In `crates/fornax-tiff/src/lib.rs`, apply `#[cfg(feature = "raster")]` to
`mod raster;` and to the complete
`pub use raster::{RasterError, RasterLayout, RasterRequest, RasterStorage, resolve_tiff_compression};`
re-export.

- [ ] **Step 4: Enable raster for DNG**

Change the DNG TIFF dependency to:

```toml
fornax-tiff = { workspace = true, features = ["raster"] }
```

No other format crate consumes the raster API; verify this with
`rg -n 'Raster(Request|Layout|Storage)|resolve_tiff_compression' crates
--glob '!fornax-tiff/**'`.

- [ ] **Step 5: Document the feature boundary**

Add a short `Raster feature` section to `crates/fornax-tiff/README.md`. State
that parser/writer-only consumers use default features and that `raster`
enables raster layout plus codec dispatch.

- [ ] **Step 6: Run focused feature tests**

```bash
cargo test -p fornax-tiff --no-default-features
cargo test -p fornax-tiff --all-features
cargo test -p fornax-dng
```

Expected: all commands exit 0; the default build has no raster tests.

- [ ] **Step 7: Commit the feature split**

```bash
git add crates/fornax-tiff crates/fornax-dng/Cargo.toml
git commit -m "refactor: make tiff raster support optional"
```

### Task 2: Prove dependency isolation and full-workspace behavior

**Files:**
- No source changes expected.

**Interfaces:**
- Consumes: the feature split from Task 1.
- Produces: validation evidence for the PR.

- [ ] **Step 1: Check default and raster dependency trees**

```bash
cargo tree -p fornax-tiff -e normal --depth 1
cargo tree -p fornax-tiff -e normal --depth 1 --features raster
cargo tree -p fornax-codec -e normal --depth 1
```

Expected:

- default TIFF tree contains `num_enum` and `strum`, with no
  `fornax-codec`, `image`, or `rayon`;
- raster tree contains `fornax-codec`;
- codec tree remains free of raw workflow crates.

- [ ] **Step 2: Run repository validation**

```bash
scripts/agent-validation.sh
```

Expected: exit 0. `compare-demosaic` must not be compiled.

- [ ] **Step 3: Verify decoder golden hashes**

```bash
cargo run -q -p decoder-tracker -- hash \
  --manifest crates/decoder-tracker/golden/selected-manifest.csv \
  --check crates/decoder-tracker/golden/decode-hashes.csv
```

Expected: `golden: unchanged (9 rows)`.

- [ ] **Step 4: Inspect and commit validation evidence**

Run `git diff --check`, update the branch plan, and commit only the plan's
recording changes.

```bash
git add docs/plans/chore-tiff-optional-raster.md
git commit -m "docs: record optional tiff raster validation"
```
