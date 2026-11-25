# Directional LMMSE demosaicing

Fornax's `Demosaic::Lmmse` implements Zhang-Wu directional linear minimum
mean-square-error Bayer demosaicing directly on signed, unclamped, scene-linear
floating-point samples.

## Classification

**Done: production ready.**

## Sources

- L. Zhang and X. Wu, "Color demosaicking via directional linear minimum
  mean-square-error estimation," *IEEE Transactions on Image Processing*,
  vol. 14, no. 12, pp. 2167-2178, 2005:
  [doi:10.1109/TIP.2005.857260](https://doi.org/10.1109/TIP.2005.857260).
- P. Getreuer, "Zhang-Wu Directional LMMSE Image Demosaicking," *Image
  Processing On Line*, 2011:
  [doi:10.5201/ipol.2011.g_zwld](https://doi.org/10.5201/ipol.2011.g_zwld).

## Algorithm

Let $F(x, y)$ be the scale-normalized linear CFA mosaic ($F_{\mathrm{norm}} = F / s$,
where $s > 0$ is a robust high percentile of $|F|$). All directional estimates,
variances, and optional post-passes run in normalized linear coordinates and
rescale by $s$ on output, preserving positive scale equivariance and shift
invariance without a gamma-domain workspace.

### 1. Directional color-difference observations

Along each row (horizontal) and column (vertical), second-order Laplacian
interpolation produces green-minus-primary observations $\Phi = G - C$
($C = R, B$). Indexing by 1D offset $n$ along the active axis:

- **At a measured $R$ or $B$ sample ($F_0 = C_0$):**
  $$f_0 = \frac{1}{2}(F_{-1} + F_1) - \frac{1}{4}(F_{-2} + F_2) - \frac{1}{2} F_0.$$
- **At a measured $G$ sample ($F_0 = G_0$):**
  $$f_0 = \frac{1}{2} F_0 - \frac{1}{2}(F_{-1} + F_1) + \frac{1}{4}(F_{-2} + F_2).$$

These affine equations are exact on linear ramps and symmetric for positive and
negative values.

### 2. Gaussian low-pass filter

Both directional color-difference fields are smoothed by a normalized 9-tap
Gaussian kernel ($\sigma = 2$):

$$
h_k = \frac{e^{-k^2 / 8}}{\sum_{j=-4}^{4} e^{-j^2 / 8}},
\qquad k = -4, \dots, 4,
$$

yielding the low-pass signal $s = f * h$.

### 3. LMMSE denoising and directional fusion

Over the 9-tap window centered at $n$, let $\mu_x = \frac{1}{9}\sum_{k=-4}^{4} s_{n+k}$.
Using a dimensionless scale-relative variance floor $\tau^2 = 10^{-12}$ in
normalized coordinates, the local signal and residual variances are:

$$
\sigma_x^2 = \tau^2 + \sum_{k=-4}^{4} (s_{n+k} - \mu_x)^2,
\qquad
\sigma_n^2 = \tau^2 + \sum_{k=-4}^{4} (s_{n+k} - f_{n+k})^2.
$$

The denoised directional estimate $\widehat{x}_n$ and its error variance
$\sigma_{\mathrm{err}}^2$ are:

$$
\widehat{x}_n = \frac{\sigma_x^2\, f_n + \sigma_n^2\, s_n}{\sigma_x^2 + \sigma_n^2},
\qquad
\sigma_{\mathrm{err}}^2 = \frac{\sigma_x^2\, \sigma_n^2}{\sigma_x^2 + \sigma_n^2},
$$

falling back to $s_n$ and $0$ if $\sigma_x^2 + \sigma_n^2 \le 0$. Horizontal
($\widehat{\Phi}_h, \sigma_{e,h}^2$) and vertical ($\widehat{\Phi}_v, \sigma_{e,v}^2$)
estimates fuse via:

$$
\widehat{\Phi} = \frac{\sigma_{e,v}^2\, \widehat{\Phi}_h + \sigma_{e,h}^2\, \widehat{\Phi}_v}{\sigma_{e,h}^2 + \sigma_{e,v}^2},
$$

or $\frac{1}{2}(\widehat{\Phi}_h + \widehat{\Phi}_v)$ if both error variances
are non-positive. Green at non-green sites is $G = F_0 + \widehat{\Phi}$.

### 4. Red and blue reconstruction and optional `LmmseMode` passes

Missing $R$ and $B$ channels are reconstructed by bilinear interpolation of
$R - G$ and $B - G$ color differences (2-neighbor axial at $G$ sites, 4-neighbor
axial at opposite-primary sites). Measured CFA samples are preserved exactly.

`LmmseMode` controls optional linear-domain post-passes:

| Mode | Median passes | Refinement passes | Required tile halo |
| --- | ---: | ---: | ---: |
| `Basic` | `0` | `0` | `12` |
| `Median` (default) | `1` | `0` | `14` |
| `ThreeMedians` | `3` | `0` | `16` |
| `Refine(r)` | `3` | `r` | `16 + 6 * r` |

Median passes apply a $3 \times 3$ median filter to $R - G$ and $B - G$
color differences. Refinement passes apply EECI-style edge-weighted
color-difference reinforcement with a scale-normalized edge denominator.

## Configuration and tile geometry

`Demosaic::Lmmse(LmmseOptions)` exposes `mode` and `tile_size`:

| Option | Default | Constraint |
| --- | --- | --- |
| `LmmseOptions::mode` | `LmmseMode::Median` | Any `LmmseMode` variant |
| `LmmseOptions::tile_size` | `136` | Even and `>= 2 * halo + 2` |

Image boundaries use parity-preserving reflection across the full tile halo so
uniform and colored constant fields are exact up to `(0, 0)` and across every
tile seam. Minimum supported image size is `2x2`. Tiles are processed in
parallel with Rayon.

## Numeric contract and verification

- Generic over `RawFloat` (`f32` and `f64`), operating directly on signed,
  unclamped scene-linear samples with no gamma transfer or `[0, 1]` clamp.
- Unit tests in [`lmmse.rs`](../../../crates/fornax-raw/src/demosaic/lmmse.rs)
  verify: fixed 16x16 RGGB `insta` snapshot, positive/zero/negative/colored
  constants across the whole image (including borders and seams), signed
  zero-crossing ramps, positive-scale equivariance and shift invariance (`f32`
  and `f64`), horizontal and vertical flip symmetry, tile-size invariance,
  all `LmmseMode` variants, and measured-sample preservation.
- Synthetic chart, signed/HDR ROI, noisy mosaic, and slanted-edge MTF results
  are tracked in [`compare-demosaic`](../../../crates/compare-demosaic/README.md).
