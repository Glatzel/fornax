# compare-demosaic

Compares Fornax demosaic output with LibRaw, darktable, and RawTherapee at the
same stage: the active Bayer mosaic enters each implementation and reconstructed
RGB leaves it. White balance and the default output crop are not applied.

`baseline/canonical.json` is the source of truth for results. The blocks marked
as generated below are derived from that JSON. Text outside the markers and the
notes beside each algorithm are written by the author and reviewed with the
implementation.

Canonical schema version 3 adds the reference-independent `ground_truth`
section and the reference disagreement floor described below.

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

      --no-valid-range
          Disable scaling inputs below the integer-reference saturation range

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

Pin `OMP_NUM_THREADS=1` for deterministic canonical runs.

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

## Ground truth

Reference agreement is one signal, not the definition of correctness. This
section compares each demosaic implementation with deterministic synthetic
truth and with half-resolution truth binned directly from fixed RAW corpus
quads. Both metrics are scene-linear and HDR-safe; no PSNR peak of one is used.

<!-- compare-demosaic:begin ground-truth -->
| Rank | Algorithm | Relative mean | Relative p99 | Relative p99.9 | Log2 mean | Zipper mean | False-color mean | Vs Linear | Worst |
| ---: | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- |
| 1 | RCD | 0.065768 | 0.931823 | 0.975702 | 0.194144 | 0.129033 | 0.476067 | 0.889x | colored_edges (20, 85, c2) edge |
| 2 | PPG | 0.068210 | 0.931838 | 1.196070 | 0.200254 | 0.132463 | 0.502916 | 0.922x | corpus:Sigma/fp/A002_653_20240229_000001_2k_8bit.DNG (44, 75, c2) edge |
| 3 | LMMSE | 0.069145 | 0.931823 | 0.980299 | 0.201888 | 0.131668 | 0.468623 | 0.935x | colored_edges (120, 2, c0) border |
| 4 | AMaZE | 0.069376 | 0.931833 | 1.195926 | 0.202114 | 0.134309 | 0.505641 | 0.938x | colored_edges (82, 85, c2) flat |
| 5 | VNG | 0.073676 | 0.931825 | 0.984831 | 0.211962 | 0.144924 | 0.453238 | 0.996x | hdr_blocks (96, 62, c2) edge |
| 6 | VNG4 | 0.073676 | 0.931825 | 0.984831 | 0.211962 | 0.144924 | 0.453238 | 0.996x | hdr_blocks (96, 62, c2) edge |
| 7 | Linear | 0.073991 | 0.931821 | 0.939347 | 0.213540 | 0.144554 | 0.486986 | 1.000x | hdr_blocks (64, 43, c2) edge |

- Relative error uses denominator `max(|truth|, |output|, 0.001000)`; log2 error floors both values at `0.001000`. Lower is better.
- Relative, log2, zipper, and false-color summaries exclude the 8 px border. Worst pixels are selected over the full frame and report their location.
- Zipper is relative luma error on interior edge pixels. False color is the magnitude of output-minus-truth `R-G`/`B-G` chroma relative to luma.
- Linear is the required floor. An algorithm is suspicious when its per-scene relative mean is over 1.0% worse than Linear on a zone-plate or edge scene.
- Pattern `GBRG`; synthetic and half-resolution corpus sources are evaluated at 128x128.
- Excluded ground-truth sources: none.
- Suspicious versus Linear: PPG: colored_edges; LMMSE: colored_edges; AMaZE: colored_edges; VNG: slanted_edge, colored_edges; VNG4: slanted_edge, colored_edges.
- Corpus sources: Canon/EOS 5D Mark III/M24-1221_000202.dng (evaluated); Sigma/fp/A002_653_20240229_000001_2k_8bit.DNG (evaluated); Nikon/D80/DSC_0143.dng (evaluated).
- Literature expectation: advanced directional or MMSE methods (AMaZE, LMMSE, RCD) should generally beat the bilinear floor; VNG is the next comparison tier, followed by PPG. Surprises are retained in the ranking rather than waived.
<!-- compare-demosaic:end ground-truth -->

## Reference disagreement floor

LibRaw and darktable intentionally use different integer, border, and local
direction conventions. Where both implement the same algorithm, their mutual
difference is a practical floor for meaningful reference agreement.

