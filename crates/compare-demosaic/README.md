# compare-demosaic

Compares Fornax demosaic output with LibRaw, darktable, and RawTherapee at the
same stage: the active Bayer mosaic enters each implementation and reconstructed
RGB leaves it. White balance and the default output crop are not applied.

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
| AHD | rawtherapee | ❌ | GBRG ❌ |
| AMaZE | darktable | ✅ | GBRG ✅ |
| AMaZE | rawtherapee | ✅ | GBRG ✅ |
| DCB | libraw | ✅ | GBRG ✅ |
| DCB | rawtherapee | ❌ | GBRG ❌ |
| DHT | libraw | ❌ | GBRG ❌ |
| Linear | libraw | 🟢 | GBRG 🟢 |
| LMMSE | darktable | ✅ | GBRG ✅ |
| LMMSE | rawtherapee | ❌ | GBRG ❌ |
| PPG | darktable | ✅ | GBRG ✅ |
| PPG | libraw | ⚖️ | GBRG ⚖️ |
| RCD | darktable | ⚖️ | GBRG ⚖️ |
| RCD | rawtherapee | ⚠️ | GBRG ⚠️ |
| VNG | darktable | ✅ | GBRG ✅ |
| VNG | libraw | ❌ | GBRG ❌ |
| VNG4 | libraw | ✅ | GBRG ✅ |
| VNG4 | rawtherapee | ❌ | GBRG ❌ |
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
<!-- compare-demosaic:end algo:aahd -->

### LibRaw `aahd_interpolate`

**Why this reference:** it is a widely used advanced AHD implementation
with an independently maintained interpolation pipeline.

**Why we match:** Fornax ports AAHD's four-pixel padded working planes,
hot-pixel masking, horizontal and vertical green candidates, directional
red/blue reconstruction, 16-bit gamma and YUV homogeneity scoring, direction
refinement, and per-pixel candidate selection. Both use LibRaw's channel
extrema, integer clamps, and truncating `ushort` output.

**Known differences:** the comparison bridge supplies an identity
camera-to-RGB matrix and no white balance, so only the demosaic stage is
compared. Fornax accepts conventional 2x2 RGB Bayer input only and does not
reproduce LibRaw's camera metadata or output-color pipeline. Sparse
high-frequency and clipped-chart decisions can remain different when an
independent floating-point evaluation lands on the other side of a
homogeneity or gradient comparison.

**Accepted outliers:** aahd vs LibRaw, all patterns: at most 4026 samples above 1024, maximum error at most 5941. Reason: Independent floating-point homogeneity and gradient comparisons can select the other AAHD directional candidate at sparse, high-frequency and clipped chart details. Evidence: The full-resolution error map was reviewed in all four CFA phases: every robust limit passes, sampled Bayer sites are exact, all high-error values are interpolated-site outliers, and the map is sparse point noise without a border strip or full-frame structure. The reviewed maxima are 4026 samples above 1024 and maximum error 5941, with at most 32 high-error samples in any 64x64 tile (GRBG).

## AHD

<!-- compare-demosaic:begin algo:ahd -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | BGGR | [1, 0] | no | ⚖️ | 0.876028 | 24.653182 | 5222 | 0.019920 | 1 | 296 | 6767 (193) | 0.022485 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | GBRG | [0, 0] | yes | ⚖️ | 0.875973 | 24.651269 | 5222 | 0.019917 | 1 | 296 | 6767 (193) | 0.022486 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | GRBG | [1, 1] | no | ⚖️ | 0.875274 | 24.637366 | 5222 | 0.019827 | 1 | 295 | 6754 (190) | 0.022486 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | RGGB | [0, 1] | no | ⚖️ | 0.875219 | 24.635455 | 5222 | 0.019824 | 1 | 295 | 6754 (190) | 0.022486 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/ahd_demosaic_RT.cc#L44) | BGGR | [1, 0] | no | ❌ | 6.723789 | 62.199928 | 5841 | -0.122462 | 154 | 926 | 41372 (854) | 0.196035 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/ahd_demosaic_RT.cc#L44) | GBRG | [0, 0] | yes | ❌ | 6.723326 | 62.195036 | 5841 | -0.122399 | 154 | 926 | 41372 (854) | 0.196034 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/ahd_demosaic_RT.cc#L44) | GRBG | [1, 1] | no | ❌ | 6.720516 | 62.159150 | 5841 | -0.122432 | 154 | 925 | 41296 (841) | 0.196038 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/ahd_demosaic_RT.cc#L44) | RGGB | [0, 1] | no | ❌ | 6.720053 | 62.154262 | 5841 | -0.122369 | 154 | 925 | 41296 (841) | 0.196037 |
<!-- compare-demosaic:end algo:ahd -->

