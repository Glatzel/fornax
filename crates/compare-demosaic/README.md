<!-- BEGIN GENERATED -->
# compare-demosaic

Compares Fornax demosaic implementations with LibRaw and darktable at the
demosaic stage. The canonical comparison uses the full active area returned by
`fornax_devtool::raw_file()` and evaluates the same mosaic under all four Bayer
labels. White balance and the default output crop are not applied.

`canonical.json` is the source of truth for quality verdicts. The tables below
are generated from the committed baseline; do not edit their numbers by hand.
See [AGENTS.md](AGENTS.md) for the investigation workflow.

```bash
source scripts/setup.sh
cargo build --release -p compare-demosaic
target/release/compare-demosaic run [--artifacts] [--jobs N]
target/release/compare-demosaic diff <old.json> <new.json> [--fail-on regression|any|never]
target/release/compare-demosaic bless <canonical.json> --yes
```

The default run writes `target/compare-demosaic/canonical.json`. Failed rows
and `--artifacts` write log-scaled error maps and tile summaries under
`target/compare-demosaic/artifacts/`. A filtered selection must pass
`--shard NAME`, writes `NAME.shard.json`, and is never valid as a baseline.
Use `merge` to combine shards into `canonical.json`.

## Evaluation

Robust verdicts require all three conditions:

- MAE at most 2 codes.
- P99 absolute error at most 64 codes.
- At most a 0.001 fraction of channel samples above 1024 codes.

All errors are 16-bit codes. Exact means every sample is bit-identical.
Rounding-only means every difference is one code upward from the truncating
LibRaw reference. A warning means every robust limit passed but at least one
sample exceeded 1024 codes.

## AAHD

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | P999 | >1024 frac | Changed frac |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | ❌ | 747.885294 | 2099.339848 | 22138 | 8777 | 11968 | 0.128480 | 0.629272 |
| GBRG | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | ❌ | 103.105439 | 287.955685 | 18089 | 1327 | 2118 | 0.020369 | 0.629560 |
| GRBG | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | ❌ | 103.088593 | 287.963386 | 18089 | 1327 | 2119 | 0.020378 | 0.629544 |
| RGGB | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | ❌ | 747.938956 | 2099.423686 | 22138 | 8778 | 11965 | 0.128496 | 0.629277 |

> [!NOTE]
> AAHD exceeds the robust limits across every pattern. The green-red and green-blue label pair is materially closer to LibRaw than the red-first and blue-first pair.

## AHD

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | P999 | >1024 frac | Changed frac |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | ❌ | 6.673725 | 182.315588 | 22478 | 1 | 3020 | 0.001539 | 0.022739 |
| GBRG | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | ⚠️ | 0.875973 | 24.651269 | 5222 | 1 | 296 | 0.000125 | 0.022486 |
| GRBG | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | ⚠️ | 0.989094 | 26.458811 | 4835 | 1 | 352 | 0.000143 | 0.022518 |
| RGGB | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | ❌ | 6.817086 | 184.347885 | 23556 | 1 | 3093 | 0.001569 | 0.022979 |

> [!NOTE]
> AHD is close to LibRaw on the green-red and green-blue labels and diverges strongly on the red-first and blue-first labels. The heavy tail is concentrated in discrete interpolation decisions rather than a global offset.

## AMaZE

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | P999 | >1024 frac | Changed frac |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | ❌ | 525.714046 | 1278.370104 | 21316 | 5293 | 7820 | 0.141383 | 0.661846 |
| GBRG | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | ❌ | 79.498569 | 186.002581 | 9623 | 808 | 1442 | 0.004437 | 0.659406 |
| GRBG | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | ❌ | 79.498569 | 186.002581 | 9623 | 808 | 1442 | 0.004437 | 0.659406 |
| RGGB | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | ❌ | 525.714046 | 1278.370104 | 21316 | 5293 | 7820 | 0.141383 | 0.661846 |

> [!NOTE]
> AMaZE exceeds the robust limits for every pattern, with the strongest divergence on the red-first and blue-first labels.

## DCB

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | P999 | >1024 frac | Changed frac |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | ❌ | 656.695242 | 1795.569469 | 31865 | 8002 | 11223 | 0.131078 | 0.670707 |
| GBRG | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | ❌ | 88.009547 | 228.137404 | 7094 | 1105 | 1902 | 0.012320 | 0.668585 |
| GRBG | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | ❌ | 88.009547 | 228.137404 | 7094 | 1105 | 1902 | 0.012320 | 0.668585 |
| RGGB | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | ❌ | 656.695242 | 1795.569469 | 31865 | 8002 | 11223 | 0.131078 | 0.670707 |

> [!NOTE]
> DCB exceeds the robust limits for every pattern. The green-red and green-blue labels are substantially closer than the opposite diagonal.

## DHT

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | P999 | >1024 frac | Changed frac |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | ❌ | 527.009423 | 1382.556065 | 24042 | 5936 | 11447 | 0.135363 | 0.660867 |
| GBRG | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | ❌ | 96.496395 | 220.513723 | 15912 | 906 | 1620 | 0.006594 | 0.662065 |
| GRBG | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | ❌ | 96.496395 | 220.513723 | 15912 | 906 | 1620 | 0.006594 | 0.662065 |
| RGGB | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | ❌ | 527.009423 | 1382.556065 | 24042 | 5936 | 11447 | 0.135363 | 0.660867 |

> [!NOTE]
> DHT exceeds the robust limits for every pattern and remains materially divergent even on its closest label pair.

