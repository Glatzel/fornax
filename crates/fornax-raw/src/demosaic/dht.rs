//! Clean-room linear-light directional homogenization and Thomson-style
//! (DHT) Bayer demosaicing.
//!
//! Operates directly on signed, unclamped scene-linear 2x2 RGB Bayer mosaics
//! using per-image scale normalization, symmetric 7-tap directional costs with
//! Jacobi majority-vote refinement, sign-safe ratio / additive zero-crossing
//! green reconstruction, and inverse-gradient additive color-difference
//! red/blue reconstruction.
//!
//! See the [DHT reference notes](https://github.com/Glatzel/fornax/blob/main/docs/reference/demosaic/dht.md).

use generic_num::num;
use rayon::prelude::*;

use super::DhtOptions;
use super::buffer::{DenseLazy, channel, dense_from_vec};
use super::numerics::normalization_scale;
use crate::error::DemosaicError;
use crate::{BayerPattern, MonoImage, RawFloat};

const BORDER: usize = 10;
const MIN_TILE_SIZE: usize = 2 * BORDER + 4;

const EPS: f64 = 1.0e-5;
const EPS_RATIO: f64 = 1.0e-6;
const HV_STRONG_RATIO: f64 = 2.0;
const DIAG_STRONG_RATIO: f64 = 1.4;

const HV_HOR: u8 = 1 << 0;
const HV_VER: u8 = 1 << 1;
const HV_STRONG: u8 = 1 << 2;
const HV_MASK: u8 = HV_HOR | HV_VER | HV_STRONG;

const DIAG_LURD: u8 = 1 << 3;
const DIAG_RULD: u8 = 1 << 4;
const DIAG_STRONG: u8 = 1 << 5;

/// Clean-room linear-light DHT Bayer demosaicer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicDht;

impl DemosaicDht {
    /// Runs DHT with default [`DhtOptions`].
    #[cfg(test)]
    pub fn lazy<T>(
        &self,
        image: &MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<DenseLazy<T>, DemosaicError>
    where
        T: RawFloat + Send + Sync,
    {
        self.lazy_with_options(image, pattern, DhtOptions::DEFAULT)
    }

    /// Runs DHT with explicit [`DhtOptions`] and validates inputs.
    pub fn lazy_with_options<T>(
        &self,
        image: &MonoImage<T>,
        pattern: BayerPattern,
        options: DhtOptions,
    ) -> Result<DenseLazy<T>, DemosaicError>
    where
        T: RawFloat + Send + Sync,
    {
        let width = image.width() as usize;
        let height = image.height() as usize;
        if width < 2 || height < 2 {
            return Err(DemosaicError::InvalidInput(
                "DHT requires an image at least 2x2".into(),
            ));
        }
        if options.tile_size < MIN_TILE_SIZE || !options.tile_size.is_multiple_of(2) {
            return Err(DemosaicError::InvalidInput(
                format!(
                    "DHT tile_size must be even and at least {MIN_TILE_SIZE}, got {}",
                    options.tile_size
                )
                .into(),
            ));
        }
        let output = render(image.as_raw(), width, height, pattern, options.tile_size);
        Ok(DenseLazy::new(dense_from_vec(
            image.width(),
            image.height(),
            output,
        )))
    }
}

struct DhtScratch<T> {
    cfa: Vec<T>,
    rgb: Vec<[T; 3]>,
    dir_a: Vec<u8>,
    dir_b: Vec<u8>,
}

impl<T: RawFloat> DhtScratch<T> {
    fn new(tile_size: usize) -> Self {
        let len = tile_size * tile_size;
        Self {
            cfa: vec![T::zero(); len],
            rgb: vec![[T::zero(); 3]; len],
            dir_a: vec![0; len],
            dir_b: vec![0; len],
        }
    }

