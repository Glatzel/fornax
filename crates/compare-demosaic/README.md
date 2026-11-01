# compare-demosaic

Compares Fornax demosaic output with LibRaw and darktable at the same stage:
the active Bayer mosaic enters each implementation and reconstructed RGB leaves
it. White balance and the default output crop are not applied.

`baseline/canonical.json` is the source of truth for results. The blocks marked
as generated below are derived from that JSON. Text outside the markers and the
notes beside each algorithm are written by the author and reviewed with the
implementation.

## Commands

<!-- compare-demosaic:begin usage -->
```text
$ compare-demosaic run --help
Compute comparison rows and write canonical or shard JSON

Usage: run [OPTIONS]

Options:
      --algorithm <ALGORITHM>
          Restrict rows to an algorithm; repeat for more than one

      --reference <REFERENCE>
          Restrict rows to a reference; repeat for more than one

      --pattern <PATTERN>
          Restrict rows to a Bayer pattern; repeat for more than one

          [possible values: RGGB, BGGR, GRBG, GBRG]

      --input <INPUT>
          Use a RAW input; repeat for more than one. Defaults to the devtool fixture

      --out <OUT>
          Directory for canonical JSON and artifacts

          [default: target/compare-demosaic]

      --shard <SHARD>
          Write <name>.shard.json instead of canonical.json

      --artifacts
          Write a log-scaled error map and high-error tile summary for every row

      --border-px <BORDER_PX>
          Border width excluded from interior statistics

          [default: 16]

      --jobs <JOBS>
          Number of rows computed in parallel

          [default: 4]

      --format <FORMAT>
          Output format written to stdout

          [default: text]
          [possible values: text, json]

  -h, --help
          Print help

$ compare-demosaic diff --help
Compare a baseline JSON document with a new run

Usage: diff [OPTIONS] <BASELINE> <NEW>

Arguments:
  <BASELINE>
          Accepted baseline JSON. It must be a full run

  <NEW>
          New run JSON. A partial run compares only its selected rows

Options:
      --verdict-only
          Ignore numeric-only changes when choosing the exit code

      --format <FORMAT>
          Output format written to stdout

          [default: text]
          [possible values: text, json]

  -h, --help
          Print help

$ compare-demosaic readme --help
Generate or verify the README regions and notes

Usage: readme [OPTIONS]

Options:
      --check
          Fail instead of updating when the README is out of date

      --from <FROM>
          Read a different full JSON report instead of the committed baseline

  -h, --help
          Print help
```
<!-- compare-demosaic:end usage -->

Run commands through Cargo from the repository root:

```bash
cargo run --release -p compare-demosaic -- run
cargo run --release -p compare-demosaic -- diff \
  crates/compare-demosaic/baseline/canonical.json \
  target/compare-demosaic/canonical.json
cargo run --release -p compare-demosaic -- readme --check
```

Partial runs are diagnostic inputs. They record their filters in
`meta.selection`, so `diff` can compare the selected rows and `readme` refuses
to treat the file as a baseline.

## Evaluation

All errors are integer 16-bit codes. Percentiles and maxima are exact values
from a 65536-bin histogram. Floating-point summaries are rounded before they
are written so the baseline is deterministic.

Use `all` for the complete frame and `interior` after excluding the border
strip. A small `p99` with a larger `mae` indicates a heavy tail or discrete
decision flips. A positive `mean_signed` means Fornax is above the reference.
The channel and sampled/interpolated-site scopes show where those errors live.
`worst` gives coordinates and both values for the largest errors.

Only a native-pattern row is an independent fidelity test. The other three
patterns relabel the same mosaic and are consistency checks.

## Summary

<!-- compare-demosaic:begin summary -->
| Algorithm | Reference | Worst verdict | Native-pattern result |
| --- | --- | --- | --- |
| AAHD | libraw | ❌ fail | GBRG ❌ fail |
| AHD | libraw | ❌ fail | GBRG ⚠️ pass with outliers |
| AMaZE | darktable | ❌ fail | GBRG ❌ fail |
| DCB | libraw | ❌ fail | GBRG ❌ fail |
| DHT | libraw | ❌ fail | GBRG ❌ fail |
| Linear | libraw | ✅ pass | GBRG 🟢 rounding |
| LMMSE | darktable | ❌ fail | GBRG ❌ fail |
| PPG | darktable | ✅ pass | GBRG ✅ exact |
| PPG | libraw | ✅ pass | GBRG ⚠️ pass with outliers |
| RCD | darktable | ✅ pass | GBRG ⚠️ pass with outliers |
| VNG | darktable | ❌ fail | GBRG ❌ fail |
| VNG | libraw | ❌ fail | GBRG ❌ fail |
<!-- compare-demosaic:end summary -->

