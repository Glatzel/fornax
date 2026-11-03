# Compare-demosaic reference algorithms: investigation and implementation plans

> **For agentic workers:** Implement one bracketed `TODO.md` item per branch
> and pull request. Use the task checklist for that algorithm, preserve the
> pinned reference versions, and run every validation command before claiming
> completion.

**Goal:** Replace the six approximate or broken Fornax demosaic paths with
ports of their named reference routines, one independently reviewable task at
a time, until the native GBRG comparison rows satisfy the repository limits.

**Architecture:** Keep the existing `IDemosaic<T>` and
`LazyDemosaic<T>` boundary. Start each algorithm from the 16-bit normalized
active Bayer mosaic and emit normalized RGB. Use `DenseLazy` where the
reference algorithm is whole-frame, retaining row-copy behavior at the lazy
boundary. Keep `directional.rs` until the last of its five profiles is replaced,
so unfinished algorithms continue to work during incremental pulls.

**Tech Stack:** Rust 2024, `fornax-core`, `fornax-postprocessor`,
`compare-demosaic`, LibRaw 0.22.3, and darktable 5.6.2.

## Global constraints

- Only conventional 2x2 RGB Bayer input is supported.
- Native fidelity is measured on the `GBRG` row at crop `[0, 0]`.
- Every native `GBRG` row for the algorithm must have:
  - `MAE <= 2.000000`
  - `P99 <= 64`
  - `fraction above 1024 codes <= 0.001000`
- A failing row is never waived. `⚖️` is not a target or fallback.
- Do not loosen `meta.limits`.
- Never edit `baseline/canonical.json` by hand.
- Never edit text between generated `compare-demosaic` README markers by hand.
- Complete and test one algorithm per `TODO.md` item. Remove that item from
  `TODO.md` only in the pull request that completes it.
- Keep reference links and `build.rs` at LibRaw commit
  `cf02d66eb5c19d4e46df7aee0582e47a4918509a` and darktable commit
  `20891f6c6fa7be995e5bf9dff9d1ee5062051773`.

## Investigation method

The six native rows were regenerated from `colorchart-eos-7d.cr2` with:

```bash
cargo run --release -p compare-demosaic -- run \
  --algorithm <id> --pattern GBRG --artifacts \
  --out target/investigation/<id>
```

The error maps and tile summaries were inspected together with the JSON
channel, sampled/interpolated-site, all/interior, and worst-pixel statistics.
All 64x64 tiles reported below are nonzero-high-error tiles.

| Algorithm | Reference | MAE | P99 | Max | >1024 count | Changed | Interior MAE | Interpolated-site MAE | Sampled-site MAE |
| --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| AAHD | LibRaw | 103.105439 | 1327 | 18089 | 1101122 | 0.629560 | 101.868229 | 154.658158 | 0.000000 |
| AMaZE | darktable | 79.498569 | 808 | 9623 | 239862 | 0.659406 | 79.188272 | 119.247854 | 0.000000 |
| DCB | LibRaw | 88.009547 | 1105 | 7094 | 665986 | 0.668585 | 87.717931 | 130.763942 | 2.500758 |
| DHT | LibRaw | 96.496395 | 906 | 15912 | 356469 | 0.662065 | 94.691852 | 144.744592 | 0.000000 |
| LMMSE | darktable | 44.776609 | 463 | 9522 | 27014 | 0.657319 | 44.385435 | 67.164914 | 0.000000 |
| VNG | LibRaw | 2762.780291 | 30351 | 65535 | 20978050 | 0.665211 | 2757.419036 | 4144.170436 | 0.000000 |
| VNG | darktable | 2782.662776 | 30349 | 65535 | 20998328 | 0.827285 | 2777.233600 | 4144.778187 | 58.431955 |

## Review focus

- A port that matches the reference only inside a preferred tile alignment
  without matching seam pixels must be rejected.
- A port that passes by accepting decision flips outside the required uniform
  tolerance must be rejected; acceptance is every limit, not a lower average.
- Bayer phase handling must remain correct for all four patterns even though
  only native `GBRG` is the independent acceptance row.
- Integer `ushort`, fixed-point, truncation, and saturation conventions must
  be tested explicitly because changing them can move thousands of pixels by
  one or more codes.
- A GPL implementation must not enter `fornax-postprocessor` before the
  project's licensing question is resolved.

---

## VNG

### Goal

Replace the current VNG logic with LibRaw 0.22.3's VNG routine and make both
native `GBRG` reference rows pass every limit.

### Scope

**In scope:** VNG phase precomputation, initial bilinear interpolation,
the eight-gradient selection, green/red/blue reconstruction, ring-buffer row
update, border handling, and immediate tests.

**Out of scope:** VNG4, four-color Bayer, X-Trans, and generic filter arrays.

### Root-cause analysis

The native LibRaw row has MAE `2762.780291`, P99 `30351`, maximum `65535`,
and `20978050` samples above 1024. The interior MAE is `2757.419036`, so this
is not a border-only failure. The error map is nearly saturated over the whole
frame, and all 4503 populated 64x64 tiles contain high-error samples.

Fornax preserves measured Bayer samples exactly relative to LibRaw
(`sites.sampled.mae == 0`), while interpolated samples have MAE `4144.170436`;
this rules out black-level subtraction and input normalization as the primary
cause. Arithmetic widths also rule out generic `u16` overflow: the largest
linear-filter accumulation is well inside `u32`, gradient and neighbor sums
are `i64`, and final conversion is clamped deliberately. The dominant worst
pixels are interpolated red/green values that underflow to `0` while the
reference is near `65535`, for example `(2694,71,c1) = 0 vs 65535`.

