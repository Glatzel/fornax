//! Ratio-corrected demosaicing (RCD) in sensor-linear space.
//!
//! Independently structured implementation following Luis Sanz Rodriguez's
//! RCD 2.3 directional discrimination and color-difference formulation, adapted
//! for signed, unclamped, scale-equivariant linear Bayer mosaics.

use std::sync::Mutex;

use generic_num::num;
use rayon::prelude::*;

use super::numerics::normalization_scale;
use super::ppg::DemosaicPpg;
use super::protocol::apply_post_in_place;
use super::{LazyDemosaic, RcdOptions};
use crate::error::DemosaicError;
use crate::{BayerPattern, MonoImage, RawFloat, RgbImage};

const BORDER: usize = 10;
const MARGIN: usize = 9;
const MIN_TILE_SIZE: usize = 2 * BORDER + 2;

pub struct RcdLazy<'a, T: RawFloat> {
    bayer: &'a MonoImage<T>,
    pattern: BayerPattern,
    options: RcdOptions,
    image: Mutex<Option<RgbImage<T>>>,
}

impl<T> LazyDemosaic<T> for RcdLazy<'_, T>
where
    T: RawFloat + Send + Sync,
{
    fn dimensions(&self) -> (u32, u32) { self.bayer.dimensions() }

    fn fill_rows(&self, y0: usize, out: &mut [T]) {
        let mut cache = self
            .image
            .lock()
            .expect("RCD image cache mutex should not be poisoned");
        let image = cache.get_or_insert_with(|| {
            render(self.bayer, self.pattern, self.options)
                .expect("RCD lazy construction already validated the input")
        });
        let row_stride = image.width() as usize * 3;
        let start = y0 * row_stride;
        let end = start + out.len();
        out.copy_from_slice(&image.as_raw()[start..end]);
    }

    fn collect_with(&self, post: impl Fn(usize, &mut [T]) + Sync) -> RgbImage<T> {
        let mut image = self
            .image
            .lock()
            .expect("RCD image cache mutex should not be poisoned")
            .take()
            .unwrap_or_else(|| {
                render(self.bayer, self.pattern, self.options)
                    .expect("RCD lazy construction already validated the input")
            });
        apply_post_in_place(&mut image, post);
        image
    }
}

impl<T: RawFloat> RcdLazy<'_, T> {
    #[cfg(test)]
    fn cached_image_is_absent(&self) -> bool {
        self.image
            .lock()
            .expect("RCD image cache mutex should not be poisoned")
            .is_none()
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicRcd;

impl DemosaicRcd {
    #[cfg(test)]
    pub fn lazy<'a, T>(
        &self,
        bayer: &'a MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<RcdLazy<'a, T>, DemosaicError>
    where
        T: RawFloat + Send + Sync,
    {
        self.lazy_with_options(bayer, pattern, RcdOptions::DEFAULT)
    }

    pub fn lazy_with_options<'a, T>(
        &self,
        bayer: &'a MonoImage<T>,
        pattern: BayerPattern,
        options: RcdOptions,
    ) -> Result<RcdLazy<'a, T>, DemosaicError>
    where
        T: RawFloat + Send + Sync,
    {
        if options.tile_size < MIN_TILE_SIZE || !options.tile_size.is_multiple_of(2) {
            return Err(DemosaicError::InvalidInput(
                format!(
                    "RCD tile_size must be even and at least {MIN_TILE_SIZE}, got {}",
                    options.tile_size
                )
                .into(),
            ));
        }
        let image = render(bayer, pattern, options)?;
        Ok(RcdLazy {
            bayer,
            pattern,
            options,
            image: Mutex::new(Some(image)),
        })
    }
}

fn render<T>(
    bayer: &MonoImage<T>,
    pattern: BayerPattern,
    options: RcdOptions,
) -> Result<RgbImage<T>, DemosaicError>
where
    T: RawFloat + Send + Sync,
{
    let mut image = DemosaicPpg.lazy(bayer, pattern)?.collect();
    if image.width() >= (2 * BORDER) as u32 && image.height() >= (2 * BORDER) as u32 {
        process_image(
            bayer.as_raw(),
            image.as_mut(),
            bayer.width() as usize,
            bayer.height() as usize,
            pattern,
            options.tile_size,
        );
    }
    Ok(image)
}

struct RcdScratch<T> {
    cfa: Vec<T>,
    vh: Vec<T>,
    lpf: Vec<T>,
    pq: Vec<T>,
    p_hpf: Vec<T>,
    q_hpf: Vec<T>,
    rgb: Vec<[T; 3]>,
    vertical_hp: Vec<T>,
    horizontal_hp: Vec<T>,
}

