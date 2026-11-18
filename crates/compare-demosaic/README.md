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
| 1 | DHT | 0.068475 | 0.931832 | 0.931897 | 0.202597 | 0.130945 | 0.431911 | 0.863x | colored_edges (99, 0, c0) border |
| 2 | RCD | 0.071887 | 0.931826 | 0.931954 | 0.213947 | 0.130553 | 0.454756 | 0.906x | slanted_edge (78, 1, c2) border |
| 3 | PPG | 0.074243 | 0.931843 | 1.000000 | 0.223846 | 0.132707 | 0.470603 | 0.936x | slanted_edge (78, 1, c2) border |
| 4 | LMMSE | 0.074514 | 0.931824 | 0.931900 | 0.220813 | 0.133958 | 0.468054 | 0.939x | slanted_edge (78, 1, c2) border |
| 5 | AMaZE | 0.075552 | 0.931839 | 1.000000 | 0.225923 | 0.134808 | 0.472679 | 0.952x | slanted_edge (79, 1, c2) border |
| 6 | AHD | 0.075817 | 0.931848 | 1.000000 | 0.226932 | 0.133530 | 0.472657 | 0.955x | slanted_edge (77, 5, c0) border |
| 7 | AAHD | 0.076015 | 0.931827 | 1.000000 | 0.226514 | 0.134856 | 0.472173 | 0.958x | slanted_edge (79, 1, c2) border |
| 8 | DCB | 0.076181 | 0.931840 | 1.000000 | 0.228721 | 0.132506 | 0.466044 | 0.960x | slanted_edge (78, 1, c2) border |
| 9 | VNG | 0.078106 | 0.931815 | 0.931892 | 0.222386 | 0.142903 | 0.460515 | 0.984x | slanted_edge (78, 4, c0) border |
| 10 | VNG4 | 0.078106 | 0.931815 | 0.931892 | 0.222386 | 0.142903 | 0.460515 | 0.984x | slanted_edge (78, 4, c0) border |
| 11 | Linear | 0.079355 | 0.931821 | 0.931896 | 0.225596 | 0.143823 | 0.478996 | 1.000x | fine_lines (96, 22, c0) edge |

- Relative error uses denominator `max(|truth|, |output|, 0.001000)`; log2 error floors both values at `0.001000`. Lower is better.
- Relative, log2, zipper, and false-color summaries exclude the 8 px border. Worst pixels are selected over the full frame and report their location.
- Zipper is relative luma error on interior edge pixels. False color is the magnitude of output-minus-truth `R-G`/`B-G` chroma relative to luma.
- Linear is the required floor. An algorithm is suspicious when its per-scene relative mean is over 1.0% worse than Linear on a zone-plate or edge scene.
- Pattern `GBRG`; synthetic and half-resolution corpus sources are evaluated at 128x128.
- Excluded ground-truth sources: `hdr_blocks` (not all algorithms accept the explicit bounded reference range).
- Suspicious versus Linear: PPG: colored_edges; LMMSE: colored_edges; AMaZE: colored_edges; AHD: colored_edges; AAHD: colored_edges; DCB: colored_edges; VNG: colored_edges; VNG4: colored_edges.
- Corpus sources: Canon/EOS 5D Mark III/M24-1221_000202.dng (evaluated); Sigma/fp/A002_653_20240229_000001_2k_8bit.DNG (evaluated); Nikon/D80/DSC_0143.dng (evaluated).
- Literature expectation: advanced directional or MMSE methods (AMaZE, DCB, DHT, LMMSE, RCD) should generally beat the bilinear floor; AHD, AAHD, and VNG are the next comparison tier, followed by PPG. Surprises are retained in the ranking rather than waived.
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
| AAHD | libraw | ❌ | RGGB ❌ |
| AHD | libraw | ❌ | RGGB ⚖️ |
| AHD | rawtherapee | ❌ | RGGB ❌ |
| AMaZE | darktable | ❌ | RGGB ❌ |
| AMaZE | rawtherapee | ❌ | RGGB ❌ |
| DCB | libraw | ✅ | RGGB ✅ |
| DCB | rawtherapee | ❌ | RGGB ❌ |
| DHT | libraw | ⚠️ | RGGB ⚠️ |
| Linear | libraw | 🟢 | RGGB 🟢 |
| LMMSE | darktable | ✅ | RGGB ✅ |
| LMMSE | rawtherapee | ❌ | RGGB ❌ |
| PPG | darktable | ✅ | RGGB ✅ |
| PPG | libraw | ✅ | RGGB ✅ |
| RCD | darktable | ✅ | RGGB ✅ |
| RCD | rawtherapee | ❌ | RGGB ❌ |
| VNG | darktable | ❌ | RGGB ✅ |
| VNG | libraw | ❌ | RGGB ❌ |
| VNG4 | libraw | ✅ | RGGB ✅ |
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

