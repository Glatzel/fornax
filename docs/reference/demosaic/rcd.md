# Ratio-corrected demosaicing (RCD)

Fornax's `Demosaic::Rcd` implements Luis Sanz Rodriguez's RCD 2.3 directional
discrimination and color-difference reconstruction in unclamped, signed,
scene-linear floating-point arithmetic.

## Classification

**Done: production ready.**

## Sources

- Luis Sanz Rodriguez, *RCD-Demosaicing*, release 2.3
  ([pinned commit `0aa9d6e`](https://github.com/LuisSR/RCD-Demosaicing/tree/0aa9d6e234f55adb0abf55a82be371ad32dbc60d)).

## Algorithm

Let $F(x, y)$ be the scale-normalized CFA mosaic ($F_{\mathrm{norm}} = F / s$,
where $s > 0$ is a robust high percentile of $|F|$). All tile operations run in
normalized linear coordinates and rescale by $s$ on output, preserving positive
scale equivariance and preventing `f32` overflow/underflow when squaring high-pass
responses.

### 1. Low-pass field

At each red/blue site $p = (x, y)$, RCD computes a 9-tap same-color/local
low-pass sum:

$$
l_p = F_p + \frac{1}{2}\sum_{q \in \mathcal{N}_4(p)} F_q
          + \frac{1}{4}\sum_{q \in \mathcal{N}_D(p)} F_q,
$$

where $\mathcal{N}_4(p)$ are the 4 cardinal neighbors and $\mathcal{N}_D(p)$
are the 4 diagonal neighbors. The weights sum to $4$, so $l_p = 4 L_p$ is
four times the local mean.

### 2. Vertical/horizontal directional discrimination

RCD evaluates 7-tap squared color-difference high-pass filters along vertical
($v$) and horizontal ($h$) axes:

$$
\mathrm{HP}_a(p) = \left(
  (F_{-3a} - F_{-a} - F_{a} + F_{3a})
  - 3(F_{-2a} + F_{2a})
  + 6 F_0
\right)^2,
\qquad a = v, h.
$$

Over a 3-pixel neighborhood along each axis, the directional energies are
floored symmetrically by $\varepsilon^2$:

$$
E_v = \max\left(\varepsilon^2,\ \sum_{k=-1}^{1} \mathrm{HP}_v(y+k, x)\right),
\qquad
E_h = \max\left(\varepsilon^2,\ \sum_{k=-1}^{1} \mathrm{HP}_h(y, x+k)\right),
$$

giving $\alpha_{vh} = E_v / (E_v + E_h)$. At each interpolation site, RCD
compares $\alpha_{vh}(p)$ with its 4-diagonal neighbor average
$\overline{\alpha}_{vh}(p)$, selects whichever is closer to $1/2$, and clamps
the interpolation weight $\alpha_{vh}^* \in [0, 1]$.

### 3. Sign-safe green reconstruction at red and blue sites

For each cardinal green neighbor $F_d$ ($d = N, S, W, E$), classic RCD scales
$F_d$ by the ratio $2 L_0 / (L_0 + L_{2d})$. On signed linear data, however,
$L_0 + L_{2d}$ can cross or touch zero, causing a pole where the multiplicative
ratio diverges or flips sign.

Fornax preserves the ratio correction wherever the local low-pass field has
consistent sign and falls back to the first-order additive color difference at
zero crossings:

$$
\widehat{C}_d =
\begin{cases}
F_d \cdot \dfrac{2 |l_0|}{|l_0| + |l_{2d}| + \varepsilon},
  & \text{if } l_0 \cdot l_{2d} > 0, \\[8pt]
F_d + \dfrac{l_0 - l_{2d}}{8},
  & \text{otherwise.}
\end{cases}
$$

**Why this rule:**
- When $l_0 \cdot l_{2d} > 0$, both sides share sign (covering all ordinary
  positive RAW scenes as well as uniformly negative regions). Using
  $|l_0|$ and $|l_{2d}|$ keeps the multiplier strictly bounded in $[0, 2]$,
  exact on both positive and negative constant fields, and free of sign flips.
- When $l_0 \cdot l_{2d} \le 0$, the local illumination/signal crosses or
  touches zero, so a multiplicative ratio is physically undefined. Because
  $l_p = 4 L_p$, the fallback $\frac{1}{8}(l_0 - l_{2d}) = \frac{1}{2}(L_0 - L_{2d})$
  is the exact first-order linear color-difference step, which is exact on
  signed linear ramps crossing zero, positive-scale-equivariant, and finite
  everywhere.

Cardinal candidates are combined by inverse-gradient weights $D_N, D_S, D_W, D_E$
into $\widehat{G}_v$ and $\widehat{G}_h$, then blended by $\alpha_{vh}^*$:

$$
G_p = \alpha_{vh}^* \widehat{G}_h + (1 - \alpha_{vh}^*) \widehat{G}_v.
$$

### 4. Red and blue reconstruction

1. **Diagonal P/Q stage at R/B sites:** Evaluates 7-tap diagonal high-pass
   filters at non-green sites along NW-SE ($P$) and NE-SW ($Q$) axes, forms
   the neighbor-refined weight $\alpha_{pq}^* \in [0, 1]$, and blends the
   inverse-gradient diagonal color-difference estimates $\widehat{\Phi}_P$ and
   $\widehat{\Phi}_Q$:
   $$C_p = G_p + \alpha_{pq}^* \widehat{\Phi}_Q + (1 - \alpha_{pq}^*) \widehat{\Phi}_P.$$
2. **Cardinal stage at G sites:** Reconstructs missing $R$ and $B$ from
   inverse-gradient cardinal color differences $\widehat{\Phi}_v$ and
   $\widehat{\Phi}_h$ blended by $\alpha_{vh}^*$:
   $$C_p = G_p + \alpha_{vh}^* \widehat{\Phi}_h + (1 - \alpha_{vh}^*) \widehat{\Phi}_v.$$

## Configuration and tile geometry

`Demosaic::Rcd(RcdOptions)` exposes `tile_size`:

| Option / Constant | Default | Meaning |
| --- | ---: | --- |
| `RcdOptions::tile_size` | `112` | Square tile extent including the 10-pixel halo (`even`, `>= 22`) |
| `BORDER` | `10` | Overlap discarded between interior tiles |
| `MARGIN` | `9` | Outer image border filled by `Demosaic::Ppg` |

Images smaller than `20x20` use `Demosaic::Ppg` directly. Tiles are processed in
parallel with Rayon.

## Numeric contract and verification

- Generic over `RawFloat` (`f32` and `f64`), operating directly on signed,
  unclamped scene-linear samples with no gamma transfer or `[0, 1]` output
  clamp.
- Unit tests in [`rcd.rs`](../../../crates/fornax-raw/src/demosaic/rcd.rs)
  verify: fixed 24x24 RGGB `insta` snapshot, positive/zero/negative/colored
  constants, signed zero-crossing ramps, positive-scale equivariance (`f32` and
  `f64`, `1e-12` to `1e12`), horizontal and vertical flip symmetry, tile-size
  invariance, measured-sample preservation, and row-chunk consistency.
- Synthetic chart, signed/HDR ROI, noisy mosaic, and slanted-edge MTF results
  are tracked in [`compare-demosaic`](../../../crates/compare-demosaic/README.md).