impl<T: RawFloat> RcdScratch<T> {
    fn new(tile_size: usize) -> Self {
        let tile_area = tile_size * tile_size;
        Self {
            cfa: vec![T::zero(); tile_area],
            vh: vec![T::zero(); tile_area],
            lpf: vec![T::zero(); tile_area],
            pq: vec![T::zero(); tile_area],
            p_hpf: vec![T::zero(); tile_area],
            q_hpf: vec![T::zero(); tile_area],
            rgb: vec![[T::zero(); 3]; tile_area],
            vertical_hp: vec![T::zero(); 3 * (tile_size - 8)],
            horizontal_hp: vec![T::zero(); tile_size],
        }
    }

    fn clear(&mut self) {
        self.cfa.fill(T::zero());
        self.vh.fill(T::zero());
        self.lpf.fill(T::zero());
        self.pq.fill(T::zero());
        self.p_hpf.fill(T::zero());
        self.q_hpf.fill(T::zero());
        self.rgb.fill([T::zero(); 3]);
        self.vertical_hp.fill(T::zero());
        self.horizontal_hp.fill(T::zero());
    }
}

struct TileCoreOutput<T> {
    first_row: usize,
    last_row: usize,
    first_col: usize,
    last_col: usize,
    pixels: Vec<T>,
}

fn process_image<T>(
    raw: &[T],
    output: &mut [T],
    width: usize,
    height: usize,
    pattern: BayerPattern,
    tile_size: usize,
) where
    T: RawFloat + Send + Sync,
{
    let tile_core = tile_size - 2 * BORDER;
    let vertical_tiles = 1 + height.saturating_sub(2 * BORDER + 1) / tile_core;
    let horizontal_tiles = 1 + width.saturating_sub(2 * BORDER + 1) / tile_core;

    let signal_scale = normalization_scale(raw);
    let inv_scale = T::one() / signal_scale;

    let coords: Vec<(usize, usize)> = (0..vertical_tiles)
        .flat_map(|ty| (0..horizontal_tiles).map(move |tx| (ty, tx)))
        .collect();

    let cores: Vec<TileCoreOutput<T>> = coords
        .into_par_iter()
        .map_init(
            || RcdScratch::new(tile_size),
            |scratch, (tile_y, tile_x)| {
                process_tile(
                    scratch,
                    raw,
                    width,
                    height,
                    pattern,
                    tile_size,
                    tile_core,
                    vertical_tiles,
                    horizontal_tiles,
                    tile_y,
                    tile_x,
                    signal_scale,
                    inv_scale,
                )
            },
        )
        .collect();

    for core in cores {
        let core_cols = core.last_col - core.first_col;
        for (dy, row) in (core.first_row..core.last_row).enumerate() {
            let src_start = dy * core_cols * 3;
            let dst_start = (row * width + core.first_col) * 3;
            output[dst_start..dst_start + core_cols * 3]
                .copy_from_slice(&core.pixels[src_start..src_start + core_cols * 3]);
        }
    }
}

