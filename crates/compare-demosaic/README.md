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

Only the native-pattern row is an independent fidelity test. The other three
patterns use the smallest one-pixel shift that makes their CFA layout valid,
and are consistency checks against the same image.

## Summary

<!-- compare-demosaic:begin summary -->
| Algorithm | Reference | Worst verdict | Native-pattern result |
| --- | --- | --- | --- |
| AAHD | libraw | ❌ fail | GBRG ❌ fail |
| AHD | libraw | ✅ pass | GBRG ⚠️ pass with outliers |
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
- A row is `native` only when it uses the input's original CFA layout and crop `[0, 0]`. Other patterns use the smallest valid one-pixel shift and are consistency checks, not independent tests.
- Interior statistics exclude a 16 px border.
- References: LibRaw 0.22.3; darktable 5.6.2.
- Inputs:
  - `colorchart-eos-7d.cr2`: 5202x3464, native `GBRG`, sha256 `4c02a6d7eb8514a441fe0fa81ff79ed3c6f86ebac1c74bc0623655d5ba06e96c`.
<!-- compare-demosaic:end legend -->

## AAHD

<!-- compare-demosaic:begin algo:aahd -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | BGGR | [1, 0] | no | ❌ fail | 103.110399 | 288.014295 | 18089 | 15.741260 | 1327 | 2119 | 1100592 (34793) | 0.629555 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | GBRG | [0, 0] | yes | ❌ fail | 103.105439 | 287.955685 | 18089 | 15.733126 | 1327 | 2118 | 1101122 (35228) | 0.629560 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | GRBG | [1, 1] | no | ❌ fail | 102.968181 | 282.967937 | 12529 | 15.936816 | 1327 | 2117 | 1101178 (37286) | 0.629571 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | RGGB | [0, 1] | no | ❌ fail | 102.954954 | 282.896321 | 12529 | 15.925241 | 1327 | 2116 | 1101523 (37725) | 0.629567 |
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
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | BGGR | [1, 0] | no | ⚠️ pass with outliers | 0.876028 | 24.653182 | 5222 | 0.019920 | 1 | 296 | 6767 (193) | 0.022485 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | GBRG | [0, 0] | yes | ⚠️ pass with outliers | 0.875973 | 24.651269 | 5222 | 0.019917 | 1 | 296 | 6767 (193) | 0.022486 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | GRBG | [1, 1] | no | ⚠️ pass with outliers | 0.875274 | 24.637366 | 5222 | 0.019827 | 1 | 295 | 6754 (190) | 0.022486 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | RGGB | [0, 1] | no | ⚠️ pass with outliers | 0.875219 | 24.635455 | 5222 | 0.019824 | 1 | 295 | 6754 (190) | 0.022486 |
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
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | BGGR | [1, 0] | no | ❌ fail | 79.506490 | 186.017209 | 9623 | 17.580096 | 808 | 1442 | 239832 (10535) | 0.659406 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GBRG | [0, 0] | yes | ❌ fail | 79.498569 | 186.002581 | 9623 | 17.574803 | 808 | 1442 | 239862 (10535) | 0.659406 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GRBG | [1, 1] | no | ❌ fail | 79.472438 | 185.943699 | 9623 | 17.631790 | 808 | 1441 | 239711 (11161) | 0.659409 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | RGGB | [0, 1] | no | ❌ fail | 79.464767 | 185.929591 | 9623 | 17.626376 | 808 | 1441 | 239741 (11160) | 0.659410 |
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
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | BGGR | [1, 0] | no | ❌ fail | 88.015023 | 228.152732 | 7094 | 18.166627 | 1105 | 1902 | 665983 (21079) | 0.668584 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | GBRG | [0, 0] | yes | ❌ fail | 88.009547 | 228.137404 | 7094 | 18.157793 | 1105 | 1902 | 665986 (21082) | 0.668585 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | GRBG | [1, 1] | no | ❌ fail | 87.970957 | 228.041357 | 7866 | 18.306786 | 1104 | 1902 | 664573 (20758) | 0.668591 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | RGGB | [0, 1] | no | ❌ fail | 87.965500 | 228.026068 | 7866 | 18.297917 | 1104 | 1902 | 664576 (20761) | 0.668592 |
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
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | BGGR | [1, 0] | no | ❌ fail | 96.188576 | 219.842766 | 15912 | 1.314828 | 902 | 1616 | 350562 (31447) | 0.662048 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | GBRG | [0, 0] | yes | ❌ fail | 96.496395 | 220.513723 | 15912 | 1.320177 | 906 | 1620 | 356469 (31484) | 0.662065 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | GRBG | [1, 1] | no | ❌ fail | 96.024958 | 212.912355 | 15912 | 1.219283 | 905 | 1604 | 353294 (28814) | 0.662069 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | RGGB | [0, 1] | no | ❌ fail | 95.759780 | 212.259519 | 15912 | 1.223951 | 901 | 1598 | 347343 (28742) | 0.662053 |
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
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | BGGR | [1, 0] | no | 🟢 rounding | 0.294064 | 0.542277 | 1 | 0.294064 | 1 | 1 | 0 (0) | 0.294064 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | GBRG | [0, 0] | yes | 🟢 rounding | 0.294064 | 0.542277 | 1 | 0.294064 | 1 | 1 | 0 (0) | 0.294064 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | GRBG | [1, 1] | no | 🟢 rounding | 0.294068 | 0.542280 | 1 | 0.294068 | 1 | 1 | 0 (0) | 0.294068 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | RGGB | [0, 1] | no | 🟢 rounding | 0.294068 | 0.542280 | 1 | 0.294068 | 1 | 1 | 0 (0) | 0.294068 |
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
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | BGGR | [1, 0] | no | ❌ fail | 44.785192 | 104.000798 | 9522 | 1.146402 | 464 | 885 | 27140 (4408) | 0.657319 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GBRG | [0, 0] | yes | ❌ fail | 44.776609 | 103.955914 | 9522 | 1.144272 | 463 | 884 | 27014 (4421) | 0.657319 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GRBG | [1, 1] | no | ❌ fail | 44.780937 | 104.020885 | 8779 | 1.196977 | 464 | 886 | 27443 (4791) | 0.657324 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | RGGB | [0, 1] | no | ❌ fail | 44.772275 | 103.976072 | 8779 | 1.195146 | 463 | 885 | 27314 (4797) | 0.657324 |
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
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | BGGR | [1, 0] | no | ✅ exact | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | GBRG | [0, 0] | yes | ✅ exact | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | GRBG | [1, 1] | no | ✅ exact | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | RGGB | [0, 1] | no | ✅ exact | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | BGGR | [1, 0] | no | ⚠️ pass with outliers | 0.186705 | 3.106230 | 2221 | 0.108934 | 1 | 1 | 53 (0) | 0.146917 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | GBRG | [0, 0] | yes | ⚠️ pass with outliers | 0.186701 | 3.106662 | 2221 | 0.108924 | 1 | 1 | 53 (0) | 0.146916 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | GRBG | [1, 1] | no | ⚠️ pass with outliers | 0.186698 | 3.106884 | 2221 | 0.108933 | 1 | 1 | 53 (0) | 0.146911 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | RGGB | [0, 1] | no | ⚠️ pass with outliers | 0.186694 | 3.107317 | 2221 | 0.108922 | 1 | 1 | 53 (0) | 0.146910 |
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
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | BGGR | [1, 0] | no | ⚠️ pass with outliers | 0.083022 | 1.012225 | 1072 | -0.000215 | 2 | 11 | 1 (0) | 0.033346 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GBRG | [0, 0] | yes | ⚠️ pass with outliers | 0.083077 | 1.015566 | 1359 | -0.000212 | 2 | 11 | 1 (0) | 0.033374 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GRBG | [1, 1] | no | ⚠️ pass with outliers | 0.083046 | 1.012371 | 1072 | -0.000215 | 2 | 11 | 1 (0) | 0.033355 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | RGGB | [0, 1] | no | ⚠️ pass with outliers | 0.083101 | 1.015712 | 1359 | -0.000212 | 2 | 11 | 1 (0) | 0.033384 |
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
> surrounding metadata pipeline are not shared. The remaining outlier tail is
> confined to interpolated samples near high-contrast chart features; sampled
> Bayer values match the reference exactly.