The concrete logic bug is mixed reference conventions. Fornax copied the
darktable `terms` table, whose fifth field is a multiplicative weight of 1 or
2, but computes `abs(difference) << weight`, which is LibRaw's shift
convention. LibRaw's corresponding table uses shift values 0 or 1. The
current code therefore doubles every weighted gradient and quadruples terms
that darktable would only double. It also builds the tables against one green
channel, while darktable explicitly separates G1/G2 and averages them after
interpolation. This is a logic/filter-phase defect, not a channel mix-up,
black-level problem, or overflow problem.

### Fix strategy

Port `LibRaw::vng_interpolate()` from LibRaw 0.22.3, called after the
reference shim's `pre_interpolate()` and LibRaw's internal
`lin_interpolate()`:

- `LibRaw 0.22.3`, commit
  `cf02d66eb5c19d4e46df7aee0582e47a4918509a`,
  `src/demosaic/misc_demosaic.cpp#L199`.
- Keep the 8x2 Bayer phase table (`prow = 8`, `pcol = 2`) and the unified green
  channel used by LibRaw after `pre_interpolate()`.
- Copy LibRaw's own 64-entry `terms` and `chood` tables. Do not combine
  darktable's table with LibRaw shift arithmetic.
- Keep `u16[4]` pixels, `i32` gradients/sums, 3-row ring buffer, negative
  difference clipping, and final 16-bit quantization.
- Reuse `buffer::{codes, dense_from_codes}` for the harness boundary.
- Implement the LibRaw one-row/one-column linear border initialization before
  VNG rather than clamping coordinates at the image edge.

The current file is structurally close to LibRaw, so first encode the exact
table and shift behavior in tests, then replace the inconsistent phase builder
and indexing. A darktable-exact G1/G2 implementation cannot also be
bit-for-bit identical to LibRaw because the references represent the two green
phases differently; the shared output must still meet both acceptance rows.
LibRaw's progress callbacks, image-pipeline metadata, and non-Bayer filter
modes are outside the typed Fornax stage and cannot be reproduced there; the
comparison adapter pins the supported Bayer branch and direct RGB copy-out.

### Task checklist

- [ ] Add a failing 8x8 Bayer phase-table unit test that asserts the exact
  LibRaw term offsets, shift weights, and gradient masks for at least one
  phase at every odd/even row parity and both column parities.
- [ ] Add a failing linear-interpolation fixture with known border values and
  assert the one-pixel border result above, below, left, and right.
- [ ] Add a failing VNG fixture with a small hand-computable directional
  pattern and an explicit expected 16-bit RGB buffer.
- [ ] Replace `TERMS`, phase generation, gradient accumulation, neighbor sums,
  and ring-buffer copy order with the LibRaw 0.22.3 conventions.
- [ ] Keep the existing measured-sample invariant for both green phases under
  LibRaw's unified-green representation.
- [ ] Add `rows_can_be_filled_independently` and small-image rejection tests.
- [ ] Run the native LibRaw and darktable comparisons with artifacts and
  inspect both error maps.

### Files expected to change

- `crates/fornax-postprocessor/src/demosaic/vng.rs`
- `crates/fornax-postprocessor/src/demosaic.rs` only if the public lazy wrapper
  must change; the preferred result keeps `DenseLazy`
- `crates/fornax-postprocessor/src/demosaic/buffer.rs` only for tested
  code-domain helpers
- `crates/compare-demosaic/src/snapshots/*-VNG-*.snap`
- `crates/compare-demosaic/README.md` VNG note outside generated markers
- `crates/compare-demosaic/baseline/canonical.json`
- `TODO.md`

### Acceptance criteria

- Native `GBRG` VNG versus LibRaw: MAE <= 2, P99 <= 64, high-error fraction
  <= 0.001 above 1024 codes.
- Native `GBRG` VNG versus darktable: the same three limits.
- No waiver, changed limit, or sampled-site exclusion is used.
- All four Bayer phases remain exact at measured CFA samples relative to
  LibRaw and respect the same bounds at the non-native shifted crops.

### Validation and updates

```bash
cargo test -p fornax-postprocessor vng
cargo run --release -p compare-demosaic -- run \
  --algorithm vng --pattern GBRG --artifacts --out target/vng
cargo run --release -p compare-demosaic -- run --algorithm vng
cargo insta test -p compare-demosaic
cargo insta accept
```

Review every accepted VNG snapshot before continuing. Then refresh the
baseline and generated README regions exactly as documented for the repository.
Update the hand-written VNG note to name the LibRaw routine, the pinned
revision, the retained G1/G2 difference, and the border/quantization
conventions. Remove the old "three-row generic u16" characterization once it
is no longer true.

### Risks, dependencies, and size

- **Dependencies:** none. Do this first because it is an isolated confirmed
  bug and establishes confidence in the comparison harness.
- **Risks:** a LibRaw-exact implementation may leave a small darktable-only
  residual because darktable separates G1/G2; the acceptance limits, not
  bitwise equality, decide.
- **Size:** M.

### License implications

LibRaw `misc_demosaic.cpp` is dual-licensed LGPL-2.1 or CDDL-1.0. Porting or
translating the routine may create a derivative work even when expressed in
Rust. Human legal/project-license review is required before implementation;
this plan does not decide which license path is acceptable. Do not copy
darktable's terms while trying to avoid that review.

### Decisions and notes

- Use LibRaw as the implementation source and both references as acceptance
  oracles.
- Replacing only the shift bug is useful for diagnosis but is not completion;
  the full phase/border conventions must be ported.

---

## LMMSE

### Goal