### LibRaw `ahd_interpolate`

**Why this reference:** it is the established AHD implementation used by
other RAW processors and has a stable pinned source revision.

**Why we match:** Fornax builds horizontal and vertical green candidates,
evaluates both in Lab with the same homogeneity idea and neighbor threshold
comparisons, then selects the locally consistent direction.

**Known differences:** Fornax uses the published camera-to-XYZ matrix,
512-pixel working tiles, floor intermediates and nearest-neighbor output
quantization. LibRaw owns its tile, clipping and final quantization
conventions.
PR #602's valid-crop experiment shows the remaining outlier tail is stable
across all four CFA phases and sampled Bayer values are exact. The former
BGGR/RGGB failures were invalid green-phase test input.

**Accepted outliers:** ahd vs LibRaw, all patterns: at most 6767 samples above 1024, maximum error at most 5222. Reason: Independent AHD tile, clipping, matrix, and quantization conventions change local directional decisions at high-contrast chart edges. Evidence: PR #602's valid-crop experiment has the same outlier tail in all four CFA phases with exact sampled Bayer values; the former BGGR and RGGB failures were invalid green-phase test input.

### RawTherapee `ahd_demosaic`

**Why this reference:** it provides an independently maintained AHD pipeline
with RawTherapee's own tile and Lab conventions.

**Why we match:** both reconstruct horizontal and vertical green candidates,
evaluate directional homogeneity in Lab, and select the locally consistent
direction for the remaining channels.

**Known differences:** RawTherapee normally receives channel-scaled working
data after `scaleColors`, while the comparison supplies the common active
mosaic without white balance. Its tile boundaries, matrix handling, clipping,
and final quantization also differ from Fornax.

## AMaZE

<!-- compare-demosaic:begin algo:amaze -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | BGGR | [1, 0] | no | ✅ | 0.230944 | 3.815430 | 890 | -0.001803 | 1 | 53 | 0 (0) | 0.011564 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GBRG | [0, 0] | yes | ✅ | 0.230396 | 3.806977 | 890 | -0.001913 | 1 | 53 | 0 (0) | 0.011536 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GRBG | [1, 1] | no | ✅ | 0.195362 | 3.501375 | 878 | -0.001293 | 1 | 48 | 0 (0) | 0.010065 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | RGGB | [0, 1] | no | ✅ | 0.194886 | 3.493895 | 878 | -0.001508 | 1 | 48 | 0 (0) | 0.010034 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | BGGR | [1, 0] | no | ✅ | 0.230944 | 3.815430 | 890 | -0.001803 | 1 | 53 | 0 (0) | 0.011564 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | GBRG | [0, 0] | yes | ✅ | 0.230396 | 3.806977 | 890 | -0.001913 | 1 | 53 | 0 (0) | 0.011536 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | GRBG | [1, 1] | no | ✅ | 0.195362 | 3.501375 | 878 | -0.001293 | 1 | 48 | 0 (0) | 0.010065 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | RGGB | [0, 1] | no | ✅ | 0.194886 | 3.493895 | 878 | -0.001508 | 1 | 48 | 0 (0) | 0.010034 |
<!-- compare-demosaic:end algo:amaze -->

### darktable `amaze_demosaic`

**Why this reference:** darktable is the primary open-source AMaZE
implementation used for reproducing this published algorithm.

**Why we match:** both preserve measured CFA values and use the same 160-pixel
tiles, adaptive green ratios, directional variance weighting, Nyquist texture
refinement, diagonal red/blue correction, and final chroma interpolation.

**Known differences:** darktable runs this stage inside its pixelpipe, while
Fornax accepts normalized active Bayer data directly. The comparison pins an
identity scale and clipping point so both implementations receive the same
mosaic. Floating-point operation ordering and the final nearest-code
conversion account for the remaining one-code differences.

### RawTherapee `amaze_demosaic_RT`

