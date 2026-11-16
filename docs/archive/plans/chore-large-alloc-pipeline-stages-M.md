# M: Additional Pipeline Measurements

## Result

This item adds measurements only. It changes no production code and does not
commit the temporary stage probe.

The new data covers:

- round 1 separately from rounds 2-5;
- `GLIBC_TUNABLES=glibc.malloc.hugetlb=1` as a closer one-shot proxy;
- all stages at 15 Rayon workers;
- TIFF and JPEG encoding after the render.

## Host

- WSL2 Debian, AMD Ryzen 7 5800H, 15 visible CPUs
- Kernel `6.6.87.1-microsoft-standard-WSL2`
- glibc `2.41-12+deb13u3`
- THP `enabled=[madvise]`, `defrag=[madvise]`
- Cargo `1.100.0`, page-cache-hot inputs
- Baseline: plan branch after merged R4 and R6a

## Round-One Split

Three fresh processes by five interleaved rounds. Values are render median
`[min-max]` in milliseconds. `hugetlb=1` is the glibc tunable proxy.

| Input | Workers | Proxy | Round 1 | Rounds 2-5 |
| --- | ---: | --- | ---: | ---: |
| 5D3 uncompressed | 1 | default | 2566.942 `[2555.441-2568.590]` | 2573.208 `[2552.625-2611.061]` |
| 5D3 uncompressed | 1 | `hugetlb=1` | 2348.331 `[2253.970-2408.165]` | 2266.450 `[2250.829-2396.918]` |
| 5D3 uncompressed | 15 | default | 939.721 `[914.121-943.730]` | 927.186 `[909.859-955.849]` |
| 5D3 uncompressed | 15 | `hugetlb=1` | 881.439 `[877.455-906.947]` | 883.218 `[874.711-922.811]` |
| Sigma fp 4K | 1 | default | 951.602 `[951.525-958.417]` | 954.311 `[946.653-965.638]` |
| Sigma fp 4K | 1 | `hugetlb=1` | 856.031 `[845.520-910.390]` | 844.922 `[841.862-887.745]` |
| Sigma fp 4K | 15 | default | 351.830 `[346.095-354.150]` | 347.972 `[342.956-362.072]` |
| Sigma fp 4K | 15 | `hugetlb=1` | 340.246 `[337.409-342.721]` | 334.972 `[330.826-342.753]` |

Cold round 1 is not a separate first-touch cliff in these CLI-style probes. At
15 workers, allocator reuse still saves about 5.1% on 5D3 and 3.2% on Sigma;
at one worker it saves about 11.8% and 10.8%. These shares are smaller than the
pre-R4 22.8% proxy because the owned-demosaic change removed one full RGB
allocation.

## Fifteen-Worker Stage Table

Three fresh processes by five interleaved rounds, default allocator mode.
Wall time is median `[min-max]` milliseconds, followed by median minor faults
and peak RSS.

| Input | Stage | Wall ms | Minor faults | Peak RSS KiB |
| --- | --- | ---: | ---: | ---: |
| 5D3 uncompressed | decode | 36.967 `[30.552-43.156]` | 4419 `[4053-4472]` | 96588 `[84812-96864]` |
| 5D3 uncompressed | crop/cast | 52.267 `[49.550-58.984]` | 27769 `[27692-27829]` | 188120 `[187728-188380]` |
| 5D3 uncompressed | preprocess | 61.911 `[58.302-63.853]` | 27804 `[27717-28178]` | 188236 `[187668-188368]` |
| 5D3 uncompressed | demosaic | 891.457 `[884.710-924.613]` | 124570 `[123816-125091]` | 358616 `[358240-359052]` |
| 5D3 uncompressed | render | 925.462 `[909.859-955.849]` | 124981 `[124184-126339]` | 358728 `[358388-359136]` |
| 5D3 uncompressed | TIFF encode | 1171.546 `[1152.941-1181.143]` | 167324 `[166175-168449]` | 398804 `[398488-398980]` |
| 5D3 uncompressed | JPEG encode | 1859.131 `[1846.656-1888.387]` | 177874 `[175669-178577]` | 358648 `[358356-359188]` |
| Sigma fp 4K | decode | 15.495 `[11.520-18.827]` | 2936 `[2898-3298]` | 25512 `[18084-32684]` |
| Sigma fp 4K | crop/cast | 22.576 `[20.468-24.456]` | 11554 `[11533-11641]` | 65320 `[33064-65960]` |
| Sigma fp 4K | preprocess | 27.100 `[24.719-35.933]` | 11593 `[11540-12045]` | 65436 `[31272-65704]` |
| Sigma fp 4K | demosaic | 338.235 `[335.022-348.346]` | 47956 `[47426-48665]` | 135100 `[134836-135376]` |
| Sigma fp 4K | render | 347.747 `[342.956-362.072]` | 48145 `[47899-48496]` | 135140 `[134828-135408]` |
| Sigma fp 4K | TIFF encode | 443.924 `[440.012-452.186]` | 63917 `[62883-64345]` | 151592 `[151292-151824]` |
| Sigma fp 4K | JPEG encode | 677.680 `[671.910-700.759]` | 66900 `[66157-67509]` | 135152 `[134824-135524]` |

The encode stage is large enough to benchmark separately:

- TIFF adds about 246 ms on 5D3 and 96 ms on Sigma at 15 workers.
- JPEG adds about 934 ms on 5D3 and 330 ms on Sigma at 15 workers.
- TIFF adds a complete output-sized memory allocation and about 42k faults on
  5D3; JPEG avoids the output-sized peak but spends more CPU in color conversion
  and compression.

These encode timings use `/dev/null` as the output sink; they include the
required Rgb conversion and encoder work but exclude filesystem writeback.

## Decisions

- Kept: none; this is a measurement-only change.
- Reverted: the temporary stage probe used to obtain the tables.
- R9 remains deferred pending a separate writer-interface design.

## Validation

- The temporary probe compiled in release mode and was removed before commit.
- `git diff --check`.
- No production or test code changed.

The `AnonHugePages` and minor-fault evidence for the Linux hint remains in the
existing DNG profile report. These numbers are host-specific and unmeasured on
Windows or macOS.
