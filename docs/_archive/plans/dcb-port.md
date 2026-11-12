# DCB Faithful Port Implementation Plan

> **For agentic workers:** REQUIRED SUB-SKILL: Use
> superpowers:subagent-driven-development (recommended) or
> superpowers:executing-plans to implement this plan task-by-task. Steps use
> checkbox (`- [ ]`) syntax for tracking.

**Goal:** Replace Fornax's generic DCB profile with an independent Rust port of
LibRaw 0.22.3's fixed `dcb(0, 1)` path and make the native GBRG comparison row
pass every repository limit.

**Architecture:** Keep `Demosaic::Dcb` as a `DenseLazy` backend. Convert the
mosaic to 16-bit codes, run a full-frame DCB state machine with `[u16; 4]`
pixels and two `[f32; 3]` candidate planes, then pack RGB through the existing
lazy boundary.

**Tech Stack:** Rust 2024, `fornax-raw-core`, `fornax-raw-pipeline`,
`compare-demosaic`, LibRaw 0.22.3, Insta, and Cargo.

**Spec:**
[`docs/superpowers/specs/2026-11-03-dcb-faithful-port-design.md`](../superpowers/specs/2026-11-03-dcb-faithful-port-design.md)

## Goal

Implement `[DCB]` from `TODO.md`: match LibRaw 0.22.3 `dcb(0, 1)` and make the
native GBRG comparison have MAE at most 2, P99 at most 64, and at most 0.1% of
samples above 1024 codes.

## Scope

**In scope:** six-pixel border interpolation, horizontal and vertical
candidates, direction selection, Nyquist correction, direction maps,
correction passes, green smoothing, refinement, full-color reconstruction,
stage-level tests, all-pattern coverage, comparison snapshots and baseline,
generated README, hand-written documentation, and removal of the completed
`[DCB]` item from `TODO.md`.

**Out of scope:** FBDD, configurable DCB iterations, disabled enhancement,
camera metadata, white balance, post-demosaic processing, X-Trans, four-color
Bayer, monochrome, non-Bayer images, and changes to comparison limits or
waivers.

## Global Constraints

- Reference source is LibRaw 0.22.3 at commit
  `cf02d66eb5c19d4e46df7aee0582e47a4918509a`,
  `src/demosaic/dcb_demosaic.cpp#L849`.
- Match the comparison adapter's `pre_interpolate()` then `dcb(0, 1)` path.
- Support only conventional 2x2 RGB Bayer patterns.
- Native GBRG acceptance requires `MAE <= 2.000000`, `P99 <= 64`, and
  `fraction above 1024 codes <= 0.001000`.
- Preserve LibRaw's six-pixel border, stage order, clipping, truncation, and
  measured-sample mutation behavior.
- Never edit generated README regions by hand, edit
  `baseline/canonical.json` by hand, loosen `meta.limits`, or waive a failing
  row.
- Retain the DCB author's BSD-3-Clause copyright and license notice.

## Review Focus

- Tiny images near the three-pixel bridge minimum must not panic or read
  outside the frame; they either produce deterministic output or return the
  documented `InvalidInput` error.
- The six-pixel border must remain valid at every corner and edge for RGGB,
  BGGR, GRBG, and GBRG.
- DCB may modify measured red or blue samples during its final neighbor limit;
  the port must preserve reference behavior instead of restoring CFA values.
- `CLIP(int(expression))` and direct `ushort` conversion must not be
  normalized, rounded, or reordered in ways that shift reference output.
- Bayer phase lookup must keep red at channel 0, unified green at channel 1,
  blue at channel 2, and the direction map at channel 3.

## Task Checklist

- [x] Add failing tests for border interpolation, directional candidates, and
      the DCB direction decision; implement those stages.
- [x] Add failing tests for Nyquist, map, correction, second correction, base
      color reconstruction, and green smoothing; implement those stages.
- [x] Add failing tests for refinement, full-color reconstruction, public API
      integration, all patterns, and measured-sample mutation; complete the
      pipeline.
- [x] Run the comparison, accept only intended DCB snapshots, refresh the
      baseline and generated README, document the reference, and remove `[DCB]`
      from `TODO.md`.
