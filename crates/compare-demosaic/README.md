# compare-demosaic

Compares Fornax demosaic output with LibRaw and darktable at the same stage:
the active Bayer mosaic enters each implementation and reconstructed RGB leaves
it. White balance and the default output crop are not applied.

`baseline/canonical.json` is the source of truth for results. The blocks marked
as generated below are derived from that JSON. Text outside the markers and the
notes beside each algorithm are written by the author and reviewed with the
implementation.

The verdict tables distinguish measured warnings from reviewed decisions. A
reviewed/accepted verdict means the row passes every limit and its remaining
high-error outliers were accepted by a person for explicit count and
maximum-error bounds. These waivers live in the comparison crate's Rust source;
canonical JSON remains purely measured.

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
| AAHD | libraw | ⚖️ | GBRG ⚖️ |
| AHD | libraw | ⚖️ | GBRG ⚖️ |
| AMaZE | darktable | ❌ | GBRG ❌ |
| DCB | libraw | ❌ | GBRG ❌ |
| DHT | libraw | ❌ | GBRG ❌ |
| Linear | libraw | 🟢 | GBRG 🟢 |
| LMMSE | darktable | ❌ | GBRG ❌ |
| PPG | darktable | ✅ | GBRG ✅ |
| PPG | libraw | ⚖️ | GBRG ⚖️ |
| RCD | darktable | ⚖️ | GBRG ⚖️ |
| VNG | darktable | ✅ | GBRG ✅ |
| VNG | libraw | ❌ | GBRG ❌ |
| VNG4 | libraw | ✅ | GBRG ✅ |
<!-- compare-demosaic:end summary -->

## Legend

<!-- compare-demosaic:begin legend -->
| Verdict | Meaning |
| --- | --- |
| `✅` | Exact or passing without high-error outliers. |
| `🟢` | Rounding-only differences. |
| `⚖️` | Every limit passes; high-error outliers remain and were reviewed and accepted for the stated bounds and reason. |
| `⚠️` | Every limit passes; high-error outliers remain and are not yet explained. |
| `❌` | At least one limit failed. |

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
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | BGGR | [1, 0] | no | ⚖️ | 0.274773 | 16.228683 | 5349 | 0.035014 | 0 | 17 | 3399 (69) | 0.001157 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | GBRG | [0, 0] | yes | ⚖️ | 0.283189 | 16.624567 | 5385 | -0.038198 | 0 | 18 | 3645 (67) | 0.001168 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | GRBG | [1, 1] | no | ⚖️ | 0.304623 | 17.566732 | 5941 | 0.037053 | 0 | 20 | 4026 (88) | 0.001190 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | RGGB | [0, 1] | no | ⚖️ | 0.295955 | 17.177471 | 5941 | -0.040052 | 0 | 19 | 3780 (92) | 0.001178 |

Accepted outliers (aahd vs LibRaw, all patterns): at most 4026 samples above 1024, maximum error at most 5941. Reason: Independent floating-point homogeneity and gradient comparisons can select the other AAHD directional candidate at sparse, high-frequency and clipped chart details. Evidence: The full-resolution error map was reviewed in all four CFA phases: every robust limit passes, sampled Bayer sites are exact, all high-error values are interpolated-site outliers, and the map is sparse point noise without a border strip or full-frame structure. The reviewed maxima are 4026 samples above 1024 and maximum error 5941, with at most 32 high-error samples in any 64x64 tile (GRBG).
<!-- compare-demosaic:end algo:aahd -->

> [!NOTE]
> **Reference:** LibRaw, `aahd_interpolate`.
> **Why this reference:** it is a widely used advanced AHD implementation
> with an independently maintained interpolation pipeline.
> **Why we match:** Fornax ports AAHD's four-pixel padded working planes,
> hot-pixel masking, horizontal and vertical green candidates, directional
> red/blue reconstruction, 16-bit gamma and YUV homogeneity scoring, direction
> refinement, and per-pixel candidate selection. Both use LibRaw's channel
> extrema, integer clamps, and truncating `ushort` output.
> **Known differences:** the comparison bridge supplies an identity
> camera-to-RGB matrix and no white balance, so only the demosaic stage is
> compared. Fornax accepts conventional 2x2 RGB Bayer input only and does not
> reproduce LibRaw's camera metadata or output-color pipeline. Sparse
> high-frequency and clipped-chart decisions can remain different when an
> independent floating-point evaluation lands on the other side of a
> homogeneity or gradient comparison.

