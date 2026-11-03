# LMMSE reference notes

This note records the authoritative material used to implement Fornax's
conventional 2x2 RGB Bayer LMMSE path.

## Sources

- L. Zhang and X. Wu, "Color demosaicing via directional linear minimum
  mean-square-error estimation," *IEEE Transactions on Image Processing*,
  vol. 14, no. 12, pp. 2167-2178, 2005:
  [doi:10.1109/TIP.2005.857267](https://doi.org/10.1109/TIP.2005.857267).
- darktable 5.6.2, `lmmse_demosaic`:
  [src/iop/demosaicing/lmmse.c](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120).
  This is the authoritative acceptance reference for the Fornax path.
- darktable 5.6.2, `demosaic_ppg`:
  [src/iop/demosaicing/ppg.c](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20).
  The LMMSE path uses a PPG reconstruction as its initial output and retains
  that seed in regions the tiled kernel does not overwrite.

## Algorithm shape

The LMMSE path works in darktable's gamma-corrected domain. It builds two
65,536-entry lookup tables for encoding and decoding, estimates horizontal and
vertical green-minus-red-or-blue differences, normalizes them with a
`exp(-i^2 / 8)` nine-tap low-pass kernel, and combines the local color
estimate with the low-pass estimate using their directional variances. Red
and blue are then reconstructed from green and color-difference planes, followed
by one nine-sample median correction pass.

The processing geometry is part of the reference behavior:

- tile size: 136 pixels;
- border: four pixels;
- overlap: eight pixels;
- valid tile stride: 112 pixels;
- comparison mode: one median pass and no refinement passes.

darktable initializes its six-plane scratch area once and reuses it across all
tiles. Fornax mirrors that lifetime because edge tiles read padding values that
were not touched by the current tile. Resetting the scratch area per tile
changes the reference boundary decisions.

Variance weighting uses the original low-pass center in the estimator while
using the low-pass-minus-raw residual for the second variance. Substituting
the residual center into both terms suppresses the correction and produces a
visible chroma lattice, so the unit and comparison tests protect that detail.

## Fornax differences and conventions

The comparison bridge pins the darktable arguments to mode `1`,
`scaler = 1.0`, a PPG threshold of `0.0`, and no white-balance multipliers.
Those pipeline-level settings are outside the Fornax demosaic stage.

Fornax accepts normalized floating-point mosaics, converts them to the same
16-bit code range used by the comparison harness, performs the tiled kernel in
`f32`, and quantizes the decoded RGB output once. The PPG seed and measured CFA
sites therefore remain code-exact. Floating-point contraction and the final
nearest-code conversion can leave one-code differences, governed by the
comparison tolerances rather than waived.

## Licensing

darktable is GPLv3. Its implementation is used here as behavioral and
specification evidence. The Fornax implementation is written in Rust around
the published algorithm and documented reference behavior. Any direct reuse
of darktable source text still requires project licensing review.
