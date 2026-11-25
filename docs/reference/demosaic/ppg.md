# Patterned pixel grouping (PPG)

Fornax's `Demosaic::Ppg` implements modern dcraw/LibRaw/darktable patterned
pixel grouping in unclamped, signed, scene-linear floating-point arithmetic.

## Classification

**Done: production ready.**

## Sources

- Alain Desbiolles, "Color interpolation (Pixel Grouping)," UFRaw discussion,
  2007:
  [author provenance thread](https://sourceforge.net/p/ufraw/discussion/434060/thread/07836bf0/).
- LibRaw 0.22.3, commit
  [`cf02d66`](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L360),
  `LibRaw::ppg_interpolate`.
- darktable 5.6.2, commit
  [`20891f6`](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee5062051773/src/iop/demosaicing/ppg.c#L20),
  `demosaic_ppg`.

Chuan-kai Lin's 2003 Pixel Grouping algorithm
([archived description](https://web.archive.org/web/20070326041737/http://web.cecs.pdx.edu/~cklin/demosaic/))
is historical context for the name; Fornax implements the modern directional
axis formulation above.

## Algorithm

### Green reconstruction at red and blue sites

For a red or blue center $F(x, y)$ at least 3 pixels from every image edge,
modern PPG evaluates two cardinal axes: horizontal ($h$) and vertical ($v$).
Along the horizontal axis ($\Delta x = 1, \Delta y = 0$) and vertical axis
($\Delta x = 0, \Delta y = 1$), let $F_k = F(x + k\,\Delta x, y + k\,\Delta y)$:

$$
\begin{aligned}
\widehat{G}_a &= 2\left(F_{-1} + F_0 + F_1\right) - F_{-2} - F_2, \\
D_a &= 3\left(|F_{-2} - F_0| + |F_2 - F_0| + |F_{-1} - F_1|\right)
     + 2\left(|F_3 - F_1| + |F_{-3} - F_{-1}|\right),
\end{aligned}
$$

for $a = h, v$. When $D_h > D_v$, PPG selects the vertical axis
($\widehat{G}_v$ with adjacent green neighbors $F_{-1}, F_1$ along the column);
otherwise it selects the horizontal axis ($\widehat{G}_h$ with adjacent green
neighbors $F_{-1}, F_1$ along the row). The estimate is bounded by those two
adjacent green samples:

$$
G_0 = \min\left(\max\left(\frac{\widehat{G}_a}{4},\ \min(F_{-1}, F_1)\right),\ \max(F_{-1}, F_1)\right).
$$

### Red and blue at green sites

At a green site at least 1 pixel from every image edge, each missing primary $C$
lies along its adjacent cardinal axis ($\pm 1$ along the row for the row's
color, $\pm 1$ along the column for the perpendicular color):

$$
C_0 = \frac{F_{-1} + F_1 + 2G_0 - G_{-1} - G_1}{2}.
$$

### Opposite primary at red and blue sites

At a red or blue center at least 1 pixel from every image edge, the missing
opposite primary $C$ is estimated along the two diagonals
($\mathrm{ne}$: $(+1,-1)/(-1,+1)$ and $\mathrm{nw}$: $(-1,-1)/(+1,+1)$):

$$
\begin{aligned}
\widehat{C}_{\mathrm{ne}} &= C_{\mathrm{ne}} + C_{\mathrm{sw}} + 2G_0 - G_{\mathrm{ne}} - G_{\mathrm{sw}}, \\
D_{\mathrm{ne}} &= |C_{\mathrm{ne}} - C_{\mathrm{sw}}| + |G_{\mathrm{ne}} - G_0| + |G_{\mathrm{sw}} - G_0|,
\end{aligned}
$$

and symmetrically for $\widehat{C}_{\mathrm{nw}}$ and $D_{\mathrm{nw}}$. If
$D_{\mathrm{ne}} > D_{\mathrm{nw}}$, PPG outputs $\widehat{C}_{\mathrm{nw}} / 2$;
if $D_{\mathrm{ne}} < D_{\mathrm{nw}}$, it outputs $\widehat{C}_{\mathrm{ne}} / 2$;
on an exact tie it outputs $(\widehat{C}_{\mathrm{ne}} + \widehat{C}_{\mathrm{nw}}) / 4$.

## Border and numerical contract

- **Minimum size:** `3x3` (`DemosaicError::InvalidInput` below `3x3`).
- **Border policy:** Green reconstruction falls back to averaging available
  in-bounds green neighbors in the $3 \times 3$ neighborhood within 3 pixels of
  any edge. Red/blue reconstruction at green and opposite-primary sites uses
  the same $3 \times 3$ neighbor average only on the outermost 1-pixel border
  ($x = 0$, $y = 0$, $x = W - 1$, or $y = H - 1$); pixels at distance 1 or 2
  from an edge still run full color-difference interpolation using the
  border-estimated green values.
- **Linear-light contract:** Generic over `RawFloat` (`f32` and `f64`). No
  gamma transfer, LUT, fixed `[0, 1]` clamp, or zero floor. All operations are
  affine/homogeneous, preserving signed values, HDR magnitudes, positive-scale
  equivariance, and additive shift invariance.
- **Implementation:** [`PpgLazy`](../../../crates/fornax-raw/src/demosaic/ppg.rs)
  evaluates rows lazily and in parallel via Rayon, caching a 1-row-padded green
  strip per chunk so each green site is computed once per strip.

## Verification

Unit tests in [`ppg.rs`](../../../crates/fornax-raw/src/demosaic/ppg.rs) cover:
- Fixed 9x9 RGGB `insta` snapshot (three decimal places on Linux x86_64)
- Uniform positive, zero, negative, and HDR constants across all 4 Bayer patterns
- Signed zero-crossing linear ramps, measured-sample preservation, and row chunking
- Positive-scale equivariance, additive shift invariance (`f32` and `f64`), and
  vertical-flip symmetry
- Synthetic chart, signed/HDR ROI, noisy mosaic, and slanted-edge MTF evaluation
  in [`compare-demosaic`](../../../crates/compare-demosaic/README.md)