## LMMSE

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | P999 | >1024 frac | Changed frac |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | ❌ | 334.214702 | 931.666478 | 20371 | 4393 | 7808 | 0.096257 | 0.658283 |
| GBRG | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | ❌ | 44.776609 | 103.955914 | 9522 | 463 | 884 | 0.000500 | 0.657319 |
| GRBG | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | ❌ | 44.776464 | 103.955704 | 9522 | 463 | 884 | 0.000500 | 0.657319 |
| RGGB | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | ❌ | 334.214427 | 931.666276 | 20371 | 4393 | 7808 | 0.096257 | 0.658283 |

> [!NOTE]
> LMMSE fails its headline limits on every pattern. The red-first and blue-first labels also exceed the high-error fraction limit.

## Linear

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | P999 | >1024 frac | Changed frac |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | 🟢 | 0.293568 | 0.541820 | 1 | 1 | 1 | 0.000000 | 0.293568 |
| GBRG | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | 🟢 | 0.294064 | 0.542277 | 1 | 1 | 1 | 0.000000 | 0.294064 |
| GRBG | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | 🟢 | 0.294064 | 0.542277 | 1 | 1 | 1 | 0.000000 | 0.294064 |
| RGGB | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | 🟢 | 0.293568 | 0.541820 | 1 | 1 | 1 | 0.000000 | 0.293568 |

> [!NOTE]
> Linear differs only because Fornax rounds to nearest while LibRaw truncates.

## PPG

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | P999 | >1024 frac | Changed frac |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | ✅ | 0.000000 | 0.000000 | 0 | 0 | 0 | 0.000000 | 0.000000 |
| GBRG | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | ✅ | 0.000000 | 0.000000 | 0 | 0 | 0 | 0.000000 | 0.000000 |
| GRBG | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | ✅ | 0.000000 | 0.000000 | 0 | 0 | 0 | 0.000000 | 0.000000 |
| RGGB | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | ✅ | 0.000000 | 0.000000 | 0 | 0 | 0 | 0.000000 | 0.000000 |
| BGGR | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | ⚠️ | 0.314865 | 21.504192 | 13493 | 1 | 1 | 0.000029 | 0.147196 |
| GBRG | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | ⚠️ | 0.186701 | 3.106662 | 2221 | 1 | 1 | 0.000001 | 0.146916 |
| GRBG | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | ⚠️ | 0.186701 | 3.106662 | 2221 | 1 | 1 | 0.000001 | 0.146916 |
| RGGB | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363) | ⚠️ | 0.314865 | 21.504192 | 13493 | 1 | 1 | 0.000029 | 0.147196 |

> [!NOTE]
> Fornax PPG is bit-identical to darktable PPG for every pattern. Against LibRaw it remains within the robust limits despite isolated outliers.

## RCD

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | P999 | >1024 frac | Changed frac |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | ⚠️ | 0.372633 | 5.609573 | 5922 | 8 | 48 | 0.000002 | 0.065598 |
| GBRG | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | ⚠️ | 0.083077 | 1.015566 | 1359 | 2 | 11 | 0.000000 | 0.033374 |
| GRBG | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | ⚠️ | 0.083077 | 1.015566 | 1359 | 2 | 11 | 0.000000 | 0.033374 |
| RGGB | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | ⚠️ | 0.372633 | 5.609573 | 5922 | 8 | 48 | 0.000002 | 0.065598 |

> [!NOTE]
> RCD remains within every robust limit. The green-red and green-blue labels are closest, while the opposite diagonal retains a small outlier tail.

## VNG

| Pattern | Reference | Verdict | MAE | RMSE | Max | P99 | P999 | >1024 frac | Changed frac |
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| BGGR | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | ❌ | 4417.576591 | 9258.126444 | 65535 | 37184 | 48831 | 0.489596 | 0.832106 |
| GBRG | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | ❌ | 2782.662776 | 6477.675994 | 65535 | 30349 | 45809 | 0.388432 | 0.827285 |
| GRBG | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | ❌ | 2784.676172 | 6479.882985 | 65535 | 30358 | 45804 | 0.388330 | 0.827284 |
| RGGB | [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | ❌ | 4417.097668 | 9256.884772 | 65535 | 37107 | 48859 | 0.489642 | 0.832107 |
| BGGR | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | ❌ | 4243.524941 | 9140.000579 | 65535 | 36685 | 46497 | 0.449676 | 0.665300 |
| GBRG | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | ❌ | 2762.780291 | 6476.794907 | 65535 | 30351 | 45810 | 0.388057 | 0.665211 |
| GRBG | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | ❌ | 2764.732896 | 6479.005956 | 65535 | 30360 | 45806 | 0.387967 | 0.665210 |
| RGGB | [libraw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199) | ❌ | 4243.041290 | 9138.746679 | 65535 | 36691 | 46499 | 0.449708 | 0.665299 |

> [!NOTE]
> Both VNG references show a broad failure rather than isolated outliers. The red-first and blue-first labels are the worse pair.

Reference builds: LibRaw 0.22.3; darktable 5.6.2 (20891f6c6fa7be995e5bf9dff9d1ee5062051773).

Legend:

- `✅`: every compared sample is bit-identical or every robust limit passes
  without high-error outliers.
- `🟢`: differences are one code upward because Fornax rounds to nearest while
  LibRaw truncates.
- `⚠️`: every robust limit passes, but isolated high-error samples remain.
- `❌`: at least one limit in `failed_limits` failed.
<!-- END GENERATED -->
