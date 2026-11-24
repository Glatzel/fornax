# compare-demosaic

Synthetic demosaic quality comparison harness for Fornax's production-ready
Bayer demosaicing methods (`linear` and `ppg`).

## Methodology

- **Chart:** [demosaic_chart.svg](chart/demosaic_chart.svg) (`2048 × 2048`, 4×4 grid of `512 × 512` tiles with even-aligned `448 × 448` evaluation ROIs).
- **Rasterization & Filter:** 4× supersampling (`8192 × 8192`) via `resvg`, exact IEC 61966-2-1 sRGB-to-linear EOTF in `float32`, followed by 4×4 box (area-average) downsampling in linear light to `2048 × 2048`.
- **Unclamped Float (`< 0.0` and `> 1.0`):** Row 4 tiles (`signed_zero_crossing` `[-0.25, +0.25]`, `hdr_specular` `[0.0, 4.0]`, and `wide_range_signed_hdr` `[-0.5, +2.5]`) are analytically synthesized directly in IEEE 754 `float32` so clamping or gamma-domain bugs are caught immediately.
- **Mosaic-Only Noise:** Tile 13 (`noisy_flat_fields`) stores noiseless flat patches in `ground_truth_rgb.tif` and injects deterministic fixed-seed Gaussian noise only into the 4 Bayer mosaics (`bayer_*.tif`), evaluated separately from clean `[0, 1]` and unclamped-float metrics.

## Usage

```bash
# Run full pipeline (chart -> Rust demosaic -> Python metrics + README update)
pixi run -e compare-demosaic compare-demosaic --method all

# Run a single method
pixi run -e compare-demosaic compare-demosaic --method ppg

# Individual stages
pixi run -e compare-demosaic chart
pixi run -e compare-demosaic demosaic --method linear,ppg
pixi run -e compare-demosaic evaluate --method all

# Lint & tests
pixi run -e compare-demosaic compare-demosaic-test
```

## Results

<!-- compare-demosaic-results:start -->

### Overall Summary (Averaged Across All 4 Bayer Patterns)

| Method | Category | MAE | MSE | PSNR (dB) | PSNR R / G / B (dB) | SSIM | Chroma MAE (R-G / B-G) | MaxAE |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: | ---: | ---: |
| `linear` | `clean_srgb` | 0.101185 | 5.848597e-02 | 18.21 | 17.27 / 21.29 / 17.29 | 0.76646 | 0.148990 / 0.149948 | 1.0000 |
| `linear` | `unclamped_float` | 0.066915 | 9.732375e-02 | 27.43 | 26.14 / 36.72 / 26.14 | 0.95788 | 0.100102 / 0.099864 | 2.2500 |
| `linear` | `noisy_mosaic` | 0.021724 | 1.044669e-03 | 29.81 | 29.74 / 29.95 / 29.74 | 0.61466 | 0.031288 / 0.031315 | 0.4868 |
| `ppg` | `clean_srgb` | 0.069612 | 5.114753e-02 | 21.40 | 20.69 / 26.67 / 20.88 | 0.84747 | 0.117481 / 0.117987 | 2.0000 |
| `ppg` | `unclamped_float` | 0.065161 | 9.799246e-02 | 35.18 | 34.13 / 44.29 / 34.15 | 0.95766 | 0.098076 / 0.097545 | 4.4500 |
| `ppg` | `noisy_mosaic` | 0.027510 | 1.473702e-03 | 28.32 | 27.90 / 29.29 / 27.90 | 0.51058 | 0.024367 / 0.024382 | 0.3552 |

### Per-Bayer Pattern Breakdown (`clean_srgb` Tiles 01-12)

| Method | Pattern | MAE | MSE | PSNR (dB) | PSNR R / G / B (dB) | SSIM |
| :--- | :--- | ---: | ---: | ---: | ---: | ---: |
| `linear` | `RGGB` | 0.101942 | 5.917855e-02 | 18.14 | 17.28 / 21.27 / 17.12 | 0.76431 |
| `linear` | `BGGR` | 0.101884 | 5.919853e-02 | 18.13 | 17.11 / 21.27 / 17.28 | 0.76370 |
| `linear` | `GRBG` | 0.100513 | 5.778790e-02 | 18.28 | 17.38 / 21.30 / 17.35 | 0.76931 |
| `linear` | `GBRG` | 0.100403 | 5.777889e-02 | 18.28 | 17.30 / 21.30 / 17.42 | 0.76854 |
| `ppg` | `RGGB` | 0.070306 | 5.136525e-02 | 21.33 | 20.61 / 26.54 / 20.78 | 0.84644 |
| `ppg` | `BGGR` | 0.070312 | 5.145598e-02 | 21.30 | 20.63 / 26.55 / 20.76 | 0.84651 |
| `ppg` | `GRBG` | 0.068980 | 5.093462e-02 | 21.48 | 20.70 / 26.80 / 21.04 | 0.84844 |
| `ppg` | `GBRG` | 0.068849 | 5.083425e-02 | 21.48 | 20.83 / 26.77 / 20.94 | 0.84848 |

### Per-Tile Breakdown (Averaged Across All 4 Bayer Patterns)