- [x] Run all workspace validation and record outcomes in the pull request.

## Files Expected to Change

- `crates/fornax-raw-pipeline/src/demosaic/dcb.rs`
- `crates/fornax-raw-pipeline/src/demosaic.rs`
- `crates/fornax-raw-pipeline/src/demosaic/directional.rs`
- `crates/compare-demosaic/src/snapshots/*-DCB-libraw.snap`
- `crates/compare-demosaic/README.md`
- `crates/compare-demosaic/baseline/canonical.json`
- `docs/references/dcb.md`
- `docs/plans/dcb-faithful-port.md`
- `TODO.md`

## Validation

```bash
cargo test -p fornax-raw-pipeline dcb
cargo run --release -p compare-demosaic -- run \
  --algorithm dcb --pattern GBRG --artifacts --out target/dcb
cargo run --release -p compare-demosaic -- run --algorithm dcb
cargo insta test -p compare-demosaic
cargo run --release -p compare-demosaic -- run
cp target/compare-demosaic/canonical.json \
  crates/compare-demosaic/baseline/canonical.json
cargo run --release -p compare-demosaic -- readme
cargo run --release -p compare-demosaic -- readme --check
cargo run --release -p compare-demosaic -- diff \
  crates/compare-demosaic/baseline/canonical.json \
  target/compare-demosaic/canonical.json
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-features --all-targets -- \
  -D warnings -A clippy::too_many_arguments
pixi exec -s markdownlint-cli2 markdownlint-cli2 \
  crates/compare-demosaic/README.md docs/references/dcb.md \
  docs/plans/dcb-faithful-port.md
git -c core.whitespace=cr-at-eol diff --check
```

## Decisions and Notes

- The user selected an independent Rust port over a production LibRaw wrapper
  and further expansion of `directional.rs`.
- The fixed reference call is `dcb(0, 1)`; iteration and denoising options are
  not exposed.
- Existing public selectors and `DenseLazy` remain unchanged.
- Stage tests use direct 16-bit codes rather than normalized floating-point
  fixtures so truncation and clipping mismatches remain visible.
- Comparison images provide whole-frame coverage; focused fixtures protect the
  stage interfaces that a single numeric acceptance row cannot diagnose.
- The focused postprocessor suite passed 49 tests across four suites.
- Native and cropped BGGR, GBRG, GRBG, and RGGB DCB comparison rows all match
  LibRaw exactly: MAE `0`, P99 `0`, maximum `0`, and zero samples above 1024.
- The full canonical run changed only the four DCB rows.
- The comparison work exposed and fixed two reference-fidelity defects: one
  far chroma weight used the wrong vertical offset, and the main sequence
  incorrectly ran the post-`dcb_correction2` map/correction stage four times
  instead of LibRaw's three.
- `dcb(0, 1)` does not execute the Nyquist helper because its iteration count
  is zero. The helper remains covered by a focused stage test.
- `cargo test --workspace --all-features` passed 203 tests across 23 suites.
- Workspace Clippy with the repository's requested lint flags passed.
- `cargo fmt --check` fails on pre-existing workspace files because stable
  rustfmt ignores the repository's nightly-only configuration. The changed
  `dcb.rs` itself was formatted by `cargo fmt --package fornax-raw-pipeline`
  and does not appear in the `cargo fmt --check` diff.
- `markdownlint-cli2` and `git diff --check` passed.

---

### Task 1: Border, candidate, and decision stages

**Files:**

- Modify: `crates/fornax-raw-pipeline/src/demosaic/dcb.rs`

**Interfaces:**

- Consumes: `buffer::{DenseLazy, codes, dense_from_codes}` and
  `BayerPattern::channel_at`.