Replace the generic LMMSE profile with darktable 5.6.2's LMMSE routine and
make the native `GBRG` row pass every limit.

### Scope

**In scope:** PPG seed, gamma LUTs, horizontal/vertical color differences,
low-pass filters, MMSE interpolation, R/B reconstruction, one median pass,
tile overlap, and normalization.

**Out of scope:** LMMSE refinement modes greater than the harness's mode 1,
camera matrices, white balance, and output crops.

### Root-cause analysis

The native row has MAE `44.776609`, P99 `463`, and `27014` samples above 1024.
It fails MAE and P99 but not the high-error-fraction limit. The interior MAE
is `44.385435`, so the complete frame, not only the border, is wrong. The
error map has a fine periodic lattice across otherwise flat chart surfaces.
All 695 populated 64x64 tiles contain high-error samples.

The sampled-site MAE is zero; interpolated-site MAE is `67.164914`, with
`98.5979%` of interpolated sites changed. The ordinary tint means are small
(`mean_signed = 1.144272` overall), which argues against a constant channel or
normalization swap. The structure instead matches missing LMMSE stages:
Fornax uses the generic directional kernel with `green_power = 1`,
`chroma_radius = 2`, and one ad hoc half-weight refinement. It does not seed
with PPG, apply the reference low-pass filters and local variance weighting,
or use the 136-pixel tiled path.

Representative interior/high-contrast errors include
`(2401,437,c2) = 46204 vs 40788`; the largest errors are an `R` channel
top-border edge around `x=5096..5112`, consistent with the independent border
path but not the source of the global failure.

### Fix strategy

Port `lmmse_demosaic()` from darktable 5.6.2 with the harness arguments
`mode = 1` and `scaler = 1.0f`:

- darktable 5.6.2, commit
  `20891f6c6fa7be995e5bf9dff9d1ee5062051773`,
  `src/iop/demosaicing/lmmse.c#L120`.
- Seed with the exact darktable-compatible PPG path, clamp threshold `0.0`,
  and border margin `4`.
- Preserve `DT_LMMSE_TILESIZE = 136`, `BORDER_AROUND = 4`,
  `LMMSE_OVERLAP = 8`, and valid tile stride `112`.
- Use two 65,536-entry gamma tables with interpolation at
  `value * 65535.0`, including the exact endpoints and coefficients.
- Use normalized `f32` intermediates, the reference 9-tap
  `h = exp(-i^2 / 8)` low-pass kernel, exact vertical/horizontal variance
  blends, one 3x3 median pass, and zero refinement passes.
- Copy only valid tile interiors and write the outermost overlap regions as
  the reference does.
- Clamp negative output to zero before the existing harness's upper clamp and
  nearest-code quantization.

Reuse `ppg.rs` by exposing a private code-domain seed helper, or move common
PPG calculations into a small tested helper. Do not route the seed through the
public row-lazy `fill_rows` API because LMMSE needs the complete tile.
The darktable pixelpipe's metadata, white-balance, and module orchestration
are outside Fornax's demosaic stage and cannot be reproduced here. The
comparison bridge pins `scaler = 1`, passes no white-balance multipliers, and
uses the identity input path; those are the reference details this port can
match. Compiler evaluation of the `f32` constants may still produce tiny
last-bit differences, which are governed by the numeric limits rather than a
waiver.

### Task checklist

- [ ] Add a failing PPG-seed test with a nonuniform Bayer fixture that pins
  the exact green and opposite-color values used by LMMSE.
- [ ] Add a failing gamma-table interpolation test at the linear/gamma
  thresholds, table endpoints, and between-entry values.
- [ ] Add a failing 136x136/odd-size tile test that expects identical center
  pixels across adjacent tiles and deterministic seam output.
- [ ] Add a failing mode-1 fixture with one median stage and no refinement
  stage.
- [ ] Implement the dedicated LMMSE path while retaining the `DemosaicLmmse`
  and `DenseLazy` public interfaces.
- [ ] Add all-pattern tests and an image smaller than twice the border.
- [ ] Run the native comparison and inspect the periodic lattice.

### Files expected to change

- `crates/fornax-postprocessor/src/demosaic/lmmse.rs`
- `crates/fornax-postprocessor/src/demosaic/ppg.rs` if the seed helper is
  extracted
- `crates/fornax-postprocessor/src/demosaic.rs` if the profile enum loses
  `Lmmse` in this task
- `crates/fornax-postprocessor/src/demosaic/directional.rs` only to remove
  the LMMSE profile after the new implementation passes
- `crates/compare-demosaic/src/snapshots/*-LMMSE-darktable.snap`
- `crates/compare-demosaic/README.md` LMMSE note outside generated markers
- `crates/compare-demosaic/baseline/canonical.json`
- `TODO.md`

### Acceptance criteria

- Native `GBRG` LMMSE versus darktable: MAE <= 2, P99 <= 64, high-error
  fraction <= 0.001 above 1024 codes.
- Tile seams in the 128x128 regression fixture have no discontinuity beyond
  one quantization code.
- The implementation uses mode `1`, including exactly one median pass and no
  refinement pass.

### Validation and updates

```bash
cargo test -p fornax-postprocessor lmmse
cargo run --release -p compare-demosaic -- run \
  --algorithm lmmse --pattern GBRG --artifacts --out target/lmmse
cargo run --release -p compare-demosaic -- run --algorithm lmmse
cargo insta test -p compare-demosaic
cargo insta accept
```

Refresh the baseline and generated README, then replace the LMMSE "shared
directional kernel" note with the pinned function, tile dimensions, gamma
tables, one-median mode, and any residual floating-point/border differences.

### Risks, dependencies, and size

