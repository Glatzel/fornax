# DHT Faithful Port Implementation Plan

This archived plan documents the completed DHT Faithful Port work.

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Fornax's generic DHT profile with a Rust port of LibRaw
0.22.3's `dht_interpolate()` path and make the native GBRG comparison pass
every repository limit.

**Architecture:** Keep `Demosaic::Dht` as a `DenseLazy` backend. Convert the
mosaic to 16-bit codes, run a padded full-frame DHT state machine with
`[f32; 3]` samples and one direction byte per sample, then pack through the
existing lazy boundary.

**Tech Stack:** Rust 2024, `fornax-raw-core`, `fornax-raw-pipeline`,
`compare-demosaic`, LibRaw 0.22.3, Insta, and Cargo.

**Spec:**
[`docs/_archive/designs/dht-port-design.md`](../designs/dht-port-design.md)

## Goal

Implement `[DHT]` from `TODO.md`: match LibRaw 0.22.3 `dht_interpolate()` and
make the native GBRG comparison have MAE at most 2, P99 at most 64, and at most
0.1% of samples above 1024 codes.

## Scope

**In scope:** padded DHT state, measured-sample loading, hot/dead detection,
horizontal/vertical direction discovery and refinement, green interpolation,
diagonal direction discovery and refinement, diagonal red/blue reconstruction,
horizontal/vertical red/blue reconstruction, hot restoration, truncating
output, focused stage tests, all-pattern coverage, comparison snapshots and
baseline, README notes, reference notes, and removal of completed `[DHT]` from
`TODO.md`.

**Out of scope:** LibRaw metadata, white balance, output color, callbacks,
shrink, half-size, four-color RGB, X-Trans, or non-Bayer input.

## Global Constraints

- Reference is LibRaw 0.22.3 at commit
  `cf02d66eb5c19d4e46df7aee0582e47a4918509a`.
- The reference call is `pre_interpolate()` followed by `dht_interpolate()`.
- The comparison bridge uses an identity camera matrix and no white balance.
- Support only RGGB, BGGR, GRBG, and GBRG 2x2 Bayer patterns.
- Preserve the 4-pixel top/left margin, `0.5` neutral samples, thresholds
  `64.0`, `256.0`, and `1.4`, direction flag bits, channel bounds, pass order,
  and truncating `ushort` output.
- Never loosen `meta.limits`, waive a failing row, edit generated README
  regions, or hand-edit the baseline.
- Preserve the LibRaw DHT copyright and dual-license attribution.

## Review Focus

- A 3x3 mosaic must either produce deterministic output or return the
  documented error without reading outside the padded plane.
- Every CFA phase must use the correct parity for non-green rows and columns.
- Strong horizontal/vertical and diagonal flags must remain distinguishable
  from ordinary direction flags.
- Hot-pixel candidates may be replaced during interpolation but their measured
  channel must be restored before output.
- Float-to-`u16` output must truncate toward zero and remain bounded.

---

### Task 1: Padded state, bounds, and hot handling

**Files:**

- Modify: `crates/fornax-raw-pipeline/src/demosaic/dht.rs`
- Test: same file

**Interfaces:**

- Consumes: `buffer::{DenseLazy, codes, dense_from_codes}` and
  `BayerPattern::channel_at`.
- Produces:
  - `struct Dht`
  - `fn cfa_channel(pattern: BayerPattern, x: isize, y: isize) -> usize`
  - `fn calc_dist(c1: f32, c2: f32) -> f32`
  - `Dht::new(input: &[u16], width: usize, height: usize, pattern: BayerPattern) -> Self`
  - `fn hide_hots(&mut self)`
  - `fn restore_hots(&mut self, input: &[u16])`

- [x] **Step 1: Write the failing tests**

Add tests named
`calc_dist_is_invariant_to_one_stop_scaling`,
`new_preserves_measured_samples_in_a_padded_neutral_plane`, and
`hide_hots_marks_and_repairs_an_isolated_sample`.

- [x] **Step 2: Run the tests and verify RED**

Run: `cargo test -p fornax-raw-pipeline dht::tests -- --nocapture`