- Produces:
  - `type DcbPixel = [u16; 4]`
  - `fn insert_measured(input: &[u16], width: usize, height: usize, pattern: BayerPattern) -> Vec<DcbPixel>`
  - `fn cfa_channel(pattern: BayerPattern, x: isize, y: isize) -> usize`
  - `fn border_interpolate(pixels: &mut [DcbPixel], width: usize, height: usize, pattern: BayerPattern, border: usize)`
  - `fn horizontal_candidates(pixels: &[DcbPixel], width: usize, height: usize, pattern: BayerPattern) -> Vec<[f32; 3]>`
  - `fn vertical_candidates(pixels: &[DcbPixel], width: usize, height: usize, pattern: BayerPattern) -> Vec<[f32; 3]>`
  - `fn direction_is_vertical(pixels: &[DcbPixel], horizontal: &[[f32; 3]], vertical: &[[f32; 3]], width: usize, height: usize, pattern: BayerPattern, index: usize) -> bool`
  - `fn apply_direction(pixels: &mut [DcbPixel], horizontal: &[[f32; 3]], vertical: &[[f32; 3]], width: usize, height: usize, pattern: BayerPattern)`

- [ ] **Step 1: Write failing tests for the front stages**

Add these test names and assertions in `dcb.rs`:

```rust
#[test]
fn border_interpolate_fills_corner_channels_from_their_cfa_neighbors() {
    let pattern = BayerPattern::RGGB;
    let mut input = vec![0_u16; 16];
    for (index, sample) in input.iter_mut().enumerate() {
        let channel = cfa_channel(pattern, (index % 4) as isize, (index / 4) as isize);
        *sample = [10, 20, 30, 40][channel] as u16;
    }
    let mut pixels = insert_measured(&input, 4, 4, pattern);
    border_interpolate(&mut pixels, 4, 4, pattern, 1);
    assert_eq!(pixels[0], [10, 20, 30, 0]);
}

#[test]
fn horizontal_and_vertical_green_candidates_use_their_own_axes() {
    let pattern = BayerPattern::GBRG;
    let mut pixels = vec![[0_u16; 4]; 15 * 15];
    let target = 3 * 15 + 2;
    pixels[3 * 15 + 1][1] = 10_000;
    pixels[3 * 15 + 3][1] = 30_000;
    pixels[2 * 15 + 2][1] = 2_000;
    pixels[4 * 15 + 2][1] = 6_000;
    assert_eq!(
        horizontal_candidates(&pixels, 15, 15, pattern)[target][1],
        20_000.0
    );
    assert_eq!(
        vertical_candidates(&pixels, 15, 15, pattern)[target][1],
        4_000.0
    );
}

#[test]
fn direction_decision_prefers_the_candidate_with_matching_local_rhythm() {
    let pattern = BayerPattern::GBRG;
    let mut pixels = vec![[100_u16, 100, 100, 0]; 15 * 15];
    let target = 3 * 15 + 2;
    pixels[target] = [10, 100, 100, 0];
    pixels[target - 2][0] = 100;
    pixels[target + 2][0] = 200;
    let mut horizontal = vec![[100.0_f32; 3]; 15 * 15];
    horizontal[target - 2][2] = 100.0;
    horizontal[target + 2][2] = 200.0;
    let mut vertical = vec![[100.0_f32; 3]; 15 * 15];
    vertical[target - 2][2] = 100.0;
    vertical[target + 2][2] = 300.0;
    assert!(!direction_is_vertical(
        &pixels,
        &horizontal,
        &vertical,
        15,
        15,
        pattern,
        target,
    ));
}
```

- [ ] **Step 2: Run the focused tests and confirm they fail**

Run: `cargo test -p fornax-raw-pipeline dcb -- --nocapture`

Expected: compilation fails because the new DCB helpers do not exist.

- [ ] **Step 3: Implement the input and border front end**

Add a module header naming LibRaw 0.22.3 commit
`cf02d66eb5c19d4e46df7aee0582e47a4918509a`, the
`dcb_demosaic.cpp` reference, copyright holder Jacek Gozdz, and the
BSD-3-Clause `docs/references/dcb.md` notice. Implement `insert_measured`,
`cfa_channel`, and `border_interpolate`.

`cfa_channel` maps `R` to 0, either green channel to 1, and `B` to 2 using
Euclidean phase lookup. `border_interpolate` follows LibRaw's unsigned-neighbor
traversal and the `col == border` row-skip convention without clamping measured
coordinates.

- [ ] **Step 4: Implement candidate construction and direction selection**

