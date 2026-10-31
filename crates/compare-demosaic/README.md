# compare-demosaic

Compares Fornax demosaic implementations with LibRaw and darktable at the
demosaic stage.

The canonical comparison uses the full active area of the pinned
`external/raw-images/images/colorchart-eos-7d.cr2` fixture. Its metadata
identifies a `GBRG` CFA. Samples are normalized using the CR2 black and white
levels and passed unchanged as 16-bit codes to every implementation. White
balance is not applied, the default crop is not applied, and the active-area
geometry is preserved.

Seeded synthetic images remain as deterministic stress cases for odd
dimensions, partial row chunks, tiny images, and saturation. They are not the
primary quality signal.

The comparison tests run each algorithm against one shared 512x512 center crop
of the CR2 through `rstest`, so the real-image cases run quickly and in
parallel. The full active-area decode is checked separately.
`scripts/get_raw_samples.py` remains available as a manual development utility
for downloading additional RAW samples; comparison tests and CI do not use a
downloaded corpus.

```bash
source scripts/setup.sh
pixi run -e dev compare-demosaic
```

The command writes only `crates/compare-demosaic/results/canonical.json` and
shows a stderr progress bar while each algorithm/reference case runs.
Non-interactive output, including CI logs, omits the progress bar. The command
does not change this README and does not write per-fixture JSON or image
artifacts.

Regenerate the marked comparison block explicitly after refreshing the JSON:

```bash
cargo run -p compare-demosaic --features render-readme --bin render-readme
```

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

The block below is generated from `results/canonical.json` by the
`render-readme` binary. Run the comparison command and then the renderer after
changing an algorithm or the canonical fixture.

<!-- BEGIN:compare-table -->

### AAHD

| Reference Library                                                                                                            | Bayer | Verdict | MAE     | RMSE    | P99  | >1024  |
|------------------------------------------------------------------------------------------------------------------------------|-------|---------|---------|---------|------|--------|
| [LibRaw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/aahd_demosaic.cpp#L771) | GBRG  | ❌      | 102.133 | 272.076 | 1321 | 2.022% |

### AHD

| Reference Library                                                                                                           | Bayer | Verdict | MAE   | RMSE   | P99 | >1024  |
|-----------------------------------------------------------------------------------------------------------------------------|-------|---------|-------|--------|-----|--------|
| [LibRaw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/ahd_demosaic.cpp#L293) | GBRG  | ⚠️      | 0.879 | 24.711 |   1 | 0.013% |

### AMaZE

| Reference Library                                                                                                                       | Bayer | Verdict | MAE    | RMSE    | P99 | >1024  |
|-----------------------------------------------------------------------------------------------------------------------------------------|-------|---------|--------|---------|-----|--------|
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/amaze.cc#L129) | GBRG  | ❌      | 79.188 | 185.041 | 802 | 0.431% |

### DCB

| Reference Library                                                                                                           | Bayer | Verdict | MAE    | RMSE    | P99  | >1024  |
|-----------------------------------------------------------------------------------------------------------------------------|-------|---------|--------|---------|------|--------|
| [LibRaw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dcb_demosaic.cpp#L849) | GBRG  | ❌      | 87.898 | 227.661 | 1102 | 1.224% |

### DHT

| Reference Library                                                                                                            | Bayer | Verdict | MAE    | RMSE    | P99 | >1024  |
|------------------------------------------------------------------------------------------------------------------------------|-------|---------|--------|---------|-----|--------|
| [LibRaw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/dht_demosaic.cpp#L1012) | GBRG  | ❌      | 94.401 | 203.131 | 889 | 0.602% |

### Linear

| Reference Library                                                                                                            | Bayer | Verdict | MAE   | RMSE  | P99 | >1024  |
|------------------------------------------------------------------------------------------------------------------------------|-------|---------|-------|-------|-----|--------|
| [LibRaw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146) | GBRG  | 🟢      | 0.294 | 0.542 |   1 | 0.000% |

### LMMSE

| Reference Library                                                                                                                      | Bayer | Verdict | MAE    | RMSE    | P99 | >1024  |
|----------------------------------------------------------------------------------------------------------------------------------------|-------|---------|--------|---------|-----|--------|
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120) | GBRG  | ❌      | 44.691 | 103.502 | 462 | 0.048% |

### PPG

| Reference Library                                                                                                                   | Bayer | Verdict | MAE   | RMSE  | P99 | >1024  |
|-------------------------------------------------------------------------------------------------------------------------------------|-------|---------|-------|-------|-----|--------|
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20) | GBRG  | ✅      | 0.000 | 0.000 |   0 | 0.000% |
| [LibRaw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L363)        | GBRG  | ⚠️      | 0.187 | 3.107 |   1 | 0.000% |

### RCD

| Reference Library                                                                                                                    | Bayer | Verdict | MAE   | RMSE  | P99 | >1024  |
|--------------------------------------------------------------------------------------------------------------------------------------|-------|---------|-------|-------|-----|--------|
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c#L119) | GBRG  | ⚠️      | 0.084 | 1.020 |   2 | 0.000% |

### VNG

| Reference Library                                                                                                                    | Bayer | Verdict | MAE      | RMSE     | P99   | >1024   |
|--------------------------------------------------------------------------------------------------------------------------------------|-------|---------|----------|----------|-------|---------|
| [darktable](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171) | GBRG  | ❌      | 2787.958 | 6483.910 | 30364 | 38.917% |
| [LibRaw](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199)         | GBRG  | ❌      | 2768.098 | 6483.031 | 30367 | 38.880% |
<!-- END:compare-table -->

Legend:

- `✅`: every compared sample is bit-identical.
- `🟢`: Fornax rounds to nearest while LibRaw truncates, so differences are one code upward.
- `⚠️`: every sample is within the case's declared maximum error.
- For robust cases, `⚠️` means the MAE, P99, and high-error limits all pass.
- `❌`: at least one sample exceeds that tolerance, or the robust limits fail.
