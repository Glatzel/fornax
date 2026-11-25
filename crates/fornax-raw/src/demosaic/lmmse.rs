//! Tiled directional LMMSE Bayer demosaicing in sensor-linear space.
//!
//! Implements Zhang-Wu directional linear minimum mean-square-error
//! interpolation directly on signed, unclamped, scale-normalized linear CFA
//! samples, with optional color-difference median and EECI refinement passes.

use generic_num::num;
use rayon::prelude::*;

use super::buffer::{DenseLazy, channel, dense_from_vec};
use super::numerics::normalization_scale;
use super::{LmmseMode, LmmseOptions};
use crate::error::DemosaicError;
use crate::{BayerPattern, MonoImage, RawFloat};

const RED: usize = 0;
const GREEN: usize = 1;
const BLUE: usize = 2;

const VARIANCE_FLOOR: f64 = 1.0e-12;
const REFINE_EDGE_SCALE: f64 = 1.0e-2;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicLmmse;

impl DemosaicLmmse {
    #[cfg(test)]
    pub fn lazy<T>(
        &self,
        image: &MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<DenseLazy<T>, DemosaicError>
    where
        T: RawFloat + Send + Sync,
    {
        self.lazy_with_options(image, pattern, LmmseOptions::DEFAULT)
    }

    pub fn lazy_with_options<T>(
        &self,
        image: &MonoImage<T>,
        pattern: BayerPattern,
        options: LmmseOptions,
    ) -> Result<DenseLazy<T>, DemosaicError>
    where
        T: RawFloat + Send + Sync,
    {
        let width = image.width() as usize;
        let height = image.height() as usize;
        if width < 2 || height < 2 {
            return Err(DemosaicError::InvalidInput(
                "LMMSE requires an image at least 2x2".into(),
            ));
        }

        let halo = required_halo(options.mode);
        let min_tile_size = 2 * halo + 2;
        if options.tile_size < min_tile_size || !options.tile_size.is_multiple_of(2) {
            return Err(DemosaicError::InvalidInput(
                format!(
                    "LMMSE tile_size must be even and at least {min_tile_size} for {:?}, got {}",
                    options.mode, options.tile_size
                )
                .into(),
            ));
        }

        let mut output = vec![T::zero(); width * height * 3];
        interpolate(
            image.as_raw(),
            &mut output,
            width,
            height,
            pattern,
            options,
            halo,
        );

        Ok(DenseLazy::new(dense_from_vec(
            image.width(),
            image.height(),
            output,
        )))
    }
}

const fn required_halo(mode: LmmseMode) -> usize {
    match mode {
        LmmseMode::Basic => 12,
        LmmseMode::Median => 14,
        LmmseMode::ThreeMedians => 16,
        LmmseMode::Refine(passes) => 16 + 6 * passes.get() as usize,
    }
}

struct LmmseTileOutput<T> {
    row_start: usize,
    row_end: usize,
    col_start: usize,
    col_end: usize,
    pixels: Vec<T>,
}

fn interpolate<T>(
    input: &[T],
    output: &mut [T],
    width: usize,
    height: usize,
    pattern: BayerPattern,
    options: LmmseOptions,
    halo: usize,
) where
    T: RawFloat + Send + Sync,
{
    let (median_passes, refinement_passes) = match options.mode {
        LmmseMode::Basic => (0, 0),
        LmmseMode::Median => (1, 0),
        LmmseMode::ThreeMedians => (3, 0),
        LmmseMode::Refine(passes) => (3, usize::from(passes.get())),
    };

    let tile_size = options.tile_size;
    let tile_core = tile_size - 2 * halo;
    let scale = normalization_scale(input);
    let inv_scale = T::one() / scale;

    let h0 = T::one();
    let h1 = (-T::one() / num!(8.0)).exp();
    let h2 = (-num!(4.0) / num!(8.0)).exp();
    let h3 = (-num!(9.0) / num!(8.0)).exp();
    let h4 = (-num!(16.0) / num!(8.0)).exp();
    let hsum = num!(2.0) * (h1 + h2 + h3 + h4) + h0;
    let h = [h0 / hsum, h1 / hsum, h2 / hsum, h3 / hsum, h4 / hsum];

    let num_vertical = height.div_ceil(tile_core);
    let num_horizontal = width.div_ceil(tile_core);
    let coords: Vec<(usize, usize)> = (0..num_vertical)
        .flat_map(|tv| (0..num_horizontal).map(move |th| (tv, th)))
        .collect();

    let tile_area = tile_size * tile_size;
    let tiles: Vec<LmmseTileOutput<T>> = coords
        .into_par_iter()
        .map_init(
            || vec![T::zero(); 6 * tile_area],
            |qix, (tile_vertical, tile_horizontal)| {
                process_tile(
                    qix,
                    input,
                    width,
                    height,
                    pattern,
                    tile_size,
                    tile_core,
                    halo,
                    tile_vertical,
                    tile_horizontal,
                    median_passes,
                    refinement_passes,
                    h,
                    scale,
                    inv_scale,
                )
            },
        )
        .collect();

    for tile in tiles {
        let core_cols = tile.col_end - tile.col_start;
        for (dy, row) in (tile.row_start..tile.row_end).enumerate() {
            let src_start = dy * core_cols * 3;
            let dst_start = (row * width + tile.col_start) * 3;
            output[dst_start..dst_start + core_cols * 3]
                .copy_from_slice(&tile.pixels[src_start..src_start + core_cols * 3]);
        }
    }
}

fn process_tile<T>(
    qix: &mut [T],
    input: &[T],
    width: usize,
    height: usize,
    pattern: BayerPattern,
    tile_size: usize,
    tile_core: usize,
    halo: usize,
    tile_vertical: usize,
    tile_horizontal: usize,
    median_passes: usize,
    refinement_passes: usize,
    h: [T; 5],
    scale: T,
    inv_scale: T,
) -> LmmseTileOutput<T>
where
    T: RawFloat,
{
    qix.fill(T::zero());
    let tile_area = tile_size * tile_size;

    let row_start = tile_vertical * tile_core;
    let row_end = (row_start + tile_core).min(height);
    let col_start = tile_horizontal * tile_core;
    let col_end = (col_start + tile_core).min(width);
    let core_rows = row_end - row_start;
    let core_cols = col_end - col_start;
    let last_rr = core_rows + 2 * halo;
    let last_cc = core_cols + 2 * halo;

    for rr in 0..last_rr {
        let row = reflect_index(row_start as isize + rr as isize - halo as isize, height);
        for cc in 0..last_cc {
            let col = reflect_index(col_start as isize + cc as isize - halo as isize, width);
            qix[tile_index(5, rr, cc, tile_size, tile_area)] = input[row * width + col] * inv_scale;
        }
    }

    color_differences(qix, last_rr, last_cc, pattern, tile_size, tile_area);
    low_pass(qix, last_rr, last_cc, h, tile_size, tile_area);
    mmse_interpolate(qix, last_rr, last_cc, pattern, tile_size, tile_area);
    copy_cfa_samples(qix, last_rr, last_cc, pattern, tile_size, tile_area);
    interpolate_red_blue(qix, last_rr, last_cc, pattern, tile_size, tile_area);

    for pass in 0..median_passes {
        let margin = 12 + pass;
        median_pass(qix, margin, last_rr, last_cc, pattern, tile_size, tile_area);
    }

    let restore_margin = if median_passes > 0 { 14 } else { 11 };
    for rrr in restore_margin..last_rr - restore_margin {
        for ccc in restore_margin..last_cc - restore_margin {
            let c = tile_channel(pattern, rrr, ccc);
            qix[tile_index(c, rrr, ccc, tile_size, tile_area)] =
                q(qix, 5, rrr, ccc, tile_size, tile_area);
        }
    }

    for pass in 0..refinement_passes {
        let base = 14 + 6 * pass;
        refine_tile(qix, base, last_rr, last_cc, pattern, tile_size, tile_area);
    }

    let mut pixels = Vec::with_capacity(core_rows * core_cols * 3);
    for rr in halo..halo + core_rows {
        let row = row_start + rr - halo;
        for cc in halo..halo + core_cols {
            let col = col_start + cc - halo;
            let mut out_rgb = [
                q(qix, RED, rr, cc, tile_size, tile_area) * scale,
                q(qix, GREEN, rr, cc, tile_size, tile_area) * scale,
                q(qix, BLUE, rr, cc, tile_size, tile_area) * scale,
            ];
            out_rgb[channel(pattern, col as u32, row as u32)] = input[row * width + col];
            pixels.extend_from_slice(&out_rgb);
        }
    }

    LmmseTileOutput {
        row_start,
        row_end,
        col_start,
        col_end,
        pixels,
    }
}

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

fn refine_tile<T>(
    qix: &mut [T],
    base: usize,
    last_rr: usize,
    last_cc: usize,
    pattern: BayerPattern,
    tile_size: usize,
    tile_area: usize,
) where
    T: RawFloat,
{
    let edge_scale: T = num!(REFINE_EDGE_SCALE);

    let m0 = base + 2;
    for rr in m0..last_rr - m0 {
        let mut cc = m0 + (tile_channel(pattern, rr, m0) & 1);
        let c = tile_channel(pattern, rr, cc);
        while cc < last_cc - m0 {
            let center_color = q(qix, c, rr, cc, tile_size, tile_area);
            let horizontal_green = (q(qix, GREEN, rr, cc + 1, tile_size, tile_area)
                - q(qix, GREEN, rr, cc - 1, tile_size, tile_area))
            .abs();
            let vertical_green = (q(qix, GREEN, rr + 1, cc, tile_size, tile_area)
                - q(qix, GREEN, rr - 1, cc, tile_size, tile_area))
            .abs();
            let left = T::one()
                / (edge_scale
                    + (q(qix, c, rr, cc - 2, tile_size, tile_area) - center_color).abs()
                    + horizontal_green);
            let right = T::one()
                / (edge_scale
                    + (q(qix, c, rr, cc + 2, tile_size, tile_area) - center_color).abs()
                    + horizontal_green);
            let up = T::one()
                / (edge_scale
                    + (q(qix, c, rr - 2, cc, tile_size, tile_area) - center_color).abs()
                    + vertical_green);
            let down = T::one()
                / (edge_scale
                    + (q(qix, c, rr + 2, cc, tile_size, tile_area) - center_color).abs()
                    + vertical_green);
            let left_difference = q(qix, GREEN, rr, cc - 1, tile_size, tile_area)
                - q(qix, c, rr, cc - 1, tile_size, tile_area);
            let right_difference = q(qix, GREEN, rr, cc + 1, tile_size, tile_area)
                - q(qix, c, rr, cc + 1, tile_size, tile_area);
            let up_difference = q(qix, GREEN, rr - 1, cc, tile_size, tile_area)
                - q(qix, c, rr - 1, cc, tile_size, tile_area);
            let down_difference = q(qix, GREEN, rr + 1, cc, tile_size, tile_area)
                - q(qix, c, rr + 1, cc, tile_size, tile_area);
            let numerator = left_difference.mul_add(
                left,
                right_difference.mul_add(right, up_difference.mul_add(up, down_difference * down)),
            );
            qix[tile_index(GREEN, rr, cc, tile_size, tile_area)] =
                center_color + numerator / (left + right + up + down);
            cc += 2;
        }
    }

    let m1 = base + 4;
    for rr in m1..last_rr - m1 {
        let mut cc = m1 + (tile_channel(pattern, rr, m1 + 1) & 1);
        let mut c = tile_channel(pattern, rr, cc + 1);
        while cc < last_cc - m1 {
            for _ in 0..2 {
                let center_green = q(qix, GREEN, rr, cc, tile_size, tile_area);
                let left_green =
                    (q(qix, GREEN, rr, cc - 2, tile_size, tile_area) - center_green).abs();
                let right_green =
                    (q(qix, GREEN, rr, cc + 2, tile_size, tile_area) - center_green).abs();
                let up_green =
                    (q(qix, GREEN, rr - 2, cc, tile_size, tile_area) - center_green).abs();
                let down_green =
                    (q(qix, GREEN, rr + 2, cc, tile_size, tile_area) - center_green).abs();
                let horizontal_color = (q(qix, c, rr, cc + 1, tile_size, tile_area)
                    - q(qix, c, rr, cc - 1, tile_size, tile_area))
                .abs();
                let vertical_color = (q(qix, c, rr + 1, cc, tile_size, tile_area)
                    - q(qix, c, rr - 1, cc, tile_size, tile_area))
                .abs();
                let left = T::one() / (edge_scale + left_green + horizontal_color);
                let right = T::one() / (edge_scale + right_green + horizontal_color);
                let up = T::one() / (edge_scale + up_green + vertical_color);
                let down = T::one() / (edge_scale + down_green + vertical_color);
                let left_difference = q(qix, GREEN, rr, cc - 1, tile_size, tile_area)
                    - q(qix, c, rr, cc - 1, tile_size, tile_area);
                let right_difference = q(qix, GREEN, rr, cc + 1, tile_size, tile_area)
                    - q(qix, c, rr, cc + 1, tile_size, tile_area);
                let up_difference = q(qix, GREEN, rr - 1, cc, tile_size, tile_area)
                    - q(qix, c, rr - 1, cc, tile_size, tile_area);
                let down_difference = q(qix, GREEN, rr + 1, cc, tile_size, tile_area)
                    - q(qix, c, rr + 1, cc, tile_size, tile_area);
                let numerator = left_difference.mul_add(
                    left,
                    right_difference
                        .mul_add(right, up_difference.mul_add(up, down_difference * down)),
                );
                qix[tile_index(c, rr, cc, tile_size, tile_area)] =
                    center_green - numerator / (left + right + up + down);
                c = 2 - c;
            }
            cc += 2;
        }
    }

    let m2 = base + 6;
    for rr in m2..last_rr - m2 {
        let mut cc = m2 + (tile_channel(pattern, rr, m2) & 1);
        while cc < last_cc - m2 {
            let sampled = tile_channel(pattern, rr, cc);
            let c = 2 - sampled;
            let d = sampled;
            let center_green = q(qix, GREEN, rr, cc, tile_size, tile_area);
            let center_integrated = q(qix, d, rr, cc, tile_size, tile_area);
            let horizontal_green = (q(qix, GREEN, rr, cc + 1, tile_size, tile_area)
                - q(qix, GREEN, rr, cc - 1, tile_size, tile_area))
            .abs();
            let vertical_green = (q(qix, GREEN, rr + 1, cc, tile_size, tile_area)
                - q(qix, GREEN, rr - 1, cc, tile_size, tile_area))
            .abs();
            let left = T::one()
                / (edge_scale
                    + (q(qix, d, rr, cc - 2, tile_size, tile_area) - center_integrated).abs()
                    + horizontal_green);
            let right = T::one()
                / (edge_scale
                    + (q(qix, d, rr, cc + 2, tile_size, tile_area) - center_integrated).abs()
                    + horizontal_green);
            let up = T::one()
                / (edge_scale
                    + (q(qix, d, rr - 2, cc, tile_size, tile_area) - center_integrated).abs()
                    + vertical_green);
            let down = T::one()
                / (edge_scale
                    + (q(qix, d, rr + 2, cc, tile_size, tile_area) - center_integrated).abs()
                    + vertical_green);
            let left_difference = q(qix, GREEN, rr, cc - 1, tile_size, tile_area)
                - q(qix, c, rr, cc - 1, tile_size, tile_area);
            let right_difference = q(qix, GREEN, rr, cc + 1, tile_size, tile_area)
                - q(qix, c, rr, cc + 1, tile_size, tile_area);
            let up_difference = q(qix, GREEN, rr - 1, cc, tile_size, tile_area)
                - q(qix, c, rr - 1, cc, tile_size, tile_area);
            let down_difference = q(qix, GREEN, rr + 1, cc, tile_size, tile_area)
                - q(qix, c, rr + 1, cc, tile_size, tile_area);
            let numerator = left_difference.mul_add(
                left,
                right_difference.mul_add(right, up_difference.mul_add(up, down_difference * down)),
            );
            qix[tile_index(c, rr, cc, tile_size, tile_area)] =
                center_green - numerator / (left + right + up + down);
            cc += 2;
        }
    }
}

fn color_differences<T>(
    qix: &mut [T],
    last_rr: usize,
    last_cc: usize,
    pattern: BayerPattern,
    tile_size: usize,
    tile_area: usize,
) where
    T: RawFloat,
{
    let half: T = num!(0.5);
    let quarter: T = num!(0.25);

    for rr in 2..last_rr - 2 {
        let mut cc = 2 + (tile_channel(pattern, rr, 2) & 1);
        while cc < last_cc - 2 {
            let center = q(qix, 5, rr, cc, tile_size, tile_area);
            let horizontal = half.mul_add(
                q(qix, 5, rr, cc - 1, tile_size, tile_area)
                    + q(qix, 5, rr, cc + 1, tile_size, tile_area)
                    - center,
                -quarter
                    * (q(qix, 5, rr, cc - 2, tile_size, tile_area)
                        + q(qix, 5, rr, cc + 2, tile_size, tile_area)),
            );
            let vertical = half.mul_add(
                q(qix, 5, rr - 1, cc, tile_size, tile_area)
                    + q(qix, 5, rr + 1, cc, tile_size, tile_area)
                    - center,
                -quarter
                    * (q(qix, 5, rr - 2, cc, tile_size, tile_area)
                        + q(qix, 5, rr + 2, cc, tile_size, tile_area)),
            );
            qix[tile_index(0, rr, cc, tile_size, tile_area)] = horizontal;
            qix[tile_index(1, rr, cc, tile_size, tile_area)] = vertical;
            cc += 2;
        }

        let mut cc = 2 + (tile_channel(pattern, rr, 3) & 1);
        while cc < last_cc - 2 {
            let center = q(qix, 5, rr, cc, tile_size, tile_area);
            let horizontal = half.mul_add(
                center
                    - q(qix, 5, rr, cc - 1, tile_size, tile_area)
                    - q(qix, 5, rr, cc + 1, tile_size, tile_area),
                quarter
                    * (q(qix, 5, rr, cc - 2, tile_size, tile_area)
                        + q(qix, 5, rr, cc + 2, tile_size, tile_area)),
            );
            let vertical = half.mul_add(
                center
                    - q(qix, 5, rr - 1, cc, tile_size, tile_area)
                    - q(qix, 5, rr + 1, cc, tile_size, tile_area),
                quarter
                    * (q(qix, 5, rr - 2, cc, tile_size, tile_area)
                        + q(qix, 5, rr + 2, cc, tile_size, tile_area)),
            );
            qix[tile_index(0, rr, cc, tile_size, tile_area)] = horizontal;
            qix[tile_index(1, rr, cc, tile_size, tile_area)] = vertical;
            cc += 2;
        }
    }
}

fn low_pass<T>(
    qix: &mut [T],
    last_rr: usize,
    last_cc: usize,
    h: [T; 5],
    tile_size: usize,
    tile_area: usize,
) where
    T: RawFloat,
{
    let [h0, h1, h2, h3, h4] = h;
    for rr in 6..last_rr - 6 {
        for cc in 6..last_cc - 6 {
            let horizontal = T::mul_add(
                h4,
                q(qix, 0, rr, cc - 4, tile_size, tile_area)
                    + q(qix, 0, rr, cc + 4, tile_size, tile_area),
                T::mul_add(
                    h3,
                    q(qix, 0, rr, cc - 3, tile_size, tile_area)
                        + q(qix, 0, rr, cc + 3, tile_size, tile_area),
                    T::mul_add(
                        h2,
                        q(qix, 0, rr, cc - 2, tile_size, tile_area)
                            + q(qix, 0, rr, cc + 2, tile_size, tile_area),
                        T::mul_add(
                            h1,
                            q(qix, 0, rr, cc - 1, tile_size, tile_area)
                                + q(qix, 0, rr, cc + 1, tile_size, tile_area),
                            h0 * q(qix, 0, rr, cc, tile_size, tile_area),
                        ),
                    ),
                ),
            );
            let vertical = T::mul_add(
                h4,
                q(qix, 1, rr - 4, cc, tile_size, tile_area)
                    + q(qix, 1, rr + 4, cc, tile_size, tile_area),
                T::mul_add(
                    h3,
                    q(qix, 1, rr - 3, cc, tile_size, tile_area)
                        + q(qix, 1, rr + 3, cc, tile_size, tile_area),
                    T::mul_add(
                        h2,
                        q(qix, 1, rr - 2, cc, tile_size, tile_area)
                            + q(qix, 1, rr + 2, cc, tile_size, tile_area),
                        T::mul_add(
                            h1,
                            q(qix, 1, rr - 1, cc, tile_size, tile_area)
                                + q(qix, 1, rr + 1, cc, tile_size, tile_area),
                            h0 * q(qix, 1, rr, cc, tile_size, tile_area),
                        ),
                    ),
                ),
            );
            qix[tile_index(2, rr, cc, tile_size, tile_area)] = horizontal;
            qix[tile_index(3, rr, cc, tile_size, tile_area)] = vertical;
        }
    }
}

fn mmse_interpolate<T>(
    qix: &mut [T],
    last_rr: usize,
    last_cc: usize,
    pattern: BayerPattern,
    tile_size: usize,
    tile_area: usize,
) where
    T: RawFloat,
{
    let half: T = num!(0.5);
    for rr in 10..last_rr - 10 {
        let mut cc = 10 + (tile_channel(pattern, rr, 10) & 1);
        while cc < last_cc - 10 {
            let horizontal_filtered = [
                q(qix, 2, rr, cc - 4, tile_size, tile_area),
                q(qix, 2, rr, cc - 3, tile_size, tile_area),
                q(qix, 2, rr, cc - 2, tile_size, tile_area),
                q(qix, 2, rr, cc - 1, tile_size, tile_area),
                q(qix, 2, rr, cc, tile_size, tile_area),
                q(qix, 2, rr, cc + 1, tile_size, tile_area),
                q(qix, 2, rr, cc + 2, tile_size, tile_area),
                q(qix, 2, rr, cc + 3, tile_size, tile_area),
                q(qix, 2, rr, cc + 4, tile_size, tile_area),
            ];
            let horizontal_raw = [
                q(qix, 0, rr, cc - 4, tile_size, tile_area),
                q(qix, 0, rr, cc - 3, tile_size, tile_area),
                q(qix, 0, rr, cc - 2, tile_size, tile_area),
                q(qix, 0, rr, cc - 1, tile_size, tile_area),
                q(qix, 0, rr, cc, tile_size, tile_area),
                q(qix, 0, rr, cc + 1, tile_size, tile_area),
                q(qix, 0, rr, cc + 2, tile_size, tile_area),
                q(qix, 0, rr, cc + 3, tile_size, tile_area),
                q(qix, 0, rr, cc + 4, tile_size, tile_area),
            ];
            let (xh, vh) = mmse_value(horizontal_filtered, horizontal_raw);

            let vertical_filtered = [
                q(qix, 3, rr - 4, cc, tile_size, tile_area),
                q(qix, 3, rr - 3, cc, tile_size, tile_area),
                q(qix, 3, rr - 2, cc, tile_size, tile_area),
                q(qix, 3, rr - 1, cc, tile_size, tile_area),
                q(qix, 3, rr, cc, tile_size, tile_area),
                q(qix, 3, rr + 1, cc, tile_size, tile_area),
                q(qix, 3, rr + 2, cc, tile_size, tile_area),
                q(qix, 3, rr + 3, cc, tile_size, tile_area),
                q(qix, 3, rr + 4, cc, tile_size, tile_area),
            ];
            let vertical_raw = [
                q(qix, 1, rr - 4, cc, tile_size, tile_area),
                q(qix, 1, rr - 3, cc, tile_size, tile_area),
                q(qix, 1, rr - 2, cc, tile_size, tile_area),
                q(qix, 1, rr - 1, cc, tile_size, tile_area),
                q(qix, 1, rr, cc, tile_size, tile_area),
                q(qix, 1, rr + 1, cc, tile_size, tile_area),
                q(qix, 1, rr + 2, cc, tile_size, tile_area),
                q(qix, 1, rr + 3, cc, tile_size, tile_area),
                q(qix, 1, rr + 4, cc, tile_size, tile_area),
            ];
            let (xv, vv) = mmse_value(vertical_filtered, vertical_raw);

            let denom = vh + vv;
            let fused = if denom > T::zero() {
                T::mul_add(xv, vh, xh * vv) / denom
            } else {
                (xh + xv) * half
            };
            qix[tile_index(4, rr, cc, tile_size, tile_area)] = fused;
            cc += 2;
        }
    }
}

fn mmse_value<T>(mut filtered: [T; 9], raw: [T; 9]) -> (T, T)
where
    T: RawFloat,
{
    let floor: T = num!(VARIANCE_FLOOR);
    let mean = filtered.iter().fold(T::zero(), |sum, value| sum + *value) / num!(9.0);
    let filtered_center = filtered[4];
    let variance_filtered = floor
        + filtered
            .iter()
            .map(|sample| square(*sample - mean))
            .fold(T::zero(), |sum, value| sum + value);
    for (filtered, raw) in filtered.iter_mut().zip(raw) {
        *filtered = *filtered - raw;
    }
    let variance_raw = floor
        + filtered
            .iter()
            .map(|sample| square(*sample))
            .fold(T::zero(), |sum, value| sum + value);
    let denom = variance_filtered + variance_raw;
    let value = if denom > T::zero() {
        filtered_center.mul_add(variance_raw, raw[4] * variance_filtered) / denom
    } else {
        filtered_center
    };
    let error_variance = if denom > T::zero() {
        variance_filtered * variance_raw / denom
    } else {
        T::zero()
    };
    (value, error_variance)
}

fn copy_cfa_samples<T>(
    qix: &mut [T],
    last_rr: usize,
    last_cc: usize,
    pattern: BayerPattern,
    tile_size: usize,
    tile_area: usize,
) where
    T: RawFloat,
{
    for rr in 0..last_rr {
        for cc in 0..last_cc {
            let sample = q(qix, 5, rr, cc, tile_size, tile_area);
            let c = tile_channel(pattern, rr, cc);
            qix[tile_index(c, rr, cc, tile_size, tile_area)] = sample;
            if c != GREEN {
                qix[tile_index(GREEN, rr, cc, tile_size, tile_area)] =
                    sample + q(qix, 4, rr, cc, tile_size, tile_area);
            }
        }
    }
}

fn interpolate_red_blue<T>(
    qix: &mut [T],
    last_rr: usize,
    last_cc: usize,
    pattern: BayerPattern,
    tile_size: usize,
    tile_area: usize,
) where
    T: RawFloat,
{
    for rr in 11..last_rr - 11 {
        let mut cc = 11 + (tile_channel(pattern, rr, 12) & 1);
        while cc < last_cc - 11 {
            let mut c = tile_channel(pattern, rr, cc + 1);
            let green = q(qix, GREEN, rr, cc, tile_size, tile_area);
            let value = num!(0.5).mul_add(
                q(qix, c, rr, cc - 1, tile_size, tile_area)
                    - q(qix, GREEN, rr, cc - 1, tile_size, tile_area)
                    + q(qix, c, rr, cc + 1, tile_size, tile_area)
                    - q(qix, GREEN, rr, cc + 1, tile_size, tile_area),
                green,
            );
            qix[tile_index(c, rr, cc, tile_size, tile_area)] = value;
            c = 2 - c;
            let value = num!(0.5).mul_add(
                q(qix, c, rr - 1, cc, tile_size, tile_area)
                    - q(qix, GREEN, rr - 1, cc, tile_size, tile_area)
                    + q(qix, c, rr + 1, cc, tile_size, tile_area)
                    - q(qix, GREEN, rr + 1, cc, tile_size, tile_area),
                green,
            );
            qix[tile_index(c, rr, cc, tile_size, tile_area)] = value;
            cc += 2;
        }
    }

    for rr in 11..last_rr - 11 {
        let mut cc = 11 + (tile_channel(pattern, rr, 11) & 1);
        while cc < last_cc - 11 {
            let c = 2 - tile_channel(pattern, rr, cc);
            let green = q(qix, GREEN, rr, cc, tile_size, tile_area);
            let value = num!(0.25).mul_add(
                q(qix, c, rr - 1, cc, tile_size, tile_area)
                    - q(qix, GREEN, rr - 1, cc, tile_size, tile_area)
                    + q(qix, c, rr, cc - 1, tile_size, tile_area)
                    - q(qix, GREEN, rr, cc - 1, tile_size, tile_area)
                    + q(qix, c, rr, cc + 1, tile_size, tile_area)
                    - q(qix, GREEN, rr, cc + 1, tile_size, tile_area)
                    + q(qix, c, rr + 1, cc, tile_size, tile_area)
                    - q(qix, GREEN, rr + 1, cc, tile_size, tile_area),
                green,
            );
            qix[tile_index(c, rr, cc, tile_size, tile_area)] = value;
            cc += 2;
        }
    }
}

fn median_pass<T>(
    qix: &mut [T],
    margin: usize,
    last_rr: usize,
    last_cc: usize,
    pattern: BayerPattern,
    tile_size: usize,
    tile_area: usize,
) where
    T: RawFloat,
{
    for rr in margin..last_rr - margin {
        for color in [RED, BLUE] {
            let correction_channel = if color == RED { 3 } else { 4 };
            for cc in margin..last_cc - margin {
                let mut samples = [T::zero(); 9];
                let mut slot = 0;
                for dy in -1_i32..=1 {
                    for dx in -1_i32..=1 {
                        let ny = (rr as isize + dy as isize) as usize;
                        let nx = (cc as isize + dx as isize) as usize;
                        samples[slot] = q(qix, color, ny, nx, tile_size, tile_area)
                            - q(qix, GREEN, ny, nx, tile_size, tile_area);
                        slot += 1;
                    }
                }
                qix[tile_index(correction_channel, rr, cc, tile_size, tile_area)] =
                    median9(&samples);
            }
        }
    }

    for rr in margin..last_rr - margin {
        for cc in margin..last_cc - margin {
            let c = tile_channel(pattern, rr, cc);
            if c == GREEN {
                let green = q(qix, GREEN, rr, cc, tile_size, tile_area);
                qix[tile_index(RED, rr, cc, tile_size, tile_area)] =
                    green + q(qix, 3, rr, cc, tile_size, tile_area);
                qix[tile_index(BLUE, rr, cc, tile_size, tile_area)] =
                    green + q(qix, 4, rr, cc, tile_size, tile_area);
            } else {
                let opposite = 2 - c;
                let correction_channel = if opposite == RED { 3 } else { 4 };
                qix[tile_index(opposite, rr, cc, tile_size, tile_area)] =
                    q(qix, GREEN, rr, cc, tile_size, tile_area)
                        + q(qix, correction_channel, rr, cc, tile_size, tile_area);
                qix[tile_index(GREEN, rr, cc, tile_size, tile_area)] = num!(0.5)
                    * (q(qix, RED, rr, cc, tile_size, tile_area)
                        - q(qix, 3, rr, cc, tile_size, tile_area)
                        + q(qix, BLUE, rr, cc, tile_size, tile_area)
                        - q(qix, 4, rr, cc, tile_size, tile_area));
            }
        }
    }
}

#[inline(always)]
const fn tile_index(
    channel: usize,
    rr: usize,
    cc: usize,
    tile_size: usize,
    tile_area: usize,
) -> usize {
    channel * tile_area + rr * tile_size + cc
}

#[inline(always)]
const fn q<T>(
    qix: &[T],
    channel: usize,
    rr: usize,
    cc: usize,
    tile_size: usize,
    tile_area: usize,
) -> T
where
    T: RawFloat,
{
    qix[tile_index(channel, rr, cc, tile_size, tile_area)]
}

#[inline(always)]
const fn tile_channel(pattern: BayerPattern, rr: usize, cc: usize) -> usize {
    channel(pattern, cc as u32, rr as u32)
}

#[inline]
fn median9<T>(pixels: &[T; 9]) -> T
where
    T: RawFloat,
{
    let mut sorted = *pixels;
    sorted.sort_unstable_by(|left, right| {
        left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
    });
    sorted[4]
}

#[inline(always)]
fn square<T>(value: T) -> T
where
    T: RawFloat,
{
    value * value
}

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;