<!-- compare-demosaic:begin reference-floor -->
| Input | Algorithm | Pattern | MAE | P99 | P99.9 | Max | Within 64 | >64 B/E/F |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| colorchart-5D2-6000K.dng | PPG | BGGR | 0.180037 | 1 | 8 | 618 | 0.999696 | 46/892/5434 |
| colorchart-5D2-6000K.dng | PPG | GBRG | 0.180036 | 1 | 8 | 618 | 0.999696 | 50/892/5434 |
| colorchart-5D2-6000K.dng | PPG | GRBG | 0.180035 | 1 | 8 | 618 | 0.999696 | 48/892/5434 |
| colorchart-5D2-6000K.dng | PPG | RGGB | 0.180034 | 1 | 8 | 618 | 0.999697 | 52/892/5434 |
| colorchart-5D2-6000K.dng | VNG | BGGR | 17.791031 | 118 | 411 | 2958 | 0.931635 | 9231/368611/1055823 |
| colorchart-5D2-6000K.dng | VNG | GBRG | 17.790381 | 118 | 411 | 2958 | 0.931640 | 9236/368611/1055963 |
| colorchart-5D2-6000K.dng | VNG | GRBG | 17.789645 | 118 | 411 | 2958 | 0.931647 | 9279/368611/1055966 |
| colorchart-5D2-6000K.dng | VNG | RGGB | 17.788996 | 118 | 411 | 2958 | 0.931652 | 9284/368611/1056106 |

- Achievable floor is LibRaw versus darktable on the same scaled active mosaic, pattern, and border mask (16 px).
- A Fornax row is a floor match when interior MAE, P99, and P99.9 are each no more than one code above this floor for the same algorithm, input, and pattern.
<!-- compare-demosaic:end reference-floor -->

## Summary

<!-- compare-demosaic:begin summary -->
| Algorithm | Reference | Worst verdict | Native-pattern result |
| --- | --- | --- | --- |
| AMaZE | darktable | ⚖️ | RGGB ⚖️ |
| AMaZE | rawtherapee | ⚖️ | RGGB ⚖️ |
| Linear | libraw | 🟢 | RGGB 🟢 |
| LMMSE | darktable | ❌ | RGGB ❌ |
| LMMSE | rawtherapee | ❌ | RGGB ❌ |
| PPG | darktable | ✅ | RGGB ✅ |
| PPG | libraw | ✅ | RGGB ✅ |
| RCD | darktable | ❌ | RGGB ❌ |
| RCD | rawtherapee | ❌ | RGGB ❌ |
| VNG | darktable | ❌ | RGGB ❌ |
| VNG | libraw | ❌ | RGGB ❌ |
| VNG4 | libraw | ❌ | RGGB ❌ |
| VNG4 | rawtherapee | ⚖️ | RGGB ⚖️ |
<!-- compare-demosaic:end summary -->

## Legend

<!-- compare-demosaic:begin legend -->
| Verdict | Meaning |
| --- | --- |
| `✅` | Exact or passing without high-error outliers. |
| `🟢` | Rounding-only differences. |
| `⚖️` | Reference-floor or ground-truth evidence accepts the divergence, or a person reviewed explicit outlier bounds. |
| `⚠️` | Every limit passes; high-error outliers remain and are not yet explained. |
| `❌` | At least one limit failed. |

- Limits: MAE <= 2.000000, P99 <= 64, high-error fraction <= 0.001000 above 1024 codes.
- Kinds: `exact` requires equality; `rounding` permits one-code upward rounding differences; `robust` applies the numeric limits.
- A row is `native` only when it uses the input's original CFA layout and crop `[0, 0]`. Other patterns use the smallest valid one-pixel shift and are consistency checks, not independent tests.
- Verdicts and distribution metrics use the interior after excluding a 16 px border. Worst pixels still scan the full frame.
- Valid-range scaling is enabled; active mosaics are scaled below 60000 codes before comparison.
- Fornax output is quantized once to the comparison adapter's 16-bit convention before differences are measured; Linear retains its one-code rounding classification.
- Border policy: LibRaw runs `pre_interpolate` before the selected algorithm; darktable and RawTherapee use algorithm-specific tile and edge handling. The common masked border keeps those conventions out of acceptance.
- References: LibRaw 0.22.3; darktable 5.6.2.
- Inputs:
  - `colorchart-5D2-6000K.dng`: 5634x3753, native `RGGB`, reference scale `1.000000`, sha256 `fcf04ee83ae2c6bdee5b2e324e0ee7c51284cff1671c1f0dbbbbbbc6377fd660`.