## Legend

<!-- compare-demosaic:begin legend -->
- `✅`: exact or passing without high-error outliers.
- `🟢`: rounding-only differences.
- `⚠️`: every limit passes, but high-error outliers remain.
- `❌`: at least one limit failed.
- Limits: MAE <= 2.000000, P99 <= 64, high-error fraction <= 0.001000 above 1024 codes.
- Kinds: `exact` requires equality; `rounding` permits one-code upward rounding differences; `robust` applies the numeric limits.
- A row is `native` only when its pattern equals the input's original CFA layout. Relabelled patterns are consistency checks, not independent tests.
- Interior statistics exclude a 16 px border.
- References: LibRaw 0.22.3; darktable 5.6.2.
- Inputs:
  - `colorchart-eos-7d.cr2`: 5202x3464, native `GBRG`, sha256 `4c02a6d7eb8514a441fe0fa81ff79ed3c6f86ebac1c74bc0623655d5ba06e96c`.
<!-- compare-demosaic:end legend -->

## AAHD

<!-- compare-demosaic:begin algo:aahd -->
| Reference | Pattern | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | BGGR | no | ❌ fail | 747.885294 | 2099.339848 | 22138 | -119.338890 | 8777 | 11968 | 6945544 (127378) | 0.629272 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | GBRG | yes | ❌ fail | 103.105439 | 287.955685 | 18089 | 15.733126 | 1327 | 2118 | 1101122 (35228) | 0.629560 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | GRBG | no | ❌ fail | 103.088593 | 287.963386 | 18089 | 15.767824 | 1327 | 2119 | 1101640 (35314) | 0.629544 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | RGGB | no | ❌ fail | 747.938956 | 2099.423686 | 22138 | -119.450251 | 8778 | 11965 | 6946380 (127401) | 0.629277 |
<!-- compare-demosaic:end algo:aahd -->

> [!NOTE]
> **Reference:** LibRaw, `aahd_interpolate`.
> **Why this reference:** it is a widely used advanced AHD implementation
> with an independently maintained interpolation pipeline.
> **Why we match:** both preserve the measured CFA samples, estimate missing
> green from horizontal and vertical candidates, and reconstruct red and blue
> from color differences around the estimated green plane.
> **Known differences:** Fornax uses its shared directional kernel with an
> edge-weight exponent of four and a three-pixel chroma radius; LibRaw AAHD
> applies its own homogeneity and refinement stages. Fornax clamps normalized
> output and quantizes to nearest, while the reference controls its own
> intermediate and output conventions.

## AHD

<!-- compare-demosaic:begin algo:ahd -->
| Reference | Pattern | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | BGGR | no | ❌ fail | 6.673725 | 182.315588 | 22478 | 0.022883 | 1 | 3020 | 83207 (739) | 0.022739 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | GBRG | yes | ⚠️ pass with outliers | 0.875973 | 24.651269 | 5222 | 0.019917 | 1 | 296 | 6767 (193) | 0.022486 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | GRBG | no | ⚠️ pass with outliers | 0.989094 | 26.458811 | 4835 | 0.024568 | 1 | 352 | 7722 (192) | 0.022518 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | RGGB | no | ❌ fail | 6.817086 | 184.347885 | 23556 | 0.010033 | 1 | 3093 | 84797 (845) | 0.022979 |
<!-- compare-demosaic:end algo:ahd -->

> [!NOTE]
> **Reference:** LibRaw, `ahd_interpolate`.
> **Why this reference:** it is the established AHD implementation used by
> other RAW processors and has a stable pinned source revision.
> **Why we match:** Fornax builds horizontal and vertical green candidates,
> evaluates both in Lab with the same homogeneity idea and neighbor threshold
> comparisons, then selects the locally consistent direction.
> **Known differences:** Fornax uses the published camera-to-XYZ matrix,
> 512-pixel working tiles, floor intermediates and nearest-neighbor output
> quantization. LibRaw owns its tile, clipping and final quantization
> conventions, so decision boundaries can differ even with the same AHD rules.

