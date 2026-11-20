# Directional LMMSE demosaicing eligibility

This report audits Zhang-Wu directional linear minimum mean-square-error
demosaicing and the darktable-compatible gamma-domain variant currently exposed
by Fornax.

## Classification

- **Canonical Zhang-Wu LMMSE: Convertible.** The published estimator can be
  evaluated directly on sensor-linear color-difference signals. Fixed variance
  tolerances must be replaced by a scale-relative floor, but no gamma transfer
  or fixed sample range is inherent.
- **Current darktable-compatible LMMSE: Conditional.** Fornax's current
  `Demosaic::Lmmse` interpolates in an analytic nonlinear transfer workspace
  and decodes back to linear values. It is mathematically reversible and
  scale-normalized, but it is a distinct nonlinear-domain variant rather than
  the canonical sensor-linear estimator.

The compatibility variant may survive if explicitly named. The default
sensor-linear candidate should use the published LMMSE equations without the
gamma workspace.

## Sources

- L. Zhang and X. Wu, "Color demosaicking via directional linear minimum
  mean-square-error estimation," *IEEE Transactions on Image Processing*,
  vol. 14, no. 12, pp. 2167-2178, 2005:
  [doi:10.1109/TIP.2005.857260](https://doi.org/10.1109/TIP.2005.857260).
  Crossref identifies this as the paper at those pages and title.
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
  `demosaic_ppg`. The darktable LMMSE path uses PPG as its initial output.

## Original formulation

### Directional estimates

For a red row, Zhang and Wu use second-order Laplacian interpolation to estimate
the missing green and red samples:

$$
\begin{aligned}
\widehat G_n&=\frac{1}{2}(G_{n-1}+G_{n+1})
-\frac{1}{4}(R_{n-2}-2R_n+R_{n+2}),\\
\widehat R_n&=\frac{1}{2}(R_{n-1}+R_{n+1})
-\frac{1}{4}(G_{n-2}-2G_n+G_{n+2}).
\end{aligned}
$$

The primary difference signals are

$$
\Phi_{g,r}=G-R,\qquad \Phi_{g,b}=G-B.
$$

At a red sample, the horizontal green-red difference observation is
`Ghat_n-R_n`; at a green sample it is `G_n-Rhat_n`. Vertical estimates follow
the same construction along a column. Blue rows are symmetric.

### LMMSE denoising

The IPOL exposition applies its fixed nine-tap low-pass kernel

$$
h(z)=\frac{1}{128}
\left(26+23(z+z^{-1})+15(z^2+z^{-2})
+9(z^3+z^{-3})+4(z^4+z^{-4})\right).
$$

For original observations `f` and filtered observations `s=f*h`, the local
statistics are

```math
\begin{aligned}
\mu_x&=\operatorname{mean}(s),\\
\sigma_x^2&=\operatorname{mean}\left((s-\mu_x)^2\right),\\
\sigma_v^2&=\operatorname{mean}\left((s-f)^2\right).
\end{aligned}
```

The denoised directional estimate is

```math
\widehat x=\mu_x+
\frac{\sigma_x^2}{\sigma_x^2+\sigma_v^2}(f-\mu_x),
```

and its estimated error variance is

$$
\sigma_{\mathrm{err}}^2=
\frac{\sigma_x^2\sigma_v^2}
{\sigma_x^2+\sigma_v^2}.
$$

### Directional fusion

Let `h` and `v` be the horizontal and vertical estimates and let
`sigma_h^2` and `sigma_v^2` be their estimated error variances. The fused
estimate is

$$
\widehat\phi=(1-\lambda)h+\lambda v,
\qquad
\lambda=\frac{\sigma_h^2}{\sigma_h^2+\sigma_v^2}.
$$

This fuses the direction with the smaller estimated error variance more
strongly. A non-zero denominator convention is required at zero variance; the
IPOL exposition uses `0.1` and `0.2` relative to its display-code range.

### Red and blue reconstruction

Once the fused green channel is known, measured green-minus-red and
green-minus-blue differences are interpolated:

$$
\Phi(i,j)=\frac{1}{4}\sum_{q\in N_D}\Phi(q)
$$

at the opposite-primary site, and

$$
\Phi(i,j)=\frac{1}{4}\sum_{q\in N_4}\Phi(q)
$$

at green sites. The final channels are `R=G-\Phi_{g,r}` and
`B=G-\Phi_{g,b}`.

## Numeric contract audit

### Linear sensor measurements

The reconstruction equations operate on channel differences and local
statistics. They contain no required transform to a display or perceptual
space.

The original paper notes that equal-difference assumptions are often better
for gamma-corrected images than equal-ratio assumptions. That is a statistical
model observation about image data, not an equation-level gamma dependency.
Linear-light operation remains mathematically defined; its reconstruction
quality must be measured separately.

### Required input range

The equations have no fixed $[0,1]$ or $[0,255]$ requirement. The paper's
experiments and the IPOL variance constants use an intensity range, but the
estimator itself is affine/homogeneous apart from those regularization terms.

### Positive scale equivariance

For positive $s$, differences scale by $s$, variances scale by $s^2$, and
`lambda` is unchanged. The estimate

$$
\widehat x=\mu+
\frac{\sigma_x^2}{\sigma_x^2+\sigma_v^2}(f-\mu)
$$

therefore scales by `s` when no fixed additive floor dominates. A floor must
also be proportional to `s^2`.

The current gamma variant first divides by a robust input scale and multiplies
the result back afterward. That wrapper preserves positive-scale equivariance,
but it does not make the intermediate gamma-domain operation linear.

### Signed finite inputs

Color differences and their means may be negative. Variances are sums of
squares and remain non-negative. Negative samples do not create invalid
logarithms or roots in canonical LMMSE.

The current analytic transfer uses odd extension below zero and a power-law
continuation above one. It is finite for finite inputs, but the transformed
intermediate is still nonlinear and is not a faithful linear-domain value.

### Singularities and non-finite behavior

- The estimator denominator
  `sigma_x^2 + sigma_v^2` can be zero on a constant zero signal.
- Fusion has the same issue when both directional error variances are zero.
- The IPOL implementation uses fixed positive constants relative to its chosen
  intensity range; the paper does not make a particular sample range essential.
- A Fornax conversion needs a scale-relative variance floor
  `tau^2 \propto s^2`, plus a defined all-zero fallback.
- A fixed code-domain constant makes behavior differ when the same signal is
  represented at another positive scale.

### Generic `f32`/`f64` arithmetic

The canonical algorithm needs addition, multiplication, division, means, and
non-negative squares. It can be generic over Fornax's `Primitive` bound.

The current gamma variant uses `powf`, `exp`, absolute value, comparisons, and
`mul_add`; those operations are available generically. Its `f64` support does
not by itself turn the transfer-domain reconstruction into canonical LMMSE.

## Current Fornax behavior

[`lmmse.rs`](../../../crates/fornax-raw-pipeline/src/demosaic/lmmse.rs)
currently implements a darktable-compatible variant:

- PPG seeding;
- 136-pixel tiles with four-pixel borders and eight-pixel overlap;
- normalization by a robust positive scale;
- an analytic darktable-style sRGB transfer with odd signed extension and
  power-law HDR continuation;
- horizontal/vertical color-difference filtering and variance fusion;
- optional median and refinement passes;
- no lookup table, integer working buffer, or output clamp;
- generic `f32`/`f64` arithmetic.

The current `mmse_value` function adds a fixed `1.0e-7` to each variance after
global normalization. This is scale-equivariant because the input was first
normalized, but it mixes global scale normalization with a fixed local floor.
A canonical linear-domain implementation should make that regularization
explicit and tied to the normalized signal model.

The current tile-boundary regression in
[`lmmse.rs`](../../../crates/fornax-raw-pipeline/src/demosaic/lmmse.rs)
passes for the current implementation.

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
- Exact darktable behavior retains bounded difference clamps, gamma LUT
  history, median passes, and refinement passes that are not part of Zhang-Wu's
  core estimator.

## Unresolved questions

- What scale-relative variance floor best preserves the original estimator
  while remaining stable in `f32`?
- Should Fornax make canonical linear LMMSE the default and keep darktable
  compatibility as a separate selector?
- Which reference becomes the primary validation oracle after removal of the
  comparison harness?