<!-- compare-demosaic:end legend -->

## AMaZE

<!-- compare-demosaic:begin algo:amaze -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | Within 64 | >64 B/E/F | >1024 (border) | Floor | GT evidence |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | BGGR | [1, 1] | no | ⚖️ | 0.684439 | 6.366531 | 2252 | -0.017810 | 14 | 87 | 0.996317 | 54/56681/20062 | 47 (0) | no | not worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GBRG | [0, 1] | no | ⚖️ | 0.684361 | 6.365868 | 2252 | -0.017853 | 14 | 87 | 0.996317 | 53/56684/20067 | 47 (0) | no | not worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GRBG | [1, 0] | no | ⚖️ | 0.684360 | 6.365938 | 2252 | -0.017827 | 14 | 87 | 0.996317 | 56/56688/20070 | 47 (0) | no | not worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | RGGB | [0, 0] | yes | ⚖️ | 0.684282 | 6.365276 | 2252 | -0.017869 | 14 | 87 | 0.996317 | 55/56691/20075 | 47 (0) | no | not worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | BGGR | [1, 1] | no | ⚖️ | 0.684439 | 6.366531 | 2252 | -0.017810 | 14 | 87 | 0.996317 | 54/56681/20062 | 47 (0) | no | not worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | GBRG | [0, 1] | no | ⚖️ | 0.684361 | 6.365868 | 2252 | -0.017853 | 14 | 87 | 0.996317 | 53/56684/20067 | 47 (0) | no | not worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | GRBG | [1, 0] | no | ⚖️ | 0.684360 | 6.365938 | 2252 | -0.017827 | 14 | 87 | 0.996317 | 56/56688/20070 | 47 (0) | no | not worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | RGGB | [0, 0] | yes | ⚖️ | 0.684282 | 6.365276 | 2252 | -0.017869 | 14 | 87 | 0.996317 | 55/56691/20075 | 47 (0) | no | not worse |

**Automatic ⚖️ evidence:** darktable BGGR: 68324 divergent synthetic samples; Fornax MAE/P99 14900.188001/52178.000000 codes versus reference 14934.316477/52561.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** darktable GBRG: 67553 divergent synthetic samples; Fornax MAE/P99 15160.925940/52178.000000 codes versus reference 15183.719346/52561.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** darktable GRBG: 66600 divergent synthetic samples; Fornax MAE/P99 14812.042282/52179.000000 codes versus reference 14865.809144/52562.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** darktable RGGB: 68193 divergent synthetic samples; Fornax MAE/P99 14850.458419/52178.000000 codes versus reference 14916.977637/52561.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** rawtherapee BGGR: 68324 divergent synthetic samples; Fornax MAE/P99 14900.188001/52178.000000 codes versus reference 14934.316477/52561.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** rawtherapee GBRG: 67553 divergent synthetic samples; Fornax MAE/P99 15160.925940/52178.000000 codes versus reference 15183.719346/52561.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** rawtherapee GRBG: 66600 divergent synthetic samples; Fornax MAE/P99 14812.042282/52179.000000 codes versus reference 14865.809144/52562.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** rawtherapee RGGB: 68193 divergent synthetic samples; Fornax MAE/P99 14850.458419/52178.000000 codes versus reference 14916.977637/52561.000000; reference-floor match: no.
<!-- compare-demosaic:end algo:amaze -->

### darktable `amaze_demosaic`

**Why this reference:** darktable is the primary open-source AMaZE
implementation used for reproducing this published algorithm.

**Why we match:** both preserve measured CFA values and use the same 160-pixel
tiles, adaptive green ratios, directional variance weighting, Nyquist texture
refinement, diagonal red/blue correction, and final chroma interpolation.

**Known differences:** darktable runs this stage inside its pixelpipe, while
Fornax accepts normalized active Bayer data directly. The comparison pins an
identity scale and valid range so both implementations receive the same
mosaic. Fornax preserves signed and HDR values with scale-relative thresholds;
the bounded reference makes different decisions where those values are
quantized for comparison.

**Open issues:** the broad horizontal-flip invariant still exposes a
direction-selection asymmetry at the image border.

