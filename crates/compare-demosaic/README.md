# compare-demosaic

Synthetic demosaic quality comparison harness for Fornax's production-ready
Bayer demosaicing methods (`linear`, `ppg`, `rcd`, and `lmmse`).

## Methodology

- **Chart:** `temp/compare-demosaic/demosaic_chart.svg` (`2048 × 2048`, 4×4 grid of `512 × 512` tiles with even-aligned `448 × 448` evaluation ROIs).
- **Rasterization & Filter:** 4× supersampling (`8192 × 8192`) via `resvg`, exact IEC 61966-2-1 sRGB-to-linear EOTF in `float32`, followed by 4×4 box (area-average) downsampling in linear light to `2048 × 2048`.
- **Unclamped Float (`< 0.0` and `> 1.0`):** Row 4 tiles (`signed_zero_crossing` `[-0.25, +0.25]`, `hdr_specular` `[0.0, 4.0]`, and `wide_range_signed_hdr` `[-0.5, +2.5]`) are analytically synthesized directly in IEEE 754 `float32` so clamping or gamma-domain bugs are caught immediately.
- **Mosaic-Only Noise:** Tile 13 (`noisy_flat_fields`) stores noiseless flat patches in `ground_truth_rgb.tif` and injects deterministic fixed-seed Gaussian noise only into the 4 Bayer mosaics (`bayer_*.tif`), evaluated separately from clean `[0, 1]` and unclamped-float metrics.
- **Slanted-Edge MTF (SVG):** Computes ISO 12233-style 4×-oversampled SFR/MTF on the four 5°-slanted edges (achromatic B/W, chromatic R/B, signed zero-crossing, wide-range HDR) and writes `temp/compare-demosaic/mtf_comparison.svg` plus `temp/compare-demosaic/{method}/mtf_{pattern}.svg`.

## Usage

