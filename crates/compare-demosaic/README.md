# compare-demosaic

Compares Fornax demosaic implementations with LibRaw and darktable at the
demosaic stage.

The canonical quality fixture is a centered `512x512` `GBRG` crop from
`external/raw-images/images/colorchart-eos-7d.cr2` (`CC-BY 4.0`). It is decoded
from the active area, normalized using the black and white levels, and passed
unchanged as 16-bit codes to every implementation. White balance is not applied.

Seeded synthetic images remain as deterministic stress cases for odd
dimensions, partial row chunks, tiny images, and saturation. They are not the
primary quality signal.

```bash
source scripts/setup.sh
pixi run -e dev compare-demosaic
```

The command writes `results/canonical.json`; it does not edit this README.
Set `FORNAX_COMPARE_ARTIFACTS=1` to write reference, actual, and amplified-diff
PNGs under `target/compare-demosaic/` whenever a comparison differs.

## Evaluation

For robust comparisons, verdicts require:

- MAE at most 2 codes.
- P99 absolute error at most 64 codes.
- At most 0.1% of channel samples above 1024 codes.

MAE, RMSE, PSNR, P95/P99/P99.9, high-error percentages, per-channel MAE,
luminance/chroma RMSE, changed percentage, and maximum error are stored in the
JSON result. Maximum error is diagnostic rather than the sole pass/fail rule.

Exact and rounding-only cases retain their stricter semantics. Fornax clamps
reconstructed RGB to `[0, 1]` before 16-bit quantization in comparison cases.

## Results

<!-- BEGIN:compare-table -->
### AAHD

| Fixture | Reference Library | Bayer | Verdict | MAE | RMSE | P99 | >1024 |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| real-colorchart-center | [LibRaw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | GBRG | ❌ | 63.019 | 119.944 | 462 | 0.001% |

### AHD

| Fixture | Reference Library | Bayer | Verdict | MAE | RMSE | P99 | >1024 |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| real-colorchart-center | [LibRaw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | GBRG | ⚠️ | 0.503 | 9.255 | 1 | 0.000% |

> [!NOTE]
> AHD passes the robust real-photo limits; its isolated maximum error remains
> listed in JSON for diagnostics.

### AMaZE

| Fixture | Reference Library | Bayer | Verdict | MAE | RMSE | P99 | >1024 |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| real-colorchart-center | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GBRG | ❌ | 54.327 | 96.616 | 351 | 0.006% |

### DCB

| Fixture | Reference Library | Bayer | Verdict | MAE | RMSE | P99 | >1024 |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| real-colorchart-center | [LibRaw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | GBRG | ❌ | 56.541 | 103.025 | 404 | 0.000% |

### DHT

| Fixture | Reference Library | Bayer | Verdict | MAE | RMSE | P99 | >1024 |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| real-colorchart-center | [LibRaw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | GBRG | ❌ | 81.677 | 139.560 | 485 | 0.035% |

### Linear

| Fixture | Reference Library | Bayer | Verdict | MAE | RMSE | P99 | >1024 |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| real-colorchart-center | [LibRaw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | GBRG | 🟢 | 0.307 | 0.554 | 1 | 0.000% |

### LMMSE

| Fixture | Reference Library | Bayer | Verdict | MAE | RMSE | P99 | >1024 |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| real-colorchart-center | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GBRG | ❌ | 35.618 | 63.921 | 240 | 0.000% |

### PPG

| Fixture | Reference Library | Bayer | Verdict | MAE | RMSE | P99 | >1024 |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| real-colorchart-center | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | GBRG | ✅ | 0.000 | 0.000 | 0 | 0.000% |
| real-colorchart-center | [LibRaw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | GBRG | ⚠️ | 0.181 | 2.053 | 1 | 0.000% |

> [!NOTE]
> The LibRaw PPG comparison differs around clipping and rounding boundaries;
> the darktable PPG comparison is exact.

### RCD

| Fixture | Reference Library | Bayer | Verdict | MAE | RMSE | P99 | >1024 |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| real-colorchart-center | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GBRG | ⚠️ | 0.039 | 0.731 | 1 | 0.000% |

> [!NOTE]
> RCD differs from darktable only in isolated boundary samples on this crop.

### VNG

| Fixture | Reference Library | Bayer | Verdict | MAE | RMSE | P99 | >1024 |
| --- | --- | --- | --- | ---: | ---: | ---: | ---: |
| real-colorchart-center | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | GBRG | ❌ | 1808.794 | 3004.037 | 9499 | 49.093% |
| real-colorchart-center | [LibRaw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | GBRG | ❌ | 1796.821 | 3003.810 | 9490 | 49.092% |

Reference builds: LibRaw 0.22.3; darktable 5.6.2 (20891f6c6fa7be995e5bf9dff9d1ee5062051773).
<!-- END:compare-table -->

Legend:

- `✅`: every compared sample is bit-identical.
- `🟢`: Fornax rounds to nearest while LibRaw truncates, so differences are one code upward.
- `⚠️`: every sample is within the case's declared maximum error.
- For robust cases, `⚠️` means the MAE, P99, and high-error limits all pass.
- `❌`: at least one sample exceeds that tolerance, or the robust limits fail.