## AHD

<!-- compare-demosaic:begin algo:ahd -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | Within 64 | >64 B/E/F | >1024 (border) | Floor | GT evidence |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | BGGR | [1, 1] | no | ❌ | 0.245669 | 4.455163 | 1220 | 0.019065 | 1 | 68 | 0.997771 | 189/10797/35654 | 1 (0) | no | worse |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | GBRG | [0, 1] | no | ⚠️ | 0.245649 | 4.454854 | 1220 | 0.019062 | 1 | 68 | 0.997771 | 190/10797/35656 | 1 (0) | no | none |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | GRBG | [1, 0] | no | ⚖️ | 0.245644 | 4.454735 | 1220 | 0.019059 | 1 | 68 | 0.997771 | 196/10797/35656 | 1 (0) | no | not worse |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | RGGB | [0, 0] | yes | ⚖️ | 0.245623 | 4.454426 | 1220 | 0.019057 | 1 | 68 | 0.997771 | 197/10797/35658 | 1 (0) | no | not worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/ahd_demosaic_RT.cc#L44) | BGGR | [1, 1] | no | ⚖️ | 2.916957 | 15.395501 | 1796 | -0.097034 | 73 | 183 | 0.974380 | 3651/83112/450690 | 12 (0) | no | not worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/ahd_demosaic_RT.cc#L44) | GBRG | [0, 1] | no | ⚖️ | 2.916920 | 15.395109 | 1796 | -0.097009 | 73 | 183 | 0.974381 | 3642/83119/450777 | 12 (0) | no | not worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/ahd_demosaic_RT.cc#L44) | GRBG | [1, 0] | no | ⚖️ | 2.916866 | 15.394919 | 1796 | -0.097034 | 73 | 183 | 0.974382 | 3677/83112/450806 | 12 (0) | no | not worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/ahd_demosaic_RT.cc#L44) | RGGB | [0, 0] | yes | ❌ | 2.916829 | 15.394527 | 1796 | -0.097010 | 73 | 183 | 0.974382 | 3668/83119/450893 | 12 (0) | no | worse |

**Automatic ⚖️ evidence:** libraw GRBG: 1 divergent synthetic samples; Fornax MAE/P99 3.000000/3.000000 codes versus reference 5323.000000/5323.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** libraw RGGB: 10 divergent synthetic samples; Fornax MAE/P99 24158.800000/37410.000000 codes versus reference 27521.900000/42949.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** rawtherapee BGGR: 98 divergent synthetic samples; Fornax MAE/P99 6120.765306/48576.000000 codes versus reference 6748.000000/48584.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** rawtherapee GBRG: 44 divergent synthetic samples; Fornax MAE/P99 750.704545/5328.000000 codes versus reference 911.113636/5330.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** rawtherapee GRBG: 36 divergent synthetic samples; Fornax MAE/P99 700.111111/5318.000000 codes versus reference 1077.055556/10655.000000; reference-floor match: no.
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
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | Within 64 | >64 B/E/F | >1024 (border) | Floor | GT evidence |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | BGGR | [1, 1] | no | ✅ | 0.003898 | 0.335805 | 321 | -0.000038 | 0 | 0 | 0.999990 | 1/104/113 | 0 (0) | no | none |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GBRG | [0, 1] | no | ❌ | 0.003899 | 0.335961 | 321 | -0.000039 | 0 | 0 | 0.999990 | 1/104/114 | 0 (0) | no | worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GRBG | [1, 0] | no | ❌ | 0.003892 | 0.335683 | 321 | -0.000038 | 0 | 0 | 0.999990 | 2/104/113 | 0 (0) | no | worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | RGGB | [0, 0] | yes | ❌ | 0.003894 | 0.335839 | 321 | -0.000039 | 0 | 0 | 0.999990 | 2/104/114 | 0 (0) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | BGGR | [1, 1] | no | ✅ | 0.003898 | 0.335805 | 321 | -0.000038 | 0 | 0 | 0.999990 | 1/104/113 | 0 (0) | no | none |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | GBRG | [0, 1] | no | ❌ | 0.003899 | 0.335961 | 321 | -0.000039 | 0 | 0 | 0.999990 | 1/104/114 | 0 (0) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | GRBG | [1, 0] | no | ❌ | 0.003892 | 0.335683 | 321 | -0.000038 | 0 | 0 | 0.999990 | 2/104/113 | 0 (0) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/amaze_demosaic_RT.cc#L48) | RGGB | [0, 0] | yes | ❌ | 0.003894 | 0.335839 | 321 | -0.000039 | 0 | 0 | 0.999990 | 2/104/114 | 0 (0) | no | worse |
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