- **Dependencies:** the PPG seed must match darktable before LMMSE can match.
  Fornax's standalone PPG darktable comparison already passes exactly.
- **Risks:** tile copy bounds, gamma interpolation endpoints, and compiler
  floating-point contraction can create small decision changes. Keep tests
  around each.
- **Size:** L.

### License implications

darktable `lmmse.c` is GPLv3. Translating it into the BSD-3-Clause
`fornax-postprocessor` library is likely a derivative-work licensing change.
Flag this for human legal/project-license review before implementation. Do not
decide whether to copy, relicense, isolate, or independently reimplement.

### Decisions and notes

- Match darktable's harness mode 1, not the more expensive modes 4+ that add
  refinements.
- Keep the PPG implementation shared by behavior, not by duplicating the
  entire public lazy type.

---

## AAHD

### Goal

Replace the generic AAHD profile with LibRaw 0.22.3's AAHD routine and make
the native `GBRG` row pass every limit.

### Scope

**In scope:** hot-pixel handling, two directional AHD candidate images,
homogeneity evaluation and refinement, YUV/gamma conversion, green and
red/blue reconstruction, directional combination, and 16-bit output.

**Out of scope:** standard AHD changes, camera metadata, and output color
processing outside the harness's identical RGB matrix.

### Root-cause analysis

The native row has MAE `103.105439`, P99 `1327`, maximum `18089`, and
`1101122` samples above 1024. Interior MAE is `101.868229`; all 1370
populated 64x64 tiles contain high errors. The error map is a full-frame
texture over ordinary and chart surfaces, not a border strip.

Sampled-site MAE is zero, while interpolated-site MAE is `154.658158` and
`94.4332%` of interpolated sites change. Red and blue are worse than green,
which is expected when a different green plane feeds color differences.
Representative worst pixels are `(5166,0,c0) = 30219 vs 48308` and
`(3216,0,c0) = 27788 vs 44713`; the top boundary is severe, but the interior
maximum is still `8349`.

The cause is not normalization or channel order. `aahd.rs` only selects
`Profile::Aahd`, giving the shared 5x5 kernel `green_power = 4` and
`chroma_radius = 3`. It omits LibRaw AAHD's two complete directional images,
YUV homogeneity counts, directional refinement, channel extrema, and final
candidate selection.

### Fix strategy

Port LibRaw 0.22.3 `LibRaw::aahd_interpolate()` and its private `AAHD`
helpers:

- LibRaw 0.22.3, commit
  `cf02d66eb5c19d4e46df7aee0582e47a4918509a`,
  `src/demosaic/aahd_demosaic.cpp#L771`.
- The reference shim calls `pre_interpolate()` first; under the harness's
  non-shrink, non-four-color path this only unifies both green CFA codes.
- Preserve the four-pixel `nr_margin`, `Thot = 4`, `Tdead = 4`, and
  `OverFraction = 8`.
- Use a 65,536-entry sRGB-to-linear LUT generated with the reference
  thresholds `0.0181`, `4.5`, `1.0993`, and `0.45`.
- Maintain two full-frame `ushort3` candidates and full-frame YUV and
  homogeneity planes; do not collapse them into the current green plus
  chroma-radius approximation.
- Preserve integer `ushort3`/`int3` operations, channel min/max clipping,
  directional flags, hot-pixel restore, and the final truncating
  `ushort` conversion.
- Reuse the existing AHD `CBRT` table or extract a shared tested LUT only if
  AAHD needs the same LUT; keep AHD behavior unchanged.
LibRaw's outer processing metadata, progress callbacks, and camera-specific
matrix pipeline are not part of the Fornax demosaic boundary and cannot be
reproduced there. The comparison bridge deliberately supplies an identity
`rgb_cam` and no white balance, so the port targets that exact reference path
and the reference's own integer output conventions.

### Task checklist

- [x] Add a failing gamma-LUT test at dark values, the linear/gamma threshold,
  and white.
- [x] Add a failing two-direction fixture that distinguishes horizontal and
  vertical green candidates and asserts the homogeneity decision.
- [x] Add a failing hot-pixel fixture that verifies the original CFA sample is
  restored before directional combination.
- [x] Add a failing final-`ushort` truncation/clipping fixture, including
  negative and over-white intermediates.
- [x] Implement the dedicated AAHD path and remove `Aahd` from the shared
  profile only after the comparison passes; keep `DemosaicAahd` and
  `DenseLazy`.
- [x] Add all-pattern, measured-sample, uniform-field, and row-boundary tests.
- [x] Run the native comparison and inspect the error map for full-frame
  structure and chart-edge tails.

### Files expected to change

- `crates/fornax-postprocessor/src/demosaic/aahd.rs`
- `crates/fornax-postprocessor/src/demosaic/ahd.rs` only if shared LUT helpers
  are extracted
- `crates/fornax-postprocessor/src/demosaic.rs` to remove the AAHD profile
  variant
- `crates/fornax-postprocessor/src/demosaic/directional.rs` to remove AAHD-only
  code after migration
- `crates/compare-demosaic/src/snapshots/*-AAHD-libraw.snap`
- `crates/compare-demosaic/README.md` AAHD note outside generated markers
- `crates/compare-demosaic/baseline/canonical.json`
- `docs/references/aahd.md`
- `TODO.md`

### Acceptance criteria

- Native `GBRG` AAHD versus LibRaw: MAE <= 2, P99 <= 64, high-error fraction
  <= 0.001 above 1024 codes.
- The final RGB values are derived from the exact integer/`ushort` clamps and
  truncation conventions of the pinned AAHD path, not nearest rounding added
  before comparison.
- All four Bayer patterns preserve the measured CFA sample at non-hot sites.