## AMaZE

<!-- compare-demosaic:begin algo:amaze -->
| Reference | Pattern | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | BGGR | no | ❌ fail | 525.714046 | 1278.370104 | 21316 | -132.779921 | 5293 | 7820 | 7643060 (100304) | 0.661846 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GBRG | yes | ❌ fail | 79.498569 | 186.002581 | 9623 | 17.574803 | 808 | 1442 | 239862 (10535) | 0.659406 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GRBG | no | ❌ fail | 79.498569 | 186.002581 | 9623 | 17.574803 | 808 | 1442 | 239862 (10535) | 0.659406 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | RGGB | no | ❌ fail | 525.714046 | 1278.370104 | 21316 | -132.779921 | 5293 | 7820 | 7643060 (100304) | 0.661846 |
<!-- compare-demosaic:end algo:amaze -->

> [!NOTE]
> **Reference:** darktable, `amaze_demosaic`.
> **Why this reference:** darktable is the primary open-source AMaZE
> implementation used for reproducing this published algorithm.
> **Why we match:** both retain measured CFA values, choose green from
> directional edge evidence, and interpolate the remaining channels from
> local color differences.
> **Known differences:** Fornax uses the shared directional kernel with an
> edge-weight exponent of three and a three-pixel chroma radius instead of
> darktable's full AMaZE tile processing. Fornax also clamps in normalized
> space and quantizes to nearest; darktable's reference bridge performs the
> same final normalization from its tile result.

## DCB

<!-- compare-demosaic:begin algo:dcb -->
| Reference | Pattern | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | BGGR | no | ❌ fail | 656.695242 | 1795.569469 | 31865 | -124.893244 | 8002 | 11223 | 7085982 (110130) | 0.670707 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | GBRG | yes | ❌ fail | 88.009547 | 228.137404 | 7094 | 18.157793 | 1105 | 1902 | 665986 (21082) | 0.668585 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | GRBG | no | ❌ fail | 88.009547 | 228.137404 | 7094 | 18.157793 | 1105 | 1902 | 665986 (21082) | 0.668585 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | RGGB | no | ❌ fail | 656.695242 | 1795.569469 | 31865 | -124.893244 | 8002 | 11223 | 7085982 (110130) | 0.670707 |
<!-- compare-demosaic:end algo:dcb -->

> [!NOTE]
> **Reference:** LibRaw, `dcb`.
> **Why this reference:** it is the maintained implementation of directional
> color balance demosaicing and exposes the complete LibRaw pipeline.
> **Why we match:** both start from measured CFA samples, estimate green with
> horizontal and vertical confidence, and reconstruct red and blue by carrying
> green-normalized color differences into neighboring sites.
> **Known differences:** Fornax uses the shared directional kernel with an
> edge-weight exponent of two and a two-pixel chroma radius. LibRaw DCB uses
> its own confidence and color-balance refinement sequence; Fornax also clamps
> normalized output and quantizes to nearest.

## DHT

<!-- compare-demosaic:begin algo:dht -->
| Reference | Pattern | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | BGGR | no | ❌ fail | 527.009423 | 1382.556065 | 24042 | -65.080811 | 5936 | 11447 | 7317594 (118114) | 0.660867 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | GBRG | yes | ❌ fail | 96.496395 | 220.513723 | 15912 | 1.320177 | 906 | 1620 | 356469 (31484) | 0.662065 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | GRBG | no | ❌ fail | 96.496395 | 220.513723 | 15912 | 1.320177 | 906 | 1620 | 356469 (31484) | 0.662065 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | RGGB | no | ❌ fail | 527.009423 | 1382.556065 | 24042 | -65.080811 | 5936 | 11447 | 7317594 (118114) | 0.660867 |
<!-- compare-demosaic:end algo:dht -->

> [!NOTE]
> **Reference:** LibRaw, `dht_interpolate`.
> **Why this reference:** it provides the directional homogeneity transform
> implementation targeted by Fornax.
> **Why we match:** both preserve sampled Bayer values, derive candidate green
> values in both directions, and use green-referenced color differences for
> red and blue reconstruction.
> **Known differences:** Fornax's shared directional profile uses an
> edge-weight exponent of one and a two-pixel chroma radius. LibRaw DHT runs
> its own directional transform and refinement stages. Intermediate clipping,
> border fill and final quantization are therefore not shared.

