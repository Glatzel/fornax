# Variable-number-of-gradients demosaicing (VNG and VNG4)

Fornax provides two linear-light VNG selectors sharing one parameterized
streaming kernel in [`vng.rs`](../../../crates/fornax-raw/src/demosaic/vng.rs):

- `Demosaic::Vng`: **3-color VNG** with a unified green channel (`R, G, B`).
  Preserves all measured `R`, `G`, and `B` CFA samples bit-exactly and delivers
  higher sharpness on green-balanced sensors.
- `Demosaic::Vng4`: **4-color split-green VNG4** with separate `G1` (`GRedRow`)
  and `G2` (`GBlueRow`) planes averaged on output as $(G_1 + G_2) / 2$ to
  suppress maze artifacts under green-channel imbalance.

## Classification

**Done: production ready.**

## Sources

- Ting Chen, *Interpolation using a Threshold-based variable number of
  gradients*, Stanford EE392J project report (1999):
  [original VNG description](http://scien.stanford.edu/pages/labsite/1999/psych221/projects/99/tingchen/algodep/vargra.html).
- darktable 5.6.2, `vng_interpolate`
  ([`src/iop/demosaicing/vng.c`](https://github.com/darktable-org/darktable/blob/20891f6c6fa7be995e5bf9dff9d1ee05062051773/src/iop/demosaicing/vng.c#L171)):
  behavioral reference for the 4-color split-green convention.
- LibRaw 0.22.3, `vng_interpolate`
  ([`src/demosaic/misc_demosaic.cpp`](https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L199)):
  behavioral reference for 3-color vs `four_color_rgb = 1` modes.

darktable (GPL) and LibRaw (LGPL/CDDL) are used strictly as behavioral
specifications; Fornax's `vng.rs` is an original generic floating-point
implementation.

## Algorithm

### 1. Parity-preserving linear seed strip

For each output row strip of height `chunk_rows`, `VngLazy` allocates a
$(chunk\_rows + 4) \times (width + 4)$ scratch strip using parity-preserving
reflection (`reflect_index`) at all four image boundaries. Every strip pixel is
initialized by closed-form bilinear interpolation over the reflected CFA:

- **3-color (`Vng`):** `R = 0`, `G1 = G2 = 1`, `B = 2` (544 phase terms and 96
  alternate neighbor offsets across 16 phase templates).
- **4-color (`Vng4`):** `R = 0`, `G1 = 1`, `B = 2`, `G2 = 3` (512 phase terms
  and 128 alternate neighbor offsets across 16 phase templates).

### 2. Eight-direction gradient scoring and threshold

At each pixel, the 8 directional gradients $G_0, \dots, G_7$ (NW through W) are
accumulated from absolute differences of same-channel neighbors in the $5 \times 5$
window, with weights $w \in \{1, 2\}$:

$$
G_k = \sum_{(p, q) \in \mathcal{T}_k} w_{p,q} \left| I_c(p) - I_c(q) \right|.
$$

Let $G_{\min} = \min_k G_k$ and $G_{\max} = \max_k G_k$. If $G_{\max} = 0$, the
linear seed is emitted directly; otherwise, directions satisfying

$$
G_k \le T = G_{\min} + \frac{1}{2} G_{\max}
$$

are selected into the active neighborhood $\mathcal{S}$ ($N = |\mathcal{S}|$).

### 3. Color-difference reconstruction

For the center pixel with measured CFA channel $c_0$ and value $I_{c_0}(0)$,
each non-measured channel $c \ne c_0$ is reconstructed from the mean color
difference over $\mathcal{S}$:

$$
\widehat{I}_c(0) = I_{c_0}(0) + \frac{1}{N} \sum_{k \in \mathcal{S}}
  \left( I_c(d_k) - \tilde{I}_{c_0}(d_k) \right),
$$

where $\tilde{I}_{c_0}(d_k)$ uses the midpoint $\frac{1}{2}(I_{c_0}(0) + I_{c_0}(2 d_k))$
when the neighbor at $d_k$ is a different CFA phase and $2 d_k$ shares $c_0$.

- **`Demosaic::Vng`:** outputs $[\widehat{R}, \widehat{G}, \widehat{B}]$,
  preserving all measured `R`, `G`, and `B` samples bit-exactly.
- **`Demosaic::Vng4`:** outputs
  $[\widehat{R}, \frac{1}{2}(\widehat{G}_1 + \widehat{G}_2), \widehat{B}]$,
  preserving measured `R` and `B` bit-exactly while averaging `G1` and `G2`.

## Numeric contract and verification

- Generic over `RawFloat` (`f32` and `f64`), operating directly on signed,
  unclamped scene-linear samples with exact positive-scale, shift, and sign
  equivariance and no `[0, 1]` output clamp.
- Streaming `VngLazy` execution integrates with `LazyDemosaic::collect_with` for
  Rayon row-strip parallelism with $O(\text{width})$ per-task memory.
- Unit tests in [`vng.rs`](../../../crates/fornax-raw/src/demosaic/vng.rs)
  verify: fixed 16x16 RGGB `insta` snapshots for both `Vng` and `Vng4`, 3-color
  vs 4-color phase plan counts (`544`/`96` vs `512`/`128`), unbalanced-green
  distinction and measured-sample preservation, positive/zero/negative/colored
  constants, exact signed zero-crossing affine ramps, scale (`1e-12` to `1e12`),
  shift, and sign invariance (`f32` and `f64`), horizontal and vertical flip
  symmetry across all four Bayer patterns, and tiny-image/row-chunking
  consistency (`2x2`, `3x3`, `5x7`).
- Synthetic chart, signed/HDR ROI, noisy mosaic, and slanted-edge MTF results
  are tracked in [`compare-demosaic`](../../../crates/compare-demosaic/README.md).
