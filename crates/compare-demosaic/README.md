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
export OMP_NUM_THREADS=1
cargo run --release -p compare-demosaic -- run
cargo run --release -p compare-demosaic -- diff \
  crates/compare-demosaic/baseline/canonical.json \
  target/compare-demosaic/canonical.json
cargo run --release -p compare-demosaic -- readme --check
```

Pin `OMP_NUM_THREADS=1` for canonical runs. LibRaw DHT uses OpenMP and can
change small numeric statistics with the thread count.

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
| AAHD | libraw | ⚖️ | RGGB ⚖️ |
| AHD | libraw | ⚖️ | RGGB ⚖️ |
| AHD | rawtherapee | ❌ | RGGB ❌ |
| AMaZE | darktable | ✅ | RGGB ✅ |
| AMaZE | rawtherapee | ✅ | RGGB ✅ |
| DCB | libraw | ✅ | RGGB ✅ |
| DCB | rawtherapee | ❌ | RGGB ❌ |
| DHT | libraw | ✅ | RGGB ✅ |
| Linear | libraw | 🟢 | RGGB 🟢 |
| LMMSE | darktable | ✅ | RGGB ✅ |
| LMMSE | rawtherapee | ❌ | RGGB ❌ |
| PPG | darktable | ✅ | RGGB ✅ |
| PPG | libraw | ✅ | RGGB ✅ |
| RCD | darktable | ✅ | RGGB ✅ |
| RCD | rawtherapee | ⚠️ | RGGB ⚠️ |
| VNG | darktable | ✅ | RGGB ✅ |
| VNG | libraw | ❌ | RGGB ❌ |
| VNG4 | libraw | ✅ | RGGB ✅ |
| VNG4 | rawtherapee | ❌ | RGGB ❌ |
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
  - `colorchart-5D2-6000K.dng`: 5634x3753, native `RGGB`, sha256 `fcf04ee83ae2c6bdee5b2e324e0ee7c51284cff1671c1f0dbbbbbbc6377fd660`.
<!-- compare-demosaic:end legend -->

## AAHD

<!-- compare-demosaic:begin algo:aahd -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | BGGR | [1, 1] | no | ⚖️ | 0.102921 | 3.697554 | 1246 | 0.000056 | 0 | 30 | 4 (0) | 0.001896 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | GBRG | [0, 1] | no | ⚖️ | 0.104815 | 3.713635 | 1073 | -0.006387 | 0 | 30 | 1 (0) | 0.001919 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | GRBG | [1, 0] | no | ⚖️ | 0.105236 | 3.718776 | 1231 | 0.000097 | 0 | 31 | 1 (0) | 0.001928 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | RGGB | [0, 0] | yes | ⚖️ | 0.103509 | 3.705762 | 1246 | -0.006448 | 0 | 30 | 4 (0) | 0.001907 |
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

**Accepted outliers:** aahd vs LibRaw, all patterns, input `colorchart-5D2-6000K.dng`: at most 4 samples above 1024, maximum error at most 1246. Reason: Independent floating-point homogeneity and gradient comparisons can select the other AAHD directional candidate at sparse high-contrast chart details. Evidence: The full-resolution DNG error maps contain at most four interior high-error samples in any CFA phase, with a maximum error of 1246. Sampled Bayer sites are exact, every high-error sample is interpolated, each 64x64 tile contains at most one such sample, and the same physical chart sites recur after pattern-crop alignment without border or full-frame structure.

## AHD

<!-- compare-demosaic:begin algo:ahd -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | BGGR | [1, 1] | no | ⚖️ | 0.244838 | 4.431481 | 1220 | 0.020171 | 1 | 67 | 1 (0) | 0.025801 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | GBRG | [0, 1] | no | ⚖️ | 0.244817 | 4.431201 | 1220 | 0.020170 | 1 | 67 | 1 (0) | 0.025801 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | GRBG | [1, 0] | no | ⚖️ | 0.244824 | 4.431271 | 1220 | 0.020166 | 1 | 67 | 1 (0) | 0.025803 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | RGGB | [0, 0] | yes | ⚖️ | 0.244802 | 4.430990 | 1220 | 0.020165 | 1 | 67 | 1 (0) | 0.025802 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/ahd_demosaic_RT.cc#L44) | BGGR | [1, 1] | no | ❌ | 2.899419 | 15.331739 | 1796 | -0.096686 | 73 | 183 | 12 (0) | 0.210986 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/ahd_demosaic_RT.cc#L44) | GBRG | [0, 1] | no | ❌ | 2.899410 | 15.331496 | 1796 | -0.096665 | 73 | 183 | 12 (0) | 0.210987 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/ahd_demosaic_RT.cc#L44) | GRBG | [1, 0] | no | ❌ | 2.899425 | 15.331497 | 1796 | -0.096745 | 73 | 183 | 12 (0) | 0.210987 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/ahd_demosaic_RT.cc#L44) | RGGB | [0, 0] | yes | ❌ | 2.899416 | 15.331253 | 1796 | -0.096724 | 73 | 183 | 12 (0) | 0.210989 |
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
conventions. Sparse high-error decisions can remain different at chart edges.

**Accepted outliers:** ahd vs LibRaw, all patterns, input `colorchart-5D2-6000K.dng`: at most 1 samples above 1024, maximum error at most 1220. Reason: Independent AHD tile, clipping, matrix, and quantization conventions can change a local directional decision at a high-contrast chart edge. Evidence: Each DNG CFA phase has one interior high-error sample with a maximum error of 1220. Sampled Bayer sites are exact, the high-error sample is interpolated, the same physical chart site recurs after pattern-crop alignment, and the full error map has no border strip or global structure.

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
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | BGGR | [1, 1] | no | ✅ | 0.003910 | 0.334578 | 321 | -0.000053 | 0 | 0 | 0 (0) | 0.000970 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GBRG | [0, 1] | no | ✅ | 0.003862 | 0.334465 | 321 | -0.000036 | 0 | 0 | 0 (0) | 0.000948 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GRBG | [1, 0] | no | ✅ | 0.004089 | 0.340450 | 321 | -0.000245 | 0 | 0 | 0 (0) | 0.000975 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | RGGB | [0, 0] | yes | ✅ | 0.003859 | 0.334366 | 321 | -0.000036 | 0 | 0 | 0 (0) | 0.000948 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | BGGR | [1, 1] | no | ✅ | 0.003910 | 0.334578 | 321 | -0.000053 | 0 | 0 | 0 (0) | 0.000970 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | GBRG | [0, 1] | no | ✅ | 0.003862 | 0.334465 | 321 | -0.000036 | 0 | 0 | 0 (0) | 0.000948 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | GRBG | [1, 0] | no | ✅ | 0.004089 | 0.340450 | 321 | -0.000245 | 0 | 0 | 0 (0) | 0.000975 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | RGGB | [0, 0] | yes | ✅ | 0.003859 | 0.334366 | 321 | -0.000036 | 0 | 0 | 0 (0) | 0.000948 |
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
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | BGGR | [1, 1] | no | ✅ | 0.000137 | 0.011704 | 2 | -0.000001 | 0 | 0 | 0 (0) | 0.000136 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | GBRG | [0, 1] | no | ✅ | 0.000137 | 0.011702 | 2 | -0.000001 | 0 | 0 | 0 (0) | 0.000136 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | GRBG | [1, 0] | no | ✅ | 0.000136 | 0.011698 | 2 | -0.000001 | 0 | 0 | 0 (0) | 0.000136 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | RGGB | [0, 0] | yes | ✅ | 0.000136 | 0.011696 | 2 | -0.000001 | 0 | 0 | 0 (0) | 0.000136 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/demosaic_algos.cc#L1406) | BGGR | [1, 1] | no | ❌ | 14.861552 | 33.798575 | 3756 | -0.708235 | 146 | 300 | 219 (6) | 0.648116 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/demosaic_algos.cc#L1406) | GBRG | [0, 1] | no | ❌ | 14.861415 | 33.797519 | 3756 | -0.707623 | 146 | 300 | 219 (6) | 0.648115 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/demosaic_algos.cc#L1406) | GRBG | [1, 0] | no | ❌ | 14.860519 | 33.796159 | 3756 | -0.710197 | 146 | 299 | 219 (6) | 0.648113 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/demosaic_algos.cc#L1406) | RGGB | [0, 0] | yes | ❌ | 14.860341 | 33.795017 | 3756 | -0.709594 | 146 | 299 | 219 (6) | 0.648111 |
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
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | BGGR | [1, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | GBRG | [0, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | GRBG | [1, 0] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | RGGB | [0, 0] | yes | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
<!-- compare-demosaic:end algo:dht -->

### LibRaw `dht_interpolate`

**Why this reference:** it is the maintained directional homogeneity transform
implementation targeted by Fornax.

**Why we match:** Fornax ports LibRaw's four-pixel neutral working margin,
hot/dead-sample handling, one-stop luminance distance, horizontal/vertical and
diagonal direction discovery, two parity refinement passes, directional green
interpolation, diagonal and HV color reconstruction, channel bounds, and
truncating 16-bit output in the same order. Measured Bayer values remain exact.

**Known differences:** The comparison bridge supplies an identity
camera-to-RGB matrix without white balance, while Fornax supports only the four
conventional 2x2 RGB Bayer patterns. No output differences are observed on the
canonical comparison fixture.

## Linear

<!-- compare-demosaic:begin algo:linear -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | BGGR | [1, 1] | no | 🟢 | 0.302806 | 0.550278 | 1 | 0.302806 | 1 | 1 | 0 (0) | 0.302806 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | GBRG | [0, 1] | no | 🟢 | 0.302806 | 0.550278 | 1 | 0.302806 | 1 | 1 | 0 (0) | 0.302806 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | GRBG | [1, 0] | no | 🟢 | 0.302806 | 0.550278 | 1 | 0.302806 | 1 | 1 | 0 (0) | 0.302806 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | RGGB | [0, 0] | yes | 🟢 | 0.302806 | 0.550278 | 1 | 0.302806 | 1 | 1 | 0 (0) | 0.302806 |
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
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | BGGR | [1, 1] | no | ✅ | 0.000189 | 0.013734 | 1 | 0.000003 | 0 | 0 | 0 (0) | 0.000189 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GBRG | [0, 1] | no | ✅ | 0.000189 | 0.013761 | 1 | 0.000002 | 0 | 0 | 0 (0) | 0.000189 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GRBG | [1, 0] | no | ✅ | 0.000188 | 0.013727 | 1 | 0.000003 | 0 | 0 | 0 (0) | 0.000188 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | RGGB | [0, 0] | yes | ✅ | 0.000189 | 0.013759 | 1 | 0.000003 | 0 | 0 | 0 (0) | 0.000189 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | BGGR | [1, 1] | no | ❌ | 4.655629 | 17.906428 | 3028 | -0.039551 | 40 | 110 | 3981 (3841) | 0.476042 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | GBRG | [0, 1] | no | ❌ | 4.628258 | 15.952154 | 3028 | -0.015067 | 40 | 110 | 3278 (3138) | 0.476022 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | GRBG | [1, 0] | no | ❌ | 4.648753 | 17.583996 | 3028 | -0.033050 | 40 | 110 | 3470 (3329) | 0.476031 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | RGGB | [0, 0] | yes | ❌ | 4.621489 | 15.590486 | 3028 | -0.008584 | 40 | 110 | 2767 (2626) | 0.476013 |
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
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | BGGR | [1, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | GBRG | [0, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | GRBG | [1, 0] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | RGGB | [0, 0] | yes | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | BGGR | [1, 1] | no | ✅ | 0.180093 | 1.515138 | 618 | 0.106902 | 1 | 8 | 0 (0) | 0.145826 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | GBRG | [0, 1] | no | ✅ | 0.180099 | 1.515393 | 618 | 0.106902 | 1 | 8 | 0 (0) | 0.145826 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | GRBG | [1, 0] | no | ✅ | 0.180091 | 1.515076 | 618 | 0.106895 | 1 | 8 | 0 (0) | 0.145826 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | RGGB | [0, 0] | yes | ✅ | 0.180096 | 1.515331 | 618 | 0.106894 | 1 | 8 | 0 (0) | 0.145826 |
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
ties at high-contrast chart edges can therefore select different directions.

## RCD

<!-- compare-demosaic:begin algo:rcd -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | BGGR | [1, 1] | no | ✅ | 0.176074 | 1.244913 | 268 | -0.000185 | 4 | 17 | 0 (0) | 0.068262 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GBRG | [0, 1] | no | ✅ | 0.176001 | 1.243337 | 277 | -0.000060 | 4 | 17 | 0 (0) | 0.068258 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GRBG | [1, 0] | no | ✅ | 0.176102 | 1.245073 | 268 | -0.000195 | 4 | 17 | 0 (0) | 0.068270 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | RGGB | [0, 0] | yes | ✅ | 0.176026 | 1.243431 | 277 | -0.000066 | 4 | 17 | 0 (0) | 0.068266 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | BGGR | [1, 1] | no | ⚠️ | 0.342697 | 4.175746 | 1902 | -0.001262 | 6 | 53 | 25 (25) | 0.072874 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | GBRG | [0, 1] | no | ⚠️ | 0.342661 | 4.176720 | 1902 | -0.000476 | 6 | 53 | 25 (25) | 0.072871 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | GRBG | [1, 0] | no | ⚠️ | 0.342918 | 4.179122 | 1902 | -0.002588 | 6 | 53 | 25 (25) | 0.072880 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | RGGB | [0, 0] | yes | ⚠️ | 0.342880 | 4.180076 | 1902 | -0.001801 | 6 | 53 | 25 (25) | 0.072877 |
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
pipeline are not shared.

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
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | BGGR | [1, 1] | no | ✅ | 0.380470 | 0.824245 | 894 | 0.018450 | 1 | 8 | 0 (0) | 0.361410 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | GBRG | [0, 1] | no | ✅ | 0.380469 | 0.824238 | 894 | 0.018437 | 1 | 8 | 0 (0) | 0.361408 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | GRBG | [1, 0] | no | ✅ | 0.380464 | 0.824217 | 894 | 0.018438 | 1 | 8 | 0 (0) | 0.361405 |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | RGGB | [0, 0] | yes | ✅ | 0.380463 | 0.824210 | 894 | 0.018425 | 1 | 8 | 0 (0) | 0.361404 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | BGGR | [1, 1] | no | ❌ | 17.641782 | 41.462699 | 2958 | 0.065925 | 117 | 407 | 14118 (2) | 0.759202 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | GBRG | [0, 1] | no | ❌ | 17.641210 | 41.460269 | 2958 | 0.066807 | 117 | 407 | 14118 (2) | 0.759203 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | GRBG | [1, 0] | no | ❌ | 17.640452 | 41.458634 | 2958 | 0.064322 | 117 | 407 | 14118 (2) | 0.759201 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | RGGB | [0, 0] | yes | ❌ | 17.639879 | 41.456204 | 2958 | 0.065205 | 117 | 407 | 14118 (2) | 0.759202 |
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
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | BGGR | [1, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | GBRG | [0, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | GRBG | [1, 0] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | RGGB | [0, 0] | yes | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 0 (0) | 0.000000 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | BGGR | [1, 1] | no | ❌ | 9.562998 | 40.779974 | 3233 | 0.155914 | 75 | 545 | 24400 (5) | 0.630406 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | GBRG | [0, 1] | no | ❌ | 9.562398 | 40.776681 | 3233 | 0.156244 | 75 | 544 | 24400 (5) | 0.630404 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | GRBG | [1, 0] | no | ❌ | 9.562078 | 40.775001 | 3233 | 0.154218 | 75 | 544 | 24400 (5) | 0.630406 |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | RGGB | [0, 0] | yes | ❌ | 9.561479 | 40.771710 | 3233 | 0.154547 | 75 | 544 | 24400 (5) | 0.630405 |
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
