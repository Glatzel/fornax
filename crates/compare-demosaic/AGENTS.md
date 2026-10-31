# compare-demosaic: agent guide

## Purpose

Compares Fornax demosaic implementations against LibRaw and darktable at the
demosaic stage. `canonical.json` is the single source of truth for quality
verdicts. If a number is not in `canonical.json`, it is not evidence.

## Files

| Path | What it is |
| --- | --- |
| `target/compare-demosaic/canonical.json` | Generated output of the canonical full-frame run. Never edit by hand. |
| `target/compare-demosaic/canonical.filtered.json` | Output from a filtered diagnostic run. Never use it as a baseline. |
| `target/compare-demosaic/artifacts/` | Error maps and tile summaries referenced from the JSON. |
| `crates/compare-demosaic/baseline/canonical.json` | Committed baseline. Changed only with `--bless`. |
| `crates/compare-demosaic/canonical.schema.json` | Generated JSON Schema. Regenerate, do not hand-edit. |
| `crates/compare-demosaic/notes.json` | Prose notes by algorithm. Notes contain no metric values. |
| `crates/compare-demosaic/README.md` | Tables generated from the canonical baseline. Do not hand-edit numbers. |

The `rstest` snapshots and `synthetic_mono_image` smoke test are regression and
fast-failure guards only. They are never evidence of quality.

## Commands

```bash
source scripts/setup.sh
cargo run --release -p compare-demosaic
cargo run --release -p compare-demosaic -- --artifacts
cargo run --release -p compare-demosaic -- diff old.json new.json
cargo run --release -p compare-demosaic -- validate new.json
cargo run --release -p compare-demosaic -- --bless
cargo run --release -p compare-demosaic -- schema --check
cargo run --release -p compare-demosaic -- readme --check
```

## How to read canonical.json

- `schema_version`: do not compare files with different versions.
- `meta.limits`: the pass/fail thresholds. Verdicts are already computed.
- `meta.border_px`: border width excluded from `metrics_interior`.
- `meta.input.native_pattern`: the CFA layout the input really has.
- Each entry in `results` is one algorithm/pattern/reference combination:
  - `native`: true only for the input's real CFA layout. Prioritise these
    rows. Relabelled patterns are consistency checks, not independent tests.
  - `verdict`: `exact`, `rounding_only`, `pass_with_outliers`, `pass`, or
    `fail`.
  - `failed_limits`: which limits failed (`mae`, `p99`, `high_err_frac`).
  - `metrics`, `metrics_interior`: whole-frame and border-excluded metrics.
  - `per_channel`, `per_site`: breakdown by R/G/B and sampled vs interpolated
    site.
  - `worst_pixels`: the largest errors with coordinates and both values.
  - `artifacts`: repository-relative paths to the error map and tile summary.

All errors are in 16-bit codes. Fractions are fractions, never percentages.

## Useful queries

```bash
jq '.results[] | select(.verdict == "fail" and .native)
    | {algorithm, reference, failed_limits, mae: .metrics.mae}' \
    target/compare-demosaic/canonical.json
jq '.results[] | select(.algorithm == "AHD") | .worst_pixels' \
    target/compare-demosaic/canonical.json
```

## Metric glossary

- `mae`, `rmse`, `max`: mean absolute, root-mean-square, and maximum error.
- `mean_signed`: positive means Fornax is higher than the reference on
  average. A consistent bias usually means a rounding convention difference.
- `p99`, `p999`, `p9999`: exact percentiles from the error histogram. A large
  gap between `p99` and `mae`/`rmse` means a heavy tail.
- `frac_gt1`, `frac_gt16`: share of samples above one or sixteen codes.
  `frac_gt1` is mostly rounding noise; `frac_gt16` is real divergence.
- `count_gt1024`, `frac_gt1024`: high-error count and fraction.
- `changed_frac`: share of samples that differ at all.

## Conventions that cause expected differences

- Fornax clamps reconstructed RGB to `[0, 1]` and rounds to nearest before
  quantization. LibRaw truncates. Expect one-code upward differences against
  LibRaw for Linear.
- White balance and the default output crop are not applied.
- Reference builds and commits are recorded in `meta.references`.

## Investigating a failing row

Work in this order and stop when you find the cause:

1. Check the tail: compare `p999`, `count_gt1024`, and `max` with `mae`.
   A heavy tail with a small `p99` means a few discrete decision flips.
2. Compare `metrics` with `metrics_interior`. A clean interior points to border
   handling.
3. Open the artifact error map and tile summary. Classify errors as border,
   tile seam, high-contrast edge, clipped region, or global.
4. Check `per_site`. Errors at sampled sites point at normalization or
   black/white level handling rather than interpolation.
5. Check `per_channel`. A single bad channel points at a specific stage.
6. Compare the same pattern split across algorithms. A shared cause in a
   common helper beats a per-algorithm fix.
7. Only then dump intermediate stages from the reference build and diff them
   against Fornax stage by stage until the first divergence.

## Rules

- Never loosen `meta.limits` to make a case pass.
- Never edit `canonical.json`, the baseline, `notes.json` metric claims, or
  README tables by hand.
- Do not bless the baseline unless the change is intentional and you can state,
  per affected row, what improved and why.
- After changing an algorithm, rerun and report `diff` against the baseline.
- A pass against LibRaw or darktable shows fidelity to that reference, not
  image quality. Do not claim quality from this file alone.
- Do not draw conclusions from a single pattern. State which pattern a finding
  applies to and whether it is native.
- When `schema_version` changes, update this guide in the same commit.

## Report format

For every change, report one line per affected row:

`<algorithm> <pattern> <reference>: <old verdict> -> <new verdict>; <metrics that moved>`

Then state what changed, which stage diverged, and what was verified.

## Adding a new algorithm

1. Implement it in Fornax and register it in the comparison harness.
2. Choose the reference and record its version in `meta`.
3. Generate the canonical comparison and check every pattern.
4. Add a prose note without numbers, regenerate the README, and bless the
   baseline only if every row's verdict is explained.