### Validation and updates

```bash
cargo test -p fornax-postprocessor aahd
cargo run --release -p compare-demosaic -- run \
  --algorithm aahd --pattern GBRG --artifacts --out target/aahd
cargo run --release -p compare-demosaic -- run --algorithm aahd
cargo insta test -p compare-demosaic
cargo insta accept
```

Refresh the baseline and generated README. Replace the AAHD note's description
of the shared kernel with the pinned AAHD stages and the exact border,
homogeneity, gamma, and output conventions.

### Risks, dependencies, and size

- **Dependencies:** none functional; follow the VNG task so comparison changes
  are isolated. It can reuse tested AHD LUT concepts.
- **Risks:** large full-frame buffers, exact `char`/`ushort` overflow behavior,
  hot-pixel flags, and final truncation.
- **Size:** L.

### License implications

LibRaw `aahd_demosaic.cpp` is dual-licensed LGPL-2.1 or CDDL-1.0. A direct
translation may remain a derivative work. Human legal/project-license review
is required before implementation; this plan does not choose a license.

### Decisions and notes

- Port AAHD, not standard AHD. The existing AHD implementation is a behavioral
  reference and must not be replaced to make AAHD easier.
- Stage-level comparison against an instrumented copy of the pinned LibRaw
  routine found that the second red/blue endpoint must use `-dirs`, not
  `+dirs`; otherwise zero padding contaminates gradients at image edges and
  ±1 candidate drift cascades into homogeneity decision changes.
- The full native GBRG run has MAE `0.262976`, P99 `0`, and `3310` samples
  above 1024 codes, which is `0.000061` of all output values. All four Bayer
  phases pass the unchanged robust limits, and the error map is sparse texture
  noise without a full-frame or border pattern.

---

## DHT

### Goal

Replace the generic DHT profile with LibRaw 0.22.3's directional homogeneity
transform and make the native `GBRG` row pass every limit.

### Scope

**In scope:** four-pixel margins, hot-pixel masking, horizontal/vertical and
diagonal direction maps, green generation, red/blue generation, and exact
16-bit copy back.

**Out of scope:** fallback to AHD for unsupported filters and non-Bayer CFAs.
Fornax continues rejecting unsupported input before demosaicing.

### Root-cause analysis

The native row has MAE `96.496395`, P99 `906`, maximum `15912`, and `356469`
high-error samples. Interior MAE is `94.691852`; all 1563 populated 64x64
tiles have high errors. The error map shows a fine reconstruction texture
throughout the image.

Measured CFA sites are exact and interpolated-site MAE is `144.744592`.
Channel MAEs are similar (`R = 102.872735`, `G = 88.479774`,
`B = 98.136674`), and overall `mean_signed = 1.320177`, so an input offset,
single-channel mix-up, or normalization error is inconsistent with the
evidence. The largest interior event is a decision flip at
`(4048,1617)`: both green and blue are near zero in the reference but about
`16k` in Fornax. The generic directional kernel omits DHT's direction fields,
refinement, and diagonal color reconstruction.

### Fix strategy

Port LibRaw 0.22.3 `LibRaw::dht_interpolate()` and the private `DHT` helpers:

- LibRaw 0.22.3, commit
  `cf02d66eb5c19d4e46df7aee0582e47a4918509a`,
  `src/demosaic/dht_demosaic.cpp#L1012`.
- Preserve four-pixel top/left/right/bottom margins.
- Use `float3` working data initialized to `0.5`, original CFA maxima/minima,
  `char` direction flags, hot-pixel masks, and the exact `hide_hots()` /
  `restore_hots()` behavior.
- Follow the stage order: `hide_hots`, `make_hv_dirs`, `make_greens`,
  `make_diag_dirs`, `make_rb`, `restore_hots`, `copy_to_image`.
- Preserve intermediate floating-point operations and the final truncating
  `unsigned short` conversion.
- Use the current Bayer pattern at the boundary and normalize green row phase
  only where LibRaw does.
LibRaw's progress callbacks and AHD fallback for unsupported filter masks are
not applicable at the Fornax boundary: the pipeline rejects non-2x2-Bayer
input before demosaicing. LibRaw's surrounding raw metadata and output
pipeline also cannot be reproduced as part of this stage, so the adapter's
pre-interpolated green normalization and direct `ushort[4]` copy-out are the
contract to match.

### Task checklist

- [ ] Add a failing hot-pixel test that verifies a marked sample is excluded
  during direction selection and restored unchanged at output.
- [ ] Add a failing horizon/vertical edge fixture whose direction map and green
  line must choose different directions.
- [ ] Add a failing diagonal red/blue fixture that pins the `make_rbdiag` and
  `make_rbhv` decision.
- [ ] Add a failing float-to-`ushort` truncation test at half-code boundaries.
- [ ] Implement the dedicated DHT path and remove `Dht` from the shared profile
  after comparison passes; keep `DemosaicDht` and `DenseLazy`.
- [ ] Add all-pattern, uniform-field, measured-sample, and border-margin tests.
- [ ] Run the native comparison and inspect the error map and decision flips.

### Files expected to change

- `crates/fornax-postprocessor/src/demosaic/dht.rs`
- `crates/fornax-postprocessor/src/demosaic.rs` to remove the DHT profile
  variant
- `crates/fornax-postprocessor/src/demosaic/directional.rs` to remove DHT-only
  code after migration
- `crates/compare-demosaic/src/snapshots/*-DHT-libraw.snap`
- `crates/compare-demosaic/README.md` DHT note outside generated markers
- `crates/compare-demosaic/baseline/canonical.json`
- `TODO.md`

