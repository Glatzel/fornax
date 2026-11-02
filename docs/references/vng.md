# VNG reference notes

This note records the authoritative material used to diagnose and implement
Fornax's conventional 2x2 RGB Bayer VNG path. It is explanatory material, not
a copied implementation.

## Sources

- Ting Chen, "Interpolation using a Threshold-based variable number of
  gradients," Stanford EE392J project report for dcraw:
  [original VNG description](http://scien.stanford.edu/pages/labsite/1999/psych221/projects/99/tingchen/algodep/vargra.html).
- darktable 5.6.2, `vng_interpolate`:
  [src/iop/demosaicing/vng.c](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/vng.c#L171).
  This is the authoritative acceptance reference for the Fornax port.
- LibRaw 0.22.3, `vng_interpolate`:
  [src/demosaic/misc_demosaic.cpp](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199).
- darktable notes that its implementation was adapted from dcraw 9.20. The
  original dcraw distribution remains the historical source for the published
  algorithm:
  [dcraw](https://www.dechifro.org/dcraw/).

## Algorithm shape

VNG begins with a linear CFA interpolation, then classifies eight directional
gradients around each interior pixel. The in-tree gradient descriptor records
two same-CFA samples, a weight, and the gradient directions affected by their
absolute difference. The lower-gradient neighbors, selected using
`minimum + maximum / 2`, correct red and blue relative to green. A three-row
ring buffer delays writes until the rows needed by the local neighborhood are
available.

The gradient weight is a shift for Fornax's integer implementation:

- weight 1 means no shift;
- weight 2 means a one-bit left shift.

## Green-plane convention

This is the most important compatibility detail.

darktable derives `filters4` so `G1` and `G2` remain separate while linear
interpolation, phase generation, gradient scoring, and neighbor correction run.
It averages the two green planes only when writing final RGB.

Fornax mirrors that behavior:

- `GRedRow` is channel 1;
- `GBlueRow` is channel 3;
- final green is `(G1 + G2) / 2`;
- red is channel 0 and blue is channel 2.

The split changes the accepted phase-term and special-neighbor plans. For each
of the four Bayer patterns, darktable's `GBRG`-style split plan has 512 term
associations and 128 special green-neighbor flags across the 16 phase
templates. Collapsing both greens produces 544 terms and only 96 flags. The
split-green regression tests in `crates/fornax-postprocessor/src/demosaic/vng.rs`
protect those invariants.

LibRaw is intentionally different for the comparison harness: the adapter sets
`four_color_rgb = 0`, retaining one green plane. Its output is recorded as
informational evidence and must not be used as an acceptance gate for the
darktable-compatible Fornax implementation.

## Expected comparison behavior

On the 5202x3464 `GBRG` color-chart fixture, the darktable-compatible port has
approximately:

- MAE 0.386;
- RMSE 1.245;
- P99 1;
- P99.9 9;
- maximum 733;
- zero samples above 1024 codes.

All four Bayer patterns produce effectively the same summary. The maximum is a
sparse interpolated-green saturation/clipping difference before final packing:
one green plane is 64069 and the other is clamped to 65535 at `(2402, 441)`.
Final-average rounding can account for at most one code, so it is not the root
cause.

## Licensing

darktable is GPL and LibRaw is LGPL/CDDL. Their source is used here as
behavioral and specification evidence, not translated or copied into the
Fornax postprocessor. Any future direct reuse must be reviewed against the
project's licensing requirements.
