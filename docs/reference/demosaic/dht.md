# Directional homogenization and Thomson-style demosaicing (DHT)

Fornax's `Demosaic::Dht` implements a clean-room, linear-light DHT
formulation in unclamped, signed, scene-linear floating-point arithmetic.

## Classification

**Done: production ready.**

## Provenance and clean-room design

The former LibRaw `dht_demosaic.cpp` port was removed in `#763` because it relied
on strictly-positive multiplicative ratios, `0.5` mid-gray padding, global
channel clamping, and LGPL-2.1/CDDL-1.0 code structure.

`Demosaic::Dht` is an original clean-room Rust implementation designed for
Fornax's scene-linear contract:

- **No `0.5` mid-gray padding or global `[min, max]` clamps:** border halos use
  parity-preserving reflection (`reflect_index`) and all arithmetic is
  unclamped.
- **No hot-pixel hide/restore pass:** hot/dead pixel suppression belongs to
  preprocessing, not linear demosaicing.
- **Scale normalization:** each image is normalized by a robust high percentile
  $s = \text{normalization\_scale}(F)$ so directional floors $\varepsilon$ are
  positive-scale equivariant.
- **Symmetric directional costs & Jacobi refinement:** out-of-place `src -> dst`
  majority-vote passes preserve exact horizontal and vertical flip symmetry,
  and the NE-SW diagonal cost evaluates the NE-SW samples directly (avoiding
  the upstream `druld` copy-paste defect).

## Algorithm

Let $F_{\mathrm{norm}} = F / s$ be the scale-normalized CFA mosaic.

### 1. Symmetric HV direction estimation and refinement

At each site $p = (r, c)$, Fornax evaluates symmetric 7-tap directional costs
along vertical ($v$) and horizontal ($h$) axes using the 1D neighbors
$f_{\pm 1}, f_{\pm 2}, f_{\pm 3}$:

$$
D_a(p) = 2 \left|(f_{-1} - f_{+1}) - \frac{1}{2}(f_{-2} - f_{+2})\right|
       + \left|2 f_0 - (f_{-2} + f_{+2})\right|
       + \frac{1}{2}\left|(f_{-1} + f_{+1}) - (f_{-3} + f_{+3})\right|,
\qquad a \in \{v, h\}.
$$

The initial HV label selects the smaller cost ($D_h < D_v \implies H$,
$D_v < D_h \implies V$, exact tie $\implies \mathrm{Tie}$) and marks the site
strong when the opposing cost exceeds $2.0 \cdot (D_{\min} + \varepsilon)$.
Three out-of-place Jacobi majority-vote passes refine non-strong pixels: two
co-directional cardinal passes followed by one isolated-pixel pass.

### 2. Sign-safe green reconstruction at R/B sites

Along each axis $a \in \{v, h\}$, the two directional green candidates
$g_m, g_p$ at $d = \pm 1$ are combined with squared inverse-gradient weights:

$$
\widehat{C}_s =
\begin{cases}
g_s \cdot \dfrac{2|c_0|}{|c_0| + |c_s|},
  & \text{if } c_0 c_s > 0,\ c_0 g_s > 0,\ \text{and } |c_0| + |c_s| > \varepsilon_{\mathrm{ratio}}, \\[8pt]
g_s + \frac{1}{2}(c_0 - c_s),
  & \text{otherwise,}
\end{cases}
\qquad
w_s = \frac{1}{(|c_0 - c_s| + \varepsilon)^2}.
$$

The final green estimate selects the vertical axis, horizontal axis, or their
average on ties.

### 3. Diagonal direction estimation and R/B reconstruction at R/B sites

Using reconstructed green, symmetric NW-SE ($P$) and NE-SW ($Q$) costs are
evaluated at R/B sites from diagonal color differences and green curvature, and
at G sites from green curvature:

$$
\begin{aligned}
D_P^{\mathrm{RB}} &= \left|(G_{NW} - C_{NW}) - (G_{SE} - C_{SE})\right| + \left|2 G_0 - (G_{NW} + G_{SE})\right|, \\
D_Q^{\mathrm{RB}} &= \left|(G_{NE} - C_{NE}) - (G_{SW} - C_{SW})\right| + \left|2 G_0 - (G_{NE} + G_{SW})\right|.
\end{aligned}
$$

Sites are marked strong when the opposing cost exceeds
$1.4 \cdot (D_{\min} + \varepsilon)$, followed by one out-of-place Jacobi
isolated-diagonal refinement pass. Missing R/B at R/B sites is reconstructed
along the selected diagonal with cubed inverse green-difference weights:

$$
C_0 = G_0 + \frac{d_2 (C_1 - G_1) + d_1 (C_2 - G_2)}{d_1 + d_2},
\qquad d_k = \left(|G_0 - G_k| + \varepsilon\right)^3.
$$

### 4. Cardinal R/B reconstruction at G sites

At G sites, missing R and B are reconstructed along the refined HV axis using
squared inverse green-difference weights $d_k = (|G_0 - G_k| + \varepsilon)^2$
over the adjacent R/B neighbors. Measured CFA samples are restored bit-exactly
on output.

## Configuration and tile geometry

`Demosaic::Dht(DhtOptions)` exposes `tile_size`:

| Option / Constant | Default | Meaning |
| --- | ---: | --- |
| `DhtOptions::tile_size` | `128` | Square tile extent including the 10-pixel halo (`even`, `>= 24`) |
| `BORDER` | `10` | Overlap discarded between interior tiles |

Tiles are processed in parallel with Rayon.

## Numeric contract and verification

- Generic over `RawFloat` (`f32` and `f64`), operating directly on signed,
  unclamped scene-linear samples with no gamma transfer or `[0, 1]` output
  clamp.
- Unit tests in [`dht.rs`](../../../crates/fornax-raw/src/demosaic/dht.rs)
  verify: fixed 24x24 RGGB `insta` snapshot, positive/zero/negative/colored
  constants, signed zero-crossing ramps, positive-scale equivariance (`f32` and
  `f64`, `1e-12` to `1e12`), sign symmetry, horizontal and vertical flip
  symmetry, tile-size invariance (`24`, `32`, `128`), bit-exact measured-sample
  preservation, and tiny-image handling (`2x2`, `3x3`, `5x5`).
- Synthetic chart, signed/HDR ROI, noisy mosaic, and slanted-edge MTF results
  are tracked in [`compare-demosaic`](../../../crates/compare-demosaic/README.md).