## VNG

<!-- compare-demosaic:begin algo:vng -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | BGGR | [1, 0] | no | ❌ fail | 2908.244251 | 6278.863563 | 65535 | -205.327950 | 27623 | 41993 | 21565603 (299936) | 0.827316 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | GBRG | [0, 0] | yes | ❌ fail | 2782.662776 | 6477.675994 | 65535 | -687.175060 | 30349 | 45809 | 20998328 (288980) | 0.827285 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | GRBG | [1, 1] | no | ❌ fail | 2906.591015 | 6275.848437 | 65535 | -205.018867 | 27614 | 41987 | 21555394 (299756) | 0.827327 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | RGGB | [0, 1] | no | ❌ fail | 2781.053846 | 6474.143142 | 65535 | -686.327066 | 30332 | 45804 | 20988507 (288866) | 0.827296 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | BGGR | [1, 0] | no | ❌ fail | 2888.494132 | 6278.175031 | 65535 | -205.131550 | 27625 | 41998 | 21537751 (298052) | 0.665242 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | GBRG | [0, 0] | yes | ❌ fail | 2762.780291 | 6476.794907 | 65535 | -686.974152 | 30351 | 45810 | 20978050 (287078) | 0.665211 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | GRBG | [1, 1] | no | ❌ fail | 2886.824296 | 6275.098482 | 65535 | -204.883123 | 27616 | 41993 | 21527687 (297943) | 0.665241 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | RGGB | [0, 1] | no | ❌ fail | 2761.159370 | 6473.230877 | 65535 | -686.186804 | 30333 | 45806 | 20968345 (287052) | 0.665211 |
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