fn process_tile<T>(
    scratch: &mut RcdScratch<T>,
    raw: &[T],
    width: usize,
    height: usize,
    pattern: BayerPattern,
    tile_size: usize,
    tile_core: usize,
    vertical_tiles: usize,
    horizontal_tiles: usize,
    tile_y: usize,
    tile_x: usize,
    signal_scale: T,
    inv_scale: T,
) -> TileCoreOutput<T>
where
    T: RawFloat,
{
    scratch.clear();
    let cfa = &mut scratch.cfa;
    let vh = &mut scratch.vh;
    let lpf = &mut scratch.lpf;
    let pq = &mut scratch.pq;
    let p_hpf = &mut scratch.p_hpf;
    let q_hpf = &mut scratch.q_hpf;
    let rgb = &mut scratch.rgb;
    let vertical_hp = &mut scratch.vertical_hp;
    let horizontal_hp = &mut scratch.horizontal_hp;

    let eps = T::epsilon();
    let epssq = eps * eps;
    let half: T = num!(0.5);
    let quarter: T = num!(0.25);
    let three: T = num!(3);
    let six: T = num!(6);

    let row_start = tile_y * tile_core;
    let col_start = tile_x * tile_core;
    let row_end = (row_start + tile_size).min(height);
    let col_end = (col_start + tile_size).min(width);
    let tile_rows = row_end - row_start;
    let tile_cols = col_end - col_start;

    for row in 0..tile_rows {
        let global_y = row_start + row;
        let first = color_index(pattern.channel_at(col_start as u32, global_y as u32));
        let second = color_index(pattern.channel_at((col_start + 1) as u32, global_y as u32));
        for col in 0..tile_cols {
            let index = row * tile_size + col;
            let value = raw[global_y * width + col_start + col] * inv_scale;
            cfa[index] = value;
            rgb[index][first] = value;
            rgb[index][second] = value;
        }
    }

    let vhp_stride = tile_size - 8;
    for row in 3..tile_rows.saturating_sub(3).min(5) {
        let base = (row - 3) * vhp_stride;
        for col in 4..tile_cols.saturating_sub(4) {
            let index = row * tile_size + col;
            vertical_hp[base + col - 4] = square(
                (cfa[index - 3 * tile_size] - cfa[index - tile_size] - cfa[index + tile_size]
                    + cfa[index + 3 * tile_size])
                    - three * (cfa[index - 2 * tile_size] + cfa[index + 2 * tile_size])
                    + six * cfa[index],
            );
        }
    }

    for row in 4..tile_rows.saturating_sub(4) {
        for col in 3..tile_cols.saturating_sub(3) {
            let index = row * tile_size + col;
            horizontal_hp[col - 3] = square(
                (cfa[index - 3] - cfa[index - 1] - cfa[index + 1] + cfa[index + 3])
                    - three * (cfa[index - 2] + cfa[index + 2])
                    + six * cfa[index],
            );
        }
        for col in 4..tile_cols.saturating_sub(4) {
            let index = (row + 1) * tile_size + col;
            let slot = (row + 1 - 3) % 3;
            vertical_hp[slot * vhp_stride + col - 4] = square(
                (cfa[index - 3 * tile_size] - cfa[index - tile_size] - cfa[index + tile_size]
                    + cfa[index + 3 * tile_size])
                    - three * (cfa[index - 2 * tile_size] + cfa[index + 2 * tile_size])
                    + six * cfa[index],
            );
        }
        let v0 = (row - 4) % 3 * vhp_stride;
        let v1 = (row - 3) % 3 * vhp_stride;
        let v2 = (row - 2) % 3 * vhp_stride;
        for col in 4..tile_cols.saturating_sub(4) {
            let index = row * tile_size + col;
            let vertical = epssq.max(
                vertical_hp[v0 + col - 4] + vertical_hp[v1 + col - 4] + vertical_hp[v2 + col - 4],
            );
            let horizontal =
                epssq.max(horizontal_hp[col - 4] + horizontal_hp[col - 3] + horizontal_hp[col - 2]);
            vh[index] = vertical / (vertical + horizontal);
        }
    }

    for row in 2..tile_rows.saturating_sub(2) {
        let global_y = row_start + row;
        let start = 2
            + (pattern
                .channel_at(col_start as u32, global_y as u32)
                .index()
                & 1);
        for col in (start..tile_cols.saturating_sub(2)).step_by(2) {
            let index = row * tile_size + col;
            lpf[index] = cfa[index]
                + half
                    * (cfa[index - tile_size]
                        + cfa[index + tile_size]
                        + cfa[index - 1]
                        + cfa[index + 1])
                + quarter
                    * (cfa[index - tile_size - 1]
                        + cfa[index - tile_size + 1]
                        + cfa[index + tile_size - 1]
                        + cfa[index + tile_size + 1]);
        }
    }

    for row in 4..tile_rows.saturating_sub(4) {
        let global_y = row_start + row;
        let start = 4
            + (pattern
                .channel_at(col_start as u32, global_y as u32)
                .index()
                & 1);
        for col in (start..tile_cols.saturating_sub(4)).step_by(2) {
            let index = row * tile_size + col;
            let center = cfa[index];
            let north_gradient = eps
                + (cfa[index - tile_size] - cfa[index + tile_size]).abs()
                + (center - cfa[index - 2 * tile_size]).abs()
                + (cfa[index - tile_size] - cfa[index - 3 * tile_size]).abs()
                + (cfa[index - 2 * tile_size] - cfa[index - 4 * tile_size]).abs();
            let south_gradient = eps
                + (cfa[index + tile_size] - cfa[index - tile_size]).abs()
                + (center - cfa[index + 2 * tile_size]).abs()
                + (cfa[index + tile_size] - cfa[index + 3 * tile_size]).abs()
                + (cfa[index + 2 * tile_size] - cfa[index + 4 * tile_size]).abs();
            let west_gradient = eps
                + (cfa[index - 1] - cfa[index + 1]).abs()
                + (center - cfa[index - 2]).abs()
                + (cfa[index - 1] - cfa[index - 3]).abs()
                + (cfa[index - 2] - cfa[index - 4]).abs();
            let east_gradient = eps
                + (cfa[index + 1] - cfa[index - 1]).abs()
                + (center - cfa[index + 2]).abs()
                + (cfa[index + 1] - cfa[index + 3]).abs()
                + (cfa[index + 2] - cfa[index + 4]).abs();

            let local_low = lpf[index];
            let north = ratio_corrected_green(
                cfa[index - tile_size],
                local_low,
                lpf[index - 2 * tile_size],
                eps,
            );
            let south = ratio_corrected_green(
                cfa[index + tile_size],
                local_low,
                lpf[index + 2 * tile_size],
                eps,
            );
            let west = ratio_corrected_green(cfa[index - 1], local_low, lpf[index - 2], eps);
            let east = ratio_corrected_green(cfa[index + 1], local_low, lpf[index + 2], eps);

            let vertical = (south_gradient * north + north_gradient * south)
                / (north_gradient + south_gradient);
            let horizontal =
                (west_gradient * east + east_gradient * west) / (east_gradient + west_gradient);
            let central = vh[index];
            let neighbours = quarter
                * (vh[index - tile_size - 1]
                    + vh[index - tile_size + 1]
                    + vh[index + tile_size - 1]
                    + vh[index + tile_size + 1]);
            let discrimination = if (half - central).abs() < (half - neighbours).abs() {
                neighbours
            } else {
                central
            };
            rgb[index][1] = interpolate(
                discrimination.max(T::zero()).min(T::one()),
                horizontal,
                vertical,
            );
        }
    }

    for row in 3..tile_rows.saturating_sub(3) {
        let global_y = row_start + row;
        let start = 3
            + (pattern
                .channel_at((col_start + 3) as u32, global_y as u32)
                .index()
                & 1);
        for col in (start..tile_cols.saturating_sub(3)).step_by(2) {
            let index = row * tile_size + col;
            p_hpf[index] = square(
                (cfa[index - 3 * tile_size - 3]
                    - cfa[index - tile_size - 1]
                    - cfa[index + tile_size + 1]
                    + cfa[index + 3 * tile_size + 3])
                    - three * (cfa[index - 2 * tile_size - 2] + cfa[index + 2 * tile_size + 2])
                    + six * cfa[index],
            );
            q_hpf[index] = square(
                (cfa[index - 3 * tile_size + 3]
                    - cfa[index - tile_size + 1]
                    - cfa[index + tile_size - 1]
                    + cfa[index + 3 * tile_size - 3])
                    - three * (cfa[index - 2 * tile_size + 2] + cfa[index + 2 * tile_size - 2])
                    + six * cfa[index],
            );
        }
    }

    for row in 4..tile_rows.saturating_sub(4) {
        let global_y = row_start + row;
        let start = 4
            + (pattern
                .channel_at(col_start as u32, global_y as u32)
                .index()
                & 1);
        for col in (start..tile_cols.saturating_sub(4)).step_by(2) {
            let index = row * tile_size + col;
            let p_stat = epssq
                .max(p_hpf[index - tile_size - 1] + p_hpf[index] + p_hpf[index + tile_size + 1]);
            let q_stat = epssq
                .max(q_hpf[index - tile_size + 1] + q_hpf[index] + q_hpf[index + tile_size - 1]);
            pq[index] = p_stat / (p_stat + q_stat);
        }
    }

    for row in 4..tile_rows.saturating_sub(4) {
        let global_y = row_start + row;
        let start = 4
            + (pattern
                .channel_at(col_start as u32, global_y as u32)
                .index()
                & 1);
        for col in (start..tile_cols.saturating_sub(4)).step_by(2) {
            let index = row * tile_size + col;
            let c = 2 - pattern
                .channel_at((col_start + col) as u32, global_y as u32)
                .index();
            let northwest = index - tile_size - 1;
            let northeast = index - tile_size + 1;
            let southwest = index + tile_size - 1;
            let southeast = index + tile_size + 1;
            let pq_central = pq[index];
            let pq_neighbours =
                quarter * (pq[northwest] + pq[northeast] + pq[southwest] + pq[southeast]);
            let discrimination = if (half - pq_central).abs() < (half - pq_neighbours).abs() {
                pq_neighbours
            } else {
                pq_central
            };

            let nw_gradient = eps
                + (rgb[northwest][c] - rgb[southeast][c]).abs()
                + (rgb[northwest][c] - rgb[index - 3 * tile_size - 3][c]).abs()
                + (rgb[index][1] - rgb[index - 2 * tile_size - 2][1]).abs();
            let ne_gradient = eps
                + (rgb[northeast][c] - rgb[southwest][c]).abs()
                + (rgb[northeast][c] - rgb[index - 3 * tile_size + 3][c]).abs()
                + (rgb[index][1] - rgb[index - 2 * tile_size + 2][1]).abs();
            let sw_gradient = eps
                + (rgb[northeast][c] - rgb[southwest][c]).abs()
                + (rgb[southwest][c] - rgb[index + 3 * tile_size - 3][c]).abs()
                + (rgb[index][1] - rgb[index + 2 * tile_size - 2][1]).abs();
            let se_gradient = eps
                + (rgb[northwest][c] - rgb[southeast][c]).abs()
                + (rgb[southeast][c] - rgb[index + 3 * tile_size + 3][c]).abs()
                + (rgb[index][1] - rgb[index + 2 * tile_size + 2][1]).abs();
            let nw_estimate = rgb[northwest][c] - rgb[northwest][1];
            let ne_estimate = rgb[northeast][c] - rgb[northeast][1];
            let sw_estimate = rgb[southwest][c] - rgb[southwest][1];
            let se_estimate = rgb[southeast][c] - rgb[southeast][1];
            let p_estimate = (nw_gradient * se_estimate + se_gradient * nw_estimate)
                / (nw_gradient + se_gradient);
            let q_estimate = (ne_gradient * sw_estimate + sw_gradient * ne_estimate)
                / (ne_gradient + sw_gradient);
            rgb[index][c] = rgb[index][1]
                + interpolate(
                    discrimination.max(T::zero()).min(T::one()),
                    q_estimate,
                    p_estimate,
                );
        }
    }

    for row in 4..tile_rows.saturating_sub(4) {
        let global_y = row_start + row;
        let start = 4
            + (pattern
                .channel_at((col_start + 1) as u32, global_y as u32)
                .index()
                & 1);
        for col in (start..tile_cols.saturating_sub(4)).step_by(2) {
            let index = row * tile_size + col;
            let central = vh[index];
            let neighbours = quarter
                * (vh[index - tile_size - 1]
                    + vh[index - tile_size + 1]
                    + vh[index + tile_size - 1]
                    + vh[index + tile_size + 1]);
            let discrimination = if (half - central).abs() < (half - neighbours).abs() {
                neighbours
            } else {
                central
            }
            .max(T::zero())
            .min(T::one());
            let green = rgb[index][1];
            let north = eps + (green - rgb[index - 2 * tile_size][1]).abs();
            let south = eps + (green - rgb[index + 2 * tile_size][1]).abs();
            let west = eps + (green - rgb[index - 2][1]).abs();
            let east = eps + (green - rgb[index + 2][1]).abs();
            for channel in [0, 2] {
                let north_south =
                    (rgb[index - tile_size][channel] - rgb[index + tile_size][channel]).abs();
                let east_west = (rgb[index - 1][channel] - rgb[index + 1][channel]).abs();
                let north_gradient = north
                    + north_south
                    + (rgb[index - tile_size][channel] - rgb[index - 3 * tile_size][channel]).abs();
                let south_gradient = south
                    + north_south
                    + (rgb[index + tile_size][channel] - rgb[index + 3 * tile_size][channel]).abs();
                let west_gradient =
                    west + east_west + (rgb[index - 1][channel] - rgb[index - 3][channel]).abs();
                let east_gradient =
                    east + east_west + (rgb[index + 1][channel] - rgb[index + 3][channel]).abs();
                let north_estimate = rgb[index - tile_size][channel] - rgb[index - tile_size][1];
                let south_estimate = rgb[index + tile_size][channel] - rgb[index + tile_size][1];
                let west_estimate = rgb[index - 1][channel] - rgb[index - 1][1];
                let east_estimate = rgb[index + 1][channel] - rgb[index + 1][1];
                let vertical = (north_gradient * south_estimate + south_gradient * north_estimate)
                    / (north_gradient + south_gradient);
                let horizontal = (east_gradient * west_estimate + west_gradient * east_estimate)
                    / (east_gradient + west_gradient);
                rgb[index][channel] = green + interpolate(discrimination, horizontal, vertical);
            }
        }
    }

    let first_row = row_start + if tile_y == 0 { MARGIN } else { BORDER };
    let last_row = row_end
        - if tile_y + 1 == vertical_tiles {
            MARGIN
        } else {
            BORDER
        };
    let first_col = col_start + if tile_x == 0 { MARGIN } else { BORDER };
    let last_col = col_end
        - if tile_x + 1 == horizontal_tiles {
            MARGIN
        } else {
            BORDER
        };
    let mut pixels = Vec::with_capacity((last_row - first_row) * (last_col - first_col) * 3);
    for row in first_row..last_row {
        for col in first_col..last_col {
            let index = (row - row_start) * tile_size + col - col_start;
            let [r, g, b] = rgb[index];
            let mut out_rgb = [r * signal_scale, g * signal_scale, b * signal_scale];
            out_rgb[color_index(pattern.channel_at(col as u32, row as u32))] =
                raw[row * width + col];
            pixels.extend_from_slice(&out_rgb);
        }
    }

    TileCoreOutput {
        first_row,
        last_row,
        first_col,
        last_col,
        pixels,
    }
}

