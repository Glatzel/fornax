# Ratio-corrected demosaicing eligibility

This report audits RCD 2.3, especially its direction statistics, low-pass
ratio correction, fixed tolerances, and signed-value behavior.

## Classification

**Convertible.** RCD's reconstruction is not fundamentally display-referred:
its directional statistics and color corrections are homogeneous arithmetic
and do not require gamma. The published reference nevertheless assumes
normalization to `[0,1]`, adds fixed tolerances, and clamps channel outputs.
Those are removable or replaceable implementation requirements.

The unavoidable mathematical issue is the ratio denominator
`L_0 + L_2`, which can approach or cross zero when signed samples are allowed.
Fornax therefore needs a documented scale-relative denominator policy and a
finite fallback. The signed variant should be named if exact RCD 2.3 behavior
is also retained.

## Sources

- Luis Sanz Rodríguez, *RCD-Demosaicing*, release 2.3:
  [author repository](https://github.com/LuisSR/RCD-Demosaicing/tree/0aa9d6e234f55adb0abf55a82be371ad32dbc60d).
  The repository commit is `0aa9d6e234f55adb0abf55a82be371ad32dbc60d`.
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

## Original formulation

The reference first normalizes unsigned 16-bit samples to `[0,1]` and creates a
low-pass field from same-CFA-color sites:

```math
L_p=\frac{1}{4}F_p+\frac{1}{8}\sum_{q\in N_4}F_q
+\frac{1}{16}\sum_{q\in N_D}F_q,
```

where `N_4` and `N_D` contain axial and diagonal CFA neighbors. This is the
`1 2 1 / 2 4 2 / 1 2 1` kernel divided by sixteen.

### Directional discrimination

For vertical/horizontal selection, release 2.3 forms quadratic directional
energies `E_v` and `E_h` over a nine-sample CFA support. The source spells them
out as products of the center and neighboring CFA samples with integer
coefficients. In compact form:

```math
E_d=\max\left(\epsilon^2,\ F^\mathsf{T}A_dF\right),
\qquad
w_{vh}=\frac{E_v}{E_v+E_h},
\qquad d\in\{v,h\}.
```

The diagonal `P/Q` stage uses the same construction with its own coefficient
matrices. The source's exact coefficients must be transcribed from the pinned
release rather than inferred from this compact notation.

At each interpolation site, RCD compares the central discrimination value with
the average of its four diagonal neighbors and uses whichever is closer to
`1/2`. It then clamps that discrimination weight to `[0,1]`; this is a
normalized interpolation weight, not a signal-range clamp.

### Green via ratio correction

For cardinal neighbor estimates `N`, `S`, `W`, and `E`, release 2.3 uses

```math
\widehat C_d=F_d\left(
1+\frac{L_0-L_{2d}}{\epsilon+L_0+L_{2d}}
\right),
\qquad d\in\{N,S,W,E\}.
```

The first direction gradients are sums of absolute CFA differences plus a
fixed `epsilon`:

```math
\begin{aligned}
D_N&=\epsilon+|F_N-F_S|+|F_0-F_{2N}|\\
&\quad+|F_N-F_{3N}|+|F_{2N}-F_{4N}|,
\end{aligned}
```

and analogously for `S`, `W`, and `E`. The directional estimates are
inverse-gradient weighted:

```math
\widehat G_v=\frac{D_S\widehat C_N+D_N\widehat C_S}{D_N+D_S},
\qquad
\widehat G_h=\frac{D_W\widehat C_E+D_E\widehat C_W}{D_E+D_W}.
```

The output is `w_h \widehat G_h + (1-w_h)\widehat G_v`.

### Red and blue

At red/blue sites, diagonal `P` and `Q` estimates use inverse-gradient weighted
color differences and a `P/Q` discrimination weight. At green sites, cardinal
color differences are computed from the full green plane, and the same
vertical/horizontal discrimination weight is reused.

The reference clamps every reconstructed RGB channel to `[0,1]` before
converting back to 16-bit codes.

## Numeric contract audit

### Linear sensor measurements

The author formulation applies the same arithmetic directly to normalized CFA
values. It contains no gamma transfer, perceptual color space, or display
encoding. Its low-pass and color-difference operations can operate on linear
sensor values.

### Required input range

The mathematical equations do not demand `[0,1]`. Release 2.3 chooses that
range because dcraw stores 16-bit image data. Its output clamp suppresses
negative results and values above one for display/reference compatibility.

Those bounds must not be carried into Fornax. The `[0,1]` discrimination clamps
are different: they validate an interpolation fraction and are mathematically
meaningful.

### Positive scale equivariance

The quadratic energy terms in `E_d` scale as `s^2`; therefore the ratio
`E_v/(E_v+E_h)` is unchanged for positive `s`, and the interpolated signal
scales by `s`. Gradients, color differences, and estimates are homogeneous
apart from tolerance floors.

Fixed `epsilon` and `epsilon^2` destroy exact scale equivariance when either
becomes comparable to the signal terms. A scale-relative tolerance restores
the intended behavior.

### Signed finite inputs

Absolute gradients and additive color differences remain valid for signed
values. The ratio correction is the difficult case:

```math
1+\frac{L_0-L_{2d}}{\epsilon+L_0+L_{2d}}
```

The ratio itself is scale-invariant, but the paired low-pass values can sum to
zero or change sign even when the field is finite. Adding a fixed epsilon does
not define a stable result at every signed crossing.

### Singularities and non-finite behavior

- `L_0+L_{2d}=0` is a pole of the exact ratio-correction formula.
- Direction and color-difference averages divide by sums of non-negative
  gradients. A relative floor can keep those denominators finite.
- The discrimination ratio requires a zero-energy convention; release 2.3
  uses `max(epsilon^2, energy)`.
- Near cancellation, no unique value follows from the original ratio alone.
  Fornax must define a derivative-free, deterministic fallback.

### Generic `f32`/`f64` arithmetic

RCD requires ordinary floating-point arithmetic and absolute values. The
source's integer coefficients and unsigned-code storage are optimizations or
range policies, not mathematical necessities. The current Fornax kernel is
generic over `Primitive`, with only `f32` and `f64` supported.

## Current Fornax behavior

The current kernel in
[`rcd.rs`](../../../crates/fornax-raw-pipeline/src/demosaic/rcd.rs):

- implements the release-2.3 directional and color-difference structure;
- removes the `[0,1]` RGB output clamp;
- keeps `[0,1]` clamps only for discrimination fractions;
- computes `epsilon` from a robust input scale and the working type;
- accepts signed and HDR values without an integer working buffer;
- passes current signed, HDR, `f64`, and positive-scale invariant checks.

The remaining gap is mathematical policy rather than public API: the code
regularizes `L_0+L_2` with a very small scale-relative epsilon but does not
explicitly define a fallback at the signed zero-crossing.

## Mandatory changes

1. Keep the directional and color-difference equations, but evaluate all
   intermediate values in the input floating-point type.
2. Replace every reference-domain `epsilon` and `epsilon^2` with an explicitly
   scale-relative tolerance derived from the input scale.
3. Define ratio correction as:
   - the original ratio when `abs(L_0+L_2)` exceeds the tolerance; and
   - a finite color-difference fallback when it does not.
4. Make the fallback positive-scale homogeneous and test it on signed fields
   whose low-pass values cross zero.
5. Preserve the discrimination bounds `[0,1]`; do not restore the RGB clamp.
6. Document the modified denominator rule as part of the Fornax RCD contract.

## Unavoidable compromises

- The exact RCD ratio has no unique extension through a signed zero
  denominator.
- A tolerance/fallback changes behavior only near cancellation but is necessary
  for a total finite function.
- The only authoritative formulation found is the author's code and README;
  there is no peer-reviewed equation paper to arbitrate ambiguous intent.
- The direction weights and empirical gradient constants remain part of RCD's
  mathematical model rather than removable numerical noise.

## Unresolved questions

- Should the zero-crossing fallback use the unratioed color-difference
  correction, the opposite-side estimate, or another documented interpolation?
- What tolerance ratio gives stable `f32` behavior without changing ordinary
  positive RAW results?
- Which independent reference can replace the removed comparison harness for
  future behavioral checks?