## Linear

<!-- compare-demosaic:begin algo:linear -->
| Reference | Pattern | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | BGGR | no | 🟢 rounding | 0.293568 | 0.541820 | 1 | 0.293568 | 1 | 1 | 0 (0) | 0.293568 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | GBRG | yes | 🟢 rounding | 0.294064 | 0.542277 | 1 | 0.294064 | 1 | 1 | 0 (0) | 0.294064 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | GRBG | no | 🟢 rounding | 0.294064 | 0.542277 | 1 | 0.294064 | 1 | 1 | 0 (0) | 0.294064 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | RGGB | no | 🟢 rounding | 0.293568 | 0.541820 | 1 | 0.293568 | 1 | 1 | 0 (0) | 0.293568 |
<!-- compare-demosaic:end algo:linear -->

> [!NOTE]
> **Reference:** LibRaw, `lin_interpolate`.
> **Why this reference:** it isolates the basic bilinear interpolation that
> provides a simple convention check for the comparison harness.
> **Why we match:** both preserve the measured sample, average horizontal and
> vertical green neighbors at red or blue sites, and average diagonals for the
> missing opposite color.
> **Known differences:** Fornax produces typed floating-point samples, clamps
> them to `[0, 1]`, and rounds to nearest before 16-bit quantization. LibRaw's
> reference path truncates its final integer conversion; its border handling
> also comes from the LibRaw pipeline.

## LMMSE

<!-- compare-demosaic:begin algo:lmmse -->
| Reference | Pattern | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | BGGR | no | ❌ fail | 334.214702 | 931.666478 | 20371 | -22.419190 | 4393 | 7808 | 5203568 (78826) | 0.658283 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GBRG | yes | ❌ fail | 44.776609 | 103.955914 | 9522 | 1.144272 | 463 | 884 | 27014 (4421) | 0.657319 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GRBG | no | ❌ fail | 44.776464 | 103.955704 | 9522 | 1.144345 | 463 | 884 | 27014 (4421) | 0.657319 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | RGGB | no | ❌ fail | 334.214427 | 931.666276 | 20371 | -22.419055 | 4393 | 7808 | 5203554 (78814) | 0.658283 |
<!-- compare-demosaic:end algo:lmmse -->

> [!NOTE]
> **Reference:** darktable, `lmmse_demosaic`.
> **Why this reference:** darktable is the maintained LMMSE implementation
> used for the reference comparison.
> **Why we match:** both estimate green in horizontal and vertical directions,
> blend the local color differences, and perform a refinement pass that keeps
> measured CFA values fixed.
> **Known differences:** Fornax uses its shared directional kernel with an
> edge-weight exponent of one, a two-pixel chroma radius and one refinement
> pass. darktable LMMSE uses 136-pixel tiles and its own linear MMSE and gamma
> lookup stages. Fornax uses normalized floating point and nearest quantization
> at the harness boundary.

## PPG

<!-- compare-demosaic:begin algo:ppg -->
| Reference | Pattern | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | BGGR | no | ✅ exact | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | GBRG | yes | ✅ exact | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | GRBG | no | ✅ exact | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | RGGB | no | ✅ exact | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | BGGR | no | ⚠️ pass with outliers | 0.314865 | 21.504192 | 13493 | 0.123634 | 1 | 1 | 1582 (11) | 0.147196 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | GBRG | yes | ⚠️ pass with outliers | 0.186701 | 3.106662 | 2221 | 0.108924 | 1 | 1 | 53 (0) | 0.146916 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | GRBG | no | ⚠️ pass with outliers | 0.186701 | 3.106662 | 2221 | 0.108924 | 1 | 1 | 53 (0) | 0.146916 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | RGGB | no | ⚠️ pass with outliers | 0.314865 | 21.504192 | 13493 | 0.123634 | 1 | 1 | 1582 (11) | 0.147196 |
<!-- compare-demosaic:end algo:ppg -->