### Acceptance criteria

- Native `GBRG` DHT versus LibRaw: MAE <= 2, P99 <= 64, high-error fraction
  <= 0.001 above 1024 codes.
- No pixel is accepted through the unsupported-filter AHD fallback.
- All four Bayer patterns preserve measured non-hot samples and have
  deterministic margin values.

### Validation and updates

```bash
cargo test -p fornax-postprocessor dht
cargo run --release -p compare-demosaic -- run \
  --algorithm dht --pattern GBRG --artifacts --out target/dht
cargo run --release -p compare-demosaic -- run --algorithm dht
cargo insta test -p compare-demosaic
cargo insta accept
```

Refresh the baseline and generated README. Replace the DHT note's generic
kernel description with the pinned stage list, margins, hot-pixel handling,
and truncation convention.

### Risks, dependencies, and size

- **Dependencies:** AAHD's hot-pixel and full-frame float-plane implementation
  provides a reviewed idiom, though the formulas must remain algorithm-local.
- **Risks:** the many mutually dependent direction arrays and `char` flags are
  easy to transpose. Add one failing fixture per stage before implementation.
- **Size:** L.

### License implications

LibRaw `dht_demosaic.cpp` is dual-licensed LGPL-2.1 or CDDL-1.0. Translation
may be a derivative work. Human legal/project-license review is required
before implementation; this plan does not decide the license.

### Decisions and notes

- Match the exact four standard Bayer filter masks used by the reference
  branch; all other input is rejected earlier in the Fornax pipeline.

---

## DCB

### Goal

Replace the generic DCB profile with LibRaw 0.22.3's `dcb(0, 1)` path and make
the native `GBRG` row pass every limit.

### Scope

**In scope:** six-pixel border interpolation, horizontal/vertical candidate
colors, DCB decision, Nyquist correction, chroma map/correction, refinement,
and full color reconstruction.

**Out of scope:** FBDD, alternate DCB iteration counts, and LibRaw processing
before or after demosaicing.

### Root-cause analysis

The native row has MAE `88.009547`, P99 `1105`, maximum `7094`, and `665986`
high-error samples. Interior MAE is `87.717931`; all 1225 populated 64x64
tiles have high errors. The error map is global and textured.

Interpolated-site MAE is `130.763942`; sampled-site MAE is `2.500758`, with
only nine sampled errors above 1024. That small sampled residual is from the
reference DCB operations rather than a black/white-level error in the common
input. Red and blue are worse than green and `mean_signed = 18.157793`.
Worst examples include top-border `(5104,0,c0) = 15338 vs 8244` and interior
edge `(2393,291,c0) = 30367 vs 23584`.

Fornax only applies exponent `2`, chroma radius `2`, and no refinement. It
does not perform LibRaw's `dcb_hor`/`dcb_ver`, `dcb_color2`/`dcb_color3`,
direction decision, Nyquist passes, maps/corrections, refinement, or
`dcb_color_full`.

### Fix strategy

Port LibRaw 0.22.3 `LibRaw::dcb(0, 1)` and its called helpers:

- LibRaw 0.22.3, commit
  `cf02d66eb5c19d4e46df7aee0582e47a4918509a`,
  `src/demosaic/dcb_demosaic.cpp#L849`.
- The reference shim calls `pre_interpolate()` first and then
  `dcb(iterations = 0, dcb_enhance = 1)`.
- Preserve `border_interpolate(6)`, the order and repetition count of
  `dcb_nyquist`/`dcb_map`/`dcb_correction`, and the final
  `dcb_refinement` plus `dcb_color_full`.
- Use full-frame `float[3]` candidate buffers and the reference's `CLIP`
  and integer/truncating operations at the exact points where it converts.
- Reuse the existing CFA border traversal pattern only if a fixture proves
  identical averaging and bounds.
- Preserve reference-modified measured samples; do not force the CFAs back to
  input after the final stage unless LibRaw does.
LibRaw's outer camera metadata, white balance, progress callbacks, and
post-demosaic processing are unavailable inside the isolated Fornax stage and
are not part of the comparison adapter. The port targets the reference
`pre_interpolate()` plus `dcb(0, 1)` path with the adapter's direct integer
copy-out; any remaining differences must fit the numeric limits.

### Task checklist

- [ ] Add a failing two-direction fixture that distinguishes the horizontal
  and vertical DCB candidates and the `dcb_decide` result.
- [ ] Add a failing Nyquist-pattern fixture that requires all three
  `dcb_nyquist()` passes before mapping.
- [ ] Add a failing color-map/correction fixture with a local edge and an
  expected mixed green value.
- [ ] Add a failing enhancement fixture that reaches `dcb_refinement` and
  `dcb_color_full`.
- [ ] Implement the dedicated DCB path and remove `Dcb` from the shared profile
  after comparison passes; keep `DemosaicDcb` and `DenseLazy`.
- [ ] Add all-pattern, uniform-field, six-pixel-border, and small-image tests.
- [ ] Run the native comparison and inspect the error map and worst samples.

### Files expected to change

- `crates/fornax-postprocessor/src/demosaic/dcb.rs`
- `crates/fornax-postprocessor/src/demosaic.rs` to remove the DCB profile
  variant
- `crates/fornax-postprocessor/src/demosaic/directional.rs` to remove DCB-only
  code after migration
- `crates/compare-demosaic/src/snapshots/*-DCB-libraw.snap`
- `crates/compare-demosaic/README.md` DCB note outside generated markers
- `crates/compare-demosaic/baseline/canonical.json`
- `TODO.md`

### Acceptance criteria

