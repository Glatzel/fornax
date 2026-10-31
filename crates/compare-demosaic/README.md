# compare-demosaic

Compares Fornax demosaic implementations with LibRaw and darktable at the
demosaic stage.

The canonical comparison uses the full active area of the colorchart CR2
returned by `fornax_devtool::raw_file()`. The mosaic is normalized using its
black and white levels and evaluated under all four Bayer pattern labels.
White balance and the default output crop are not applied.

The regression tests share a once-decoded, centered 128x128 crop. Each
pattern-algorithm-reference combination is an independent `rstest` case with
its own checksum snapshot, allowing the test harness to run them in parallel.
A separate 128x128 `synthetic_mono_image` smoke test runs every algorithm for
fast failure feedback, but it is not used for quality verdicts.

```bash
source scripts/setup.sh
cargo run --release -p compare-demosaic
```

Setup installs the Rust environment, initializes `external/raw-images`, and
materializes the darktable reference sources. The comparison command writes
`target/compare-demosaic/summary.json` and does not modify this README.

## Evaluation

For robust comparisons, verdicts require:

- MAE at most 2 codes.
- P99 absolute error at most 64 codes.
- At most 0.1% of channel samples above 1024 codes.

The summary records MAE, RMSE, maximum error, changed percentage, P99, and the
percentage above 1024 codes. Maximum error is diagnostic rather than the sole
pass/fail rule.

Exact and rounding-only cases retain their stricter semantics. Fornax clamps
reconstructed RGB to `[0, 1]` before 16-bit quantization in comparison cases.

## Results

These tables were reviewed by an agent using
`target/compare-demosaic/summary.json`.

### AAHD

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | >1024 | Changed |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [LibRaw][aahd] | ❌ | 748.027 | 2099.650 | 22138 | 8772 | 12.837% | 62.925% |
| GBRG | [LibRaw][aahd] | ❌ | 102.133 | 272.076 | 8349 | 1321 | 2.022% | 62.956% |
| GRBG | [LibRaw][aahd] | ❌ | 102.116 | 272.082 | 8349 | 1321 | 2.023% | 62.954% |
| RGGB | [LibRaw][aahd] | ❌ | 748.091 | 2099.763 | 22138 | 8772 | 12.838% | 62.926% |

> [!NOTE]
> AAHD fails all robust limits for every pattern. `GBRG` and `GRBG` are the
> closest at MAE 102.1, P99 1321, and about 2.02% above 1024 codes; `BGGR` and
> `RGGB` are substantially worse at MAE about 748 and 12.84% above 1024 codes.

### AHD

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | >1024 | Changed |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [LibRaw][ahd] | ❌ | 6.705 | 182.755 | 22478 | 1 | 0.155% | 2.159% |
| GBRG | [LibRaw][ahd] | ⚠️ | 0.879 | 24.711 | 5222 | 1 | 0.013% | 2.128% |
| GRBG | [LibRaw][ahd] | ⚠️ | 0.993 | 26.523 | 4835 | 1 | 0.014% | 2.131% |
| RGGB | [LibRaw][ahd] | ❌ | 6.849 | 184.792 | 23556 | 1 | 0.158% | 2.183% |

> [!NOTE]
> AHD passes every robust limit for `GBRG` and `GRBG`. `BGGR` and `RGGB` fail
> both the MAE limit and the 0.1% high-error limit, while their isolated
> maximum errors remain diagnostic rather than the direct failure criterion.

### AMaZE

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | >1024 | Changed |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [darktable][amaze] | ❌ | 526.780 | 1280.791 | 21316 | 5288 | 14.170% | 66.196% |
| GBRG | [darktable][amaze] | ❌ | 79.188 | 185.041 | 9623 | 802 | 0.431% | 65.947% |
| GRBG | [darktable][amaze] | ❌ | 79.188 | 185.041 | 9623 | 802 | 0.431% | 65.947% |
| RGGB | [darktable][amaze] | ❌ | 526.780 | 1280.791 | 21316 | 5288 | 14.170% | 66.196% |