### RawTherapee `amaze_demosaic_RT`

**Why this reference:** it exercises RawTherapee's independently optimized
AMaZE implementation and its own tile and clipping decisions.

**Why we match:** both implement the complete AMaZE tile pipeline and preserve
measured CFA samples. The comparison facade uses RawTherapee's scalar path,
identity matrix, and normalized 16-bit reference scale.

**Known differences:** RawTherapee normally receives channel-scaled working
data from `scaleColors`; the comparison deliberately omits white balance.
Fornax uses generic floating-point powers of two and scale-relative highlight
thresholds without clipping. Reference quantization and different bounded
highlight decisions account for the remaining differences.

**Open issues:** the broad horizontal-flip invariant still exposes a
direction-selection asymmetry at the image border.

## Linear

<!-- compare-demosaic:begin algo:linear -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | Within 64 | >64 B/E/F | >1024 (border) | Floor | GT evidence |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | BGGR | [1, 1] | no | 🟢 | 0.302907 | 0.550370 | 1 | 0.302907 | 1 | 1 | 1.000000 | 0/0/0 | 0 (0) | no | none |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | GBRG | [0, 1] | no | 🟢 | 0.302906 | 0.550369 | 1 | 0.302906 | 1 | 1 | 1.000000 | 0/0/0 | 0 (0) | no | none |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | GRBG | [1, 0] | no | 🟢 | 0.302907 | 0.550370 | 1 | 0.302907 | 1 | 1 | 1.000000 | 0/0/0 | 0 (0) | no | none |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | RGGB | [0, 0] | yes | 🟢 | 0.302906 | 0.550369 | 1 | 0.302906 | 1 | 1 | 1.000000 | 0/0/0 | 0 (0) | no | none |
<!-- compare-demosaic:end algo:linear -->

### LibRaw `lin_interpolate`

**Why this reference:** it isolates the basic bilinear interpolation that
provides a simple convention check for the comparison harness.

**Why we match:** both preserve the measured sample, average horizontal and
vertical green neighbors at red or blue sites, and average diagonals for the
missing opposite color.

**Known differences:** Fornax preserves typed floating-point samples, including
negative and HDR values, and never clamps in the processing path. The
comparison explicitly scales and rounds to the bounded LibRaw reference. Its
border handling also comes from the LibRaw pipeline.

## LMMSE

<!-- compare-demosaic:begin algo:lmmse -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | Within 64 | >64 B/E/F | >1024 (border) | Floor | GT evidence |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | BGGR | [1, 1] | no | ❌ | 0.793513 | 4.615191 | 499 | -0.024612 | 17 | 63 | 0.998046 | 1174/14550/26161 | 0 (0) | no | worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GBRG | [0, 1] | no | ❌ | 0.791267 | 4.591937 | 499 | -0.024402 | 17 | 62 | 0.998069 | 1190/14264/25984 | 0 (0) | no | worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GRBG | [1, 0] | no | ❌ | 0.793999 | 4.622261 | 516 | -0.024755 | 17 | 63 | 0.998035 | 1307/14692/26256 | 0 (0) | no | worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | RGGB | [0, 0] | yes | ❌ | 0.791843 | 4.600630 | 516 | -0.024523 | 17 | 62 | 0.998058 | 1329/14401/26090 | 0 (0) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | BGGR | [1, 1] | no | ❌ | 4.186373 | 10.642207 | 2094 | 0.104388 | 35 | 75 | 0.995725 | 30539/73184/15885 | 3980 (3841) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | GBRG | [0, 1] | no | ❌ | 4.186297 | 10.641742 | 2094 | 0.104394 | 35 | 75 | 0.995726 | 30374/73185/15886 | 3277 (3138) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | GRBG | [1, 0] | no | ❌ | 4.186118 | 10.641297 | 2094 | 0.104384 | 35 | 75 | 0.995726 | 30378/73184/15885 | 3469 (3330) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | RGGB | [0, 0] | yes | ❌ | 4.186042 | 10.640832 | 2094 | 0.104389 | 35 | 75 | 0.995727 | 30222/73185/15886 | 2766 (2627) | no | worse |
<!-- compare-demosaic:end algo:lmmse -->

### darktable `lmmse_demosaic`

**Why this reference:** darktable is the maintained LMMSE implementation
used for the reference comparison.