/// Sign-safe ratio correction for green at a red/blue site.
///
/// When `l0` and `l2` have the same sign (`l0 * l2 > 0`), uses the bounded
/// multiplicative ratio `2 * |l0| / (|l0| + |l2| + eps)` (identical to
/// classic RCD for positive data and symmetric for negative fields). When `l0`
/// and `l2` cross or touch zero (`l0 * l2 <= 0`), falls back to the
/// first-order additive color difference `green_neighbor + (l0 - l2) / 8`
/// (`lpf` weights sum to 4, so `1 / 8` corresponds to `(L_0 - L_2) / 2`).
#[inline(always)]
fn ratio_corrected_green<T>(green_neighbor: T, l0: T, l2: T, eps: T) -> T
where
    T: RawFloat,
{
    if l0 * l2 > T::zero() {
        let a0 = l0.abs();
        let a2 = l2.abs();
        green_neighbor * (a0 + a0) / (a0 + a2 + eps)
    } else {
        green_neighbor + (l0 - l2) * num!(0.125)
    }
}

#[inline(always)]
fn square<T>(value: T) -> T
where
    T: RawFloat,
{
    value * value
}

const fn color_index(channel: crate::BayerChannel) -> usize {
    match channel {
        crate::BayerChannel::R => 0,
        crate::BayerChannel::GRedRow | crate::BayerChannel::GBlueRow => 1,
        crate::BayerChannel::B => 2,
    }
}