> [!NOTE]
> AMaZE fails MAE, P99, and the high-error limit for all patterns. `BGGR` and
> `RGGB` share the worst failures, with about 14.17% of samples above 1024.

### DCB

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | >1024 | Changed |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [LibRaw][dcb] | ❌ | 657.622 | 1797.708 | 31865 | 8002 | 13.111% | 67.092% |
| GBRG | [LibRaw][dcb] | ❌ | 87.898 | 227.661 | 6783 | 1102 | 1.224% | 66.882% |
| GRBG | [LibRaw][dcb] | ❌ | 87.898 | 227.661 | 6783 | 1102 | 1.224% | 66.882% |
| RGGB | [LibRaw][dcb] | ❌ | 657.622 | 1797.708 | 31865 | 8002 | 13.111% | 67.092% |

> [!NOTE]
> DCB exceeds all robust limits for every pattern. Its paired `GBRG`/`GRBG`
> rows are materially closer than `BGGR`/`RGGB`, but still fail MAE at 87.9,
> P99 at 1102, and the high-error limit at 1.224%.

### DHT

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | >1024 | Changed |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [LibRaw][dht] | ❌ | 527.470 | 1385.385 | 24042 | 5913 | 13.536% | 66.113% |
| GBRG | [LibRaw][dht] | ❌ | 94.401 | 203.131 | 15912 | 889 | 0.602% | 66.216% |
| GRBG | [LibRaw][dht] | ❌ | 94.401 | 203.131 | 15912 | 889 | 0.602% | 66.216% |
| RGGB | [LibRaw][dht] | ❌ | 527.470 | 1385.385 | 24042 | 5913 | 13.536% | 66.113% |

> [!NOTE]
> DHT fails all robust limits for every pattern. Even the closer
> `GBRG`/`GRBG` cases have MAE 94.4, P99 889, and 0.602% above 1024 codes.

### Linear

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | >1024 | Changed |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [LibRaw][linear] | 🟢 | 0.294 | 0.542 | 1 | 1 | 0.000% | 29.367% |
| GBRG | [LibRaw][linear] | 🟢 | 0.294 | 0.542 | 1 | 1 | 0.000% | 29.416% |
| GRBG | [LibRaw][linear] | 🟢 | 0.294 | 0.542 | 1 | 1 | 0.000% | 29.416% |
| RGGB | [LibRaw][linear] | 🟢 | 0.294 | 0.542 | 1 | 1 | 0.000% | 29.367% |

> [!NOTE]
> Linear differs only because Fornax rounds to nearest while LibRaw truncates;
> every difference is one code upward for every pattern.

### LMMSE

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | >1024 | Changed |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [darktable][lmmse] | ❌ | 334.642 | 932.606 | 20371 | 4395 | 9.644% | 65.850% |
| GBRG | [darktable][lmmse] | ❌ | 44.691 | 103.502 | 5416 | 462 | 0.048% | 65.751% |
| GRBG | [darktable][lmmse] | ❌ | 44.691 | 103.501 | 5416 | 462 | 0.048% | 65.751% |
| RGGB | [darktable][lmmse] | ❌ | 334.642 | 932.606 | 20371 | 4395 | 9.644% | 65.850% |

> [!NOTE]
> LMMSE fails MAE and P99 for all patterns. `GBRG` and `GRBG` remain below the
> high-error limit at 0.048%, while `BGGR` and `RGGB` also fail it at 9.644%.