    fn clear(&mut self) {
        self.rgb.fill([T::zero(); 3]);
        self.dir_a.fill(0);
        self.dir_b.fill(0);
    }
}

struct DhtTileOutput<T> {
    x_tile: usize,
    y_tile: usize,
    core_w: usize,
    core_h: usize,
    pixels: Vec<T>,
}

fn render<T>(
    input: &[T],
    width: usize,
    height: usize,
    pattern: BayerPattern,
    tile_size: usize,
) -> Vec<T>
where
    T: RawFloat + Send + Sync,
{
    let scale = normalization_scale(input);
    let inv_scale = T::one() / scale;
    let core = tile_size - 2 * BORDER;
    let tiles_x = width.div_ceil(core);
    let tiles_y = height.div_ceil(core);
    let tile_coords = (0..tiles_y)
        .flat_map(|ty| (0..tiles_x).map(move |tx| (tx, ty)))
        .collect::<Vec<_>>();

    let tile_outputs: Vec<DhtTileOutput<T>> = tile_coords
        .into_par_iter()
        .map_init(
            || DhtScratch::<T>::new(tile_size),
            |scratch, (tx, ty)| {
                process_tile(
                    input, width, height, pattern, tile_size, core, tx, ty, scale, inv_scale,
                    scratch,
                )
            },
        )
        .collect();

    let mut output = vec![T::zero(); width * height * 3];
    for tile in tile_outputs {
        for dy in 0..tile.core_h {
            let dst_start = ((tile.y_tile + dy) * width + tile.x_tile) * 3;
            let src_start = dy * tile.core_w * 3;
            let len = tile.core_w * 3;
            output[dst_start..dst_start + len]
                .copy_from_slice(&tile.pixels[src_start..src_start + len]);
        }
    }

    // Preserve measured CFA samples bit-exactly.
    for y in 0..height {
        let row_offset = y * width;
        for x in 0..width {
            let ch = channel(pattern, x as u32, y as u32);
            let idx = row_offset + x;
            output[idx * 3 + ch] = input[idx];
        }
    }

    output
}

#[allow(
    clippy::too_many_arguments,
    reason = "tile worker coordinates and scale factors are explicit"
)]
fn process_tile<T: RawFloat>(
    input: &[T],
    width: usize,
    height: usize,
    pattern: BayerPattern,
    tile_size: usize,
    core: usize,
    tx: usize,
    ty: usize,
    scale: T,
    inv_scale: T,
    scratch: &mut DhtScratch<T>,
) -> DhtTileOutput<T> {
    scratch.clear();
    let ts = tile_size;
    let x_tile = tx * core;
    let y_tile = ty * core;
    let core_w = core.min(width - x_tile);
    let core_h = core.min(height - y_tile);

    let rx = (0..ts)
        .map(|tc| reflect_index(x_tile as isize + tc as isize - BORDER as isize, width))
        .collect::<Vec<_>>();
    for tr in 0..ts {
        let ry = reflect_index(y_tile as isize + tr as isize - BORDER as isize, height) * width;
        let row_base = tr * ts;
        for tc in 0..ts {
            let v = input[ry + rx[tc]] * inv_scale;
            let idx = row_base + tc;
            scratch.cfa[idx] = v;
            let ch = channel(pattern, (tc & 1) as u32, (tr & 1) as u32);
            scratch.rgb[idx][ch] = v;
        }
    }

    let eps: T = num!(EPS);
    let eps_ratio: T = num!(EPS_RATIO);
    let hv_strong: T = num!(HV_STRONG_RATIO);
    let diag_strong: T = num!(DIAG_STRONG_RATIO);
    let half: T = num!(0.5);

    // Stage 1: Symmetric 7-tap HV direction estimation (3..ts - 3).
    for r in 3..ts - 3 {
        let row_base = r * ts;
        for c in 3..ts - 3 {
            let idx = row_base + c;
            let f0 = scratch.cfa[idx];

            let vm1 = scratch.cfa[(r - 1) * ts + c];
            let vp1 = scratch.cfa[(r + 1) * ts + c];
            let vm2 = scratch.cfa[(r - 2) * ts + c];
            let vp2 = scratch.cfa[(r + 2) * ts + c];
            let vm3 = scratch.cfa[(r - 3) * ts + c];
            let vp3 = scratch.cfa[(r + 3) * ts + c];
            let dv_cd = ((vm1 - vp1) - half * (vm2 - vp2)).abs();
            let dv_c = ((f0 + f0) - (vm2 + vp2)).abs();
            let dv_g = ((vm1 + vp1) - (vm3 + vp3)).abs();
            let dv = (dv_cd + dv_cd) + dv_c + half * dv_g;

            let hm1 = scratch.cfa[row_base + c - 1];
            let hp1 = scratch.cfa[row_base + c + 1];
            let hm2 = scratch.cfa[row_base + c - 2];
            let hp2 = scratch.cfa[row_base + c + 2];
            let hm3 = scratch.cfa[row_base + c - 3];
            let hp3 = scratch.cfa[row_base + c + 3];
            let dh_cd = ((hm1 - hp1) - half * (hm2 - hp2)).abs();
            let dh_c = ((f0 + f0) - (hm2 + hp2)).abs();
            let dh_g = ((hm1 + hp1) - (hm3 + hp3)).abs();
            let dh = (dh_cd + dh_cd) + dh_c + half * dh_g;

            scratch.dir_a[idx] = if dh < dv {
                HV_HOR
                    | if dv > hv_strong * (dh + eps) {
                        HV_STRONG
                    } else {
                        0
                    }
            } else if dv < dh {
                HV_VER
                    | if dh > hv_strong * (dv + eps) {
                        HV_STRONG
                    } else {
                        0
                    }
            } else {
                0
            };
        }
    }

    // 3-pass out-of-place Jacobi HV majority-vote refinement.
    refine_hv_pass(&scratch.dir_a, &mut scratch.dir_b, ts, 4, false);
    refine_hv_pass(&scratch.dir_b, &mut scratch.dir_a, ts, 5, false);
    refine_hv_pass(&scratch.dir_a, &mut scratch.dir_b, ts, 6, true);

    // Stage 2: Sign-safe green reconstruction at R/B sites (6..ts - 6).
    for r in 6..ts - 6 {
        let row_base = r * ts;
        for c in 6..ts - 6 {
            let known = channel(pattern, (c & 1) as u32, (r & 1) as u32);
            if known == 1 {
                continue;
            }
            let idx = row_base + c;
            let c0 = scratch.cfa[idx];
            let dir = scratch.dir_b[idx];
            let est = if dir & HV_VER != 0 {
                estimate_green_axis(
                    c0,
                    scratch.cfa[(r - 2) * ts + c],
                    scratch.cfa[(r + 2) * ts + c],
                    scratch.cfa[(r - 1) * ts + c],
                    scratch.cfa[(r + 1) * ts + c],
                    eps,
                    eps_ratio,
                )
            } else if dir & HV_HOR != 0 {
                estimate_green_axis(
                    c0,
                    scratch.cfa[row_base + c - 2],
                    scratch.cfa[row_base + c + 2],
                    scratch.cfa[row_base + c - 1],
                    scratch.cfa[row_base + c + 1],
                    eps,
                    eps_ratio,
                )
            } else {
                let est_v = estimate_green_axis(
                    c0,
                    scratch.cfa[(r - 2) * ts + c],
                    scratch.cfa[(r + 2) * ts + c],
                    scratch.cfa[(r - 1) * ts + c],
                    scratch.cfa[(r + 1) * ts + c],
                    eps,
                    eps_ratio,
                );
                let est_h = estimate_green_axis(
                    c0,
                    scratch.cfa[row_base + c - 2],
                    scratch.cfa[row_base + c + 2],
                    scratch.cfa[row_base + c - 1],
                    scratch.cfa[row_base + c + 1],
                    eps,
                    eps_ratio,
                );
                half * (est_v + est_h)
            };
            scratch.rgb[idx][1] = est;
        }
    }

    // Stage 3: Symmetric diagonal direction estimation (7..ts - 7).
    for r in 7..ts - 7 {
        let row_base = r * ts;
        for c in 7..ts - 7 {
            let idx = row_base + c;
            let known = channel(pattern, (c & 1) as u32, (r & 1) as u32);
            let g0 = scratch.rgb[idx][1];
            let nw = (r - 1) * ts + c - 1;
            let ne = (r - 1) * ts + c + 1;
            let sw = (r + 1) * ts + c - 1;
            let se = (r + 1) * ts + c + 1;

            let g_nw = scratch.rgb[nw][1];
            let g_ne = scratch.rgb[ne][1];
            let g_sw = scratch.rgb[sw][1];
            let g_se = scratch.rgb[se][1];

            let (dp, dq) = if known != 1 {
                let opp = 2 - known;
                let c_nw = scratch.rgb[nw][opp];
                let c_ne = scratch.rgb[ne][opp];
                let c_sw = scratch.rgb[sw][opp];
                let c_se = scratch.rgb[se][opp];
                let dp = ((g_nw - c_nw) - (g_se - c_se)).abs() + ((g0 + g0) - (g_nw + g_se)).abs();
                let dq = ((g_ne - c_ne) - (g_sw - c_sw)).abs() + ((g0 + g0) - (g_ne + g_sw)).abs();
                (dp, dq)
            } else {
                let dp = ((g0 + g0) - (g_nw + g_se)).abs();
                let dq = ((g0 + g0) - (g_ne + g_sw)).abs();
                (dp, dq)
            };

            scratch.dir_a[idx] = if dp < dq {
                DIAG_LURD
                    | if dq > diag_strong * (dp + eps) {
                        DIAG_STRONG
                    } else {
                        0
                    }
            } else if dq < dp {
                DIAG_RULD
                    | if dp > diag_strong * (dq + eps) {
                        DIAG_STRONG
                    } else {
                        0
                    }
            } else {
                0
            };
        }
    }

    // 1-pass Jacobi isolated-diagonal refinement (8..ts - 8).
    for r in 8..ts - 8 {
        let row_base = r * ts;
        for c in 8..ts - 8 {
            let idx = row_base + c;
            let cur = scratch.dir_a[idx];
            let refined = if cur & DIAG_STRONG != 0 {
                cur
            } else {
                let mut np = 0_u8;
                let mut nq = 0_u8;
                for dr in -1_isize..=1 {
                    for dc in -1_isize..=1 {
                        if dr == 0 && dc == 0 {
                            continue;
                        }
                        let nb = scratch.dir_a
                            [(r as isize + dr) as usize * ts + (c as isize + dc) as usize];
                        if nb & DIAG_LURD != 0 {
                            np += 1;
                        }
                        if nb & DIAG_RULD != 0 {
                            nq += 1;
                        }
                    }
                }
                if cur & DIAG_LURD != 0 {
                    if nq == 8 { DIAG_RULD } else { cur }
                } else if cur & DIAG_RULD != 0 {
                    if np == 8 { DIAG_LURD } else { cur }
                } else if nq >= 7 {
                    DIAG_RULD
                } else if np >= 7 {
                    DIAG_LURD
                } else {
                    0
                }
            };
            scratch.dir_b[idx] = (scratch.dir_b[idx] & HV_MASK) | refined;
        }
    }

    // Stage 4: Diagonal R/B reconstruction at R/B sites (8..ts - 8).
    for r in 8..ts - 8 {
        let row_base = r * ts;
        for c in 8..ts - 8 {
            let known = channel(pattern, (c & 1) as u32, (r & 1) as u32);
            if known == 1 {
                continue;
            }
            let opp = 2 - known;
            let idx = row_base + c;
            let g0 = scratch.rgb[idx][1];
            let dir = scratch.dir_b[idx];

            let nw = (r - 1) * ts + c - 1;
            let ne = (r - 1) * ts + c + 1;
            let sw = (r + 1) * ts + c - 1;
            let se = (r + 1) * ts + c + 1;

            let est = if dir & DIAG_LURD != 0 {
                estimate_diag_rb(
                    g0,
                    scratch.rgb[nw][1],
                    scratch.rgb[nw][opp],
                    scratch.rgb[se][1],
                    scratch.rgb[se][opp],
                    eps,
                )
            } else if dir & DIAG_RULD != 0 {
                estimate_diag_rb(
                    g0,
                    scratch.rgb[ne][1],
                    scratch.rgb[ne][opp],
                    scratch.rgb[sw][1],
                    scratch.rgb[sw][opp],
                    eps,
                )
            } else {
                let est_p = estimate_diag_rb(
                    g0,
                    scratch.rgb[nw][1],
                    scratch.rgb[nw][opp],
                    scratch.rgb[se][1],
                    scratch.rgb[se][opp],
                    eps,
                );
                let est_q = estimate_diag_rb(
                    g0,
                    scratch.rgb[ne][1],
                    scratch.rgb[ne][opp],
                    scratch.rgb[sw][1],
                    scratch.rgb[sw][opp],
                    eps,
                );
                half * (est_p + est_q)
            };
            scratch.rgb[idx][opp] = est;
        }
    }

    // Stage 5: Cardinal HV R/B reconstruction at G sites (9..ts - 9).
    for r in 9..ts - 9 {
        let row_base = r * ts;
        for c in 9..ts - 9 {
            let known = channel(pattern, (c & 1) as u32, (r & 1) as u32);
            if known != 1 {
                continue;
            }
            let idx = row_base + c;
            let g0 = scratch.rgb[idx][1];
            let dir = scratch.dir_b[idx];

            let up = scratch.rgb[(r - 1) * ts + c];
            let down = scratch.rgb[(r + 1) * ts + c];
            let left = scratch.rgb[row_base + c - 1];
            let right = scratch.rgb[row_base + c + 1];

            let (r_est, b_est) = if dir & HV_VER != 0 {
                estimate_hv_rb(g0, up, down, eps)
            } else if dir & HV_HOR != 0 {
                estimate_hv_rb(g0, left, right, eps)
            } else {
                let (rv, bv) = estimate_hv_rb(g0, up, down, eps);
                let (rh, bh) = estimate_hv_rb(g0, left, right, eps);
                (half * (rv + rh), half * (bv + bh))
            };
            scratch.rgb[idx][0] = r_est;
            scratch.rgb[idx][2] = b_est;
        }
    }

    let mut pixels = Vec::with_capacity(core_w * core_h * 3);
    for dy in 0..core_h {
        let row_base = (BORDER + dy) * ts + BORDER;
        for dx in 0..core_w {
            let px = scratch.rgb[row_base + dx];
            pixels.push(px[0] * scale);
            pixels.push(px[1] * scale);
            pixels.push(px[2] * scale);
        }
    }

    DhtTileOutput {
        x_tile,
        y_tile,
        core_w,
        core_h,
        pixels,
    }
}