Expected: compilation fails because `Dht` and its helpers do not exist.

- [x] **Step 3: Implement the state and hot stage**

Initialize all padded samples to `[[0.5; 3]; area]`, copy measured samples,
retain `channel_maximum` and `channel_minimum` using LibRaw's zero-initialized
unmeasured channels plus the `0.5` lower delta, then port `hide_hots` with the
source's eight-neighbor, one-stop, and axis-choice comparisons.

- [x] **Step 4: Run the tests and verify GREEN**

Run: `cargo test -p fornax-raw-pipeline dht::tests -- --nocapture`

Expected: the new focused tests pass.

### Task 2: Direction discovery and refinement

**Files:**

- Modify: `crates/fornax-raw-pipeline/src/demosaic/dht.rs`
- Test: same file

**Interfaces:**

- Consumes: `Dht` state from Task 1.
- Produces:
  - `fn make_hv_dirs(&mut self)`
  - `fn make_diag_dirs(&mut self)`
  - `fn get_hv_grb(&self, x: usize, y: usize, kc: usize) -> u8`
  - `fn get_hv_rbg(&self, x: usize, y: usize, hc: usize) -> u8`
  - `fn get_diag_grb(&self, x: usize, y: usize, kc: usize) -> u8`
  - `fn get_diag_rbg(&self, x: usize, y: usize) -> u8`

- [x] **Step 1: Write the failing tests**

Add tests named
`hv_direction_marks_strong_and_regular_horizontal_edges`,
`diag_direction_marks_strong_and_regular_diagonal_edges`, and
`hv_refinement_flips_an_unsupported_direction`.

- [x] **Step 2: Run the tests and verify RED**

Run: `cargo test -p fornax-raw-pipeline dht::tests -- --nocapture`

Expected: compilation fails because direction stages and flags do not exist.

- [x] **Step 3: Implement direction calculation and refinement**

Port `get_hv_grb`, `get_hv_rbg`, `get_diag_grb`, `get_diag_rbg`,
`make_hv_dirs`, and `make_diag_dirs`, preserving the `1.4` diagonal threshold,
the three repeated squarings of HV confidence, row-parity refinement order, and
the column-wide refinement.

- [x] **Step 4: Run the tests and verify GREEN**

Run: `cargo test -p fornax-raw-pipeline dht::tests -- --nocapture`

Expected: all focused tests pass.

### Task 3: Green and red/blue reconstruction

**Files:**

- Modify: `crates/fornax-raw-pipeline/src/demosaic/dht.rs`
- Test: same file

**Interfaces:**

- Consumes: directions and bounded state from Tasks 1-2.
- Produces:
  - `fn make_greens(&mut self)`
  - `fn make_rb(&mut self)`
  - `fn scale_over(ec: f32, base: f32) -> f32`
  - `fn scale_under(ec: f32, base: f32) -> f32`
  - `fn process(&mut self, input: &[u16]) -> Vec<u16>`

- [x] **Step 1: Write the failing tests**

Add tests named
`make_greens_uses_the_selected_axis_and_channel_bounds`,
`make_rb_reconstructs_diagonal_and_hv_missing_samples`, and
`process_restores_hot_samples_and_truncates_output`.

- [x] **Step 2: Run the tests and verify RED**

Run: `cargo test -p fornax-raw-pipeline dht::tests -- --nocapture`

Expected: compilation fails because reconstruction and processing do not exist.

- [x] **Step 3: Implement the pass sequence**

Port `make_gline`, `make_rbdiag`, `make_rbhv`, `scale_over`, `scale_under`,
`restore_hots`, and the final float-to-`u16` packing in the reference order:
hot handling, HV directions, greens, diagonal directions, red/blue, hot
restoration, copy.

- [x] **Step 4: Wire the public backend**

Replace `directional::lazy(...)` in `DemosaicDht::lazy` with the dedicated
3x3-validated pipeline using `codes` and `dense_from_codes`.

- [x] **Step 5: Run focused and crate tests**

Run: `cargo test -p fornax-raw-pipeline dht`
Run: `cargo test -p fornax-raw-pipeline`