**Why this reference:** it exercises RawTherapee's independently optimized
AMaZE implementation and its own tile and clipping decisions.

**Why we match:** both implement the complete AMaZE tile pipeline and preserve
measured CFA samples. The comparison facade uses RawTherapee's scalar path,
exponent-shift division helper, identity matrix, and normalized 16-bit working
scale.

**Known differences:** RawTherapee normally receives channel-scaled working
data from `scaleColors`; the comparison deliberately omits white balance.
Fornax clamps in normalized space and quantizes to nearest. Floating-point
operation ordering and that final conversion account for the remaining
one-code differences.

## DCB

<!-- compare-demosaic:begin algo:dcb -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | BGGR | [1, 0] | no | ✅ | 0.000258 | 0.016083 | 2 | -0.000001 | 0 | 0 | 0 (0) | 0.000258 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | GBRG | [0, 0] | yes | ✅ | 0.000258 | 0.016073 | 2 | -0.000001 | 0 | 0 | 0 (0) | 0.000257 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | GRBG | [1, 1] | no | ✅ | 0.000258 | 0.016074 | 2 | -0.000001 | 0 | 0 | 0 (0) | 0.000257 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | RGGB | [0, 1] | no | ✅ | 0.000257 | 0.016064 | 2 | -0.000001 | 0 | 0 | 0 (0) | 0.000257 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/demosaic_algos.cc#L1406) | BGGR | [1, 0] | no | ❌ | 66.609247 | 209.744859 | 6728 | -4.416030 | 1018 | 2118 | 532343 (17329) | 0.650392 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/demosaic_algos.cc#L1406) | GBRG | [0, 0] | yes | ❌ | 66.600871 | 209.719330 | 6728 | -4.421547 | 1017 | 2117 | 532258 (17324) | 0.650393 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/demosaic_algos.cc#L1406) | GRBG | [1, 1] | no | ❌ | 66.584711 | 209.700425 | 6728 | -4.425208 | 1017 | 2117 | 531894 (17634) | 0.650415 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/demosaic_algos.cc#L1406) | RGGB | [0, 1] | no | ❌ | 66.576286 | 209.673892 | 6728 | -4.431613 | 1017 | 2117 | 531807 (17627) | 0.650416 |
<!-- compare-demosaic:end algo:dcb -->

### LibRaw `dcb`

**Why this reference:** it is the maintained implementation of directional
color balance demosaicing and exposes the complete LibRaw pipeline.

**Why we match:** Fornax follows LibRaw's `pre_interpolate()` plus
`dcb(0, 1)` sequence: six-pixel CFA border interpolation, horizontal and
vertical candidate reconstruction, direction selection, green corrections,
refinement, and full-color chroma interpolation.

**Known differences:** Fornax omits camera metadata handling, callbacks, and
LibRaw's surrounding processing pipeline. The comparison starts from the same
normalized active Bayer mosaic and copies the resulting RGB channels
directly.

### RawTherapee `dcb_demosaic`

**Why this reference:** it provides a second maintained DCB implementation
with independent border and color-difference handling.

**Why we match:** both interpolate directional green candidates, build a
correction map, and reconstruct red and blue from green-referenced color
differences.

**Known differences:** The comparison bridge pins RawTherapee's compatible
zero-iteration enhanced path and omits its preprocessing and output stages.
Fornax targets LibRaw's fixed DCB variant, so RawTherapee records its distinct
directional decisions rather than serving as the acceptance reference.

## DHT

<!-- compare-demosaic:begin algo:dht -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | BGGR | [1, 0] | no | ❌ | 96.188576 | 219.842766 | 15912 | 1.314828 | 902 | 1616 | 350562 (31447) | 0.662048 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | GBRG | [0, 0] | yes | ❌ | 96.496395 | 220.513723 | 15912 | 1.320177 | 906 | 1620 | 356469 (31484) | 0.662065 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | GRBG | [1, 1] | no | ❌ | 96.024958 | 212.912355 | 15912 | 1.219283 | 905 | 1604 | 353294 (28814) | 0.662069 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | RGGB | [0, 1] | no | ❌ | 95.759780 | 212.259519 | 15912 | 1.223951 | 901 | 1598 | 347343 (28742) | 0.662053 |
<!-- compare-demosaic:end algo:dht -->

