# Patterned pixel grouping eligibility

This report audits the original Pixel Grouping formulation and the modern
dcraw/LibRaw/darktable `PPG` descendant against Fornax's sensor-linear numeric
contract.

## Classification

**Eligible for both formulations, with distinct provenance.**

- Chuan-kai Lin's 2003 Pixel Grouping formulation is deterministic arithmetic
  with data-dependent order comparisons and a singularity-free hue transit
  branch.
- The modern dcraw/LibRaw/darktable `PPG` algorithm used by Fornax is a later
  directional interpolation and color-difference variant. It is also
  sensor-linear, signed-safe, and positive-scale homogeneous.

Modern `PPG` is not the same set of equations as the 2003 page. If exact
Pixel Grouping behavior is required in addition to Fornax's current kernel, it
must be introduced under a distinct name such as `PixelGrouping`.

## Sources

- Chuan-kai Lin, "Pixel Grouping for Color Filter Array Demosaicing":
  [archived technical description](https://web.archive.org/web/20070326041737/http://web.cecs.pdx.edu/~cklin/demosaic/).
  The page records the original four-direction green gradients,
  `hue_transit`, opposite-color estimates, and a GPL reference
  implementation. It states that the algorithm was developed on 2003-04-03.
- Alain Desbiolles, "Color interpolation (Pixel Grouping)," UFRaw discussion,
  2007:
  [author provenance thread](https://sourceforge.net/p/ufraw/discussion/434060/thread/07836bf0/).
  Desbiolles identifies the original Pixel Grouping page, describes the
  adaptation, and records that dcraw 8.72 adopted the result as `PPG`.
- LibRaw 0.22.3, commit
  [`cf02d66`](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L360),
  `LibRaw::ppg_interpolate`. This is the pinned source for the modern equation
  sequence used by Fornax.
- darktable 5.6.2, commit
  [`20891f6`](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20),
  `demosaic_ppg`. darktable carries the same modern directional equations with
  a separate border and non-negative-output policy.

## Original Pixel Grouping equations

For a red or blue center `R13`, the original description computes four
gradients:

```math
\begin{aligned}
\Delta_N&=2|R_3-R_{13}|+|G_8-G_{18}|,\\
\Delta_E&=2|R_{13}-R_{15}|+|G_{12}-G_{14}|,\\
\Delta_W&=2|R_{11}-R_{13}|+|G_{12}-G_{14}|,\\
\Delta_S&=2|R_{13}-R_{23}|+|G_8-G_{18}|.
\end{aligned}
```

After selecting the smallest gradient, the green estimate is one of

```math
\begin{aligned}
G_{13}&=(3G_8+R_{13}+G_{18}-R_3)/4 &&(\Delta_N),\\
G_{13}&=(3G_{14}+R_{13}+G_{12}-R_{15})/4 &&(\Delta_E),\\
G_{13}&=(3G_{12}+R_{13}+G_{14}-R_{11})/4 &&(\Delta_W),\\
G_{13}&=(3G_{18}+R_{13}+G_8-R_{23})/4 &&(\Delta_S).
\end{aligned}
```

At green sites and opposite-primary sites, the original method uses:

```math
\operatorname{hue\_transit}(l_1,l_2,l_3,v_1,v_3)=
\begin{cases}
v_1+(v_3-v_1)\dfrac{l_2-l_1}{l_3-l_1},
& (l_1<l_2<l_3)\ \lor\ (l_1>l_2>l_3),\\
\dfrac{v_1+v_3}{2}+\dfrac{2l_2-l_1-l_3}{4},
& \text{otherwise}.
\end{cases}
```

For example, at `G8`, the original description estimates

```math
B_8=\operatorname{hue\_transit}(G_7,G_8,G_9,B_7,B_9),
\quad
R_8=\operatorname{hue\_transit}(G_3,G_8,G_{13},R_3,R_{13}).
```

At `R13`, the missing blue channel chooses between the two diagonals using
separate CFA and green-difference gradients, then applies the same
`hue_transit` function to the selected diagonal.

The division in `hue_transit` occurs only in the strict monotonic branch, where
`l_3-l_1` is nonzero. The non-monotonic branch avoids division altogether.

## Modern LibRaw/darktable PPG equations

For a red or blue center `F_c`, the modern kernel computes, for each cardinal
direction `d`,

```math
\begin{aligned}
\widehat G_d&=
2(F_{c-d,G}+F_c+F_{c+d,G})-F_{c-2d,c}-F_{c+2d,c},\\
D_d={}&3\left(|F_{c-2d,c}-F_c|+|F_{c+2d,c}-F_c|
+|F_{c-d,G}-F_{c+d,G}|\right)\\
&+2\left(|F_{c+3d,G}-F_{c+d,G}|
+|F_{c-3d,G}-F_{c-d,G}|\right).
\end{aligned}
```

It chooses the lower-direction cost and applies a local envelope:

```math
G_c=\operatorname{clamp}
\left(\frac{\widehat G_d}{4},
\min(F_{c-d,G},F_{c+d,G}),
\max(F_{c-d,G},F_{c+d,G})\right).
```

At green sites, the missing color along a cardinal direction is reconstructed
from color differences:

```math
\widehat C_c=\frac{F_{c-d,C}+F_{c+d,C}
+2G_c-G_{c-d}-G_{c+d}}{2}.
```

At a red center missing blue, or a blue center missing red, the two diagonal
guesses use

```math
\begin{aligned}
\widehat B_{\mathrm{ne}}&=
B_{\mathrm{ne}}+B_{\mathrm{sw}}+2G_c-G_{\mathrm{ne}}-G_{\mathrm{sw}},\\
D_{\mathrm{ne}}&=
|B_{\mathrm{ne}}-B_{\mathrm{sw}}|
+|G_{\mathrm{ne}}-G_c|+|G_{\mathrm{sw}}-G_c|,
\end{aligned}
```

and the symmetric north-west estimate. The lower-cost estimate is divided by
two; equal costs average both guesses and divide by four.

## Numeric contract audit

### Linear sensor measurements

Both formulations operate directly on CFA values using differences,
multipliers, absolute values, minima/maxima, and affine hue interpolation.
Neither requires a display transform. Fornax currently implements the modern
LibRaw/darktable difference equations.

### Required input range

The original equations are scale-free and do not require `[0,1]`. Its archived
reference implementation uses an unsigned 16-bit host representation, but that
is a storage limitation. Modern LibRaw uses unsigned samples and darktable
floors outputs at zero, again implementation policies rather than properties
of the mathematical estimator.

The local min/max envelope in modern `PPG` is data-dependent. It works for
signed samples because it clamps to neighboring signed values rather than to a
global zero.

### Positive scale equivariance

For `s > 0`, every absolute difference, direction cost, green guess, and
color-difference estimate scales by `s`. Direction comparisons and the
`hue_transit` ratio are unchanged. Both formulations are therefore positively
scale-equivariant in exact arithmetic.

### Signed finite inputs

- Original Pixel Grouping compares the ordered magnitudes of `l_1`, `l_2`,
  and `l_3`; it does not apply `log` or take a root of a signed value. The
  strict monotonic ratio branch can produce signed or unbounded values, which
  is mathematically valid linear-light interpolation.
- Modern `PPG` uses only signed sums, differences, absolute values, and
  data-dependent extrema. A negative constant field remains negative.

### Singularities and non-finite behavior

- Original `hue_transit` has no zero-denominator path: the only division is
  guarded by strict monotonicity.
- Modern `PPG` divides by fixed constants, never by a sample-dependent sum.
- The practical finite-risk cases are overflow when the working type is too
  small for extreme HDR values. `f32` and `f64` have the same algorithmic
  behavior; range is a documented floating-point precision issue.

### Generic `f32`/`f64` arithmetic

Neither formulation needs integer bit tricks, LUTs, or exponent manipulation.
Modern Fornax is already generic over `Primitive`, with only `f32` and `f64`
implementing that bound.

## Current Fornax behavior

[`PpgLazy`](../../../crates/fornax-raw-pipeline/src/demosaic/ppg.rs)
implements the modern LibRaw/darktable equations:

- three-pixel border averaging by available same-color samples;
- horizontal/vertical green gradient selection and local min/max envelope;
- cardinal color-difference interpolation at green sites;
- diagonal color-difference interpolation at red/blue sites;
- no gamma, LUT, global clamp, integer working buffer, or fixed input range.

The implementation preserves measured CFA samples, signed constants, HDR
magnitudes, `f64` precision, and positive scaling in the existing demosaic
tests. Its tie order is part of behavioral compatibility: horizontal wins when
the first direction cost is not greater than the alternate cost, and diagonal
ties average both guesses.

## Mandatory changes

The current modern `PPG` formulation requires no algorithmic conversion.

If original 2003 Pixel Grouping is added:

1. Give it a distinct public identity rather than replacing modern `PPG`
   silently.
2. Implement its four-gradient green estimates and guarded `hue_transit`
   equations directly.
3. Define a finite-image border rule and tie behavior.
4. Add the same signed, scale, HDR, precision, and boundary invariants used by
   the current `PPG` path.
5. Treat the archived GPL reference as evidence, not copied production source.

## Unavoidable compromises

- Modern `PPG` still needs a documented border policy and deterministic
  direction tie handling.
- The min/max green envelope changes pure linear interpolation behavior; it is
  intrinsic to the modern formulation.
- No peer-reviewed paper for the original 2003 algorithm has been located; the
  inventor's archived description is the authoritative formulation evidence.
- The name `PPG` is historically ambiguous between the original Pixel Grouping
  method and the later dcraw/LibRaw descendant.

## Unresolved questions

- Is preserving modern `PPG` behavior sufficient, or is a separately named
  original Pixel Grouping variant desired?
- Which finite-image border rule should define a standalone Pixel Grouping
  reference?
- What reference oracle should be used for modern `PPG` after the comparison
  harness is removed?