Expected: focused tests and all postprocessor tests pass.

### Task 4: Comparison, documentation, and backlog

**Files:**

- Modify: `crates/compare-demosaic/src/snapshots/*-DHT-libraw.snap`
- Modify: `crates/compare-demosaic/README.md`
- Modify: `crates/compare-demosaic/baseline/canonical.json`
- Create: `docs/reference/algorithms/dht.md`
- Modify: `TODO.md`

**Interfaces:**

- Consumes: the production DHT backend from Task 3.
- Produces: accepted canonical DHT rows and complete reference documentation.

- [x] **Step 1: Run the native diagnostic row**

Run:
`cargo run --release -p compare-demosaic -- run --algorithm dht --pattern GBRG --artifacts --out target/dht`

Expected: MAE at most 2, P99 at most 64, and at most 0.1% above 1024 codes.
Investigate the earliest failing stage instead of changing limits if it fails.

- [x] **Step 2: Run all DHT rows and update reviewed snapshots**

Run: `cargo run --release -p compare-demosaic -- run --algorithm dht`
Run: `cargo insta test -p compare-demosaic`
Run: `cargo insta accept`

Expected: all four LibRaw DHT rows pass; only DHT snapshots change.

- [x] **Step 3: Refresh the full baseline and generated README**

Run: `cargo run --release -p compare-demosaic -- run`
Copy `target/compare-demosaic/canonical.json` to
`crates/compare-demosaic/baseline/canonical.json` only after reviewing the full
diff.
Run: `cargo run --release -p compare-demosaic -- readme`
Run: `cargo run --release -p compare-demosaic -- readme --check`
Run: `cargo run --release -p compare-demosaic -- diff crates/compare-demosaic/baseline/canonical.json target/compare-demosaic/canonical.json`

Expected: full run differs only in reviewed DHT rows; diff and README check
pass.

- [x] **Step 4: Write the reference and remove the completed backlog item**

Document the pinned source, algorithm stages, numeric invariants, stage
boundary, and dual-license notice in `docs/reference/algorithms/dht.md`. Rewrite the DHT
README note to describe the faithful match and known comparison boundary.
Remove `[DHT]` from `TODO.md`.

- [x] **Step 5: Run repository validation**

Run:

```bash
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-features --all-targets -- \
    -A clippy::too_many_arguments \
    -D clippy::alloc_instead_of_core \
    -D clippy::allow_attributes_without_reason \
    -D warnings
pixi exec -s markdownlint-cli2 markdownlint-cli2 \
    crates/compare-demosaic/README.md docs/reference/algorithms/dht.md \
    docs/_archive/plans/dht-port.md \
    docs/_archive/designs/dht-port-design.md
git diff --check
```

Expected: all commands pass, except any pre-existing unrelated failures must
be reported exactly rather than described as passing.

## Decisions and Notes

- The user selected implementation of `[DHT]`; the faithful Rust port follows
  the existing dedicated DCB, AAHD, VNG, and LMMSE backend pattern.
- The production path remains independent of LibRaw.
- The native GBRG row is the independent acceptance test.
- Numeric discrepancies are localized to stage fixtures before comparison
  tuning.
- The implementation accepts the comparison bridge's 3x3 minimum; the initial
  direction helper exposed and fixed negative padded-coordinate handling.
- The native GBRG row matches LibRaw exactly: MAE `0`, P99 `0`, maximum `0`,
  and zero samples above 1024 codes.
- All four DHT rows match LibRaw exactly. Final review found and fixed the
  hot-pixel average to include all eight same-channel neighbors as LibRaw does.
- The full baseline changed only the four DHT rows. Reviewed DHT snapshots,
  generated README, and the baseline diff were accepted.
- `cargo test --workspace --all-features` passed 264 tests across 23 suites;
  `cargo +nightly fmt --check`, Markdown lint, and `git diff --check` passed.
- The exact repository Clippy command reports pre-existing failures in the
  preprocessor benchmark, Rawler test helper, `linear.rs`, and `lmmse.rs`; no
  DHT diagnostics remain.
