# Directional LMMSE demosaicing eligibility

This report explains Zhang-Wu directional linear minimum mean-square-error
demosaicing and Fornax's darktable-compatible gamma-domain variant.

## Classification

- **Canonical Zhang-Wu LMMSE: Convertible.** The published estimator can be
  evaluated directly on sensor-linear color-difference signals. Fixed variance
  tolerances must be replaced by a scale-relative floor, but no gamma transfer
  or fixed sample range is inherent.
- **Current darktable-compatible LMMSE: Conditional.** `Demosaic::Lmmse`
  interpolates in an analytic nonlinear transfer workspace and decodes back to
  linear values. It is reversible and scale-normalized, but it is a distinct
  nonlinear-domain variant rather than the canonical sensor-linear estimator.

The compatibility variant may survive if explicitly named. The default
sensor-linear candidate should use the published LMMSE equations without the
gamma workspace.

## Sources

- L. Zhang and X. Wu, "Color demosaicking via directional linear minimum
  mean-square-error estimation," *IEEE Transactions on Image Processing*,
  vol. 14, no. 12, pp. 2167-2178, 2005:
  [doi:10.1109/TIP.2005.857260](https://doi.org/10.1109/TIP.2005.857260).
- P. Getreuer, "Zhang-Wu Directional LMMSE Image Demosaicking," *Image
  Processing On Line*, 2011:
  [doi:10.5201/ipol.2011.g_zwld](https://doi.org/10.5201/ipol.2011.g_zwld).
  The article reproduces the directional equations, low-pass kernel, local
  variance estimator, fusion rule, and color-difference reconstruction.
- darktable 5.6.2, commit
  [`20891f6`](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/lmmse.c#L120),
  `lmmse_demosaic`. This is the maintained compatibility reference for the
  current Fornax selector.
- darktable 5.6.2, commit
  [`20891f6`](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20),
  `demosaic_ppg`. The darktable LMMSE path uses PPG as its initial green
  estimate.

## Notation

The equations use a one-dimensional direction index for each local estimate:

| Symbol | Meaning |
| --- | --- |
| $(i,j)$ | Pixel coordinate |
| $n$ | One-dimensional pixel index along a row or column |
| $R_n,G_n,B_n$ | Measured or reconstructed red, green, and blue channel values |
| $f_n$ | Directional color-difference observation at position $n$ |
| $s_n$ | Low-pass-filtered observation at position $n$ |
| $h_k$ | Nine-tap low-pass coefficient at offset $k$ |
| $\mu_x$ | Local mean of the filtered observation |
| $\sigma_x^2$ | Local variance of the filtered observation |
| $\sigma_n^2$ | Local variance of the low-pass residual |
| $\widehat x_n$ | Denoised directional color-difference estimate |
| $\sigma_{\mathrm{err}}^2$ | Estimated error variance of a directional estimate |
| $\sigma_{e,h}^2,\sigma_{e,v}^2$ | Estimated error variance of the horizontal and vertical estimates |
| $\widehat\Phi_h,\widehat\Phi_v$ | Horizontal and vertical color-difference estimates before fusion |
| $\widehat\Phi$ | Fused color-difference estimate |
| $\lambda$ | Fusion fraction assigned to the vertical candidate |
| $\tau^2$ | Proposed scale-relative variance floor for canonical linear LMMSE |
| $\mathcal N_4(i,j)$ | Four axial neighbors used at green sites |
| $\mathcal N_D(i,j)$ | Four diagonal neighbors used at opposite-primary sites |

The symbols with a hat are estimates. Unhatted channel values are measured
samples in the original mosaic; reconstructed values are explicitly described
as reconstructed.

## Original formulation

### Directional estimates

For a red row, Zhang and Wu estimate the missing green and red values with
second-order Laplacian interpolation:

$$
\begin{aligned}
\widehat G_n&=\frac{1}{2}(G_{n-1}+G_{n+1})
-\frac{1}{4}(R_{n-2}-2R_n+R_{n+2}),\\
\widehat R_n&=\frac{1}{2}(R_{n-1}+R_{n+1})
-\frac{1}{4}(G_{n-2}-2G_n+G_{n+2}).
\end{aligned}
$$

The first equation estimates green from the two adjacent green samples and
the red Laplacian. The second estimates red from the two adjacent red samples
and the green Laplacian. Vertical estimates follow the same construction along
a column. Blue rows are symmetric, with blue and green exchanging roles.

The primary color-difference signals are

$$
\Phi_{G,R}=G-R,
\qquad
\Phi_{G,B}=G-B.
$$

At a measured red sample, the horizontal green-red observation is
$\widehat G_n-R_n$. At a measured green sample, it is
$G_n-\widehat R_n$. These observations form the horizontal or vertical
color-difference field used by the denoiser.

### Low-pass kernel

The IPOL exposition applies the symmetric nine-tap kernel

$$
h_k=
\begin{cases}
\frac{26}{128},&k=0,\\
\frac{23}{128},&|k|=1,\\
\frac{15}{128},&|k|=2,\\
\frac{9}{128},&|k|=3,\\
\frac{4}{128},&|k|=4,\\
0,&|k|>4.
\end{cases}
$$

Equivalently,

$$
h(z)=\frac{1}{128}
\left(26+23(z+z^{-1})+15(z^2+z^{-2})
+9(z^3+z^{-3})+4(z^4+z^{-4})\right),
$$

where $z$ is a one-sample delay and $z^{-1}$ is a one-sample advance. The
coefficients sum to one and therefore preserve a constant field.

### LMMSE denoising

For a local directional window, let $f$ be the raw color-difference
observations and let $s=f*h$ be their low-pass-filtered values. Means and
variances are taken over the nine taps of the direction window. Define

$$
\mu_x=\{mean}(s),
\qquad
\sigma_x^2=\{mean}\left((s-\mu_x)^2\right),
\qquad
\sigma_n^2=\{mean}\left((s-f)^2\right).
$$

$\mu_x$ is the local DC level, $\sigma_x^2$ measures variation in the filtered
signal, and $\sigma_n^2$ estimates the residual variation removed by the
low-pass filter. The original notation uses a second subscript for this
residual variance; this note uses $\sigma_n^2$ to avoid confusing it with the
vertical directional error.

The denoised directional estimate is

$$
\widehat x
=\mu_x+\frac{\sigma_x^2}{\sigma_x^2+\sigma_n^2}(f-\mu_x).
$$

The fraction is the LMMSE gain. It approaches one when the filtered signal
contains much more variation than the residual, so the observation is trusted.
It approaches zero when the residual dominates, so the local mean is used.

The estimated error variance of this directional estimate is

$$
\sigma_{\mathrm{err}}^2
=\frac{\sigma_x^2\sigma_n^2}
{\sigma_x^2+\sigma_n^2}.
$$

### Directional fusion

Let $\widehat\Phi_h$ and $\widehat\Phi_v$ be the horizontal and vertical
directional estimates. Let $\sigma_{e,h}^2$ and $\sigma_{e,v}^2$ be their
estimated error variances. The fusion weight is

$$
\lambda=
\frac{\sigma_{e,h}^2}
{\sigma_{e,h}^2+\sigma_{e,v}^2},
\qquad
\widehat\Phi=(1-\lambda)\widehat\Phi_h+\lambda\widehat\Phi_v.
$$

If the horizontal error is smaller, $\lambda$ is smaller and the horizontal
estimate contributes more. If both error variances are zero, a nonzero
denominator convention or explicit fallback is required.

### Red and blue reconstruction

Once the fused green and color-difference fields are known, measured
green-minus-red and green-minus-blue differences are interpolated:

$$
\widehat\Phi(i,j)=\frac{1}{4}\sum_{q\in\mathcal N_D(i,j)}\widehat\Phi(q)
$$

at an opposite-primary site, and

$$
\widehat\Phi(i,j)=\frac{1}{4}\sum_{q\in\mathcal N_4(i,j)}\widehat\Phi(q)
$$

at a green site. The final channels are

$$
R=G-\Phi_{G,R},
\qquad
B=G-\Phi_{G,B}.
$$

## Current Fornax behavior

[`lmmse.rs`](../../../crates/fornax-raw/src/demosaic/lmmse.rs)
implements a darktable-compatible variant. The processing order is:

1. Normalize the mosaic by a robust positive scale.
2. Generate an initial PPG result.
3. Encode normalized samples into the compatibility workspace.
4. Build horizontal and vertical color-difference fields.
5. Apply the nine-tap low-pass filter.
6. Fuse directional estimates with local variances.
7. Restore measured CFA samples.
8. Reconstruct red and blue from color differences.
9. Apply the selected median and refinement passes.
10. Decode the workspace and multiply by the original scale.

### Tile geometry

The implementation uses overlapping square tiles:

| Parameter | Value | Meaning |
| --- | ---: | --- |
| `TILE_SIZE` | 136 | Tile width and height including halo |
| `BORDER` | 4 | Halo used by the local estimates |
| `OVERLAP` | 8 | Overlap consumed when writing neighboring tiles |
| `TILE_INTERIOR` | $136-2\cdot4=128$ | Interior tile extent available after borders |
| `TILE_VALID` | $128-2\cdot8=112$ | Valid tile advance between tile origins |

Reflected indexing supplies samples near image boundaries. The first and last
tile edges use border-based trimming rather than copying the halo into output.

### Compatibility transfer

The current darktable-compatible path encodes normalized values with an
analytical sRGB-style transfer and decodes after interpolation. Define

$$
\{sgn}(v)=
\begin{cases}
-1,&v<0,\\
1,&v\ge 0.
\end{cases}
$$

For a non-negative magnitude $r$, define the magnitude transforms

$$
e(r)=
\begin{cases}
g r,&r\le t_i,\\
a r^{p_e}-b,&t_i<r\le 1,\\
r^{p_e},&r>1,
\end{cases}
$$

and

$$
d(r)=
\begin{cases}
r/g,&r\le t_o,\\
\left(\frac{r+b}{a}\right)^{p_d},&t_o<r\le 1,\\
r^{p_d},&r>1.
\end{cases}
$$

The signed transforms apply odd extension:

$$
E(v)=\{sgn}(v)e(|v|),
\qquad
D(v)=\{sgn}(v)d(|v|).
$$

The current constants are:

| Symbol | Value | Meaning |
| --- | ---: | --- |
| $t_i$ | 0.001867 | Input slope/toe transition |
| $t_o$ | 0.031746 | Decode toe/slope transition |
| $g$ | 17 | Linear toe slope |
| $a$ | 1.044445 | Power segment scale |
| $b$ | 0.044445 | Power segment offset |
| $p_e$ | $1/2.4$ | Encode exponent |
| $p_d$ | $2.4$ | Decode exponent |

This transfer is the source of the compatibility variant's nonlinear
intermediate workspace. Canonical linear LMMSE does not use it.

### Local variance floor

After normalization, `mmse_value` adds $1.0\cdot10^{-7}$ to each variance
before division:

$$
\sigma_x^2\leftarrow\sigma_x^2+10^{-7},
\qquad
\sigma_n^2\leftarrow\sigma_n^2+10^{-7}.
$$

This makes the denominator finite, but it is a fixed local floor inside a
globally scale-normalized computation. A canonical implementation should make
the floor scale-relative and document its all-zero fallback.

### LMMSE modes

The `LmmseMode` selector controls post-processing after the core estimate:

| Mode | Median passes | Refinement passes |
| --- | ---: | ---: |
| `Basic` | 0 | 0 |
| `Median` | 1 | 0 |
| `ThreeMedians` | 3 | 0 |
| `Refine(r)` | 3 | $r$ |

The median passes operate on color differences. Refinement passes update
missing channels from weighted neighboring color differences.

## Numeric contract audit

### Linear sensor measurements

The canonical reconstruction equations operate on channel differences and
local statistics. They contain no required transform to a display or
perceptual space.

The original paper notes that equal-difference assumptions are often better
for gamma-corrected images than equal-ratio assumptions. That is a statistical
model observation about image data, not an equation-level gamma dependency.
Linear-light operation remains mathematically defined; its reconstruction
quality must be measured separately.

### Required input range

The equations have no fixed $[0,1]$ or $[0,255]$ requirement. The paper's
experiments and the IPOL variance constants use an intensity range, but the
estimator itself is affine/homogeneous apart from regularization terms.

### Positive scale equivariance

For $s>0$, differences scale by $s$, variances scale by $s^2$, and
$\lambda$ is unchanged. The estimate

$$
\widehat x=\mu+
\frac{\sigma_x^2}{\sigma_x^2+\sigma_n^2}(f-\mu)
$$

therefore scales by $s$ when no fixed additive floor dominates. A
scale-relative floor must be proportional to $s^2$.

The current gamma variant first divides by a robust input scale and multiplies
the result back afterward. That wrapper preserves positive-scale equivariance,
but it does not make the intermediate gamma-domain operation linear.

### Signed finite inputs

Color differences and their means may be negative. Variances are sums of
squares and remain non-negative. Negative samples do not create invalid
logarithms or roots in canonical LMMSE.

The compatibility transfer uses odd extension below zero and a power-law
continuation above one. It is finite for finite inputs, but the transformed
intermediate is nonlinear and is not a faithful linear-domain value.

### Singularities and non-finite behavior

- $\sigma_x^2+\sigma_n^2$ can be zero on a constant zero signal without a
  floor.
- Directional fusion has the same issue when both
  $\sigma_{e,h}^2$ and $\sigma_{e,v}^2$ are zero.
- The IPOL implementation uses fixed positive constants relative to its chosen
  intensity range; the paper does not make a particular sample range essential.
- A Fornax canonical conversion needs a scale-relative variance floor
  $\tau^2\propto s^2$, plus a defined all-zero fallback.
- A fixed code-domain constant makes behavior differ when the same signal is
  represented at another positive scale.

### Generic $f32$/$f64$ arithmetic

The canonical algorithm needs addition, multiplication, division, means, and
non-negative squares. It can be generic over Fornax's `Primitive` bound.

The compatibility variant uses `powf`, `exp`, absolute value, comparisons, and
`mul_add`; those operations are available generically. Its $f64$ support does
not turn the transfer-domain reconstruction into canonical LMMSE.

## Mandatory changes

For canonical sensor-linear LMMSE:

1. Implement the directional estimates, low-pass statistics, variance fusion,
   and color-difference reconstruction directly on linear samples.
2. Remove the gamma encode/decode workspace from the default algorithm.
3. Replace fixed variance additions with a scale-relative floor derived from
   the mosaic's global positive scale.
4. Define the all-zero/all-constant fallback explicitly so no denominator is
   zero.
5. Preserve signed color differences and HDR values; clamp only bounded
   interpolation weights where the model requires them.
6. Prove `f32`/`f64` behavior with signed, HDR, subnormal-magnitude, scaling,
   constant-field, and tile-boundary tests.

For the current darktable-compatible path:

1. Rename or document it as `LMMSE-darktable` compatibility behavior if it is
   retained.
2. Keep the transfer-domain operation explicit rather than describing the
   intermediate as sensor-linear.
3. Keep its scale normalization and reversible decode outside the canonical
   LMMSE claim.

## Unavoidable compromises

- The LMMSE estimator requires a non-zero denominator convention.
- The color-difference smoothness assumption is a prior about natural images,
  not a consequence of sensor linearity.
- Canonical linear-domain LMMSE quality may differ from the original paper's
  gamma-corrected test data and from darktable's compatibility output.
- Exact darktable behavior retains bounded difference clamps, median passes,
  and refinement passes that are not part of Zhang-Wu's core estimator.

## Unresolved questions

- What scale-relative variance floor best preserves the original estimator
  while remaining stable in $f32$?
- Should Fornax make canonical linear LMMSE the default and keep darktable
  compatibility as a separate selector?
- Which reference becomes the primary validation oracle after removal of the
  comparison harness?