```bash
# Run full pipeline (chart -> Rust demosaic -> Python metrics + MTF SVGs + README update)
pixi run -e compare-demosaic compare-demosaic --method all

# Run a single method
pixi run -e compare-demosaic compare-demosaic --method ppg

# Individual stages
pixi run -e compare-demosaic chart
pixi run -e compare-demosaic demosaic --method linear,ppg
pixi run -e compare-demosaic evaluate --method all
pixi run -e compare-demosaic mtf --method all

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
| `rcd` | `clean_srgb` | 0.073400 | 5.093138e-02 | 22.28 | 22.03 / 26.36 / 22.24 | 0.85344 | 0.119745 / 0.120571 | 2.0000 |
| `rcd` | `unclamped_float` | 0.074670 | 1.158625e-01 | 35.84 | 35.37 / 40.02 / 35.38 | 0.93242 | 0.104486 / 0.104447 | 3.5905 |
| `rcd` | `noisy_mosaic` | 0.025320 | 1.249006e-03 | 29.03 | 28.78 / 29.58 / 28.79 | 0.54291 | 0.021391 / 0.021374 | 0.2825 |
| `lmmse` | `clean_srgb` | 0.065383 | 5.074541e-02 | 23.71 | 22.77 / 27.62 / 22.97 | 0.84509 | 0.108223 / 0.108107 | 2.0000 |
| `lmmse` | `unclamped_float` | 0.107845 | 2.231904e-01 | 35.65 | 34.74 / 38.41 / 34.76 | 0.90464 | 0.130955 / 0.130389 | 5.0500 |
| `lmmse` | `noisy_mosaic` | 0.026775 | 1.360803e-03 | 28.66 | 28.56 / 28.87 / 28.56 | 0.52021 | 0.013062 / 0.013124 | 0.2678 |

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
| `rcd` | `RGGB` | 0.075540 | 5.246782e-02 | 22.11 | 21.97 / 26.15 / 21.91 | 0.84863 |
| `rcd` | `BGGR` | 0.075537 | 5.248599e-02 | 22.10 | 21.68 / 26.15 / 22.17 | 0.84834 |
| `rcd` | `GRBG` | 0.071111 | 4.934971e-02 | 22.47 | 22.31 / 26.56 / 22.43 | 0.85858 |
| `rcd` | `GBRG` | 0.071412 | 4.942200e-02 | 22.44 | 22.14 / 26.56 / 22.45 | 0.85821 |
| `lmmse` | `RGGB` | 0.067184 | 5.154828e-02 | 23.52 | 22.57 / 27.43 / 22.79 | 0.83971 |
| `lmmse` | `BGGR` | 0.067167 | 5.154346e-02 | 23.51 | 22.66 / 27.42 / 22.68 | 0.83912 |
| `lmmse` | `GRBG` | 0.063594 | 4.996193e-02 | 23.91 | 22.98 / 27.81 / 23.15 | 0.85099 |
| `lmmse` | `GBRG` | 0.063587 | 4.992796e-02 | 23.90 | 22.88 / 27.81 / 23.24 | 0.85054 |

### Per-Tile Breakdown (Averaged Across All 4 Bayer Patterns)

| Tile | Method | MAE | PSNR (dB) | SSIM | Min / Max (GT -> Est) |
| :--- | :--- | ---: | ---: | ---: | :--- |
| 01 · Siemens Star (72-spoke) | `linear` | 0.027664 | 21.05 | 0.91661 | [0.000, 1.000] -> [0.000, 1.000] |
| 01 · Siemens Star (72-spoke) | `ppg` | 0.003812 | 31.62 | 0.99306 | [0.000, 1.000] -> [-0.577, 1.541] |
| 01 · Siemens Star (72-spoke) | `rcd` | 0.005940 | 31.58 | 0.99359 | [0.000, 1.000] -> [-0.310, 1.325] |
| 01 · Siemens Star (72-spoke) | `lmmse` | 0.003419 | 39.24 | 0.99677 | [0.000, 1.000] -> [-0.302, 1.277] |
| 02 · Zone Plate (DC-Nyquist) | `linear` | 0.153334 | 12.44 | 0.45228 | [0.002, 1.000] -> [0.002, 1.000] |
| 02 · Zone Plate (DC-Nyquist) | `ppg` | 0.093134 | 14.12 | 0.71669 | [0.002, 1.000] -> [-0.500, 1.495] |
| 02 · Zone Plate (DC-Nyquist) | `rcd` | 0.088883 | 14.56 | 0.71614 | [0.002, 1.000] -> [-0.407, 1.407] |
| 02 · Zone Plate (DC-Nyquist) | `lmmse` | 0.009874 | 26.11 | 0.96737 | [0.002, 1.000] -> [0.002, 1.000] |
| 03 · Line Pairs & 5 deg Edges | `linear` | 0.069326 | 14.59 | 0.88889 | [0.000, 1.000] -> [0.000, 1.000] |
| 03 · Line Pairs & 5 deg Edges | `ppg` | 0.026349 | 15.54 | 0.96258 | [0.000, 1.000] -> [-1.000, 2.000] |
| 03 · Line Pairs & 5 deg Edges | `rcd` | 0.033808 | 16.23 | 0.95259 | [0.000, 1.000] -> [-1.000, 2.000] |
| 03 · Line Pairs & 5 deg Edges | `lmmse` | 0.035869 | 15.82 | 0.95086 | [0.000, 1.000] -> [-1.000, 2.000] |
| 04 · Curved Edges & Fine Arcs | `linear` | 0.020211 | 22.52 | 0.93308 | [0.005, 1.000] -> [0.005, 1.000] |
| 04 · Curved Edges & Fine Arcs | `ppg` | 0.006081 | 29.54 | 0.96281 | [0.005, 1.000] -> [-0.325, 1.537] |
| 04 · Curved Edges & Fine Arcs | `rcd` | 0.006344 | 30.09 | 0.97611 | [0.005, 1.000] -> [-0.290, 1.462] |
| 04 · Curved Edges & Fine Arcs | `lmmse` | 0.009527 | 27.58 | 0.92413 | [0.005, 1.000] -> [-0.462, 1.456] |
| 05 · B/W Checkerboards (1-4 px) | `linear` | 0.314970 | 6.63 | 0.48672 | [0.000, 1.000] -> [0.000, 1.000] |
| 05 · B/W Checkerboards (1-4 px) | `ppg` | 0.177901 | 8.65 | 0.72519 | [0.000, 1.000] -> [-0.625, 1.625] |
| 05 · B/W Checkerboards (1-4 px) | `rcd` | 0.238439 | 7.58 | 0.64652 | [0.000, 1.000] -> [-0.722, 2.123] |
| 05 · B/W Checkerboards (1-4 px) | `lmmse` | 0.167388 | 8.67 | 0.72861 | [0.000, 1.000] -> [-0.683, 1.697] |
| 06 · Fine Woven Texture | `linear` | 0.130526 | 13.47 | 0.63894 | [0.007, 1.000] -> [0.007, 1.000] |
| 06 · Fine Woven Texture | `ppg` | 0.094828 | 15.28 | 0.82247 | [0.007, 1.000] -> [-0.499, 1.486] |
| 06 · Fine Woven Texture | `rcd` | 0.079999 | 17.53 | 0.87378 | [0.007, 1.000] -> [-0.477, 1.212] |
| 06 · Fine Woven Texture | `lmmse` | 0.078850 | 17.38 | 0.87665 | [0.007, 1.000] -> [-0.684, 1.147] |
| 07 · 1D Frequency Chirps | `linear` | 0.110138 | 14.04 | 0.69269 | [0.001, 1.000] -> [0.001, 1.000] |
| 07 · 1D Frequency Chirps | `ppg` | 0.009709 | 28.17 | 0.98936 | [0.001, 1.000] -> [-0.547, 1.508] |
| 07 · 1D Frequency Chirps | `rcd` | 0.009643 | 27.39 | 0.98561 | [0.001, 1.000] -> [-0.499, 1.484] |
| 07 · 1D Frequency Chirps | `lmmse` | 0.000536 | 39.84 | 0.99949 | [0.001, 1.000] -> [-0.364, 1.322] |
| 08 · Phase Impulses & Micro-Grid | `linear` | 0.017415 | 20.15 | 0.92523 | [0.000, 1.000] -> [0.000, 1.000] |
| 08 · Phase Impulses & Micro-Grid | `ppg` | 0.014105 | 20.63 | 0.91591 | [0.000, 1.000] -> [-0.490, 1.490] |
| 08 · Phase Impulses & Micro-Grid | `rcd` | 0.013206 | 21.13 | 0.91796 | [0.000, 1.000] -> [-0.601, 1.320] |
| 08 · Phase Impulses & Micro-Grid | `lmmse` | 0.023145 | 19.82 | 0.81259 | [0.000, 1.000] -> [-0.598, 1.170] |
| 09 · R/B & G/M Checkerboards | `linear` | 0.353930 | 5.52 | 0.35586 | [0.000, 1.000] -> [0.000, 1.000] |
| 09 · R/B & G/M Checkerboards | `ppg` | 0.395709 | 4.39 | 0.23378 | [0.000, 1.000] -> [-0.500, 1.625] |
| 09 · R/B & G/M Checkerboards | `rcd` | 0.395625 | 4.62 | 0.23908 | [0.000, 1.000] -> [-0.327, 1.252] |
| 09 · R/B & G/M Checkerboards | `lmmse` | 0.437596 | 3.91 | 0.08346 | [0.000, 1.000] -> [-0.545, 1.500] |
| 10 · Saturated Color Boundaries | `linear` | 0.004390 | 28.29 | 0.97770 | [0.000, 1.000] -> [0.000, 1.000] |
| 10 · Saturated Color Boundaries | `ppg` | 0.006162 | 25.10 | 0.93228 | [0.000, 1.000] -> [-0.509, 1.500] |
| 10 · Saturated Color Boundaries | `rcd` | 0.003636 | 28.44 | 0.97294 | [0.000, 1.000] -> [-0.359, 1.388] |
| 10 · Saturated Color Boundaries | `lmmse` | 0.008131 | 24.60 | 0.90938 | [0.000, 1.000] -> [-0.714, 1.552] |
| 11 · Grayscale, RGB/CMY & Patches | `linear` | 0.005606 | 28.75 | 0.96523 | [0.000, 0.998] -> [0.000, 0.998] |
| 11 · Grayscale, RGB/CMY & Patches | `ppg` | 0.003531 | 30.71 | 0.95008 | [0.000, 0.998] -> [-0.488, 1.122] |
| 11 · Grayscale, RGB/CMY & Patches | `rcd` | 0.002356 | 32.39 | 0.97934 | [0.000, 0.998] -> [-0.480, 1.058] |
| 11 · Grayscale, RGB/CMY & Patches | `lmmse` | 0.003800 | 30.63 | 0.94918 | [0.000, 0.998] -> [-0.488, 1.304] |
| 12 · Random Geometric (Dead Leaves) | `linear` | 0.006716 | 31.05 | 0.96434 | [0.004, 0.913] -> [0.004, 0.913] |
| 12 · Random Geometric (Dead Leaves) | `ppg` | 0.004022 | 33.02 | 0.96541 | [0.004, 0.913] -> [-0.320, 1.031] |
| 12 · Random Geometric (Dead Leaves) | `rcd` | 0.002922 | 35.82 | 0.98760 | [0.004, 0.913] -> [-0.320, 1.031] |
| 12 · Random Geometric (Dead Leaves) | `lmmse` | 0.006466 | 30.92 | 0.94260 | [0.004, 0.913] -> [-0.331, 1.032] |
| 13 · Noisy Flat Fields | `linear` | 0.021724 | 29.81 | 0.61466 | [0.000, 0.850] -> [-0.088, 1.063] |
| 13 · Noisy Flat Fields | `ppg` | 0.027510 | 28.32 | 0.51058 | [0.000, 0.850] -> [-0.175, 1.126] |
| 13 · Noisy Flat Fields | `rcd` | 0.025320 | 29.03 | 0.54291 | [0.000, 0.850] -> [-0.102, 1.085] |
| 13 · Noisy Flat Fields | `lmmse` | 0.026775 | 28.66 | 0.52021 | [0.000, 0.850] -> [-0.088, 1.069] |
| 14 · Signed Zero-Crossing | `linear` | 0.002701 | 33.34 | 0.99089 | [-0.250, 0.250] -> [-0.250, 0.250] |
| 14 · Signed Zero-Crossing | `ppg` | 0.000042 | 52.95 | 0.99982 | [-0.250, 0.250] -> [-0.303, 0.298] |
| 14 · Signed Zero-Crossing | `rcd` | 0.000058 | 53.26 | 0.99988 | [-0.250, 0.250] -> [-0.306, 0.275] |
| 14 · Signed Zero-Crossing | `lmmse` | 0.000018 | 60.88 | 0.99997 | [-0.250, 0.250] -> [-0.290, 0.324] |
| 15 · HDR Specular Highlights | `linear` | 0.005991 | 33.97 | 0.99394 | [0.036, 4.000] -> [0.036, 4.000] |
| 15 · HDR Specular Highlights | `ppg` | 0.003048 | 37.68 | 0.99204 | [0.036, 4.000] -> [-1.900, 4.864] |
| 15 · HDR Specular Highlights | `rcd` | 0.002174 | 40.12 | 0.99682 | [0.036, 4.000] -> [-0.821, 5.210] |
| 15 · HDR Specular Highlights | `lmmse` | 0.006300 | 34.75 | 0.98011 | [0.036, 4.000] -> [-1.454, 4.478] |
| 16 · Wide-Range Signed-to-HDR | `linear` | 0.192054 | 14.99 | 0.88881 | [-0.500, 2.500] -> [-0.500, 2.500] |
| 16 · Wide-Range Signed-to-HDR | `ppg` | 0.192394 | 14.90 | 0.88111 | [-0.500, 2.500] -> [-1.800, 3.950] |
| 16 · Wide-Range Signed-to-HDR | `rcd` | 0.221776 | 14.15 | 0.80055 | [-0.500, 2.500] -> [-1.563, 3.217] |
| 16 · Wide-Range Signed-to-HDR | `lmmse` | 0.317219 | 11.32 | 0.73385 | [-0.500, 2.500] -> [-2.550, 4.118] |

### Slanted-Edge MTF Summary (Averaged Across All 4 Bayer Patterns)

| Method | Edge | Channel | MTF50 (cyc/px) | MTF20 (cyc/px) | MTF@0.25 | MTF@0.50 | Peak MTF |
| :--- | :--- | :--- | ---: | ---: | ---: | ---: | ---: |
| `linear` | `achromatic_bw` | `Y` | 0.297 | 0.563 | 0.608 | 0.219 | 1.000 |
| `linear` | `achromatic_bw` | `R` | 0.235 | 0.338 | 0.451 | 0.005 | 1.000 |
| `linear` | `achromatic_bw` | `G` | 0.339 | 0.589 | 0.671 | 0.304 | 1.000 |
| `linear` | `achromatic_bw` | `B` | 0.235 | 0.338 | 0.451 | 0.005 | 1.000 |
| `linear` | `chromatic_rb` | `R` | 0.234 | 0.336 | 0.449 | 0.003 | 1.000 |
| `linear` | `chromatic_rb` | `B` | 0.234 | 0.336 | 0.449 | 0.002 | 1.000 |
| `linear` | `signed_zero_crossing` | `Y` | 0.298 | 0.589 | 0.610 | 0.223 | 1.000 |
| `linear` | `wide_range_hdr` | `Y` | 0.298 | 0.589 | 0.610 | 0.223 | 1.000 |
| `ppg` | `achromatic_bw` | `Y` | 0.589 | 0.589 | 0.895 | 0.622 | 1.000 |
| `ppg` | `achromatic_bw` | `R` | 0.589 | 0.589 | 0.894 | 0.622 | 1.000 |
| `ppg` | `achromatic_bw` | `G` | 0.589 | 0.589 | 0.895 | 0.622 | 1.000 |
| `ppg` | `achromatic_bw` | `B` | 0.589 | 0.589 | 0.894 | 0.622 | 1.000 |
| `ppg` | `chromatic_rb` | `R` | 0.235 | 0.333 | 0.450 | 0.042 | 1.000 |
| `ppg` | `chromatic_rb` | `B` | 0.255 | 0.373 | 0.514 | 0.049 | 1.000 |
| `ppg` | `signed_zero_crossing` | `Y` | 0.589 | 0.589 | 0.898 | 0.630 | 1.000 |
| `ppg` | `wide_range_hdr` | `Y` | 0.589 | 0.589 | 0.898 | 0.630 | 1.000 |
| `rcd` | `achromatic_bw` | `Y` | 0.589 | 0.589 | 0.893 | 0.616 | 1.000 |
| `rcd` | `achromatic_bw` | `R` | 0.588 | 0.589 | 0.894 | 0.619 | 1.000 |
| `rcd` | `achromatic_bw` | `G` | 0.589 | 0.589 | 0.892 | 0.614 | 1.000 |
| `rcd` | `achromatic_bw` | `B` | 0.588 | 0.589 | 0.894 | 0.619 | 1.000 |
| `rcd` | `chromatic_rb` | `R` | 0.259 | 0.368 | 0.527 | 0.017 | 1.000 |
| `rcd` | `chromatic_rb` | `B` | 0.281 | 0.410 | 0.582 | 0.076 | 1.000 |
| `rcd` | `signed_zero_crossing` | `Y` | 0.574 | 0.589 | 0.888 | 0.598 | 1.000 |
| `rcd` | `wide_range_hdr` | `Y` | 0.589 | 0.589 | 0.904 | 0.642 | 1.000 |
| `lmmse` | `achromatic_bw` | `Y` | 0.584 | 0.589 | 0.893 | 0.614 | 1.000 |
| `lmmse` | `achromatic_bw` | `R` | 0.584 | 0.589 | 0.893 | 0.614 | 1.000 |
| `lmmse` | `achromatic_bw` | `G` | 0.584 | 0.589 | 0.893 | 0.614 | 1.000 |
| `lmmse` | `achromatic_bw` | `B` | 0.584 | 0.589 | 0.893 | 0.614 | 1.000 |
| `lmmse` | `chromatic_rb` | `R` | 0.222 | 0.316 | 0.404 | 0.040 | 1.000 |
| `lmmse` | `chromatic_rb` | `B` | 0.240 | 0.352 | 0.470 | 0.041 | 1.000 |
| `lmmse` | `signed_zero_crossing` | `Y` | 0.585 | 0.589 | 0.893 | 0.613 | 1.000 |
| `lmmse` | `wide_range_hdr` | `Y` | 0.585 | 0.589 | 0.893 | 0.613 | 1.000 |

<!-- compare-demosaic-results:end -->