Port `dcb_hor`, `dcb_color2`, `dcb_ver`, `dcb_color3`, `dcb_decide`, and the
common `CLIP` conversion used by those functions. Candidate buffers contain
three channels; direction maps stay in channel 3 of `DcbPixel`.

Implement `apply_direction` as the full-frame `dcb_decide` traversal. Its
per-pixel decision delegates to `direction_is_vertical`, which returns true
when the vertical candidate is at least as close as the horizontal candidate,
matching the reference's strict horizontal comparison.

- [ ] **Step 5: Run the focused tests**

Run: `cargo test -p fornax-raw-pipeline dcb -- --nocapture`

Expected: the three new tests pass.

- [ ] **Step 6: Format and commit the front-stage checkpoint**

Run:

```bash
cargo fmt
git add crates/fornax-raw-pipeline/src/demosaic/dcb.rs
git commit -m "feat: add DCB directional candidates"
```

---

### Task 2: Correction pipeline and base color reconstruction

**Files:**

- Modify: `crates/fornax-raw-pipeline/src/demosaic/dcb.rs`

**Interfaces:**

- Consumes: `DcbPixel`, `cfa_channel`, and candidate buffers from Task 1.
- Produces:
  - `fn clip_i32(value: i32) -> u16`
  - `fn nyquist_pass(pixels: &mut [DcbPixel], width: usize, height: usize, pattern: BayerPattern)`
  - `fn direction_map(pixels: &mut [DcbPixel], width: usize, height: usize)`
  - `fn correction(pixels: &mut [DcbPixel], width: usize, height: usize, pattern: BayerPattern)`
  - `fn correction2(pixels: &mut [DcbPixel], width: usize, height: usize, pattern: BayerPattern)`
  - `fn color(pixels: &mut [DcbPixel], width: usize, height: usize, pattern: BayerPattern)`
  - `fn smooth(pixels: &mut [DcbPixel], width: usize, height: usize)`

- [ ] **Step 1: Write failing stage tests**

Add tests with these exact assertions:

```rust
#[test]
fn nyquist_pass_mixes_green_neighbors_with_the_current_cfa_color() {
    let pattern = BayerPattern::GBRG;
    let mut pixels = vec![[0_u16; 4]; 15 * 15];
    let target = 3 * 15 + 2;
    pixels[target] = [10, 100, 0, 0];
    for (offset, green, red) in [
        (-2_isize, 10_u16, 2_u16),
        (-2 * 15, 20, 4),
        (2, 30, 6),
        (2 * 15, 40, 8),
    ] {
        let index = target.checked_add_signed(offset).unwrap();
        pixels[index][1] = green;
        pixels[index][0] = red;
    }
    nyquist_pass(&mut pixels, 15, 15, pattern);
    assert_eq!(pixels[target][1], 30);
}

#[test]
fn map_and_correction_mix_horizontal_and_vertical_green() {
    let pattern = BayerPattern::GBRG;
    let mut pixels = vec![[0_u16; 4]; 15 * 15];
    let target = 2 * 15 + 3;
    pixels[target - 1][1] = 100;
    pixels[target + 1][1] = 100;
    pixels[target - 15][1] = 300;
    pixels[target + 15][1] = 300;
    pixels[target][1] = 500;
    direction_map(&mut pixels, 15, 15);
    assert_eq!(pixels[target][3], 1);
    for pixel in &mut pixels {
        pixel[3] = 0;
    }
    pixels[target][3] = 1;
    pixels[target][1] = 200;
    correction(&mut pixels, 15, 15, pattern);
    assert_eq!(pixels[target][1], 150);
}

#[test]
fn correction2_uses_cfa_contrast_on_the_selected_axis() {
    let pattern = BayerPattern::GBRG;
    let mut pixels = vec![[100_u16, 100, 100, 0]; 15 * 15];
    let target = 4 * 15 + 5;
    pixels[target][3] = 1;
    correction2(&mut pixels, 15, 15, pattern);
    assert_eq!(pixels[target][1], 100);
}

#[test]
fn color_reconstructs_the_missing_diagonal_channel() {
    let pattern = BayerPattern::GBRG;
    let mut pixels = vec![[100_u16, 100, 200, 0]; 15 * 15];
    color(&mut pixels, 15, 15, pattern);
    assert_eq!(pixels[15 + 2][2], 200);
}

#[test]
fn smooth_preserves_green_contrast_over_eight_neighbors() {
    let mut pixels = vec![[100_u16, 100, 100, 0]; 15 * 15];
    let target = 2 * 15 + 2;
    for y in 1..=3 {
        for x in 1..=3 {
            if y == 2 && x == 2 {
                continue;
            }
            pixels[y * 15 + x] = [200, 100, 300, 0];
        }
    }
    smooth(&mut pixels, 15, 15);
    assert_eq!(pixels[target], [200, 100, 300, 0]);
}
```

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p fornax-raw-pipeline dcb -- --nocapture`

Expected: compilation fails because the correction helpers do not exist.

- [ ] **Step 3: Implement Nyquist, map, and correction helpers**

Port `dcb_nyquist`, `dcb_map`, `dcb_correction`, and `dcb_correction2` with the
reference loop bounds, four- and eight-neighbor weight patterns, channel-3 map
layout, and conversion points. `direction_map` treats true as vertical and
false as horizontal.

- [ ] **Step 4: Implement base color and green smoothing**

Port `dcb_color` and `dcb_pp`. Keep the first pass over non-green sites and the
second pass over green sites in separate loops. `smooth` averages all eight
neighbors for red, green, and blue, then adds the current green contrast.

- [ ] **Step 5: Run focused tests**

Run: `cargo test -p fornax-raw-pipeline dcb -- --nocapture`

Expected: all Task 1 and Task 2 tests pass.

- [ ] **Step 6: Format and commit the correction checkpoint**

Run:

```bash
cargo fmt
git add crates/fornax-raw-pipeline/src/demosaic/dcb.rs
git commit -m "feat: add DCB correction passes"
```

---

### Task 3: Enhancement, full color, and public integration

**Files:**

- Modify: `crates/fornax-raw-pipeline/src/demosaic/dcb.rs`
- Modify: `crates/fornax-raw-pipeline/src/demosaic/directional.rs`

**Interfaces:**

- Consumes: all previous DCB stages and `buffer::{codes, dense_from_codes}`.
- Produces:
  - `fn refinement(pixels: &mut [DcbPixel], width: usize, height: usize, pattern: BayerPattern)`
  - `fn color_full(pixels: &mut [DcbPixel], width: usize, height: usize, pattern: BayerPattern)`
  - `fn run(input: &[u16], width: usize, height: usize, pattern: BayerPattern) -> Vec<u16>`
  - `DemosaicDcb::lazy<T>(&self, image: &MonoImage<T>, pattern: BayerPattern) -> Result<DenseLazy<T>, DemosaicError>`

- [ ] **Step 1: Write failing enhancement and integration tests**

Add tests covering these exact behaviors:

```rust
#[test]
fn diagonal_chroma_weight_uses_the_matching_far_neighbor() {
    let width = 15;
    let index = 7 * width + 7;
    let mut chroma = vec![[0.0_f32; 2]; 15 * 15];
    chroma[index + width - 1][0] = 10.0;
    chroma[index - width + 1][0] = 20.0;
    chroma[index + 3 * width + 3][0] = 100.0;
    chroma[index + 3 * width - 3][0] = 50.0;
    assert_eq!(
        diagonal_chroma_weight(&chroma, index, width, 0, 2),
        1.0 / 131.0
    );
}

#[test]
fn post_correction_sequence_runs_exactly_three_passes() {
    let pattern = BayerPattern::GBRG;
    let mut source = vec![[1_000_u16, 2_000, 3_000, 0]; 15 * 15];
    source[7 * 15 + 7][1] = 50_000;
    let mut expected = source.clone();
    for _ in 0..3 {
        direction_map(&mut expected, 15, 15);
        correction(&mut expected, 15, 15, pattern);
    }
    let mut actual = source;
    apply_post_correction_passes(&mut actual, 15, 15, pattern);
    assert_eq!(actual, expected);
}

