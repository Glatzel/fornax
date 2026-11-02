# VNG faithful behavioral implementation

## Goal

Replace the incorrect Fornax VNG path with an independent BSD-only
implementation that follows darktable 5.6.2's split-green VNG behavior at all
four Bayer patterns and keeps LibRaw 0.22.3 as an informational comparison.

## Scope

**In scope:** VNG phase plans, initial interpolation and borders, the eight
gradient scores, neighbor selection, green/red/blue reconstruction, ring-buffer
updates, focused unit tests, comparison artifacts, snapshots, canonical
baseline, generated README, and the `[VNG]` backlog completion record.

**Out of scope:** VNG4, four-color Bayer, X-Trans, generic filter arrays,
changes to comparison limits, and waivers for VNG failures.

## Task checklist

- [x] Add focused tests for phase invariants, border interpolation, a small
  directional reconstruction, measured-sample preservation, independent row
  reads, and minimum dimensions.
- [x] Replace the mixed gradient convention with same-CFA terms using shift
  weights 0 or 1 and darktable-compatible split green phases.
- [x] Replace coordinate-clamped border interpolation with in-bounds CFA
  interpolation and retain the three-row VNG reconstruction.
- [x] Make darktable the native comparison acceptance oracle and record LibRaw
  distance without failing on its different green-plane convention.
- [x] Run `GBRG`, `RGGB`, `GRBG`, and `BGGR` against darktable and confirm the
  split-green behavior is not pattern-specific.
- [x] Inspect the remaining maximum error numerically, including channel, site,
  neighborhood, and pre-pack green values.
- [x] Review and accept only the intended VNG snapshots.
- [x] Refresh the full canonical baseline and generated README, then update
  the hand-written VNG note.
- [x] Remove `[VNG]` from `TODO.md` after acceptance checks pass and unrelated
  environment limitations are documented.

## Files expected to change

- `crates/fornax-postprocessor/src/demosaic/vng.rs`
- `crates/compare-demosaic/src/snapshots/*-VNG-*.snap`
- `crates/compare-demosaic/README.md`
- `crates/compare-demosaic/baseline/canonical.json`
- `AGENTS.md`
- `docs/references/vng.md`
- `TODO.md`
- This plan

## Validation

```bash
cargo test -p fornax-postprocessor vng
cargo run --release -p compare-demosaic -- run \
  --algorithm vng --pattern GBRG --artifacts --out target/vng
cargo run --release -p compare-demosaic -- run --algorithm vng
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
cargo test --workspace --all-features
cargo fmt --check
cargo clippy --workspace --all-features --all-targets -- -D warnings
markdownlint-cli2 crates/compare-demosaic/README.md \
  docs/plans/vng-faithful-port.md
git -c core.whitespace=cr-at-eol diff --check
```

The native `GBRG` rows must satisfy `MAE <= 2`, `P99 <= 64`, and high-error
fraction `<= 0.001` above 1024 codes. No numeric limit or waiver may change.

## Decisions and notes

- The selected license direction is BSD-only. No LibRaw LGPL/CDDL or
  darktable GPL source is translated or copied into `fornax-postprocessor`.
- The existing in-tree gradient descriptor and neighbor data remain
  project-owned BSD inputs. Reference adapters serve only as black-box
  acceptance oracles.
- The public `Demosaic::Vng`, `AnyLazy::Vng`, and `DenseLazy` interfaces stay
  unchanged; VNG continues to reject images smaller than 8x8.
- darktable is the authoritative VNG reference. LibRaw's `four_color_rgb = 0`
  path retains one green plane and is intentionally informational.
- `G1` remains channel 1 and `G2` remains channel 3 throughout linear and VNG
  interpolation. They are averaged only when packing RGB, matching
  darktable's `filters4` and `mix_greens` behavior.
- The current `733` maximum occurs at `(2402,441)`, a blue Bayer site with an
  interpolated green value. Before packing, its green planes are `64069` and
  `65535`; their integer average is `64802` versus darktable's `65535`.
  Final-average rounding can change that value by at most one, so the outlier
  is an isolated fixed-point/clipping divergence rather than an average
  rounding error. There are no errors above 1024.
- The focused VNG tests and full workspace suite pass. The exact workspace
  Clippy command is blocked by an unrelated pre-existing
  `clippy::too_many_arguments` errors in `ahd.rs` and the LibRaw binding;
  Clippy passes for the changed packages when that unrelated lint is allowed.
- The full comparison run changed only the eight VNG rows; every non-VNG row
  remained identical to the committed baseline.
- Reusable VNG sources and implementation decisions are recorded in
  `docs/references/vng.md`. The root agent guidance now requires durable
  algorithm research to be committed there with the code it informs.