- Native `GBRG` DCB versus LibRaw: MAE <= 2, P99 <= 64, high-error fraction
  <= 0.001 above 1024 codes.
- The exact `dcb(0, 1)` stage order and enhancement path are exercised.
- A regression test verifies the reference's non-preservation of some measured
  samples is reproduced rather than masked by a CFA restore.

### Validation and updates

```bash
cargo test -p fornax-postprocessor dcb
cargo run --release -p compare-demosaic -- run \
  --algorithm dcb --pattern GBRG --artifacts --out target/dcb
cargo run --release -p compare-demosaic -- run --algorithm dcb
cargo insta test -p compare-demosaic
cargo insta accept
```

Refresh the baseline and generated README. Replace the DCB note's generic
profile description with the pinned helper sequence, six-pixel border,
iteration/enhancement settings, and reference clipping behavior.

### Risks, dependencies, and size

- **Dependencies:** none, although it benefits from the tile/border helpers
  reviewed in earlier tasks.
- **Risks:** many named stages operate in place on two full-frame float
  buffers. A visual error can be several stages upstream, so keep stage-level
  fixtures.
- **Size:** L.

### License implications

LibRaw `dcb_demosaic.cpp` carries a BSD-3-Clause notice from the DCB/FBDD
author. A port must retain the applicable copyright and license notice and
may still require project review for attribution placement. This plan does
not make that legal decision.

### Decisions and notes

- Match `dcb(0, 1)`, not a configurable or iterative approximation.
- The common decode/preprocess stages remain unchanged.

---

## AMaZE

### Goal

Replace the generic AMaZE profile with darktable 5.6.2's AMaZE routine and
make the native `GBRG` row pass every limit.

### Scope

**In scope:** the complete AMaZE tile pipeline, adaptive green ratios and
differences, diagonal red/blue ratios, Nyquist texture handling, curvature
refinement, and output quantization.

**Out of scope:** RawTherapee integration, custom tile sizes, white balance,
and anything after demosaicing.

### Root-cause analysis

The native row has MAE `79.498569`, P99 `808`, maximum `9623`, and `239862`
high-error samples. Interior MAE is `79.188272`; all 1187 populated 64x64
tiles have high errors. The map shows fine textured error over smooth and
detailed regions rather than a border-only problem.

Sampled-site MAE is zero. Interpolated-site MAE is `119.247854`, and `98.911%`
of interpolated sites change. Green also fails substantially
(`MAE = 62.786120`), so the red/blue error is not only a chroma conversion
problem. A positive overall signed mean of `17.574803` rules against a simple
channel inversion. Representative errors cluster around chart geometry, for
example `(2699,436..452)` and `(2403,444,c0) = 40527 vs 30904`.

Fornax uses only exponent `3` and chroma radius `3` from the shared kernel.
It omits AMaZE's adaptive ratio decisions, directional variance weights,
Nyquist detection and area interpolation, diagonal color-ratio correction,
and 160-pixel tile handling.

### Fix strategy

Port darktable 5.6.2 `amaze_demosaic()`:

- darktable 5.6.2, commit
  `20891f6c6fa7be995e5bf9dff9d1ee5062051773`,
  `src/iop/demosaicing/amaze.cc#L129`.
- Use normalized `f32` input and `clip_pt = 1.0`, matching the existing
  comparison adapter.
- Preserve `AMAZETS = 160`, the 16-pixel halo, tile start `-16`, tile stride
  `128`, all reference constants, and right/bottom edge reflection.
- Preserve the reference order: CFA tile fill, directional weights, adaptive
  green candidates, color-difference variance selection, Nyquist test and
  refinement, diagonal red/blue correction, and final clamped output.
- Keep the one-channel CFA plus computed green/color-difference planes; do not
  add camera-matrix or white-balance stages absent from the harness.
The darktable pixelpipe, its metadata, and the surrounding demosaic module
orchestration cannot be represented by Fornax's typed demosaic interface. The
adapter pins the normalized input, identity scaling, `clip_pt = 1`, and final
clamp/nearest-code conversion, so those are the reference-specific conventions
the port can match. Any compiler-dependent floating-point differences remain
subject to the same numeric limits.

### Task checklist

- [ ] Add a failing tile-geometry test for `5202` width and a smaller odd/even
  matrix, asserting each destination pixel is written exactly once and tile
  starts/ends match the 16-pixel halo rules.
- [ ] Add a failing directional-gradient fixture that distinguishes
  horizontal and vertical adaptive ratio choices.
- [ ] Add a failing local-variance fixture that changes the selected green
  interpolation direction.
- [ ] Add a failing Nyquist-pattern fixture that requires area interpolation
  and curvature refinement.
- [ ] Add a failing diagonal red/blue ratio fixture.
- [ ] Implement the dedicated AMaZE path and remove `Amaze` from the shared
  profile after comparison passes; this should be the final user of
  `directional.rs`.
- [ ] Delete `directional.rs` and its `Profile` enum after the last migrated
  algorithm passes, and update `demosaic.rs`.
- [ ] Add all-pattern, uniform-field, border-reflection, and row-lazy tests.
- [ ] Run the native comparison and inspect the full-frame texture map.

### Files expected to change

- `crates/fornax-postprocessor/src/demosaic/amaze.rs`
- `crates/fornax-postprocessor/src/demosaic.rs`
- `crates/fornax-postprocessor/src/demosaic/directional.rs` (delete when all
  five profiles have migrated)
- `crates/compare-demosaic/src/snapshots/*-AMaZE-darktable.snap`
- `crates/compare-demosaic/README.md` AMaZE note outside generated markers
- `crates/compare-demosaic/baseline/canonical.json`
- `TODO.md`

### Acceptance criteria