**Why we match:** both seed from PPG, transform the CFA through the same
piecewise non-linear scoring domain, estimate horizontal and vertical color
differences, apply the nine-tap low-pass and directional MMSE blend,
reconstruct red and blue from green differences, and run one median pass.
Both use 136-pixel tiles with a four-pixel border, eight-pixel overlap, and a
112-pixel valid stride.

**Known differences:** Fornax replaces darktable's bounded gamma tables with an
analytic transfer function, odd extension for negatives, and power-law
continuation above one. The generic kernel preserves `T` internally and
quantizes only at the bounded reference boundary; measured Bayer sites remain
exact.

**Open issues:** divergent-sample evidence is marginally worse than the
reference despite the stronger aggregate ground-truth rank.

### RawTherapee `lmmse_interpolate_omp`

**Why this reference:** it provides a second maintained LMMSE implementation
with an independently structured gamma-table and tile pipeline.

**Why we match:** both seed from PPG, estimate directional color differences,
apply the nine-tap MMSE blend, and reconstruct red and blue from green.

**Known differences:** RawTherapee uses its own gamma LUT and one comparison
iteration without white balance or its preprocess/output stages. These table
and pipeline conventions can leave larger differences than darktable while
sampled Bayer sites remain exact.

**Open issues:** reconstructed samples are evidence-worse than the bounded
RawTherapee reference on divergent pixels.

## PPG