### LibRaw `dht_interpolate`

**Why this reference:** it provides the directional homogeneity transform
implementation targeted by Fornax.

**Why we match:** both preserve sampled Bayer values, derive candidate green
values in both directions, and use green-referenced color differences for
red and blue reconstruction.

**Known differences:** Fornax's shared directional profile uses an
edge-weight exponent of one and a two-pixel chroma radius. LibRaw DHT runs
its own directional transform and refinement stages. Intermediate clipping,
border fill and final quantization are therefore not shared.

## Linear

<!-- compare-demosaic:begin algo:linear -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | BGGR | [1, 0] | no | 🟢 | 0.294064 | 0.542277 | 1 | 0.294064 | 1 | 1 | 0 (0) | 0.294064 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | GBRG | [0, 0] | yes | 🟢 | 0.294064 | 0.542277 | 1 | 0.294064 | 1 | 1 | 0 (0) | 0.294064 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | GRBG | [1, 1] | no | 🟢 | 0.294068 | 0.542280 | 1 | 0.294068 | 1 | 1 | 0 (0) | 0.294068 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | RGGB | [0, 1] | no | 🟢 | 0.294068 | 0.542280 | 1 | 0.294068 | 1 | 1 | 0 (0) | 0.294068 |
<!-- compare-demosaic:end algo:linear -->

### LibRaw `lin_interpolate`

**Why this reference:** it isolates the basic bilinear interpolation that
provides a simple convention check for the comparison harness.

**Why we match:** both preserve the measured sample, average horizontal and
vertical green neighbors at red or blue sites, and average diagonals for the
missing opposite color.

**Known differences:** Fornax produces typed floating-point samples, clamps
them to `[0, 1]`, and rounds to nearest before 16-bit quantization. LibRaw's
reference path truncates its final integer conversion; its border handling
also comes from the LibRaw pipeline.

## LMMSE

<!-- compare-demosaic:begin algo:lmmse -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | BGGR | [1, 0] | no | ✅ | 0.000434 | 0.020821 | 1 | -0.000001 | 0 | 0 | 0 (0) | 0.000434 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GBRG | [0, 0] | yes | ✅ | 0.000433 | 0.020814 | 1 | -0.000001 | 0 | 0 | 0 (0) | 0.000433 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GRBG | [1, 1] | no | ✅ | 0.000432 | 0.020790 | 1 | -0.000004 | 0 | 0 | 0 (0) | 0.000432 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | RGGB | [0, 1] | no | ✅ | 0.000432 | 0.020785 | 1 | -0.000003 | 0 | 0 | 0 (0) | 0.000432 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | BGGR | [1, 0] | no | ❌ | 12.900820 | 94.391841 | 16954 | -0.847148 | 158 | 567 | 17216 (12342) | 0.493142 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | GBRG | [0, 0] | yes | ❌ | 12.897497 | 94.205491 | 16954 | -0.850165 | 157 | 570 | 17354 (12506) | 0.493153 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | GRBG | [1, 1] | no | ❌ | 12.814166 | 89.458150 | 16228 | -0.758003 | 157 | 561 | 16524 (11807) | 0.493129 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | RGGB | [0, 1] | no | ❌ | 12.812612 | 89.291776 | 16228 | -0.761280 | 157 | 564 | 16709 (12003) | 0.493143 |
<!-- compare-demosaic:end algo:lmmse -->

### darktable `lmmse_demosaic`

**Why this reference:** darktable is the maintained LMMSE implementation
used for the reference comparison.

**Why we match:** both seed from PPG, transform the CFA through the same
linear/gamma tables, estimate horizontal and vertical color differences,
apply the nine-tap low-pass and directional MMSE blend, reconstruct red and
blue from green differences, and run one median pass. Both use 136-pixel
tiles with a four-pixel border, eight-pixel overlap, and a 112-pixel valid
stride.

**Known differences:** Fornax keeps the kernel in Rust `f32` and quantizes
once at the comparison boundary. Compiler contraction can leave one-code
differences from darktable; measured Bayer sites remain exact.

### RawTherapee `lmmse_interpolate_omp`

**Why this reference:** it provides a second maintained LMMSE implementation
with an independently structured gamma-table and tile pipeline.

**Why we match:** both seed from PPG, estimate directional color differences,
apply the nine-tap MMSE blend, and reconstruct red and blue from green.