#[test]
fn refinement_preserves_a_uniform_cfa_to_green_ratio() {
    let pattern = BayerPattern::GBRG;
    let mut pixels = vec![[20_000_u16, 10_000, 20_000, 0]; 15 * 15];
    refinement(&mut pixels, 15, 15, pattern);
    for y in 5..10 {
        for x in 5..10 {
            assert_eq!(pixels[y * 15 + x][1], 10_000);
        }
    }
}

#[test]
fn color_full_interpolates_uniform_chroma_without_changing_channels() {
    let pattern = BayerPattern::GBRG;
    let mut pixels = vec![[10_000_u16, 20_000, 30_000, 0]; 15 * 15];
    color_full(&mut pixels, 15, 15, pattern);
    for y in 6..9 {
        for x in 6..9 {
            assert_eq!(
                &pixels[y * 15 + x][..3],
                &[10_000, 20_000, 30_000]
            );
        }
    }
}

#[test]
fn uniform_channel_codes_survive_the_public_dcb_path_for_all_patterns() {
    for pattern in BayerPattern::ALL {
        let input = (0..15 * 15)
            .map(|index| {
                let channel = cfa_channel(pattern, (index % 15) as isize, (index / 15) as isize);
                [10_000_u16, 20_000, 30_000][channel]
            })
            .collect::<Vec<_>>();
        let output = run(&input, 15, 15, pattern);
        assert!(output.as_chunks::<3>().0.iter().all(|pixel| {
            pixel == &[10_000, 20_000, 30_000]
        }));
    }
}

#[test]
fn full_color_limit_can_modify_a_sampled_value() {
    let pattern = BayerPattern::GBRG;
    let mut input = (0..15 * 15)
        .map(|index| {
            let channel = cfa_channel(pattern, (index % 15) as isize, (index / 15) as isize);
            [10_000_u16, 20_000, 30_000][channel]
        })
        .collect::<Vec<_>>();
    let target = 7 * 15 + 8;
    input[target] = 60_000;
    assert_ne!(run(&input, 15, 15, pattern)[target * 3], 60_000);
}

#[test]
fn public_dcb_rejects_images_smaller_than_three_pixels() {
    for (width, height) in [(2, 3), (3, 2)] {
        let mosaic = MonoImage::from_raw(
            width,
            height,
            vec![0.0_f32; width as usize * height as usize],
        )
        .unwrap();
        assert!(matches!(
            DemosaicDcb.lazy(&mosaic, BayerPattern::GBRG),
            Err(DemosaicError::InvalidInput(_))
        ));
    }
}

#[test]
fn public_dcb_accepts_the_three_pixel_bridge_minimum() {
    let mosaic = MonoImage::from_raw(3_u32, 3_u32, vec![0.0_f32; 9]).unwrap();
    let output = DemosaicDcb
        .lazy(&mosaic, BayerPattern::GBRG)
        .unwrap()
        .collect();
    assert!(output.as_raw().iter().all(|sample| *sample == 0.0));
}
```

- [ ] **Step 2: Run the tests and confirm they fail**

Run: `cargo test -p fornax-raw-pipeline dcb -- --nocapture`

Expected: compilation fails because refinement, full color, and `run` do not
exist.

- [ ] **Step 3: Implement refinement and full color**

Port `dcb_copy_to_buffer`, `dcb_restore_from_buffer`, `dcb_refinement`, and
`dcb_color_full`. Refinement uses the five directional ratio weights
`5:3:1:3:1`, current map weighting, unsigned neighbor limits, and direct
`u16` conversion. Full color builds chroma in a separate `[f32; 2]` plane,
performs diagonal and axial chroma interpolation in reference order, then
applies the eight-neighbor red and blue limits.

- [ ] **Step 4: Implement `run` and route `DemosaicDcb` through it**

Implement the exact main sequence:

```text
border_interpolate(6)
horizontal candidates + colors
vertical candidates + colors
apply direction
save red/blue
color + smooth
correction2; map/correction x3
restore red/blue
color
refinement
color_full
pack RGB
```

Convert input with `codes`, construct a three-channel result with
`dense_from_codes`, and remove `Profile::Dcb` plus its DCB-only branches from
`directional.rs` once no code references them.

- [ ] **Step 5: Run focused tests**

Run: `cargo test -p fornax-raw-pipeline dcb -- --nocapture`

Expected: every DCB stage and integration test passes.

- [ ] **Step 6: Run the postprocessor test suite**

Run: `cargo test -p fornax-raw-pipeline`

Expected: all postprocessor tests pass.

- [ ] **Step 7: Format and commit the integrated backend**

Run:

```bash
cargo fmt
git add crates/fornax-raw-pipeline/src/demosaic/dcb.rs \
  crates/fornax-raw-pipeline/src/demosaic/directional.rs
