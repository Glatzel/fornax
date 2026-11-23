# Ratio-corrected demosaicing eligibility

This report explains RCD 2.3 notation, direction statistics, ratio correction,
fixed tolerances, and signed-value behavior.

## Classification

**Convertible.** RCD reconstruction is not fundamentally display-referred:
its directional statistics and color corrections use homogeneous arithmetic
and do not require gamma. The published reference nevertheless normalizes to
$[0,1]$, adds fixed tolerances, and clamps output channels. Those are
implementation policies that Fornax can replace.

The unavoidable mathematical issue is the ratio denominator
$L_0 + L_{2d}$, which can approach or cross zero for signed samples. Fornax
therefore needs a scale-relative denominator policy and a finite fallback. A
signed variant should have a distinct name if exact RCD 2.3 behavior is also
retained.

## Sources

- Luis Sanz Rodríguez, *RCD-Demosaicing*, release 2.3:
  [author repository](https://github.com/LuisSR/RCD-Demosaicing/tree/0aa9d6e234f55adb0abf55a82be371ad32dbc60d).
  The pinned commit is `0aa9d6e234f55adb0abf55a82be371ad32dbc60d`.
- The author's
  [README equations](https://github.com/LuisSR/RCD-Demosaicing/blob/0aa9d6e234f55adb0abf55a82be371ad32dbc60d/README.md)
  define directional discrimination, the low-pass filter, ratio-corrected
  green, and color-difference red/blue reconstruction.
- The
  [release 2.3 implementation](https://github.com/LuisSR/RCD-Demosaicing/blob/0aa9d6e234f55adb0abf55a82be371ad32dbc60d/rcd_demosaicing.c)
  provides the exact coefficients, tolerances, loop bounds, and output clamp.
- darktable 5.6.2,
  [`rcd.c`](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/rcd.c),
  is the maintained behavioral reference used by Fornax.

No peer-reviewed RCD paper was located. The author repository is the primary
technical formulation; conclusions that depend on unpublished intent remain
identified as unresolved.

## Notation

The equations below use sensor coordinates and sampled CFA values:

| Symbol | Meaning |
| --- | --- |
| $(x,y)$ or $p$ | Pixel coordinate |
| $F(x,y)$ | Scalar CFA sample at a pixel |
| $F_p$ | Shorthand for $F(x,y)$ when $p=(x,y)$ |
| $F_0$ | CFA sample at the interpolation center |
| $F_d$ | CFA sample one cardinal pixel step in direction $d$ |
| $F_{2d},F_{3d},F_{4d}$ | CFA samples two, three, and four cardinal steps in direction $d$ |
| $d\in\{N,S,W,E\}$ | North, south, west, and east cardinal directions |
| $\mathcal N_4(p)$ | Four axial same-color CFA neighbors used by the low-pass field |
| $\mathcal N_D(p)$ | Four diagonal same-color CFA neighbors used by the low-pass field |
| $L_p$ | Low-pass CFA value at same-color location $p$ |
| $\mathbf f$ | 25-element vector of a $5\times5$ CFA window centered at $p$ |
| $A_v,A_h$ | Fixed symmetric coefficient matrices for vertical and horizontal discrimination energy |
| $A_P,A_Q$ | Fixed symmetric coefficient matrices for NW-SE and NE-SW discrimination energy |
| $E_v,E_h$ | Non-negative vertical and horizontal discrimination energies |
| $\tau_v,\tau_h$ | Fixed non-negative energy floors used by release 2.3 |
| $\tau_P,\tau_Q$ | Fixed non-negative diagonal energy floors used by release 2.3 |
| $\alpha_{vh}$ | Fraction of the horizontal green estimate in the final green value |
| $P,Q$ | Diagonal discrimination energies for NW-SE and NE-SW directions |
| $\alpha_{pq}$ | Fraction of the NE-SW color-difference estimate at a red or blue site |
| $\widehat C_d$ | Ratio-corrected green candidate in cardinal direction $d$ |
| $D_N,D_S,D_W,D_E$ | Non-negative directional gradients used by inverse-gradient weighting |
| $\widehat G_v,\widehat G_h$ | Vertical and horizontal green candidates before the final blend |
| $\widehat\Phi_P,\widehat\Phi_Q$ | NW-SE and NE-SW color-difference estimates |
| $\widehat\Phi_v,\widehat\Phi_h$ | Vertical and horizontal color-difference estimates |
| $\varepsilon$ | Small non-negative regularizer used in gradients and ratio denominators |
| $C_p$ | Reconstructed red or blue channel at center $p$ |

The CFA window $\mathbf f$ is stored row-major. The matrices $A_v$ and $A_h$
encode the release-2.3 products of center and neighboring samples over a
$5\times5$ support. Their exact integer coefficients belong to the pinned
source; the compact formulas below do not replace that coefficient table.

## Low-pass field

RCD first constructs a smoothed field from CFA samples of the same color:

$$
L_p=\frac{1}{4}F_p
+\frac{1}{8}\sum_{q\in\mathcal N_4(p)}F_q
+\frac{1}{16}\sum_{q\in\mathcal N_D(p)}F_q.
$$

The weights sum to one. The center sample receives $1/4$, the four axial
same-color neighbors receive $1/8$ each, and the four diagonal same-color
neighbors receive $1/16$ each. The resulting field suppresses fine CFA
alternation before the ratio correction compares nearby same-color levels.

## Vertical/horizontal discrimination

Let $\mathbf f$ contain the CFA values in a $5\times5$ window centered at
$p$. Release 2.3 forms two quadratic direction energies:

$$
E_v=\max\left(\tau_v,\mathbf f^{\mathsf T}A_v\mathbf f\right),
\qquad
E_h=\max\left(\tau_h,\mathbf f^{\mathsf T}A_h\mathbf f\right).
$$

$A_v$ and $A_h$ are fixed integer matrices. The energy is high when the window
contains strong variation along that direction. Because both energies are
non-negative, a zero-energy window needs the floors $\tau_v$ and $\tau_h$.

The interpolation fraction is

$$
\alpha_{vh}=\frac{E_v}{E_v+E_h}.
$$

Thus $\alpha_{vh}$ increases when vertical energy increases and the horizontal
estimate should contribute more. The sum of the chosen interpolation weights
is one:

$$
\widehat G=\alpha_{vh}\widehat G_h
+\left(1-\alpha_{vh}\right)\widehat G_v.
$$

### Neighbor smoothing of the weight

At each interpolation site, RCD computes the central value of
$\alpha_{vh}$ and the average of its four diagonal neighbors:

$$
\overline\alpha_{vh}(p)=
\frac{1}{4}\sum_{q\in\mathcal N_D(p)}\alpha_{vh}(q).
$$

It selects whichever value is closer to $1/2$:

$$
\alpha_{vh}^{*}(p)=
\begin{cases}
\overline\alpha_{vh}(p),
& \left|\frac{1}{2}-\alpha_{vh}(p)\right|
<
\left|\frac{1}{2}-\overline\alpha_{vh}(p)\right|,\\
\alpha_{vh}(p),& \text{otherwise}.
\end{cases}
$$

The selected value is then clamped to $[0,1]$:

$$
\operatorname{clamp}_{[0,1]}(z)=\min(1,\max(0,z)).
$$

This clamp is an interpolation-fraction constraint, not a clamp on RGB signal
values.

## Green ratio correction

At a red or blue center $p$, the cardinal neighbors $F_N,F_S,F_W,F_E$ are
green samples. For each direction $d$, release 2.3 forms a ratio-corrected
green candidate:

$$
\widehat C_d
=F_d\left(
1+\frac{L_0-L_{2d}}{\varepsilon+L_0+L_{2d}}
\right),
\qquad d\in\{N,S,W,E\}.
$$

$L_0$ is the low-pass value at the center and $L_{2d}$ is the low-pass value
two pixel steps away in direction $d$. The ratio compares the center's
same-color level with the level beyond the candidate green sample. When the
two low-pass values are equal, the multiplicative correction is one and
$\widehat C_d=F_d$.

The four green candidates are not averaged directly. Direction gradients
decide how much each opposing pair contributes:

$$
\begin{aligned}
D_N&=\varepsilon+|F_N-F_S|+|F_0-F_{2N}|\\
&\quad+|F_N-F_{3N}|+|F_{2N}-F_{4N}|,\\
D_S&=\varepsilon+|F_N-F_S|+|F_0-F_{2S}|\\
&\quad+|F_S-F_{3S}|+|F_{2S}-F_{4S}|,\\
D_W&=\varepsilon+|F_W-F_E|+|F_0-F_{2W}|\\
&\quad+|F_W-F_{3W}|+|F_{2W}-F_{4W}|,\\
D_E&=\varepsilon+|F_W-F_E|+|F_0-F_{2E}|\\
&\quad+|F_E-F_{3E}|+|F_{2E}-F_{4E}|.
\end{aligned}
$$

The subscripts identify displacement from the center. For example, $F_{2N}$
is the same-color sample two pixels north of the center, $F_{3N}$ is the green
sample three pixels north, and $F_{4N}$ is the next same-color sample.

The vertical and horizontal candidates use inverse-gradient weighting:

$$
\widehat G_v=
\frac{D_S\widehat C_N+D_N\widehat C_S}{D_N+D_S},
\qquad
\widehat G_h=
\frac{D_W\widehat C_E+D_E\widehat C_W}{D_E+D_W}.
$$

The estimate on one side is weighted by the gradient on the opposite side.
The complete green value is

$$
G_p=\alpha_{vh}^{*}(p)\widehat G_h
+\left(1-\alpha_{vh}^{*}(p)\right)\widehat G_v.
$$

This makes $G_p$ a weighted blend of the vertical and horizontal estimates
rather than a hard direction switch.

## Red and blue reconstruction

After green is known, RCD reconstructs red and blue through
color-difference fields relative to green. Define

$$
\Phi_R(p)=R(p)-G(p),
\qquad
\Phi_B(p)=B(p)-G(p).
$$

Each reconstructed channel is then $C(p)=G(p)+\Phi_C(p)$. The signs follow
the channel difference used by the implementation. RCD interpolates these
smoother color differences instead of interpolating red and blue independently.

### Diagonal P/Q stage at red and blue sites

For a red center missing blue, or a blue center missing red, let
$\widehat\Phi_P$ be the NW-SE color-difference estimate and
$\widehat\Phi_Q$ be the NE-SW estimate. Their gradients use diagonal same-color
samples and already reconstructed green:

$$
\widehat\Phi_P=
\frac{D_{SE}\Phi_{NW}+D_{NW}\Phi_{SE}}{D_{NW}+D_{SE}},
\qquad
\widehat\Phi_Q=
\frac{D_{SW}\Phi_{NE}+D_{NE}\Phi_{SW}}{D_{NE}+D_{SW}}.
$$

Here $\Phi_{NW}$ denotes the measured color difference at the northwest
diagonal neighbor, and similarly for the other three diagonals.

The diagonal gradients have the same meaning as the cardinal gradients:

$$
D_X=\varepsilon+|C_X-C_{\bar X}|+|C_X-C_{3X}|
+|G_p-G_{2X}|,
\qquad X\in\{NW,NE,SW,SE\}.
$$

Here $C_X$ is the missing primary at diagonal direction $X$,
$C_{\bar X}$ is the opposite diagonal sample, and $G_{2X}$ is the green value
two diagonal steps from the center.

The diagonal stage uses the same $5\times5$ quadratic construction. Denote the
NW-SE and NE-SW CFA windows by $\mathbf f_P$ and $\mathbf f_Q$, and their
coefficient matrices by $A_P$ and $A_Q$:

$$
P=\max\left(\tau_P,\mathbf f_P^{\mathsf T}A_P\mathbf f_P\right),
\qquad
Q=\max\left(\tau_Q,\mathbf f_Q^{\mathsf T}A_Q\mathbf f_Q\right).
$$

The diagonal interpolation fraction is

$$
\alpha_{pq}=\frac{P}{P+Q}.
$$

As in the vertical/horizontal path, RCD compares the central fraction with
the average of its diagonal neighbors, selects the value closer to $1/2$, and
clamps it to $[0,1]$. Denote the resulting value by
$\alpha_{pq}^{*}$. The missing primary is then

$$
C_p=G_p+\alpha_{pq}^{*}\widehat\Phi_Q
+\left(1-\alpha_{pq}^{*}\right)\widehat\Phi_P.
$$

### Cardinal stage at green sites

At a green site in a red row, red lies west/east and blue lies north/south.
At a green site in a blue row, the directions exchange. Let
$\widehat\Phi_v$ and $\widehat\Phi_h$ be the inverse-gradient
color-difference estimates built from the corresponding cardinal neighbors.
The missing channel is

$$
C_p=G_p+\alpha_{vh}^{*}(p)\widehat\Phi_h
+\left(1-\alpha_{vh}^{*}(p)\right)\widehat\Phi_v.
$$

The reference clamps every reconstructed RGB channel to $[0,1]$ before
converting back to 16-bit codes. Fornax does not retain that output clamp.

## Current Fornax behavior

The kernel in
[`rcd.rs`](../../../crates/fornax-raw/src/demosaic/rcd.rs) follows
the release-2.3 directional and color-difference structure with these
implementation parameters:

| Parameter | Value | Meaning |
| --- | ---: | --- |
| `TILE_SIZE` | 112 | Square tile including halo |
| `TILE_CORE` | 92 | Distance between neighboring tile origins |
| `BORDER` | 10 | Halo width needed by the stencils |
| `MARGIN` | 9 | Additional image-edge margin |
| Regularizer $\varepsilon$ | Robust-scale relative | Derived from the mosaic's positive normalization scale and the floating-point epsilon |

The kernel first creates a PPG result. Images smaller than the RCD border
requirement retain that PPG result. For larger images, RCD replaces the tile
cores while preserving halos needed by neighboring tiles.

The current implementation:

- removes the $[0,1]$ RGB output clamp;
- keeps $[0,1]$ clamps only for discrimination fractions;
- computes $\varepsilon$ from a robust input scale and the working type;
- accepts signed and HDR values without an integer working buffer;
- passes current signed, HDR, $f64$, and positive-scale invariant checks.

The remaining gap is mathematical policy rather than public API: the code
uses a very small scale-relative $\varepsilon$ for $L_0+L_{2d}$ but does not
explicitly define a fallback at a signed zero crossing.

## Numeric contract audit

### Linear sensor measurements

The author formulation applies the same arithmetic directly to normalized CFA
values. It contains no gamma transfer, perceptual color space, or display
encoding. Its low-pass and color-difference operations can operate on linear
sensor values.

### Required input range

The mathematical equations do not demand $[0,1]$. Release 2.3 chooses that
range because dcraw stores 16-bit image data. Its output clamp suppresses
negative results and values above one for display/reference compatibility.

Those bounds must not be carried into Fornax. The $[0,1]$ discrimination
clamps are different: they constrain an interpolation fraction and are
mathematically meaningful.

### Positive scale equivariance

The quadratic energy terms in $E_v$ and $E_h$ scale as $s^2$. Therefore
$\alpha_{vh}$ is unchanged for $s>0$, and the interpolated signal scales by
$s$. Gradients, color differences, and estimates are homogeneous apart from
tolerance floors.

Fixed $\tau_v,\tau_h$, and $\varepsilon$ destroy exact scale equivariance when
they become comparable to the signal terms. Scale-relative tolerances restore
the intended behavior.

### Signed finite inputs

Absolute gradients and additive color differences remain valid for signed
values. The ratio correction is the difficult case:

$$
1+\frac{L_0-L_{2d}}{\varepsilon+L_0+L_{2d}}.
$$

The ratio is scale-invariant away from zero, but the paired low-pass values can
sum to zero or change sign even when the field is finite. A fixed
$\varepsilon$ does not define a stable result at every signed crossing.

### Singularities and non-finite behavior

- $L_0+L_{2d}=0$ is a pole of the exact ratio-correction formula.
- Direction and color-difference averages divide by sums of non-negative
  gradients. A relative floor can keep those denominators finite.
- The discrimination ratio requires a zero-energy convention; release 2.3
  applies fixed floors to the quadratic energies.
- Near cancellation, no unique value follows from the original ratio alone.
  Fornax must define a deterministic, derivative-free fallback.

### Generic $f32$/$f64$ arithmetic

RCD requires ordinary floating-point arithmetic and absolute values. The
source's integer coefficients and unsigned-code storage are optimizations or
range policies, not mathematical necessities. The current Fornax kernel is
generic over `Primitive`, with `f32` and `f64` support.

## Mandatory changes

1. Keep the directional and color-difference equations, but evaluate all
   intermediate values in the input floating-point type.
2. Replace every reference-domain tolerance with an explicitly scale-relative
   tolerance derived from the input scale.
3. Define ratio correction as:
   - the original ratio when $|L_0+L_{2d}|$ exceeds the tolerance; and
   - a finite color-difference fallback when it does not.
4. Make the fallback positive-scale homogeneous and test it on signed fields
   whose low-pass values cross zero.
5. Preserve the discrimination bounds $[0,1]$; do not restore the RGB clamp.
6. Document the modified denominator rule as part of the Fornax RCD contract.

## Unavoidable compromises

- The exact RCD ratio has no unique extension through a signed zero
  denominator.
- A tolerance/fallback changes behavior only near cancellation but is necessary
  for a total finite function.
- The only authoritative formulation found is the author's code and README;
  there is no peer-reviewed equation paper to arbitrate ambiguous intent.
- Direction weights and empirical gradient constants remain part of RCD's
  mathematical model rather than removable numerical noise.

## Unresolved questions

- Should the zero-crossing fallback use the unratioed color-difference
  correction, the opposite-side estimate, or another documented interpolation?
- What tolerance ratio gives stable $f32$ behavior without changing ordinary
  positive RAW results?
- Which independent reference can replace the removed comparison harness for
  future behavioral checks?