**Known differences:** RawTherapee uses its own gamma LUT and one comparison
iteration without white balance or its preprocess/output stages. These table
and pipeline conventions can leave larger differences than darktable while
sampled Bayer sites remain exact.

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
<!-- compare-demosaic:end algo:ppg -->

### darktable `demosaic_ppg`

**Why this reference:** it exposes the patterned pixel-grouping algorithm
that Fornax implements and is the acceptance reference.

**Why we match:** the green estimate uses the same horizontal and vertical
gradient differences, candidate clamping and color-difference correction
used to reconstruct red and blue at green and opposite-color sites.

**Known differences:** Fornax computes a three-pixel border fallback from the
nearest matching color samples and consumes the Bayer pattern directly.
darktable bridges normalize both green CFA codes and run its own border
margin. Final integer rounding can therefore differ at code boundaries.

### LibRaw `ppg_interpolate`

**Why this reference:** it is a second widely used PPG implementation and
serves as an informational convention check rather than the acceptance gate.

**Why we match:** both use horizontal and vertical green gradients, candidate
clamping, and green-referenced color differences to reconstruct red and blue.

**Known differences:** LibRaw applies its pipeline around `ppg_interpolate`
and uses right shifts and truncating integer arithmetic for predictions and
diagonal ties, while Fornax follows darktable's normalized `f32` path. Near
ties at high-contrast chart edges can therefore select different directions;
the bounded outlier tail is accepted as follows.

**Accepted outliers:** ppg vs LibRaw, all patterns: at most 53 samples above 1024, maximum error at most 2221. Reason: LibRaw selects PPG directions with integer shifts and truncating arithmetic while Fornax follows darktable's normalized f32 path; near ties at high-contrast chart edges flip a small local set of directional choices. Evidence: The committed baseline matches darktable exactly in all CFA phases, preserves sampled Bayer sites exactly, and has the same 53 interior high-error samples and maximum error 2221 in every phase; each row's top-20 worst samples align after its crop offset, and the error map is sparse. LibRaw 0.22.3 ppg_interpolate uses right shifts and integer division for its predictions and diagonal tie.

## RCD

<!-- compare-demosaic:begin algo:rcd -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | BGGR | [1, 0] | no | ⚖️ | 0.083022 | 1.012225 | 1072 | -0.000215 | 2 | 11 | 1 (0) | 0.033346 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GBRG | [0, 0] | yes | ⚖️ | 0.083077 | 1.015566 | 1359 | -0.000212 | 2 | 11 | 1 (0) | 0.033374 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GRBG | [1, 1] | no | ⚖️ | 0.083046 | 1.012371 | 1072 | -0.000215 | 2 | 11 | 1 (0) | 0.033355 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | RGGB | [0, 1] | no | ⚖️ | 0.083101 | 1.015712 | 1359 | -0.000212 | 2 | 11 | 1 (0) | 0.033384 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | BGGR | [1, 0] | no | ⚠️ | 0.870757 | 23.378354 | 3727 | 0.022365 | 3 | 196 | 7180 (7179) | 0.038540 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | GBRG | [0, 0] | yes | ⚠️ | 0.871883 | 23.391745 | 3727 | 0.025217 | 3 | 196 | 7184 (7183) | 0.038567 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | GRBG | [1, 1] | no | ⚠️ | 0.873075 | 23.445153 | 3727 | 0.000155 | 3 | 197 | 7266 (7265) | 0.038554 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | RGGB | [0, 1] | no | ⚠️ | 0.874198 | 23.458456 | 3727 | 0.003023 | 3 | 197 | 7270 (7269) | 0.038579 |
<!-- compare-demosaic:end algo:rcd -->

### darktable `rcd_demosaic`

**Why this reference:** darktable provides the pinned RCD 2.3 implementation
against which the independently structured Fornax version is compared.

**Why we match:** both seed with PPG, use the same 112-pixel tile geometry,
compute directional high-pass gradient confidence, interpolate ratio-corrected
green, and then reconstruct diagonal color differences.

**Known differences:** Fornax evaluates the formulas through normalized
floating-point Rust and clamps final output to `[0, 1]`; the darktable bridge
quantizes after the same clamp. Border fill and the surrounding metadata
pipeline are not shared. The remaining outlier tail is confined to
interpolated samples near high-contrast chart features; sampled Bayer values
match the reference exactly. The sole high-error sample in each CFA phase is
the same shifted red-site chart edge, accepted as floating-point evaluation
sensitivity in a near-tie interpolation decision.