fn refine_hv_pass(src: &[u8], dst: &mut [u8], ts: usize, margin: usize, isolated_only: bool) {
    for r in margin..ts - margin {
        let row_base = r * ts;
        for c in margin..ts - margin {
            let idx = row_base + c;
            let cur = src[idx];
            if cur & HV_STRONG != 0 {
                dst[idx] = cur;
                continue;
            }
            let up = src[(r - 1) * ts + c];
            let down = src[(r + 1) * ts + c];
            let left = src[row_base + c - 1];
            let right = src[row_base + c + 1];

            let nh = ((up & HV_HOR) != 0) as u8
                + ((down & HV_HOR) != 0) as u8
                + ((left & HV_HOR) != 0) as u8
                + ((right & HV_HOR) != 0) as u8;
            let nv = ((up & HV_VER) != 0) as u8
                + ((down & HV_VER) != 0) as u8
                + ((left & HV_VER) != 0) as u8
                + ((right & HV_VER) != 0) as u8;

            dst[idx] = if isolated_only {
                if cur & HV_VER != 0 {
                    if nh == 4 { HV_HOR } else { cur }
                } else if cur & HV_HOR != 0 {
                    if nv == 4 { HV_VER } else { cur }
                } else if nh >= 3 {
                    HV_HOR
                } else if nv >= 3 {
                    HV_VER
                } else {
                    0
                }
            } else if cur & HV_VER != 0 {
                let codir = (up & HV_VER != 0) || (down & HV_VER != 0);
                if nh >= 3 && !codir { HV_HOR } else { cur }
            } else if cur & HV_HOR != 0 {
                let codir = (left & HV_HOR != 0) || (right & HV_HOR != 0);
                if nv >= 3 && !codir { HV_VER } else { cur }
            } else if nh >= 3 {
                HV_HOR
            } else if nv >= 3 {
                HV_VER
            } else {
                0
            };
        }
    }
}