### PPG

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | >1024 | Changed |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [darktable][ppg-darktable] | ✅ | 0.000 | 0.000 | 0 | 0 | 0.000% | 0.000% |
| BGGR | [LibRaw][ppg-libraw] | ⚠️ | 0.315 | 21.504 | 13493 | 1 | 0.003% | 14.720% |
| GBRG | [darktable][ppg-darktable] | ✅ | 0.000 | 0.000 | 0 | 0 | 0.000% | 0.000% |
| GBRG | [LibRaw][ppg-libraw] | ⚠️ | 0.187 | 3.107 | 2221 | 1 | 0.0001% | 14.692% |
| GRBG | [darktable][ppg-darktable] | ✅ | 0.000 | 0.000 | 0 | 0 | 0.000% | 0.000% |
| GRBG | [LibRaw][ppg-libraw] | ⚠️ | 0.187 | 3.107 | 2221 | 1 | 0.0001% | 14.692% |
| RGGB | [darktable][ppg-darktable] | ✅ | 0.000 | 0.000 | 0 | 0 | 0.000% | 0.000% |
| RGGB | [LibRaw][ppg-libraw] | ⚠️ | 0.315 | 21.504 | 13493 | 1 | 0.003% | 14.720% |

> [!NOTE]
> darktable PPG is bit-identical for every pattern. LibRaw PPG passes all robust
> limits everywhere despite isolated large errors; `BGGR`/`RGGB` are weaker
> than `GBRG`/`GRBG` but remain within MAE, P99, and high-error thresholds.

### RCD

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | >1024 | Changed |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [darktable][rcd] | ⚠️ | 0.376 | 5.610 | 5922 | 8 | 0.0002% | 6.618% |
| GBRG | [darktable][rcd] | ⚠️ | 0.084 | 1.020 | 1359 | 2 | 0.000002% | 3.367% |
| GRBG | [darktable][rcd] | ⚠️ | 0.084 | 1.020 | 1359 | 2 | 0.000002% | 3.367% |
| RGGB | [darktable][rcd] | ⚠️ | 0.376 | 5.610 | 5922 | 8 | 0.0002% | 6.618% |

> [!NOTE]
> RCD passes every robust limit for all patterns. `GBRG`/`GRBG` are closest;
> `BGGR`/`RGGB` still pass despite larger MAE, P99 8, and isolated outliers.

### VNG

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | >1024 | Changed |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [darktable][vng-darktable] | ❌ | 4425.727 | 9266.991 | 65535 | 37198 | 49.045% | 83.284% |
| BGGR | [LibRaw][vng-libraw] | ❌ | 4251.676 | 9148.800 | 65535 | 36696 | 45.054% | 66.642% |
| GBRG | [darktable][vng-darktable] | ❌ | 2787.958 | 6483.910 | 65535 | 30364 | 38.917% | 82.802% |
| GBRG | [LibRaw][vng-libraw] | ❌ | 2768.098 | 6483.031 | 65535 | 30367 | 38.880% | 66.635% |
| GRBG | [darktable][vng-darktable] | ❌ | 2789.975 | 6486.119 | 65535 | 30374 | 38.907% | 82.802% |
| GRBG | [LibRaw][vng-libraw] | ❌ | 2770.055 | 6485.244 | 65535 | 30374 | 38.871% | 66.635% |
| RGGB | [darktable][vng-darktable] | ❌ | 4425.247 | 9265.749 | 65535 | 37120 | 49.049% | 83.284% |
| RGGB | [LibRaw][vng-libraw] | ❌ | 4251.192 | 9147.545 | 65535 | 36702 | 45.057% | 66.642% |

> [!NOTE]
> Both VNG references fail every robust limit for every pattern. `BGGR` and
> `RGGB` are worst, exceeding 45% high-error samples; even
> `GBRG`/`GRBG` have MAE around 2.8k and P99 above 30k.

Reference builds: LibRaw 0.22.3; darktable 5.6.2
(20891f6c6fa7be995e5bf9dff9d1ee5062051773).

Legend:

- `✅`: every compared sample is bit-identical.
- `🟢`: Fornax rounds to nearest while LibRaw truncates, so differences are one
  code upward.
- `⚠️`: every robust limit passes, though isolated diagnostic errors remain.
- `❌`: at least one robust limit fails.

[aahd]: https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771
[ahd]: https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293
[amaze]: https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129
[dcb]: https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849
[dht]: https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012
[linear]: https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146
[lmmse]: https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120
[ppg-darktable]: https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20
[ppg-libraw]: https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363
[rcd]: https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119
[vng-darktable]: https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171
[vng-libraw]: https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199