- Native `GBRG` AMaZE versus darktable: MAE <= 2, P99 <= 64, high-error
  fraction <= 0.001 above 1024 codes.
- Tile seams match the single-tile reference behavior within one code in the
  128x128 regression fixture.
- All four Bayer phases preserve measured CFA values and remain deterministic.

### Validation and updates

```bash
cargo test -p fornax-postprocessor amaze
cargo run --release -p compare-demosaic -- run \
  --algorithm amaze --pattern GBRG --artifacts --out target/amaze
cargo run --release -p compare-demosaic -- run --algorithm amaze
cargo insta test -p compare-demosaic
cargo insta accept
```

Refresh the baseline and generated README. Replace the AMaZE note's
"shared directional kernel" text with the pinned darktable function, tile
geometry, adaptive/Nyquist stages, and normalization/quantization
conventions.

### Risks, dependencies, and size

- **Dependencies:** do this after the smaller ports so the shared
  `directional.rs` can be deleted cleanly. It has no algorithm dependency.
- **Risks:** AMaZE has the largest state workspace and the most tile-edge
  cases; a single index transposition can produce plausible but globally
  wrong output.
- **Size:** L.

### License implications

darktable `amaze.cc` is GPLv3 and includes a RawTherapee/Emil Martinec GPLv3
notice. A faithful Rust translation into the BSD-3-Clause
`fornax-postprocessor` is likely a derivative work and a project licensing
decision. Human legal/project-license review is required before
implementation; this plan does not decide whether to copy, relicense, isolate,
or independently reimplement it.

### Decisions and notes

- Use the existing darktable comparison adapter as the acceptance oracle.
- Do not keep `directional.rs` as dead fallback code after all profiles are
  replaced.

---

## Shared completion checklist

Each implementation pull request must:

- [ ] Start from current `origin/main` in its required task worktree.
- [ ] Include only one algorithm's implementation plus required tests and
  documentation.
- [ ] Update that algorithm's hand-written note, snapshots, and complete
  baseline with the commands below.
- [ ] Remove only the completed bracketed item from `TODO.md`.
- [ ] Report installed LibRaw/darktable versions, all command outcomes, and any
  skipped check.

For an algorithm `<id>`, run:

```bash
source scripts/setup.sh
cargo run --release -p compare-demosaic -- run \
  --algorithm <id> --pattern GBRG --artifacts --out target/<id>
cargo run --release -p compare-demosaic -- run --algorithm <id>
cargo insta test -p compare-demosaic
cargo insta accept
cargo run --release -p compare-demosaic -- run
cp target/compare-demosaic/canonical.json \
  crates/compare-demosaic/baseline/canonical.json
cargo run --release -p compare-demosaic -- readme
cargo run --release -p compare-demosaic -- readme --check
cargo run --release -p compare-demosaic -- diff \
  crates/compare-demosaic/baseline/canonical.json \
  target/compare-demosaic/canonical.json
cargo test -p compare-demosaic --all-features
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
git diff --check
```

The current `compare-demosaic` command set has no `--update` flag. Do not add
one for this work: the baseline is refreshed by the full `run` plus `cp`, and
snapshots are refreshed through `cargo insta test` and `cargo insta accept`.

Before committing, inspect the full diff. Confirm that generated README
changes are produced by `readme`, baseline changes are produced by `run`, and
no waiver or limit changed. After commit and push, open the one semantic pull
request for that task and report its URL; do not watch, merge, or close it.

## Proposed implementation order

1. **VNG**: isolated confirmed logic bug, no algorithm dependency, and it
   removes a severe baseline failure first.
2. **LMMSE**: reuses the already exact PPG seed and introduces the 136-pixel
   gamma-tiled migration pattern.
3. **AAHD**: reuses the AHD implementation's tested LUT/homogeneity concepts
   without changing the AHD path.
4. **DHT**: follows AAHD so hot-pixel and full-frame float-workspace handling
   has a reviewed precedent.
5. **DCB**: independent full-frame in-place pipeline; sequencing it here keeps
   each review focused.
6. **AMASE**: largest port and final shared-kernel user, allowing
   `directional.rs` to be deleted at completion.

| Order | TODO | Branch | Worktree | Dependency |
| ---: | --- | --- | --- | --- |
| 1 | `[VNG]` | `task/vng-faithful-port` | `.worktrees/vng-faithful-port` | None |
| 2 | `[LMMSE]` | `task/lmmse-faithful-port` | `.worktrees/lmmse-faithful-port` | Exact PPG seed |
| 3 | `[AAHD]` | `task/aahd-faithful-port` | `.worktrees/aahd-faithful-port` | Existing AHD LUT concepts |
| 4 | `[DHT]` | `task/dht-faithful-port` | `.worktrees/dht-faithful-port` | AAHD hot-pixel/float-plane idiom |
| 5 | `[DCB]` | `task/dcb-faithful-port` | `.worktrees/dcb-faithful-port` | None |
| 6 | `[AMASE]` | `task/amase-faithful-port` | `.worktrees/amase-faithful-port` | Earlier migrations; deletes shared kernel |

## Licensing gate

Fornax itself is BSD-3-Clause. The reference sources carry different terms:

- LibRaw AAHD, DHT, and VNG-derived code: LGPL-2.1 or CDDL-1.0.
- LibRaw DCB: BSD-3-Clause with author attribution.
- darktable AMaZE, LMMSE, and VNG: GPLv3.

Only a human can decide whether translation, linking, relicensing, isolated
subprocesses, or independent paper-based implementation is acceptable. Treat
that decision as a prerequisite for the affected task; do not infer permission
from the existing comparison-only C/C++ adapters.