## AHD

<!-- compare-demosaic:begin algo:ahd -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | BGGR | [1, 0] | no | ⚖️ | 0.876028 | 24.653182 | 5222 | 0.019920 | 1 | 296 | 6767 (193) | 0.022485 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | GBRG | [0, 0] | yes | ⚖️ | 0.875973 | 24.651269 | 5222 | 0.019917 | 1 | 296 | 6767 (193) | 0.022486 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | GRBG | [1, 1] | no | ⚖️ | 0.875274 | 24.637366 | 5222 | 0.019827 | 1 | 295 | 6754 (190) | 0.022486 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | RGGB | [0, 1] | no | ⚖️ | 0.875219 | 24.635455 | 5222 | 0.019824 | 1 | 295 | 6754 (190) | 0.022486 |

Accepted outliers (ahd vs LibRaw, all patterns): at most 6767 samples above 1024, maximum error at most 5222. Reason: Independent AHD tile, clipping, matrix, and quantization conventions change local directional decisions at high-contrast chart edges. Evidence: PR #602's valid-crop experiment has the same outlier tail in all four CFA phases with exact sampled Bayer values; the former BGGR and RGGB failures were invalid green-phase test input.
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
> PR #602's valid-crop experiment shows the remaining outlier tail is stable
> across all four CFA phases and sampled Bayer values are exact. The former
> BGGR/RGGB failures were invalid green-phase test input; the remaining
> high-contrast outliers are accepted under the bounded waiver below the table.

## AMaZE

