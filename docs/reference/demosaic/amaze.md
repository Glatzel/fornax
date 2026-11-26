# Aliasing minimization and zipper elimination (AMaZE)

Fornax's `Demosaic::Amaze` implements Emil Martinec's AMaZE directional green,
Nyquist texture, and color-difference reconstruction in unclamped, signed,
scene-linear floating-point arithmetic.

## Classification

**Done: production ready.**

## Sources

- Emil Martinec, *AMaZE (Aliasing Minimization and Zipper Elimination)*.
- RawTherapee [`rtengine/amaze_demosaic_RT.cc`](https://github.com/Beep6581/RawTherapee) (GPL-3.0, reference only).
- darktable [`src/iop/demosaicing/amaze.cc`](https://github.com/darktable-org/darktable) (GPL-3.0, reference only).

Fornax's implementation is an independent Rust formulation adapted to Fornax's
`RawFloat` (`f32` and `f64`) signed scene-linear contract; upstream GPL-3.0 code
is never copied into Fornax.

## Algorithm

Let $F(x, y)$ be the scale-normalized CFA mosaic ($F_{\mathrm{norm}} = F / s$,
where $s > 0$ is a robust high percentile of $|F|$). All tile operations run in
normalized linear coordinates and rescale by $s$ on output, preserving positive
scale equivariance and preventing `f32` overflow or underflow in squared
gradient and variance terms.

### 1. Boundary reflection and tiling

Images are partitioned into square tiles of configurable `tile_size` (default
`160`, even, at least `36`) with a `16`-pixel reflected halo (`BORDER = 16`)
per side. Outside image bounds, indices are reflected by an even number of
steps via `reflect_index`, preserving Bayer CFA phase parity for every
dimension `>= 2`.

### 2. Directional gradients and sign-safe ratios

For each interior tile sample, local cardinal gradients are floored by
$\varepsilon$:

$$
w_h(p) = \varepsilon + |F(y, x+1) - F(y, x-1)|,
\qquad
w_v(p) = \varepsilon + |F(y+1, x) - F(y-1, x)|.
$$

Hamilton-Adams additive green estimates along cardinal offsets $\Delta$ are
paired with directional color ratios when endpoints share a strict sign
($c \cdot t > 0$ and $n \cdot (w_t c + w_c t) > 0$):

$$
r(c, n, t) = \frac{n \cdot (w_t + w_c)}{w_t c + w_c t},
$$

and $r(c, n, t) = 0$ otherwise. When $r \in (0.5, 2.0)$, the adaptive estimate
uses $c \cdot r$; otherwise (including zero-crossings and opposite-sign
chroma) it falls back to the Hamilton-Adams additive estimate without
introducing zero-crossing poles or arbitrary highlight thresholds.

### 3. Color-difference variance selection and H/V blending

Smoothness selection compares 3-tap variance of ratio-adaptive vs. additive
color differences using out-of-place Jacobi buffer swaps so scan order never
breaks 180-degree flip equivariance. Directional horizontal/vertical weights
blend 4-tap sample variances computed around true 4-sample means ($\frac{1}{4}$
scaling) and 3-tap gradient-delta fluctuations.

### 4. Nyquist texture detection and refinement

High-frequency zipper/checkerboard regions are detected by comparing a
Gaussian-weighted squared color-difference response against a Gaussian-weighted
squared gradient response. Where Nyquist texture is detected, weights and
green estimates are refined using a quincunx curvature filter.

### 5. Diagonal R/B, green refinement, and chroma interpolation

At red/blue sites, diagonal color differences along NE/SW ($+$) and NW/SE ($-$)
are blended with sign-safe diagonal ratios and clamped toward neighbors only
when an overshoot shares the reference sign (`clamp_toward_neighbors`, with no
`[0, 1]` or highlight ceiling). After updating `plus_minus_weight` via an
out-of-place Jacobi pass, green at R/B sites is refined, green differences are
split into `G - R` and `G - B` over the full neighborhood read by chroma
interpolation, and remaining chroma channels are reconstructed using symmetric
directional weights and a guarded denominator at green sites.

Measured CFA samples are restored bit-exactly on output.

## Configuration

`Demosaic::Amaze(AmazeOptions)` exposes `tile_size` (`usize`, default `160`,
must be even and at least `36`). `Demosaic::amaze()` constructs the default
configuration.

## Verification

- `cargo test -p fornax-raw --lib demosaic::amaze` (12 tests covering all Bayer
  phases, signed/HDR unclamped inputs, positive scale equivariance, sign
  symmetry, 180-degree H+V flip equivariance, tile-size consistency, `f64`,
  option/non-finite validation, and a `24x24` `insta` snapshot).
- `pixi run -e compare-demosaic compare-demosaic --method all` and
  `pixi run -e compare-demosaic compare-demosaic-test`.