**Accepted outliers:** rcd vs darktable, all patterns: at most 1 samples above 1024, maximum error at most 1359. Reason: A near-tie RCD interpolation decision flips under independent floating-point evaluation at one high-contrast chart edge. Evidence: Every CFA phase has one interior high-error sample at the same shifted red-site edge; sampled Bayer values are exact.

### RawTherapee `rcd_demosaic`

**Why this reference:** it provides an independently optimized RCD pipeline
for comparison against Fornax's darktable-compatible implementation.

**Why we match:** both seed with PPG, estimate directional gradient
confidence, reconstruct ratio-corrected green, and then interpolate diagonal
color differences.

**Known differences:** RawTherapee uses its own tile processing without white
balance or its preprocess/output stages. Fornax targets darktable's normalized
`f32` RCD path, so RawTherapee's near-tie decisions can produce a larger
interpolated-sample outlier tail.

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

### darktable `vng_interpolate`

**Why this reference:** darktable defines the split-green VNG behavior that
Fornax targets.

**Why we match:** both begin with linear interpolation, score eight
directional gradients, retain lower-gradient neighbors, and correct missing
colors through green-referenced neighbor differences.

**Known differences:** darktable keeps the two green CFA phases in separate
planes through interpolation and averages them only when packing RGB. Fornax
follows that convention with a three-row rolling result in generic `u16`
channels; the bridge uses four-channel float pixels. Border windows and
fixed-point clipping can therefore produce sparse high-value differences
even when the aggregate error is small. See
[the VNG reference notes](../../docs/references/vng.md) for source links and
implementation details.

### LibRaw `vng_interpolate`

**Why this reference:** it is retained as informational evidence for the
original dcraw-family algorithm, not as an acceptance gate.

**Why we match:** both score eight directional gradients, retain
lower-gradient neighbors, and reconstruct missing colors through
green-referenced neighbor differences.

**Known differences:** LibRaw's comparison adapter sets `four_color_rgb = 0`,
so it retains one green plane and intentionally produces a materially
different result from Fornax's split-green darktable-compatible path.

## VNG4

<!-- compare-demosaic:begin algo:vng4 -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | BGGR | [1, 0] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | GBRG | [0, 0] | yes | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | GRBG | [1, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | RGGB | [0, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | BGGR | [1, 0] | no | ❌ | 17.032370 | 48.069389 | 7967 | 0.295775 | 179 | 452 | 10183 (594) | 0.634070 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | GBRG | [0, 0] | yes | ❌ | 17.030921 | 48.063943 | 7967 | 0.298141 | 179 | 452 | 10183 (594) | 0.634068 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | GRBG | [1, 1] | no | ❌ | 17.027638 | 48.065447 | 7967 | 0.248746 | 179 | 452 | 10152 (563) | 0.634084 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | RGGB | [0, 1] | no | ❌ | 17.026189 | 48.060010 | 7967 | 0.251119 | 179 | 452 | 10152 (563) | 0.634082 |
<!-- compare-demosaic:end algo:vng4 -->

### LibRaw `vng_interpolate`

**Why this reference:** this is LibRaw's explicit four-color VNG mode, which
keeps the two green CFA phases separate through interpolation.

**Why we match:** both implementations run VNG with split green planes and
average those planes into the final green channel only after interpolation.

**Known differences:** Fornax shares its integer split-green VNG kernel with
the `Demosaic::Vng` selector and exposes VNG4 as a distinct algorithm choice.
The comparison adapter sets LibRaw's four-color option before
`pre_interpolate` and performs the documented post-interpolation green mix.
No source code is copied into the postprocessor.

### RawTherapee `vng4_demosaic`

**Why this reference:** it provides a second split-green VNG implementation
with RawTherapee's own four-plane indexing and tile boundaries.

**Why we match:** both retain separate green CFA phases through
interpolation and combine them only when producing final RGB.

**Known differences:** The bridge preserves RawTherapee's original
split-green `prefilters` mask and omits its preprocessing and output stages.
Border interpolation and pixel-neighborhood indexing remain
RawTherapee-specific.
