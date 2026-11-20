# Malvar-He-Cutler demosaicing eligibility

This report audits the original Malvar-He-Cutler (MHC) interpolation
formulation against Fornax's sensor-linear, scale-independent, signed
floating-point contract.

## Classification

**Tier -1: reference only. Do not implement.**

The reconstruction is an affine 5x5 filtering operation with no intrinsic
gamma transfer, perceptual color space, lookup table, fixed input range,
clipping, or quantization requirement. That numerical analysis does not make
MHC a production candidate: the method is associated with U.S. Patent 7502505,
and the IPOL implementation carries source-license constraints.

Fornax therefore retains this note as algorithm and provenance research only.
It must not add an MHC selector, production kernel, or copied implementation.

## Sources

- H. S. Malvar, L. He, and R. Cutler, "High-quality linear interpolation for
  demosaicing of Bayer-patterned color images," *Proc. IEEE ICASSP*, 2004:
  [doi:10.1109/ICASSP.2004.1326587](https://doi.org/10.1109/ICASSP.2004.1326587).
  The paper supplies the gradient-corrected formulation and the eight 5x5
  interpolation filters.
- P. Getreuer, "Malvar-He-Cutler Linear Image Demosaicking," *Image Processing
  On Line*, 2011:
  [doi:10.5201/ipol.2011.g_mhcd](https://doi.org/10.5201/ipol.2011.g_mhcd).
  The article restates the equations, gains, final filters, and boundary
  behavior. Its source archive is pinned by the article DOI and is the basis
  for the exact finite-image handling described below.
- U.S. Patent 7502505, "High-quality gradient-corrected linear interpolation
  for demosaicing of color images," identifies the authors' related patent.
  The IPOL source states that use and redistribution may require patent and
  source-license review.

## Original equations

Let $F(i,j)$ be the scalar Bayer mosaic and let $R$, $G$, and $B$ denote
channel values at sites where they are measured or reconstructed. MHC starts
from bilinear interpolation and adds a Laplacian correction from another
channel.

At a red or blue site, the bilinear green estimate is

$$
\widehat G_{\mathrm{bl}}(i,j)
= \frac{1}{4}\left(G_{i-1,j}+G_{i+1,j}+G_{i,j-1}+G_{i,j+1}\right).
$$

The corrected green estimate is

$$
\widehat G(i,j)=\widehat G_{\mathrm{bl}}(i,j)+\alpha\Delta_C(i,j),
\qquad \alpha=\frac{1}{2},
$$

where $C$ is the color measured at the center and the 5-point discrete
Laplacian is

$$
\Delta_C(i,j)=C_{i,j}
-\frac{1}{4}\left(C_{i-2,j}+C_{i+2,j}+C_{i,j-2}+C_{i,j+2}\right).
$$

At a green site, interpolation of red uses

$$
\widehat R(i,j)=\widehat R_{\mathrm{bl}}(i,j)+\beta\Delta_G(i,j),
\qquad \beta=\frac{5}{8},
$$

with the sampled 9-point green Laplacian shown by the final MHC filter:

$$
\begin{aligned}
\Delta_G(i,j)={}&5G_{i,j}
-G_{i-2,j}-G_{i+2,j}\\
&-\sum_{d_x,d_y\in\{-1,1\}}G_{i+d_x,j+d_y}
+\frac{1}{2}\left(G_{i,j-2}+G_{i,j+2}\right).
\end{aligned}
$$

The red equation at a blue site uses the corresponding blue 5-point Laplacian
with $\gamma = 3/4$:

$$
\widehat R(i,j)=\widehat R_{\mathrm{bl}}(i,j)
+\frac{3}{4}\Delta_B(i,j).
$$

Blue interpolation is symmetric. At a green site in a red row, for example,
the complete red filter is

$$
\begin{aligned}
\widehat R(i,j)=\frac{1}{8}\Bigl(&
\tfrac{1}{2}G_{i,j-2}
-G_{i-1,j-1}-G_{i+1,j-1}
-G_{i-2,j}-G_{i+2,j}\\
&+4R_{i-1,j}+5G_{i,j}+4R_{i+1,j}\\
&-G_{i-1,j+1}-G_{i+1,j+1}
+\tfrac{1}{2}G_{i,j+2}\Bigr).
\end{aligned}
$$

The red-row and blue-row cases exchange the orientation of the measured
second color. The full set of eight filters, including red and blue at green
sites and red/blue at the opposite primary site, is tabulated in the original
paper and the IPOL article.

## Numeric contract audit

### Linear sensor measurements

Every reconstruction case is a weighted sum of CFA samples. The coefficients
contain no function of sample magnitude and no transfer curve. Applying MHC to
linear sensor measurements therefore preserves their linear-light meaning.

### Required input range

The paper derives gains from natural-image second-order statistics and rounds
them to dyadic rationals, but the resulting operation does not require samples
to lie in `[0,1]` or any 8/16-bit range. Values above one and negative values
enter the same affine sums.

MHC can overshoot the local sample range because six coefficients in several
filters are negative. That is part of the linear estimator, not a display
clamp. Fornax must not clamp its output.

### Positive scale equivariance

For every positive scalar `s`,

$$
\operatorname{MHC}(sF)=s\operatorname{MHC}(F)
$$

within the same finite arithmetic precision, because every coefficient is
constant and every correction is homogeneous of degree one. The chosen gains
are dimensionless and do not depend on a normalized code range.

### Signed finite inputs

Addition, subtraction, and multiplication are sufficient to evaluate all
filters. A signed constant field of value `c` is preserved because each filter
has coefficient sum one. No logarithm, ratio, or negative-value exponent is
present.

### Singularities and non-finite behavior

The interior formulation has no division by a data-dependent quantity. The
finite-image implementation divides by counts of in-bounds neighbors; those
counts are positive for conventionally supported CFA sites. Finite inputs can
therefore produce large finite output only through accumulated arithmetic, not
through an algorithmic pole.

### Generic `f32`/`f64` arithmetic

The equations require only arithmetic and comparisons. An implementation can
be generic over Fornax's `Primitive` bound and preserve the same formulation in
both `f32` and `f64`. Integer shifts are an optimization of the original
dyadic coefficients, not part of the mathematical definition.

## Reference boundary behavior

The IPOL implementation evaluates each 5x5 window using only in-bounds
samples. It zeroes missing neighbors, counts the available samples used by
the corresponding bilinear denominator, and excludes missing correction terms.
That method defines the finite-image result without extending the array or
inventing samples.

That policy can change outer-border pixels. It remains research context only;
the Tier -1 disposition prohibits Fornax from implementing or testing an MHC
kernel.

## Current Fornax behavior

There is no MHC kernel or public `Demosaic` selector. Linear interpolation and
the retained advanced methods are separate implementations and must not be
described as MHC.

## Tier -1 disposition

MHC remains reference material for understanding gradient-corrected linear
interpolation. The patent and source-license risks disqualify it from Fornax's
implementation roadmap. Do not promote this method to a selector or copy code
from the patent or IPOL source.

## Unavoidable compromises

- The 5x5 support requires two pixels of context; very small images need an
  explicit fallback or rejection policy.
- Available-neighbor renormalization defines an outer-border variant whose
  result is not the restriction of an infinite-image convolution.
- The patent and IPOL source-license status constrain implementation even
  though the mathematics is eligible.

## Unresolved questions

None for Fornax implementation. MHC is permanently reference-only under the
current project decision.