**Known differences:** Fornax produces typed floating-point samples, clamps
them to `[0, 1]`, and rounds to nearest before 16-bit quantization. LibRaw's
reference path truncates its final integer conversion; its border handling
also comes from the LibRaw pipeline.

## LMMSE

<!-- compare-demosaic:begin algo:lmmse -->
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | Within 64 | >64 B/E/F | >1024 (border) | Floor | GT evidence |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | BGGR | [1, 1] | no | ✅ | 0.000190 | 0.013795 | 1 | 0.000003 | 0 | 0 | 1.000000 | 0/0/0 | 0 (0) | no | none |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GBRG | [0, 1] | no | ✅ | 0.000191 | 0.013819 | 1 | 0.000002 | 0 | 0 | 1.000000 | 0/0/0 | 0 (0) | no | none |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GRBG | [1, 0] | no | ✅ | 0.000190 | 0.013790 | 1 | 0.000003 | 0 | 0 | 1.000000 | 0/0/0 | 0 (0) | no | none |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | RGGB | [0, 0] | yes | ✅ | 0.000191 | 0.013819 | 1 | 0.000003 | 0 | 0 | 1.000000 | 0/0/0 | 0 (0) | no | none |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | BGGR | [1, 1] | no | ❌ | 4.437840 | 11.439970 | 2094 | 0.129003 | 39 | 94 | 0.993530 | 28163/87240/47566 | 3981 (3841) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | GBRG | [0, 1] | no | ❌ | 4.435484 | 11.429312 | 2094 | 0.128798 | 39 | 94 | 0.993556 | 28015/86960/47334 | 3278 (3138) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | GRBG | [1, 0] | no | ❌ | 4.438037 | 11.442336 | 2094 | 0.129143 | 39 | 94 | 0.993529 | 27847/87344/47519 | 3470 (3329) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/lmmse_demosaic.cc#L42) | RGGB | [0, 0] | yes | ❌ | 4.435764 | 11.432290 | 2094 | 0.128915 | 39 | 94 | 0.993554 | 27706/87077/47287 | 2767 (2626) | no | worse |
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
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | Within 64 | >64 B/E/F | >1024 (border) | Floor | GT evidence |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | BGGR | [1, 1] | no | ✅ | 0.176913 | 1.247866 | 268 | -0.000195 | 4 | 17 | 0.999967 | 7/15/681 | 0 (0) | no | none |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GBRG | [0, 1] | no | ✅ | 0.176847 | 1.246238 | 277 | -0.000070 | 4 | 17 | 0.999967 | 6/15/674 | 0 (0) | no | none |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GRBG | [1, 0] | no | ✅ | 0.176938 | 1.247911 | 268 | -0.000190 | 4 | 17 | 0.999967 | 9/15/681 | 0 (0) | no | none |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | RGGB | [0, 0] | yes | ✅ | 0.176872 | 1.246299 | 277 | -0.000066 | 4 | 17 | 0.999967 | 7/15/674 | 0 (0) | no | none |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | BGGR | [1, 1] | no | ❌ | 0.177172 | 1.248237 | 268 | -0.000215 | 4 | 17 | 0.999966 | 31810/16/682 | 25 (25) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | GBRG | [0, 1] | no | ❌ | 0.177114 | 1.246664 | 277 | -0.000087 | 4 | 17 | 0.999967 | 31791/16/675 | 25 (25) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | GRBG | [1, 0] | no | ❌ | 0.177196 | 1.248281 | 268 | -0.000209 | 4 | 17 | 0.999967 | 31951/16/682 | 25 (25) | no | worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/rcd_demosaic.cc#L53) | RGGB | [0, 0] | yes | ❌ | 0.177138 | 1.246724 | 277 | -0.000084 | 4 | 17 | 0.999967 | 31931/16/675 | 25 (25) | no | worse |
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
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | Within 64 | >64 B/E/F | >1024 (border) | Floor | GT evidence |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | BGGR | [1, 1] | no | ❌ | 0.380479 | 0.825331 | 894 | 0.019062 | 1 | 8 | 0.999996 | 0/34/52 | 0 (0) | yes | worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | GBRG | [0, 1] | no | ✅ | 0.380477 | 0.825311 | 894 | 0.019054 | 1 | 8 | 0.999996 | 0/34/52 | 0 (0) | yes | not worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | GRBG | [1, 0] | no | ❌ | 0.380477 | 0.825326 | 894 | 0.019047 | 1 | 8 | 0.999996 | 0/34/52 | 0 (0) | yes | worse |
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | RGGB | [0, 0] | yes | ✅ | 0.380474 | 0.825306 | 894 | 0.019039 | 1 | 8 | 0.999996 | 0/34/52 | 0 (0) | yes | not worse |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | BGGR | [1, 1] | no | ❌ | 17.725608 | 41.671510 | 2958 | 0.064015 | 118 | 411 | 0.931894 | 9200/554546/864498 | 14118 (2) | yes | worse |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | GBRG | [0, 1] | no | ❌ | 17.724964 | 41.668982 | 2958 | 0.064435 | 118 | 411 | 0.931899 | 9211/554563/864621 | 14118 (2) | yes | worse |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | GRBG | [1, 0] | no | ❌ | 17.724217 | 41.667317 | 2958 | 0.063208 | 118 | 411 | 0.931905 | 9249/554547/864637 | 14118 (2) | yes | worse |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | RGGB | [0, 0] | yes | ❌ | 17.723573 | 41.664790 | 2958 | 0.063627 | 118 | 411 | 0.931911 | 9260/554564/864760 | 14118 (2) | yes | worse |
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
[the VNG reference notes](../../docs/reference/algorithms/vng.md) for source links and
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
| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | Within 64 | >64 B/E/F | >1024 (border) | Floor | GT evidence |
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: | --- | --- |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | BGGR | [1, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 1.000000 | 0/0/0 | 0 (0) | no | none |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | GBRG | [0, 1] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 1.000000 | 0/0/0 | 0 (0) | no | none |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | GRBG | [1, 0] | no | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 1.000000 | 0/0/0 | 0 (0) | no | none |
| [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L84) | RGGB | [0, 0] | yes | ✅ | 0.000000 | 0.000000 | 0 | 0.000000 | 0 | 0 | 1.000000 | 0/0/0 | 0 (0) | no | none |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | BGGR | [1, 1] | no | ⚖️ | 9.597235 | 41.012084 | 3233 | 0.155026 | 75 | 550 | 0.962735 | 6960/429552/346882 | 24400 (5) | no | not worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | GBRG | [0, 1] | no | ⚖️ | 9.596689 | 41.008767 | 3233 | 0.154939 | 75 | 550 | 0.962740 | 6937/429552/346919 | 24400 (5) | no | not worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | GRBG | [1, 0] | no | ⚖️ | 9.596174 | 41.006946 | 3233 | 0.154985 | 75 | 550 | 0.962745 | 7001/429552/346895 | 24400 (5) | no | not worse |
| [rawtherapee](https://github.com/RawTherapee/RawTherapee/blob/1924179107995b01b73cdc7f4fe91e44fc8227a7/rtengine/vng4_demosaic_RT.cc#L64) | RGGB | [0, 0] | yes | ⚖️ | 9.595628 | 41.003629 | 3233 | 0.154898 | 75 | 550 | 0.962750 | 6978/429552/346932 | 24400 (5) | no | not worse |

**Automatic ⚖️ evidence:** rawtherapee BGGR: 24205 divergent synthetic samples; Fornax MAE/P99 2468.960587/27274.000000 codes versus reference 3170.263375/36399.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** rawtherapee GBRG: 24489 divergent synthetic samples; Fornax MAE/P99 2450.134183/27274.000000 codes versus reference 3049.065172/36400.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** rawtherapee GRBG: 24478 divergent synthetic samples; Fornax MAE/P99 2466.522142/27274.000000 codes versus reference 3049.421766/36397.000000; reference-floor match: no.
**Automatic ⚖️ evidence:** rawtherapee RGGB: 24216 divergent synthetic samples; Fornax MAE/P99 2414.187521/27273.000000 codes versus reference 3183.554014/36397.000000; reference-floor match: no.
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