#[inline(always)]
fn green_candidate<T: RawFloat>(c0: T, cs: T, gs: T, eps_ratio: T) -> T {
    if c0 * cs > T::zero() && c0 * gs > T::zero() {
        let a0 = c0.abs();
        let a_s = cs.abs();
        let denom = a0 + a_s;
        if denom > eps_ratio {
            gs * ((a0 + a0) / denom)
        } else {
            gs + (c0 - cs) * num!(0.5)
        }
    } else {
        gs + (c0 - cs) * num!(0.5)
    }
}

#[inline(always)]
fn estimate_green_axis<T: RawFloat>(c0: T, cm: T, cp: T, gm: T, gp: T, eps: T, eps_ratio: T) -> T {
    let cand_m = green_candidate(c0, cm, gm, eps_ratio);
    let cand_p = green_candidate(c0, cp, gp, eps_ratio);
    let dm = square((c0 - cm).abs() + eps);
    let dp = square((c0 - cp).abs() + eps);
    (dp * cand_m + dm * cand_p) / (dm + dp)
}

#[inline(always)]
fn estimate_diag_rb<T: RawFloat>(g0: T, g1: T, c1: T, g2: T, c2: T, eps: T) -> T {
    let d1 = cube((g0 - g1).abs() + eps);
    let d2 = cube((g0 - g2).abs() + eps);
    g0 + (d2 * (c1 - g1) + d1 * (c2 - g2)) / (d1 + d2)
}