<!-- compare-demosaic:begin algo:amaze -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | BGGR | [1, 0] | no | ❌ | 79.506490 | 186.017209 | 9623 | 17.580096 | 808 | 1442 | 239832 (10535) | 0.659406 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GBRG | [0, 0] | yes | ❌ | 79.498569 | 186.002581 | 9623 | 17.574803 | 808 | 1442 | 239862 (10535) | 0.659406 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GRBG | [1, 1] | no | ❌ | 79.472438 | 185.943699 | 9623 | 17.631790 | 808 | 1441 | 239711 (11161) | 0.659409 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | RGGB | [0, 1] | no | ❌ | 79.464767 | 185.929591 | 9623 | 17.626376 | 808 | 1441 | 239741 (11160) | 0.659410 |
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
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | BGGR | [1, 0] | no | ❌ | 88.015023 | 228.152732 | 7094 | 18.166627 | 1105 | 1902 | 665983 (21079) | 0.668584 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | GBRG | [0, 0] | yes | ❌ | 88.009547 | 228.137404 | 7094 | 18.157793 | 1105 | 1902 | 665986 (21082) | 0.668585 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | GRBG | [1, 1] | no | ❌ | 87.970957 | 228.041357 | 7866 | 18.306786 | 1104 | 1902 | 664573 (20758) | 0.668591 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | RGGB | [0, 1] | no | ❌ | 87.965500 | 228.026068 | 7866 | 18.297917 | 1104 | 1902 | 664576 (20761) | 0.668592 |
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
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | BGGR | [1, 0] | no | ❌ | 96.188576 | 219.842766 | 15912 | 1.314828 | 902 | 1616 | 350562 (31447) | 0.662048 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | GBRG | [0, 0] | yes | ❌ | 96.496395 | 220.513723 | 15912 | 1.320177 | 906 | 1620 | 356469 (31484) | 0.662065 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | GRBG | [1, 1] | no | ❌ | 96.024958 | 212.912355 | 15912 | 1.219283 | 905 | 1604 | 353294 (28814) | 0.662069 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | RGGB | [0, 1] | no | ❌ | 95.759780 | 212.259519 | 15912 | 1.223951 | 901 | 1598 | 347343 (28742) | 0.662053 |
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
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | BGGR | [1, 0] | no | 🟢 | 0.294064 | 0.542277 | 1 | 0.294064 | 1 | 1 | 0 (0) | 0.294064 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | GBRG | [0, 0] | yes | 🟢 | 0.294064 | 0.542277 | 1 | 0.294064 | 1 | 1 | 0 (0) | 0.294064 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | GRBG | [1, 1] | no | 🟢 | 0.294068 | 0.542280 | 1 | 0.294068 | 1 | 1 | 0 (0) | 0.294068 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | RGGB | [0, 1] | no | 🟢 | 0.294068 | 0.542280 | 1 | 0.294068 | 1 | 1 | 0 (0) | 0.294068 |
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
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | BGGR | [1, 0] | no | ❌ | 44.785192 | 104.000798 | 9522 | 1.146402 | 464 | 885 | 27140 (4408) | 0.657319 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GBRG | [0, 0] | yes | ❌ | 44.776609 | 103.955914 | 9522 | 1.144272 | 463 | 884 | 27014 (4421) | 0.657319 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GRBG | [1, 1] | no | ❌ | 44.780937 | 104.020885 | 8779 | 1.196977 | 464 | 886 | 27443 (4791) | 0.657324 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | RGGB | [0, 1] | no | ❌ | 44.772275 | 103.976072 | 8779 | 1.195146 | 463 | 885 | 27314 (4797) | 0.657324 |
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
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | BGGR | [1, 0] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | GBRG | [0, 0] | yes | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | GRBG | [1, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | RGGB | [0, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | BGGR | [1, 0] | no | ⚖️ | 0.186705 | 3.106230 | 2221 | 0.108934 | 1 | 1 | 53 (0) | 0.146917 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | GBRG | [0, 0] | yes | ⚖️ | 0.186701 | 3.106662 | 2221 | 0.108924 | 1 | 1 | 53 (0) | 0.146916 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | GRBG | [1, 1] | no | ⚖️ | 0.186698 | 3.106884 | 2221 | 0.108933 | 1 | 1 | 53 (0) | 0.146911 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | RGGB | [0, 1] | no | ⚖️ | 0.186694 | 3.107317 | 2221 | 0.108922 | 1 | 1 | 53 (0) | 0.146910 |

Accepted outliers (ppg vs LibRaw, all patterns): at most 53 samples above 1024, maximum error at most 2221. Reason: LibRaw selects PPG directions with integer shifts and truncating arithmetic while Fornax follows darktable's normalized f32 path; near ties at high-contrast chart edges flip a small local set of directional choices. Evidence: The committed baseline matches darktable exactly in all CFA phases, preserves sampled Bayer sites exactly, and has the same 53 interior high-error samples and maximum error 2221 in every phase; each row's top-20 worst samples align after its crop offset, and the error map is sparse. LibRaw 0.22.3 ppg_interpolate uses right shifts and integer division for its predictions and diagonal tie.
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
> rounding can therefore differ at code boundaries. LibRaw also uses right
> shifts and truncating integer arithmetic for its PPG predictions and
> diagonal tie, while Fornax follows darktable's normalized `f32` path. Near
> ties at high-contrast chart edges can therefore select different local
> directions. The same bounded outlier tail appears in all four CFA phases and
> is accepted below the table.

## RCD

<!-- compare-demosaic:begin algo:rcd -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | BGGR | [1, 0] | no | ⚖️ | 0.083022 | 1.012225 | 1072 | -0.000215 | 2 | 11 | 1 (0) | 0.033346 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GBRG | [0, 0] | yes | ⚖️ | 0.083077 | 1.015566 | 1359 | -0.000212 | 2 | 11 | 1 (0) | 0.033374 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GRBG | [1, 1] | no | ⚖️ | 0.083046 | 1.012371 | 1072 | -0.000215 | 2 | 11 | 1 (0) | 0.033355 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | RGGB | [0, 1] | no | ⚖️ | 0.083101 | 1.015712 | 1359 | -0.000212 | 2 | 11 | 1 (0) | 0.033384 |

Accepted outliers (rcd vs darktable, all patterns): at most 1 samples above 1024, maximum error at most 1359. Reason: A near-tie RCD interpolation decision flips under independent floating-point evaluation at one high-contrast chart edge. Evidence: Every CFA phase has one interior high-error sample at the same shifted red-site edge; sampled Bayer values are exact.
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
> Bayer values match the reference exactly. The sole high-error sample in each
> CFA phase is the same shifted red-site chart edge, accepted as floating-point
> evaluation sensitivity in a near-tie interpolation decision.

## VNG

<!-- compare-demosaic:begin algo:vng -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | BGGR | [1, 0] | no | ✅ | 0.385723 | 1.245002 | 733 | 0.014067 | 1 | 9 | 0 (0) | 0.358690 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | GBRG | [0, 0] | yes | ✅ | 0.385726 | 1.244920 | 733 | 0.014069 | 1 | 9 | 0 (0) | 0.358696 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | GRBG | [1, 1] | no | ✅ | 0.385724 | 1.244787 | 733 | 0.014062 | 1 | 9 | 0 (0) | 0.358699 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | RGGB | [0, 1] | no | ✅ | 0.385727 | 1.244705 | 733 | 0.014064 | 1 | 9 | 0 (0) | 0.358705 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | BGGR | [1, 0] | no | ❌ | 62.978851 | 151.418190 | 6698 | 0.210467 | 692 | 1105 | 87273 (4831) | 0.752408 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | GBRG | [0, 0] | yes | ❌ | 62.973616 | 151.406061 | 6698 | 0.214976 | 692 | 1105 | 87273 (4831) | 0.752412 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | GRBG | [1, 1] | no | ❌ | 62.941900 | 151.318005 | 6698 | 0.149805 | 692 | 1104 | 86874 (4704) | 0.752458 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | RGGB | [0, 1] | no | ❌ | 62.936663 | 151.305885 | 6698 | 0.154326 | 692 | 1104 | 86874 (4704) | 0.752462 |
<!-- compare-demosaic:end algo:vng -->

> [!NOTE]
> **Reference:** darktable `vng_interpolate` is authoritative. LibRaw
> `vng_interpolate` is retained as informational evidence for the original
> dcraw-family algorithm.
> **Why this reference:** darktable defines the split-green VNG behavior that
> Fornax targets. LibRaw provides a second widely used implementation for
> understanding shared algorithmic structure, but not an acceptance gate.
> **Why we match:** both references begin with linear interpolation, score
> eight directional gradients, retain lower-gradient neighbors, and correct
> missing colors through green-referenced neighbor differences.
> **Known differences:** darktable keeps the two green CFA phases in separate
> planes through interpolation and averages them only when packing RGB.
> LibRaw's comparison adapter sets `four_color_rgb = 0`, so it retains one
> green plane and intentionally produces a materially different result.
> Fornax follows darktable's split-green convention with a three-row rolling
> result in generic `u16` channels; the bridge uses four-channel float pixels.
> Border windows and fixed-point clipping can therefore produce sparse
> high-value differences even when the aggregate error is small. See
> [the VNG reference notes](../../docs/references/vng.md) for source links and
> implementation details.

## VNG4

<!-- compare-demosaic:begin algo:vng4 -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | BGGR | [1, 0] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | GBRG | [0, 0] | yes | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | GRBG | [1, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | RGGB | [0, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
<!-- compare-demosaic:end algo:vng4 -->

> [!NOTE]
> **Reference:** LibRaw, `vng_interpolate` after `pre_interpolate` with
> `four_color_rgb = 1`.
> **Why this reference:** this is LibRaw's explicit four-color VNG mode, which
> keeps the two green CFA phases separate through interpolation.
> **Why we match:** both implementations run VNG with split green planes and
> average those planes into the final green channel only after interpolation.
> **Known differences:** Fornax shares its integer split-green VNG kernel with
> the `Demosaic::Vng` selector and exposes VNG4 as a distinct algorithm choice.
> The comparison adapter sets LibRaw's four-color option before
> `pre_interpolate` and performs the documented post-interpolation green mix;
> no source code is copied into the postprocessor.