<!-- compare-demosaic:begin algo:ppg -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | Within 64 | >64 B/E/F | >1024 (border) | Floor | GT evidence |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | BGGR | [1, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 1.000000 | 0/0/0 | 0 (0) | yes | none |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | GBRG | [0, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 1.000000 | 0/0/0 | 0 (0) | yes | none |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | GRBG | [1, 0] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 1.000000 | 0/0/0 | 0 (0) | yes | none |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | RGGB | [0, 0] | yes | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 1.000000 | 0/0/0 | 0 (0) | yes | none |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | BGGR | [1, 1] | no | ✅ | 0.180037 | 1.518689 | 618 | 0.106828 | 1 | 8 | 0.999696 | 46/899/5427 | 0 (0) | yes | none |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | GBRG | [0, 1] | no | ✅ | 0.180036 | 1.518619 | 618 | 0.106830 | 1 | 8 | 0.999696 | 50/899/5427 | 0 (0) | yes | none |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | GRBG | [1, 0] | no | ✅ | 0.180035 | 1.518568 | 618 | 0.106831 | 1 | 8 | 0.999696 | 48/899/5427 | 0 (0) | yes | none |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | RGGB | [0, 0] | yes | ✅ | 0.180034 | 1.518498 | 618 | 0.106834 | 1 | 8 | 0.999697 | 52/899/5427 | 0 (0) | yes | none |
<!-- compare-demosaic:end algo:ppg -->

### darktable `demosaic_ppg`

**Why this reference:** it exposes the patterned pixel-grouping algorithm
that Fornax implements and is the acceptance reference.

**Why we match:** the green estimate uses the same horizontal and vertical
gradient differences and green-referenced color-difference correction used to
reconstruct red and blue at green and opposite-color sites.

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
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | Within 64 | >64 B/E/F | >1024 (border) | Floor | GT evidence |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | BGGR | [1, 1] | no | ❌ | 0.057990 | 0.240819 | 8 | -0.017547 | 1 | 1 | 1.000000 | 0/0/0 | 0 (0) | no | worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GBRG | [0, 1] | no | ❌ | 0.057974 | 0.240783 | 6 | -0.017568 | 1 | 1 | 1.000000 | 0/0/0 | 0 (0) | no | worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GRBG | [1, 0] | no | ✅ | 0.057991 | 0.240820 | 8 | -0.017547 | 1 | 1 | 1.000000 | 0/0/0 | 0 (0) | no | not worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | RGGB | [0, 0] | yes | ❌ | 0.057974 | 0.240783 | 6 | -0.017568 | 1 | 1 | 1.000000 | 0/0/0 | 0 (0) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | BGGR | [1, 1] | no | ❌ | 0.058080 | 0.242394 | 76 | -0.017566 | 1 | 1 | 1.000000 | 31803/1/1 | 25 (25) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | GBRG | [0, 1] | no | ❌ | 0.058067 | 0.242620 | 81 | -0.017584 | 1 | 1 | 1.000000 | 31785/1/1 | 25 (25) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | GRBG | [1, 0] | no | ❌ | 0.058080 | 0.242395 | 76 | -0.017566 | 1 | 1 | 1.000000 | 31942/1/1 | 25 (25) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | RGGB | [0, 0] | yes | ❌ | 0.058069 | 0.242616 | 81 | -0.017586 | 1 | 1 | 1.000000 | 31924/1/1 | 25 (25) | no | worse |
<!-- compare-demosaic:end algo:rcd -->

### darktable `rcd_demosaic`

**Why this reference:** darktable provides the pinned RCD 2.3 implementation
against which the independently structured Fornax version is compared.

**Why we match:** both seed with PPG, use the same 112-pixel tile geometry,
compute directional high-pass gradient confidence, interpolate ratio-corrected
green, and then reconstruct diagonal color differences.

**Known differences:** Fornax evaluates the formulas through normalized
floating-point Rust with a scale-relative epsilon and preserves signed HDR
output. The darktable bridge quantizes at the bounded reference boundary.
Border fill and the surrounding metadata pipeline are not shared.

**Open issues:** horizontal-flip symmetry remains direction-sensitive.

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

**Open issues:** divergent-sample P99 remains worse than RawTherapee even
though aggregate DNG agreement improved substantially.

## VNG

<!-- compare-demosaic:begin algo:vng -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | Within 64 | >64 B/E/F | >1024 (border) | Floor | GT evidence |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | BGGR | [1, 1] | no | ❌ | 9.589058 | 18.812424 | 3239 | -0.028548 | 63 | 138 | 0.977471 | 2423/117236/352177 | 530 (1) | yes | worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | GBRG | [0, 1] | no | ❌ | 9.588862 | 18.811699 | 3239 | -0.028601 | 63 | 138 | 0.977473 | 2419/117236/352224 | 530 (1) | yes | worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | GRBG | [1, 0] | no | ❌ | 9.588490 | 18.810967 | 3239 | -0.028517 | 63 | 138 | 0.977476 | 2434/117236/352200 | 530 (1) | yes | worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | RGGB | [0, 0] | yes | ❌ | 9.588294 | 18.810243 | 3239 | -0.028570 | 63 | 138 | 0.977477 | 2430/117236/352247 | 530 (1) | yes | worse |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | BGGR | [1, 1] | no | ❌ | 22.063065 | 46.566431 | 3234 | 0.016405 | 138 | 435 | 0.884020 | 16467/655664/1760869 | 15270 (2) | no | worse |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | GBRG | [0, 1] | no | ❌ | 22.062369 | 46.563865 | 3234 | 0.016780 | 138 | 435 | 0.884026 | 16457/655688/1761141 | 15270 (2) | no | worse |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | GRBG | [1, 0] | no | ❌ | 22.061485 | 46.562069 | 3234 | 0.015644 | 138 | 435 | 0.884035 | 16485/655665/1761189 | 15270 (2) | no | worse |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | RGGB | [0, 0] | yes | ❌ | 22.060789 | 46.559503 | 3234 | 0.016018 | 138 | 435 | 0.884042 | 16475/655689/1761461 | 15270 (2) | no | worse |
<!-- compare-demosaic:end algo:vng -->

### darktable `vng_interpolate`

**Why this reference:** darktable defines the split-green VNG behavior that
Fornax targets.

**Why we match:** both begin with linear interpolation, score eight
directional gradients, retain lower-gradient neighbors, and correct missing
colors through green-referenced neighbor differences.

**Known differences:** darktable keeps the two green CFA phases in separate
planes through interpolation and averages them only when packing RGB. Fornax
follows that convention with generic floating-point phase samples and
gradient sums, preserving signed HDR values without clipping. The bounded
reference and border windows can therefore produce sparse high-value
differences even when the aggregate error is small. See
[the VNG reference notes](../../docs/reference/algorithms/vng.md) for source links and
implementation details.

**Open issues:** float direction decisions are evidence-worse than darktable
and are about 18% slower than the former integer kernel.

### LibRaw `vng_interpolate`

**Why this reference:** it is retained as informational evidence for the
original dcraw-family algorithm, not as an acceptance gate.

**Why we match:** both score eight directional gradients, retain
lower-gradient neighbors, and reconstruct missing colors through
green-referenced neighbor differences.

**Known differences:** LibRaw's comparison adapter sets `four_color_rgb = 0`,
so it retains one green plane and intentionally produces a materially
different result from Fornax's split-green darktable-compatible path.

**Open issues:** the practical reference floor is met, but synthetic evidence
is worse on the divergent samples.

## VNG4

<!-- compare-demosaic:begin algo:vng4 -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | Within 64 | >64 B/E/F | >1024 (border) | Floor | GT evidence |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | BGGR | [1, 1] | no | ❌ | 9.657296 | 18.830833 | 3240 | -0.047610 | 63 | 138 | 0.977446 | 2422/117355/352579 | 530 (1) | no | worse |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | GBRG | [0, 1] | no | ❌ | 9.657096 | 18.830107 | 3240 | -0.047655 | 63 | 138 | 0.977448 | 2418/117355/352626 | 530 (1) | no | worse |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | GRBG | [1, 0] | no | ❌ | 9.656725 | 18.829379 | 3240 | -0.047564 | 63 | 138 | 0.977451 | 2432/117355/352602 | 530 (1) | no | worse |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | RGGB | [0, 0] | yes | ❌ | 9.656524 | 18.828652 | 3240 | -0.047609 | 63 | 138 | 0.977452 | 2428/117355/352649 | 530 (1) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | BGGR | [1, 1] | no | ⚖️ | 15.988326 | 46.198451 | 3739 | 0.107416 | 103 | 583 | 0.921650 | 10697/508515/1123959 | 26053 (6) | no | not worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | GBRG | [0, 1] | no | ⚖️ | 15.987621 | 46.195064 | 3739 | 0.107284 | 103 | 583 | 0.921658 | 10684/508518/1124088 | 26053 (6) | no | not worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | GRBG | [1, 0] | no | ⚖️ | 15.986874 | 46.193057 | 3739 | 0.107421 | 103 | 583 | 0.921667 | 10789/508515/1124041 | 26053 (6) | no | not worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | RGGB | [0, 0] | yes | ⚖️ | 15.986169 | 46.189671 | 3739 | 0.107289 | 103 | 583 | 0.921675 | 10776/508518/1124170 | 26053 (6) | no | not worse |

**Automatic ⚖️ evidence:** rawtherapee BGGR: 29571 divergent synthetic samples; Fornax MAE/P99 2288.020358/27273.000000 codes versus reference 2798.667275/35730.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** rawtherapee GBRG: 29695 divergent synthetic samples; Fornax MAE/P99 2232.875905/27272.000000 codes versus reference 2681.992154/35733.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** rawtherapee GRBG: 29694 divergent synthetic samples; Fornax MAE/P99 2246.859635/27272.000000 codes versus reference 2681.964134/35730.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** rawtherapee RGGB: 29564 divergent synthetic samples; Fornax MAE/P99 2232.380936/27272.000000 codes versus reference 2811.942870/35729.000000; reference-floor match: no.
<!-- compare-demosaic:end algo:vng4 -->

### LibRaw `vng_interpolate`

**Why this reference:** this is LibRaw's explicit four-color VNG mode, which
keeps the two green CFA phases separate through interpolation.

**Why we match:** both implementations run VNG with split green planes and
average those planes into the final green channel only after interpolation.

**Known differences:** Fornax shares its generic split-green VNG kernel with
the `Demosaic::Vng` selector and exposes VNG4 as a distinct algorithm choice.
The comparison adapter sets LibRaw's four-color option before
`pre_interpolate` and performs the documented post-interpolation green mix.
No source code is copied into the postprocessor.

**Open issues:** shared VNG evidence and performance limitations remain.

### RawTherapee `vng4_demosaic`

**Why this reference:** it provides a second split-green VNG implementation
with RawTherapee's own four-plane indexing and tile boundaries.

**Why we match:** both retain separate green CFA phases through
interpolation and combine them only when producing final RGB.

**Known differences:** The bridge preserves RawTherapee's original
split-green `prefilters` mask and omits its preprocessing and output stages.
Border interpolation and pixel-neighborhood indexing remain
RawTherapee-specific.

**Open issues:** RawTherapee evidence is accepted, but VNG4 is about 22% slower
and is evidence-worse against LibRaw.