    use super::*;
    use crate::demosaic::LazyDemosaic;

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
    fn fixed_16x16_rggb_mosaic_matches_snapshot() -> mischief::Result<()> {
        let image: MonoImage<f32> = seeded_mosaic(16, 16, -0.5, 2.5);
        let output = DemosaicLmmse.lazy(&image, BayerPattern::RGGB)?.collect();
        insta::assert_snapshot!(crate::demosaic::test_support::format_rgb_3dp(&output));
        Ok(())
    }

    #[test]
    fn uniform_and_colored_constants_are_preserved_everywhere() -> mischief::Result<()> {
        for pattern in BayerPattern::ALL {
            for mode in [
                LmmseMode::Basic,
                LmmseMode::Median,
                LmmseMode::ThreeMedians,
                LmmseMode::Refine(NonZeroU8::new(1).unwrap()),
            ] {
                for value in [-0.25_f32, 0.0_f32, 0.25_f32, 4.0_f32] {
                    let image = MonoImage::from_raw(32, 32, vec![value; 32 * 32]).unwrap();
                    let output = DemosaicLmmse
                        .lazy_with_options(
                            &image,
                            pattern,
                            LmmseOptions {
                                mode,
                                tile_size: LmmseOptions::DEFAULT_TILE_SIZE,
                            },
                        )?
                        .collect();
                    assert!(
                        output
                            .as_raw()
                            .iter()
                            .all(|sample| (*sample - value).abs() < 1.0e-5),
                        "{pattern} {mode:?} failed at {value}"
                    );
                }
            }

            let colored = (0..32_u32)
                .flat_map(|y| {
                    (0..32_u32).map(move |x| match channel(pattern, x, y) {
                        RED => -0.25_f64,
                        GREEN => 0.5_f64,
                        _ => 2.25_f64,
                    })
                })
                .collect();
            let image = MonoImage::from_raw(32, 32, colored).unwrap();
            let output = DemosaicLmmse.lazy(&image, pattern)?.collect();
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
        let output = DemosaicLmmse.lazy(&image, BayerPattern::RGGB)?.collect();
        for y in 0..height {
            for x in 0..width {
                for ch in output.get_pixel(x, y).0 {
                    assert!(ch.is_finite());
                }
            }
        }
        for y in 10..height - 10 {
            for x in 10..width - 10 {
                let expected = -0.25 + 0.5 * (x + y) as f64 / f64::from(width + height - 2);
                for ch in output.get_pixel(x, y).0 {
                    assert!(
                        (ch - expected).abs() < 1.0e-10,
                        "zero-crossing ramp at ({x},{y}): {ch} vs {expected}"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn positive_scale_and_shift_invariance_hold() -> mischief::Result<()> {
        let base_f32: MonoImage<f32> = seeded_mosaic(32, 32, -0.5, 2.5);
        let base_out_f32 = DemosaicLmmse.lazy(&base_f32, BayerPattern::GBRG)?.collect();
        for scale in [1.0e-4_f32, 1.0e-2, 1.0e2, 1.0e4, 1.0e12, 1.0e-12] {
            let scaled = MonoImage::from_raw(
                32,
                32,
                base_f32.as_raw().iter().map(|v| *v * scale).collect(),
            )
            .unwrap();
            let out = DemosaicLmmse.lazy(&scaled, BayerPattern::GBRG)?.collect();
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

        let base_f64: MonoImage<f64> = seeded_mosaic(32, 32, -0.25, 1.5);
        let base_out_f64 = DemosaicLmmse.lazy(&base_f64, BayerPattern::BGGR)?.collect();
        for shift in [-0.37_f64, 12.5_f64] {
            let shifted = MonoImage::from_raw(
                32,
                32,
                base_f64.as_raw().iter().map(|v| *v + shift).collect(),
            )
            .unwrap();
            let out = DemosaicLmmse.lazy(&shifted, BayerPattern::BGGR)?.collect();
            for (a, b) in out.as_raw().iter().zip(base_out_f64.as_raw()) {
                let expected = *b + shift;
                let tol = 1.0e-7 * expected.abs().max(1.0);
                assert!(
                    (*a - expected).abs() <= tol,
                    "f64 shift {shift}: {a} vs {expected}"
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
            let out = DemosaicLmmse.lazy(&base, pattern)?.collect();
            let out_h = DemosaicLmmse.lazy(&h_flip, h_pat)?.collect();
            let out_v = DemosaicLmmse.lazy(&v_flip, v_pat)?.collect();
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
    fn tile_size_options_and_tiny_images_are_handled() -> mischief::Result<()> {
        let image: MonoImage<f32> = seeded_mosaic(180, 180, -0.25, 2.0);
        let default_out = DemosaicLmmse.lazy(&image, BayerPattern::RGGB)?.collect();
        for tile_size in [64_usize, 200] {
            let out = DemosaicLmmse
                .lazy_with_options(
                    &image,
                    BayerPattern::RGGB,
                    LmmseOptions {
                        mode: LmmseMode::Median,
                        tile_size,
                    },
                )?
                .collect();
            assert_eq!(
                out, default_out,
                "tile_size={tile_size} differed from default"
            );
        }

        for bad in [20_usize, 29, 137] {
            assert!(matches!(
                DemosaicLmmse.lazy_with_options(
                    &image,
                    BayerPattern::RGGB,
                    LmmseOptions {
                        mode: LmmseMode::Median,
                        tile_size: bad,
                    },
                ),
                Err(DemosaicError::InvalidInput(_))
            ));
        }

        let tiny = MonoImage::from_raw(1, 2, vec![0.5_f32; 2]).unwrap();
        assert!(matches!(
            DemosaicLmmse.lazy(&tiny, BayerPattern::RGGB),
            Err(DemosaicError::InvalidInput(_))
        ));

        let small = MonoImage::from_raw(4, 4, (0..16).map(|i| i as f64 / 15.0).collect()).unwrap();
        assert_eq!(
            DemosaicLmmse
                .lazy(&small, BayerPattern::RGGB)?
                .collect()
                .dimensions(),
            (4, 4)
        );
        Ok(())
    }

    #[test]
    fn lmmse_modes_select_median_and_refinement_passes() -> mischief::Result<()> {
        let image: MonoImage<f64> = seeded_mosaic(64, 64, 0.0, 1.0);
        let run = |mode| -> mischief::Result<_> {
            Ok(DemosaicLmmse
                .lazy_with_options(
                    &image,
                    BayerPattern::GBRG,
                    LmmseOptions {
                        mode,
                        tile_size: LmmseOptions::DEFAULT_TILE_SIZE,
                    },
                )?
                .collect())
        };

        let legacy = DemosaicLmmse.lazy(&image, BayerPattern::GBRG)?.collect();
        let basic = run(LmmseMode::Basic)?;
        let median = run(LmmseMode::Median)?;
        let three_medians = run(LmmseMode::ThreeMedians)?;
        let refined = run(LmmseMode::Refine(NonZeroU8::new(1).unwrap()))?;

        assert_eq!(median, legacy);
        assert_ne!(median, basic);
        assert_ne!(median, three_medians);
        assert_ne!(three_medians, refined);
        Ok(())
    }

    #[test]
    fn measured_bayer_samples_are_preserved() -> mischief::Result<()> {
        let image: MonoImage<f64> = seeded_mosaic(32, 32, -0.5, 2.0);
        for pattern in BayerPattern::ALL {
            let output = DemosaicLmmse.lazy(&image, pattern)?.collect();
            for y in 0..image.height() {
                for x in 0..image.width() {
                    let c = channel(pattern, x, y);
                    assert_eq!(
                        output.get_pixel(x, y)[c],
                        image.get_pixel(x, y)[0],
                        "{pattern} at ({x},{y})"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    #[allow(
        clippy::suboptimal_flops,
        reason = "synthetic ramp uses explicit linear form"
    )]
    fn tile_boundaries_do_not_introduce_large_steps() -> mischief::Result<()> {
        let width = 260_u32;
        let height = 260_u32;
        let pattern = BayerPattern::GBRG;
        let samples = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| {
                    let base = match channel(pattern, x, y) {
                        0 => 12_000.0,
                        1 => 20_000.0,
                        _ => 28_000.0,
                    };
                    (base + 24.0 * (x + y) as f32) / f32::from(u16::MAX)
                })
            })
            .collect();
        let image = MonoImage::from_raw(width, height, samples).unwrap();
        let output = DemosaicLmmse.lazy(&image, pattern)?.collect();
        let code =
            |x: u32, y: u32, c: usize| (output.get_pixel(x, y)[c] * f32::from(u16::MAX)).round();

        for boundary in [108, 216] {
            for y in 16..244 {
                for c in 0..3 {
                    let seam = (code(boundary, y, c) - code(boundary - 1, y, c)).abs();
                    let before = (code(boundary - 1, y, c) - code(boundary - 2, y, c)).abs();
                    let after = (code(boundary + 1, y, c) - code(boundary, y, c)).abs();
                    assert!(
                        seam <= before.max(after) + 2.0,
                        "horizontal seam at ({boundary}, {y}, c{c}): {seam} vs {before}/{after}"
                    );
                }
            }
            for x in 16..244 {
                for c in 0..3 {
                    let seam = (code(x, boundary, c) - code(x, boundary - 1, c)).abs();
                    let before = (code(x, boundary - 1, c) - code(x, boundary - 2, c)).abs();
                    let after = (code(x, boundary + 1, c) - code(x, boundary, c)).abs();
                    assert!(
                        seam <= before.max(after) + 2.0,
                        "vertical seam at ({x}, {boundary}, c{c}): {seam} vs {before}/{after}"
                    );
                }
            }
        }
        Ok(())
    }
}