| Tile | Method | MAE | PSNR (dB) | SSIM | Min / Max (GT -> Est) |
| :--- | :--- | ---: | ---: | ---: | :--- |
| 01 · Siemens Star (72-spoke) | `linear` | 0.027664 | 21.05 | 0.91661 | [0.000, 1.000] -> [0.000, 1.000] |
| 01 · Siemens Star (72-spoke) | `ppg` | 0.003812 | 31.62 | 0.99306 | [0.000, 1.000] -> [-0.577, 1.541] |
| 02 · Zone Plate (DC-Nyquist) | `linear` | 0.153334 | 12.44 | 0.45228 | [0.002, 1.000] -> [0.002, 1.000] |
| 02 · Zone Plate (DC-Nyquist) | `ppg` | 0.093134 | 14.12 | 0.71669 | [0.002, 1.000] -> [-0.500, 1.495] |
| 03 · Line Pairs & 5 deg Edges | `linear` | 0.069326 | 14.59 | 0.88889 | [0.000, 1.000] -> [0.000, 1.000] |
| 03 · Line Pairs & 5 deg Edges | `ppg` | 0.026349 | 15.54 | 0.96258 | [0.000, 1.000] -> [-1.000, 2.000] |
| 04 · Curved Edges & Fine Arcs | `linear` | 0.020211 | 22.52 | 0.93308 | [0.005, 1.000] -> [0.005, 1.000] |
| 04 · Curved Edges & Fine Arcs | `ppg` | 0.006081 | 29.54 | 0.96281 | [0.005, 1.000] -> [-0.325, 1.537] |
| 05 · B/W Checkerboards (1-4 px) | `linear` | 0.314970 | 6.63 | 0.48672 | [0.000, 1.000] -> [0.000, 1.000] |
| 05 · B/W Checkerboards (1-4 px) | `ppg` | 0.177901 | 8.65 | 0.72519 | [0.000, 1.000] -> [-0.625, 1.625] |
| 06 · Fine Woven Texture | `linear` | 0.130526 | 13.47 | 0.63894 | [0.007, 1.000] -> [0.007, 1.000] |
| 06 · Fine Woven Texture | `ppg` | 0.094828 | 15.28 | 0.82247 | [0.007, 1.000] -> [-0.499, 1.486] |
| 07 · 1D Frequency Chirps | `linear` | 0.110138 | 14.04 | 0.69269 | [0.001, 1.000] -> [0.001, 1.000] |
| 07 · 1D Frequency Chirps | `ppg` | 0.009709 | 28.17 | 0.98936 | [0.001, 1.000] -> [-0.547, 1.508] |
| 08 · Phase Impulses & Micro-Grid | `linear` | 0.017415 | 20.15 | 0.92523 | [0.000, 1.000] -> [0.000, 1.000] |
| 08 · Phase Impulses & Micro-Grid | `ppg` | 0.014105 | 20.63 | 0.91591 | [0.000, 1.000] -> [-0.490, 1.490] |
| 09 · R/B & G/M Checkerboards | `linear` | 0.353930 | 5.52 | 0.35586 | [0.000, 1.000] -> [0.000, 1.000] |
| 09 · R/B & G/M Checkerboards | `ppg` | 0.395709 | 4.39 | 0.23378 | [0.000, 1.000] -> [-0.500, 1.625] |
| 10 · Saturated Color Boundaries | `linear` | 0.004390 | 28.29 | 0.97770 | [0.000, 1.000] -> [0.000, 1.000] |
| 10 · Saturated Color Boundaries | `ppg` | 0.006162 | 25.10 | 0.93228 | [0.000, 1.000] -> [-0.509, 1.500] |
| 11 · Grayscale, RGB/CMY & Patches | `linear` | 0.005606 | 28.75 | 0.96523 | [0.000, 0.998] -> [0.000, 0.998] |
| 11 · Grayscale, RGB/CMY & Patches | `ppg` | 0.003531 | 30.71 | 0.95008 | [0.000, 0.998] -> [-0.488, 1.122] |
| 12 · Random Geometric (Dead Leaves) | `linear` | 0.006716 | 31.05 | 0.96434 | [0.004, 0.913] -> [0.004, 0.913] |
| 12 · Random Geometric (Dead Leaves) | `ppg` | 0.004022 | 33.02 | 0.96541 | [0.004, 0.913] -> [-0.320, 1.031] |
| 13 · Noisy Flat Fields | `linear` | 0.021724 | 29.81 | 0.61466 | [0.000, 0.850] -> [-0.088, 1.063] |
| 13 · Noisy Flat Fields | `ppg` | 0.027510 | 28.32 | 0.51058 | [0.000, 0.850] -> [-0.175, 1.126] |
| 14 · Signed Zero-Crossing | `linear` | 0.002701 | 33.34 | 0.99089 | [-0.250, 0.250] -> [-0.250, 0.250] |
| 14 · Signed Zero-Crossing | `ppg` | 0.000042 | 52.95 | 0.99982 | [-0.250, 0.250] -> [-0.303, 0.298] |
| 15 · HDR Specular Highlights | `linear` | 0.005991 | 33.97 | 0.99394 | [0.036, 4.000] -> [0.036, 4.000] |
| 15 · HDR Specular Highlights | `ppg` | 0.003048 | 37.68 | 0.99204 | [0.036, 4.000] -> [-1.900, 4.864] |
| 16 · Wide-Range Signed-to-HDR | `linear` | 0.192054 | 14.99 | 0.88881 | [-0.500, 2.500] -> [-0.500, 2.500] |
| 16 · Wide-Range Signed-to-HDR | `ppg` | 0.192394 | 14.90 | 0.88111 | [-0.500, 2.500] -> [-1.800, 3.950] |

<!-- compare-demosaic-results:end -->