#[inline(always)]
fn estimate_hv_rb<T: RawFloat>(g0: T, s1: [T; 3], s2: [T; 3], eps: T) -> (T, T) {
    let d1 = square((g0 - s1[1]).abs() + eps);
    let d2 = square((g0 - s2[1]).abs() + eps);
    let inv = T::one() / (d1 + d2);
    let r = g0 + (d2 * (s1[0] - s1[1]) + d1 * (s2[0] - s2[1])) * inv;
    let b = g0 + (d2 * (s1[2] - s1[1]) + d1 * (s2[2] - s2[1])) * inv;
    (r, b)
}

#[inline(always)]
fn square<T: RawFloat>(v: T) -> T { v * v }

#[inline(always)]
fn cube<T: RawFloat>(v: T) -> T { v * v * v }

const fn reflect_index(index: isize, length: usize) -> usize {
    if length <= 1 {
        return 0;
    }
    let period = 2 * (length as isize - 1);
    let reflected = index.rem_euclid(period);
    if reflected < length as isize {
        reflected as usize
    } else {
        (period - reflected) as usize
    }
}

#[cfg(test)]
mod tests {
    use super::super::LazyDemosaic;
    use super::*;
    use crate::BayerChannel;

    fn seeded_mosaic<T: RawFloat>(width: usize, height: usize, seed: u64) -> Vec<T> {
        let mut state = seed;
        (0..width * height)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let unit = ((state >> 32) as f64) / f64::from(u32::MAX);
                num!(unit.mul_add(2.0, -0.5))
            })
            .collect()
    }

    fn flip_h(samples: &[f32], width: usize, height: usize) -> Vec<f32> {
        let mut out = vec![0.0_f32; width * height];
        for y in 0..height {
            for x in 0..width {
                out[y * width + (width - 1 - x)] = samples[y * width + x];
            }
        }
        out
    }

    fn flip_v(samples: &[f32], width: usize, height: usize) -> Vec<f32> {
        let mut out = vec![0.0_f32; width * height];
        for y in 0..height {
            for x in 0..width {
                out[(height - 1 - y) * width + x] = samples[y * width + x];
            }
        }
        out
    }

    fn flip_pattern_h(pattern: BayerPattern) -> BayerPattern {
        match pattern {
            BayerPattern::RGGB => BayerPattern::GRBG,
            BayerPattern::GRBG => BayerPattern::RGGB,
            BayerPattern::GBRG => BayerPattern::BGGR,
            BayerPattern::BGGR => BayerPattern::GBRG,
        }
    }

    fn flip_pattern_v(pattern: BayerPattern) -> BayerPattern {
        match pattern {
            BayerPattern::RGGB => BayerPattern::GBRG,
            BayerPattern::GBRG => BayerPattern::RGGB,
            BayerPattern::GRBG => BayerPattern::BGGR,
            BayerPattern::BGGR => BayerPattern::GRBG,
        }
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn fixed_24x24_rggb_mosaic_matches_snapshot() {
        let width = 24_usize;
        let height = 24_usize;
        let samples = seeded_mosaic::<f32>(width, height, 0x4448_545F_3234_5F31);
        let mosaic = MonoImage::from_raw(width as u32, height as u32, samples).unwrap();
        let out = DemosaicDht
            .lazy(&mosaic, BayerPattern::RGGB)
            .unwrap()
            .collect();
        insta::assert_snapshot!(
            "dht_24x24_rggb_matches_snapshot",
            super::super::test_support::format_rgb_3dp(&out)
        );
    }

    #[test]
    fn uniform_and_colored_constants_are_preserved_everywhere() {
        let width = 28_u32;
        let height = 28_u32;
        for pattern in BayerPattern::ALL {
            for &(r, g, b) in &[
                (0.25_f32, 0.25, 0.25),
                (0.0, 0.0, 0.0),
                (-0.75, -0.75, -0.75),
                (16.0, 16.0, 16.0),
                (-0.5, 1.25, 4.5),
            ] {
                let samples = (0..width * height)
                    .map(|idx| {
                        let x = idx % width;
                        let y = idx / width;
                        match pattern.channel_at(x, y) {
                            BayerChannel::R => r,
                            BayerChannel::GRedRow | BayerChannel::GBlueRow => g,
                            BayerChannel::B => b,
                        }
                    })
                    .collect::<Vec<_>>();
                let mosaic = MonoImage::from_raw(width, height, samples).unwrap();
                let out = DemosaicDht
                    .lazy_with_options(&mosaic, pattern, DhtOptions { tile_size: 24 })
                    .unwrap()
                    .collect();
                for y in 0..height {
                    for x in 0..width {
                        let [ro, go, bo] = out.get_pixel(x, y).0;
                        assert!(
                            (ro - r).abs() < 1e-5 && (go - g).abs() < 1e-5 && (bo - b).abs() < 1e-5,
                            "{pattern} at ({x},{y}): got [{ro}, {go}, {bo}] expected [{r}, {g}, {b}]"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn signed_zero_crossing_ramp_is_finite_and_accurate() {
        let width = 28_u32;
        let height = 28_u32;
        let value =
            |x: u32, y: u32| -> f32 { (x as f32).mul_add(0.025, (y as f32).mul_add(0.015, -0.45)) };
        let samples = (0..width * height)
            .map(|i| value(i % width, i / width))
            .collect::<Vec<_>>();
        let mosaic = MonoImage::from_raw(width, height, samples).unwrap();
        for pattern in BayerPattern::ALL {
            let out = DemosaicDht.lazy(&mosaic, pattern).unwrap().collect();
            for y in 2..height - 2 {
                for x in 2..width - 2 {
                    let expected = value(x, y);
                    for (ch, &actual) in out.get_pixel(x, y).0.iter().enumerate() {
                        assert!(actual.is_finite());
                        assert!(
                            (actual - expected).abs() < 0.01,
                            "{pattern} ch={ch} at ({x},{y}): {actual} vs {expected}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn positive_scale_equivariance_and_sign_symmetry_hold_in_f32_and_f64() {
        let width = 24_u32;
        let height = 24_u32;
        let base_f32 = seeded_mosaic::<f32>(width as usize, height as usize, 0x4448_545F_5343);
        let base_f64 = seeded_mosaic::<f64>(width as usize, height as usize, 0x4448_545F_5343);

        let m32 = MonoImage::from_raw(width, height, base_f32.clone()).unwrap();
        let o32 = DemosaicDht
            .lazy(&m32, BayerPattern::RGGB)
            .unwrap()
            .collect();

        for &scale in &[1.0e-12_f32, 1.0e-4, 1.0e4, 1.0e12] {
            let scaled = base_f32.iter().map(|&v| v * scale).collect();
            let ms = MonoImage::from_raw(width, height, scaled).unwrap();
            let os = DemosaicDht.lazy(&ms, BayerPattern::RGGB).unwrap().collect();
            for (a, b) in os.as_raw().iter().zip(o32.as_raw().iter()) {
                let expected = *b * scale;
                let tol = 1.0e-4 * expected.abs().max(scale);
                assert!(
                    (*a - expected).abs() <= tol,
                    "f32 scale {scale}: {a} vs {expected}"
                );
            }
        }

        let negated = base_f32.iter().map(|&v| -v).collect();
        let mn = MonoImage::from_raw(width, height, negated).unwrap();
        let on = DemosaicDht.lazy(&mn, BayerPattern::RGGB).unwrap().collect();
        for (a, b) in on.as_raw().iter().zip(o32.as_raw().iter()) {
            assert!((*a + *b).abs() <= 1.0e-5, "sign symmetry: {a} vs -{b}");
        }

        let m64 = MonoImage::from_raw(width, height, base_f64.clone()).unwrap();
        let o64 = DemosaicDht
            .lazy(&m64, BayerPattern::GBRG)
            .unwrap()
            .collect();
        let scale64 = 1.0e12_f64;
        let ms64 = MonoImage::from_raw(
            width,
            height,
            base_f64.iter().map(|&v| v * scale64).collect(),
        )
        .unwrap();
        let os64 = DemosaicDht
            .lazy(&ms64, BayerPattern::GBRG)
            .unwrap()
            .collect();
        for (a, b) in os64.as_raw().iter().zip(o64.as_raw().iter()) {
            let expected = *b * scale64;
            let tol = 1.0e-10 * expected.abs().max(scale64);
            assert!((*a - expected).abs() <= tol, "f64 scale: {a} vs {expected}");
        }
    }

    #[test]
    fn horizontal_and_vertical_flip_symmetry_hold() {
        let width = 26_usize;
        let height = 22_usize;
        let base = seeded_mosaic::<f32>(width, height, 0x4448_545F_464C);
        let h_in = flip_h(&base, width, height);
        let v_in = flip_v(&base, width, height);
        let m_base = MonoImage::from_raw(width as u32, height as u32, base).unwrap();
        let m_h = MonoImage::from_raw(width as u32, height as u32, h_in).unwrap();
        let m_v = MonoImage::from_raw(width as u32, height as u32, v_in).unwrap();

        for pattern in BayerPattern::ALL {
            let o_base = DemosaicDht
                .lazy_with_options(&m_base, pattern, DhtOptions { tile_size: 24 })
                .unwrap()
                .collect();
            let o_h = DemosaicDht
                .lazy_with_options(&m_h, flip_pattern_h(pattern), DhtOptions { tile_size: 24 })
                .unwrap()
                .collect();
            let o_v = DemosaicDht
                .lazy_with_options(&m_v, flip_pattern_v(pattern), DhtOptions { tile_size: 24 })
                .unwrap()
                .collect();

            for y in 0..height as u32 {
                for x in 0..width as u32 {
                    let pb = o_base.get_pixel(x, y).0;
                    let ph = o_h.get_pixel(width as u32 - 1 - x, y).0;
                    let pv = o_v.get_pixel(x, height as u32 - 1 - y).0;
                    for ch in 0..3 {
                        assert!(
                            (pb[ch] - ph[ch]).abs() < 2.0e-4,
                            "H-flip {pattern} ({x},{y}) ch={ch}: {} vs {}",
                            pb[ch],
                            ph[ch]
                        );
                        assert!(
                            (pb[ch] - pv[ch]).abs() < 2.0e-4,
                            "V-flip {pattern} ({x},{y}) ch={ch}: {} vs {}",
                            pb[ch],
                            pv[ch]
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn tile_size_options_are_consistent_and_validated() {
        let width = 36_u32;
        let height = 34_u32;
        let samples = seeded_mosaic::<f32>(width as usize, height as usize, 0x5449_4C45);
        let mosaic = MonoImage::from_raw(width, height, samples).unwrap();

        let out_24 = DemosaicDht
            .lazy_with_options(&mosaic, BayerPattern::RGGB, DhtOptions { tile_size: 24 })
            .unwrap()
            .collect();
        let out_32 = DemosaicDht
            .lazy_with_options(&mosaic, BayerPattern::RGGB, DhtOptions { tile_size: 32 })
            .unwrap()
            .collect();
        let out_128 = DemosaicDht
            .lazy_with_options(&mosaic, BayerPattern::RGGB, DhtOptions { tile_size: 128 })
            .unwrap()
            .collect();

        for (a, b) in out_24.as_raw().iter().zip(out_128.as_raw().iter()) {
            assert!((*a - *b).abs() < 1e-6, "tile_size 24 vs 128: {a} vs {b}");
        }
        for (a, b) in out_32.as_raw().iter().zip(out_128.as_raw().iter()) {
            assert!((*a - *b).abs() < 1e-6, "tile_size 32 vs 128: {a} vs {b}");
        }

        for bad_tile in [22_usize, 25, 0] {
            assert!(matches!(
                DemosaicDht.lazy_with_options(
                    &mosaic,
                    BayerPattern::RGGB,
                    DhtOptions {
                        tile_size: bad_tile
                    }
                ),
                Err(DemosaicError::InvalidInput(_))
            ));
        }
        let tiny = MonoImage::from_raw(1, 8, vec![0.0_f32; 8]).unwrap();
        assert!(matches!(
            DemosaicDht.lazy(&tiny, BayerPattern::RGGB),
            Err(DemosaicError::InvalidInput(_))
        ));
    }

    #[test]
    fn measured_bayer_samples_are_preserved_bit_exactly() {
        let width = 20_u32;
        let height = 20_u32;
        let samples = seeded_mosaic::<f32>(width as usize, height as usize, 0x4249_5458);
        let mosaic = MonoImage::from_raw(width, height, samples).unwrap();
        for pattern in BayerPattern::ALL {
            let out = DemosaicDht.lazy(&mosaic, pattern).unwrap().collect();
            for y in 0..height {
                for x in 0..width {
                    let ch = channel(pattern, x, y);
                    assert_eq!(
                        out.get_pixel(x, y)[ch].to_bits(),
                        mosaic.get_pixel(x, y)[0].to_bits()
                    );
                }
            }
        }
    }

    #[test]
    fn tiny_images_produce_finite_accurate_output() {
        for (w, h) in [(2_u32, 2_u32), (3, 3), (5, 5)] {
            let samples = vec![0.5_f32; (w * h) as usize];
            let mosaic = MonoImage::from_raw(w, h, samples).unwrap();
            let out = DemosaicDht
                .lazy(&mosaic, BayerPattern::GBRG)
                .unwrap()
                .collect();
            assert!(out.as_raw().iter().all(|&v| (v - 0.5).abs() < 1e-6));
        }
    }
}