> [!NOTE]
> **Reference:** darktable, `demosaic_ppg`, with LibRaw `ppg_interpolate` as a
> secondary reference.
> **Why this reference:** both expose the patterned pixel-grouping algorithm
> that Fornax implements and are common production references.
> **Why we match:** the green estimate uses the same horizontal and vertical
> gradient differences, candidate clamping and color-difference correction
> used to reconstruct red and blue at green and opposite-color sites.
> **Known differences:** Fornax computes a three-pixel border fallback from the
> nearest matching color samples and consumes the Bayer pattern directly.
> darktable bridges normalize both green CFA codes and run its own border
> margin; LibRaw applies its pipeline around `ppg_interpolate`. Final integer
> rounding can therefore differ at code boundaries.

## RCD

<!-- compare-demosaic:begin algo:rcd -->
| Reference | Pattern | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | BGGR | no | ⚠️ pass with outliers | 0.372633 | 5.609573 | 5922 | -0.001610 | 8 | 48 | 109 (1) | 0.065598 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GBRG | yes | ⚠️ pass with outliers | 0.083077 | 1.015566 | 1359 | -0.000212 | 2 | 11 | 1 (0) | 0.033374 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GRBG | no | ⚠️ pass with outliers | 0.083077 | 1.015566 | 1359 | -0.000212 | 2 | 11 | 1 (0) | 0.033374 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | RGGB | no | ⚠️ pass with outliers | 0.372633 | 5.609573 | 5922 | -0.001610 | 8 | 48 | 109 (1) | 0.065598 |
<!-- compare-demosaic:end algo:rcd -->

> [!NOTE]
> **Reference:** darktable, `rcd_demosaic`.
> **Why this reference:** darktable provides the pinned RCD 2.3 implementation
> against which the independently structured Fornax version is compared.
> **Why we match:** both seed with PPG, use the same 112-pixel tile geometry,
> compute directional high-pass gradient confidence, interpolate ratio-corrected
> green, and then reconstruct diagonal color differences.
> **Known differences:** Fornax evaluates the formulas through normalized
> floating-point Rust and clamps final output to `[0, 1]`; the reference bridge
> quantizes the darktable result after the same clamp. Border fill and the
> surrounding metadata pipeline are not shared.

## VNG

<!-- compare-demosaic:begin algo:vng -->
| Reference | Pattern | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | BGGR | no | ❌ fail | 4417.576591 | 9258.126444 | 65535 | -3184.600269 | 37184 | 48831 | 26467159 (376708) | 0.832106 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | GBRG | yes | ❌ fail | 2782.662776 | 6477.675994 | 65535 | -687.175060 | 30349 | 45809 | 20998328 (288980) | 0.827285 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | GRBG | no | ❌ fail | 2784.676172 | 6479.882985 | 65535 | -687.205934 | 30358 | 45804 | 20992778 (288525) | 0.827284 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | RGGB | no | ❌ fail | 4417.097668 | 9256.884772 | 65535 | -3184.330787 | 37107 | 48859 | 26469636 (376814) | 0.832107 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | BGGR | no | ❌ fail | 4243.524941 | 9140.000579 | 65535 | -3167.610609 | 36685 | 46497 | 24309095 (341860) | 0.665300 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | GBRG | yes | ❌ fail | 2762.780291 | 6476.794907 | 65535 | -686.974152 | 30351 | 45810 | 20978050 (287078) | 0.665211 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | GRBG | no | ❌ fail | 2764.732896 | 6479.005956 | 65535 | -687.005027 | 30360 | 45806 | 20973190 (286726) | 0.665210 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | RGGB | no | ❌ fail | 4243.041290 | 9138.746679 | 65535 | -3167.341126 | 36691 | 46499 | 24310846 (341994) | 0.665299 |
<!-- compare-demosaic:end algo:vng -->

> [!NOTE]
> **Reference:** darktable, `vng_interpolate`, with LibRaw
> `vng_interpolate` as a secondary reference.
> **Why this reference:** both are widely used VNG implementations with the
> gradient tables needed to compare this variable-neighbor algorithm.
> **Why we match:** both begin with linear interpolation, score eight
> directional gradients, retain the lower-gradient neighbors, and correct
> missing colors by averaging green-referenced neighbor differences.
> **Known differences:** Fornax maintains a three-row rolling result in
> generic `u16` channels and clamps normalized output before nearest
> quantization. The darktable bridge uses four-channel float pixels and
> normalizes the two green filter codes first. Border windows and filter mask
> conventions are therefore distinct.