#[inline(always)]
fn interpolate<T>(weight: T, first: T, second: T) -> T
where
    T: RawFloat,
{
    weight * (first - second) + second
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::{BayerChannel, BayerPattern};

    fn color(channel: BayerChannel) -> usize {
        match channel {
            BayerChannel::R => 0,
            BayerChannel::GRedRow | BayerChannel::GBlueRow => 1,
            BayerChannel::B => 2,
        }
    }

    fn seeded_mosaic<T>(width: u32, height: u32, offset: f64, scale: f64) -> MonoImage<T>
    where
        T: RawFloat,
    {
        let samples = (0..width * height)
            .map(|i| {
                let h = i.wrapping_mul(0x9e37_79b9).rotate_left(13) ^ i.wrapping_mul(0x85eb_ca6b);
                let u = f64::from((h ^ (h >> 16)) & 0xffff) / 65535.0;
                num!(scale.mul_add(u, offset))
            })
            .collect();
        MonoImage::from_raw(width, height, samples).unwrap()
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn fixed_24x24_rggb_mosaic_matches_snapshot() -> mischief::Result<()> {
        let image: MonoImage<f32> = seeded_mosaic(24, 24, -0.5, 2.5);
        let output = DemosaicRcd.lazy(&image, BayerPattern::RGGB)?.collect();
        insta::assert_snapshot!(crate::demosaic::test_support::format_rgb_3dp(&output));
        Ok(())
    }

    #[test]
    fn uniform_and_colored_constants_are_preserved() -> mischief::Result<()> {
        for pattern in BayerPattern::ALL {
            for value in [-0.25_f32, 0.0_f32, 0.25_f32, 4.0_f32] {
                let image = MonoImage::from_raw(32, 32, vec![value; 32 * 32]).unwrap();
                let output = DemosaicRcd.lazy(&image, pattern)?.collect();
                assert!(
                    output
                        .as_raw()
                        .iter()
                        .all(|sample| (*sample - value).abs() < 1.0e-5),
                    "{pattern} failed at {value}"
                );
            }
            for value in [-0.25_f64, 0.0_f64, 1.0e4_f64] {
                let image = MonoImage::from_raw(32, 32, vec![value; 32 * 32]).unwrap();
                let output = DemosaicRcd.lazy(&image, pattern)?.collect();
                assert!(
                    output
                        .as_raw()
                        .iter()
                        .all(|sample| (*sample - value).abs() < 1.0e-11),
                    "{pattern} f64 failed at {value}"
                );
            }

            let colored = (0..32_u32)
                .flat_map(|y| {
                    (0..32_u32).map(move |x| match color(pattern.channel_at(x, y)) {
                        0 => -0.25_f64,
                        1 => 0.5_f64,
                        _ => 2.25_f64,
                    })
                })
                .collect();
            let image = MonoImage::from_raw(32, 32, colored).unwrap();
            let output = DemosaicRcd.lazy(&image, pattern)?.collect();
            for y in 0..32_u32 {
                for x in 0..32_u32 {
                    let px = output.get_pixel(x, y).0;
                    assert!((px[0] + 0.25).abs() < 1.0e-11, "{pattern} R at ({x},{y})");
                    assert!((px[1] - 0.5).abs() < 1.0e-11, "{pattern} G at ({x},{y})");
                    assert!((px[2] - 2.25).abs() < 1.0e-11, "{pattern} B at ({x},{y})");
                }
            }
        }
        Ok(())
    }

    #[test]
    fn signed_zero_crossing_ramp_is_finite_and_accurate() -> mischief::Result<()> {
        let width = 32_u32;
        let height = 32_u32;
        let samples = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| {
                    let t = (x + y) as f64 / f64::from(width + height - 2);
                    0.5f64.mul_add(t, -0.25)
                })
            })
            .collect();
        let image = MonoImage::from_raw(width, height, samples).unwrap();
        let output = DemosaicRcd.lazy(&image, BayerPattern::RGGB)?.collect();
        for y in 0..height {
            for x in 0..width {
                for ch in output.get_pixel(x, y).0 {
                    assert!(ch.is_finite());
                }
            }
        }
        for y in MARGIN as u32..height - MARGIN as u32 {
            for x in MARGIN as u32..width - MARGIN as u32 {
                let expected = -0.25 + 0.5 * (x + y) as f64 / f64::from(width + height - 2);
                for ch in output.get_pixel(x, y).0 {
                    assert!(
                        (ch - expected).abs() < 5.0e-4,
                        "zero-crossing ramp at ({x},{y}): {ch} vs {expected}"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn positive_scale_equivariance_holds_in_f32_and_f64() -> mischief::Result<()> {
        let base_f32: MonoImage<f32> = seeded_mosaic(32, 32, -0.5, 2.5);
        let base_out_f32 = DemosaicRcd.lazy(&base_f32, BayerPattern::GBRG)?.collect();
        for scale in [1.0e-4_f32, 1.0e-2, 1.0e2, 1.0e4, 1.0e12, 1.0e-12] {
            let scaled = MonoImage::from_raw(
                32,
                32,
                base_f32.as_raw().iter().map(|v| *v * scale).collect(),
            )
            .unwrap();
            let out = DemosaicRcd.lazy(&scaled, BayerPattern::GBRG)?.collect();
            for (a, b) in out.as_raw().iter().zip(base_out_f32.as_raw()) {
                assert!(a.is_finite());
                let expected = *b * scale;
                let tol = 1.0e-4 * expected.abs().max(scale);
                assert!(
                    (*a - expected).abs() <= tol,
                    "f32 scale {scale}: {a} vs {expected}"
                );
            }
        }

        let base_f64: MonoImage<f64> = seeded_mosaic(32, 32, -0.5, 2.5);
        let base_out_f64 = DemosaicRcd.lazy(&base_f64, BayerPattern::GBRG)?.collect();
        for scale in [1.0e-6_f64, 1.0e6] {
            let scaled = MonoImage::from_raw(
                32,
                32,
                base_f64.as_raw().iter().map(|v| *v * scale).collect(),
            )
            .unwrap();
            let out = DemosaicRcd.lazy(&scaled, BayerPattern::GBRG)?.collect();
            for (a, b) in out.as_raw().iter().zip(base_out_f64.as_raw()) {
                let expected = *b * scale;
                let tol = 1.0e-10 * expected.abs().max(scale);
                assert!(
                    (*a - expected).abs() <= tol,
                    "f64 scale {scale}: {a} vs {expected}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn horizontal_and_vertical_flip_symmetry_hold() -> mischief::Result<()> {
        let width = 32_u32;
        let height = 32_u32;
        let base: MonoImage<f64> = seeded_mosaic(width, height, -0.5, 2.0);

        let mut h_samples = Vec::with_capacity((width * height) as usize);
        let mut v_samples = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in (0..width).rev() {
                h_samples.push(base.get_pixel(x, y)[0]);
            }
        }
        for y in (0..height).rev() {
            for x in 0..width {
                v_samples.push(base.get_pixel(x, y)[0]);
            }
        }
        let h_flip = MonoImage::from_raw(width, height, h_samples).unwrap();
        let v_flip = MonoImage::from_raw(width, height, v_samples).unwrap();

        for (pattern, h_pat, v_pat) in [
            (BayerPattern::RGGB, BayerPattern::GRBG, BayerPattern::GBRG),
            (BayerPattern::GRBG, BayerPattern::RGGB, BayerPattern::BGGR),
            (BayerPattern::BGGR, BayerPattern::GBRG, BayerPattern::GRBG),
            (BayerPattern::GBRG, BayerPattern::BGGR, BayerPattern::RGGB),
        ] {
            let out = DemosaicRcd.lazy(&base, pattern)?.collect();
            let out_h = DemosaicRcd.lazy(&h_flip, h_pat)?.collect();
            let out_v = DemosaicRcd.lazy(&v_flip, v_pat)?.collect();
            for y in 0..height {
                for x in 0..width {
                    let a = out.get_pixel(x, y).0;
                    let bh = out_h.get_pixel(width - 1 - x, y).0;
                    let bv = out_v.get_pixel(x, height - 1 - y).0;
                    for c in 0..3 {
                        assert!(
                            (a[c] - bh[c]).abs() < 1.0e-11,
                            "h-flip {pattern} ({x},{y}) ch {c}"
                        );
                        assert!(
                            (a[c] - bv[c]).abs() < 1.0e-11,
                            "v-flip {pattern} ({x},{y}) ch {c}"
                        );
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn tile_size_options_are_consistent_and_validated() -> mischief::Result<()> {
        let image: MonoImage<f32> = seeded_mosaic(140, 140, -0.25, 2.0);
        let default_out = DemosaicRcd.lazy(&image, BayerPattern::RGGB)?.collect();
        for tile_size in [32_usize, 64] {
            let out = DemosaicRcd
                .lazy_with_options(&image, BayerPattern::RGGB, RcdOptions { tile_size })?
                .collect();
            assert_eq!(
                out, default_out,
                "tile_size={tile_size} differed from default"
            );
        }
        for bad in [0_usize, 20, 21, 113] {
            assert!(matches!(
                DemosaicRcd.lazy_with_options(
                    &image,
                    BayerPattern::RGGB,
                    RcdOptions { tile_size: bad }
                ),
                Err(DemosaicError::InvalidInput(_))
            ));
        }
        Ok(())
    }

    #[test]
    fn measured_bayer_samples_are_preserved() -> mischief::Result<()> {
        let samples = (0..1600)
            .map(|sample| sample as f64 / 1599.0)
            .collect::<Vec<_>>();
        let image = MonoImage::from_raw(40, 40, samples).unwrap();
        for pattern in BayerPattern::ALL {
            let output = DemosaicRcd.lazy(&image, pattern)?.collect();
            for y in 0..image.height() {
                for x in 0..image.width() {
                    let channel = color(pattern.channel_at(x, y));
                    assert_eq!(
                        output.get_pixel(x, y)[channel],
                        image.get_pixel(x, y)[0],
                        "{pattern} at ({x}, {y})"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn rows_can_be_filled_independently() -> mischief::Result<()> {
        let samples = (0..40 * 40)
            .map(|sample| sample as f32 / (40.0 * 40.0))
            .collect::<Vec<_>>();
        let image = MonoImage::from_raw(40, 40, samples).unwrap();
        let lazy = DemosaicRcd.lazy(&image, BayerPattern::GRBG)?;
        let whole = lazy.collect();

        let mut rows = vec![0.0; 40 * 3 * 40];
        lazy.fill_rows(0, &mut rows);
        assert_eq!(whole.as_raw(), &rows);
        Ok(())
    }

    #[test]
    fn collect_with_consumes_the_cached_rgb_result() -> mischief::Result<()> {
        let image = MonoImage::from_raw(40, 40, vec![0.25_f32; 40 * 40]).unwrap();
        let lazy = DemosaicRcd.lazy(&image, BayerPattern::GRBG)?;

        let output = lazy.collect_with(|_, _| {});

        assert!(lazy.cached_image_is_absent());
        assert!(
            output
                .as_raw()
                .iter()
                .all(|sample| (*sample - 0.25).abs() < 1.0e-5)
        );
        Ok(())
    }

    #[test]
    fn small_images_fall_back_to_ppg() -> mischief::Result<()> {
        let image = MonoImage::from_raw(8, 8, vec![0.5_f32; 64]).unwrap();
        let rcd = DemosaicRcd.lazy(&image, BayerPattern::RGGB)?.collect();
        let ppg = DemosaicPpg.lazy(&image, BayerPattern::RGGB)?.collect();
        assert_eq!(rcd, ppg);
        Ok(())
    }
}