git commit -m "feat: port LibRaw DCB demosaicing"
```

---

### Task 4: Comparison evidence, documentation, and completion

**Files:**

- Modify: `crates/compare-demosaic/README.md`
- Modify: `crates/compare-demosaic/baseline/canonical.json`
- Modify: `crates/compare-demosaic/src/snapshots/*-DCB-libraw.snap`
- Modify: `TODO.md`
- Modify: `docs/plans/dcb-faithful-port.md`
- Create: `docs/references/dcb.md`

**Interfaces:**

- Consumes: the completed `Demosaic::Dcb` backend.
- Produces: reviewed DCB comparison evidence, generated documentation, source
  attribution, and the completed backlog record.

- [ ] **Step 1: Run the native comparison with artifacts**

Run:

```bash
cargo run --release -p compare-demosaic -- run \
  --algorithm dcb --pattern GBRG --artifacts --out target/dcb
```

Expected: the GBRG DCB row passes MAE, P99, and high-error fraction limits.
Inspect `target/dcb/canonical.json` and the generated error/tile artifacts if a
limit fails; do not update snapshots or baseline while the row fails.

- [ ] **Step 2: Run all four DCB pattern rows**

Run: `cargo run --release -p compare-demosaic -- run --algorithm dcb`

Expected: all four DCB rows pass and match the reviewed behavior for their
cropped phase variants.

- [ ] **Step 3: Update and inspect DCB snapshots**

Run:

```bash
cargo insta test -p compare-demosaic
cargo insta accept
```

Expected: only the four `*-DCB-libraw.snap` checksum files change. Review their
diff and restore any unrelated snapshot change.

- [ ] **Step 4: Write reference notes and update the hand-written DCB note**

Create `docs/references/dcb.md` with the pinned source link, algorithm stage
order, six-pixel border, fixed iteration/enhancement settings, integer
conversion behavior, expected comparison behavior, and the complete
BSD-3-Clause notice retained from `dcb_demosaic.cpp`.

Replace the generic DCB README note outside the generated markers with the
actual stage sequence and known differences. Do not edit between
`compare-demosaic:begin` and `compare-demosaic:end`.

- [ ] **Step 5: Refresh the canonical baseline and generated README**

Run:

```bash
cargo run --release -p compare-demosaic -- run
cp target/compare-demosaic/canonical.json \
  crates/compare-demosaic/baseline/canonical.json
cargo run --release -p compare-demosaic -- readme
```

Expected: DCB rows and their generated README table are updated; unrelated
baseline rows remain byte-for-byte or numerically identical.

- [ ] **Step 6: Verify comparison consistency**

Run:

```bash
cargo run --release -p compare-demosaic -- readme --check
cargo run --release -p compare-demosaic -- diff \
  crates/compare-demosaic/baseline/canonical.json \
  target/compare-demosaic/canonical.json
```

Expected: both commands exit 0.

- [ ] **Step 7: Complete the backlog record**

Remove the `[DCB]` bullet from `TODO.md`. Mark completed plan checkboxes in
`docs/plans/dcb-faithful-port.md` and record the final accepted metrics,
validation results, and any skipped platform checks under
`Decisions and notes`.

- [ ] **Step 8: Run repository validation**

Run every command in this plan's `Validation` section. Record exact outcomes
in the pull request. Do not describe a skipped command as passing.

- [ ] **Step 9: Commit the comparison and documentation checkpoint**

Run:

```bash
git add TODO.md docs/plans/dcb-faithful-port.md docs/references/dcb.md \
  crates/compare-demosaic/README.md \
  crates/compare-demosaic/baseline/canonical.json \
  crates/compare-demosaic/src/snapshots
git commit -m "test: accept faithful DCB comparison"
```
