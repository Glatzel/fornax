//! Aliasing minimization and zipper elimination (AMaZE) Bayer demosaicing.
//!
//! Tiled directional green and color-difference reconstruction in sensor-linear
//! space, adapted for signed, unclamped, scale-equivariant 2x2 RGB Bayer
//! mosaics.

use generic_num::num;
use rayon::prelude::*;

use super::AmazeOptions;
use super::buffer::{DenseLazy, channel, dense_from_vec};
use super::numerics::normalization_scale;
use crate::error::DemosaicError;
use crate::{BayerChannel, BayerPattern, MonoImage, RawFloat};

const BORDER: usize = 16;
const MIN_TILE_SIZE: usize = 2 * BORDER + 4;

const EPS: f64 = 1.0e-12;
const EPS_SQUARED: f64 = 1.0e-24;
const ADAPTIVE_RATIO_MIN: f64 = 0.5;
const ADAPTIVE_RATIO_MAX: f64 = 2.0;
const CHROMA_TAP_CENTER: f64 = 1.325;
const CHROMA_TAP_FAR: f64 = 0.175;
const CHROMA_TAP_SIDE: f64 = 0.075;

const GAUSS_ODD: [f64; 4] = [0.1604755392, 0.1329807601, 0.0469952898, 0.0298925995];
const GAUSS_GRADIENT: [f64; 6] = [
    0.0876496215,
    0.0786557121,
    0.0705832265,
    0.0633391502,
    0.0568385704,
    0.0457706875,
];
const GAUSS_QUINCUNX: [f64; 4] = [0.17, 0.114, 0.046, 0.022];

/// AMaZE Bayer demosaicer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicAmaze;

impl DemosaicAmaze {
    /// Runs AMaZE with default [`AmazeOptions`].
    #[cfg(test)]
    pub fn lazy<T>(
        &self,
        image: &MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<DenseLazy<T>, DemosaicError>
    where
        T: RawFloat + Send + Sync,
    {
        self.lazy_with_options(image, pattern, AmazeOptions::DEFAULT)
    }

    /// Runs AMaZE with explicit [`AmazeOptions`] and validates inputs.
    pub fn lazy_with_options<T>(
        &self,
        image: &MonoImage<T>,
        pattern: BayerPattern,
        options: AmazeOptions,
    ) -> Result<DenseLazy<T>, DemosaicError>
    where
        T: RawFloat + Send + Sync,
    {
        let width = image.width() as usize;
        let height = image.height() as usize;
        if width < 2 || height < 2 {
            return Err(DemosaicError::InvalidInput(
                "AMaZE requires an image at least 2x2".into(),
            ));
        }
        if options.tile_size < MIN_TILE_SIZE || !options.tile_size.is_multiple_of(2) {
            return Err(DemosaicError::InvalidInput(
                format!(
                    "AMaZE tile_size must be even and at least {MIN_TILE_SIZE}, got {}",
                    options.tile_size
                )
                .into(),
            ));
        }

        let input = image.as_raw();
        if let Some((index, value)) = input
            .iter()
            .copied()
            .enumerate()
            .find(|(_, v)| !v.is_finite())
        {
            let x = index % width;
            let y = index / width;
            return Err(DemosaicError::InvalidInput(
                format!("non-finite sample {value:?} at ({x}, {y})").into(),
            ));
        }

        let output = run(input, width, height, pattern, options.tile_size);
        Ok(DenseLazy::new(dense_from_vec(
            image.width(),
            image.height(),
            output,
        )))
    }
}

struct Workspace<T: RawFloat> {
    tile_stride: usize,
    eps: T,
    eps_squared: T,
    cfa: Vec<T>,
    green: Vec<T>,
    red_blue_interpolated: Vec<T>,
    vertical_difference: Vec<T>,
    horizontal_difference: Vec<T>,
    vertical_alternative: Vec<T>,
    horizontal_alternative: Vec<T>,
    vertical_green_delta: Vec<T>,
    horizontal_green_delta: Vec<T>,
    gradient_squared_sum: Vec<T>,
    difference_squared: Vec<T>,
    vertical_weights: Vec<T>,
    horizontal_weights: Vec<T>,
    horizontal_vertical_weight: Vec<T>,
    green_minus_red: Vec<T>,
    green_minus_blue: Vec<T>,
    diagonal_plus_variance: Vec<T>,
    diagonal_minus_variance: Vec<T>,
    plus_gradient: Vec<T>,
    minus_gradient: Vec<T>,
    red_blue_plus: Vec<T>,
    red_blue_minus: Vec<T>,
    plus_minus_weight: Vec<T>,
    nyquist_test: Vec<T>,
    weight_scratch: Vec<T>,
    green_curvature: Vec<[T; 2]>,
    nyquist: Vec<u8>,
    nyquist_refined: Vec<u8>,
}

impl<T: RawFloat> Default for Workspace<T> {
    fn default() -> Self {
        Self {
            tile_stride: 0,
            eps: num!(EPS),
            eps_squared: num!(EPS_SQUARED),
            cfa: Vec::new(),
            green: Vec::new(),
            red_blue_interpolated: Vec::new(),
            vertical_difference: Vec::new(),
            horizontal_difference: Vec::new(),
            vertical_alternative: Vec::new(),
            horizontal_alternative: Vec::new(),
            vertical_green_delta: Vec::new(),
            horizontal_green_delta: Vec::new(),
            gradient_squared_sum: Vec::new(),
            difference_squared: Vec::new(),
            vertical_weights: Vec::new(),
            horizontal_weights: Vec::new(),
            horizontal_vertical_weight: Vec::new(),
            green_minus_red: Vec::new(),
            green_minus_blue: Vec::new(),
            diagonal_plus_variance: Vec::new(),
            diagonal_minus_variance: Vec::new(),
            plus_gradient: Vec::new(),
            minus_gradient: Vec::new(),
            red_blue_plus: Vec::new(),
            red_blue_minus: Vec::new(),
            plus_minus_weight: Vec::new(),
            nyquist_test: Vec::new(),
            weight_scratch: Vec::new(),
            green_curvature: Vec::new(),
            nyquist: Vec::new(),
            nyquist_refined: Vec::new(),
        }
    }
}

impl<T: RawFloat> Workspace<T> {
    fn prepare(&mut self, tile_height: usize, tile_width: usize) {
        self.tile_stride = tile_width;
        let len = tile_height * tile_width;
        let zero = num!(0.0);
        reset_vec(&mut self.cfa, len, zero);
        reset_vec(&mut self.green, len, zero);
        reset_vec(&mut self.red_blue_interpolated, len, zero);
        reset_vec(&mut self.vertical_difference, len, zero);
        reset_vec(&mut self.horizontal_difference, len, zero);
        reset_vec(&mut self.vertical_alternative, len, zero);
        reset_vec(&mut self.horizontal_alternative, len, zero);
        reset_vec(&mut self.vertical_green_delta, len, zero);
        reset_vec(&mut self.horizontal_green_delta, len, zero);
        reset_vec(&mut self.gradient_squared_sum, len, zero);
        reset_vec(&mut self.difference_squared, len, zero);
        reset_vec(&mut self.vertical_weights, len, zero);
        reset_vec(&mut self.horizontal_weights, len, zero);
        reset_vec(&mut self.horizontal_vertical_weight, len, zero);
        reset_vec(&mut self.green_minus_red, len, zero);
        reset_vec(&mut self.green_minus_blue, len, zero);
        reset_vec(&mut self.diagonal_plus_variance, len, zero);
        reset_vec(&mut self.diagonal_minus_variance, len, zero);
        reset_vec(&mut self.plus_gradient, len, zero);
        reset_vec(&mut self.minus_gradient, len, zero);
        reset_vec(&mut self.red_blue_plus, len, zero);
        reset_vec(&mut self.red_blue_minus, len, zero);
        reset_vec(&mut self.plus_minus_weight, len, zero);
        reset_vec(&mut self.nyquist_test, len, zero);
        reset_vec(&mut self.weight_scratch, len, zero);
        reset_vec(&mut self.green_curvature, len, [zero; 2]);
        reset_vec(&mut self.nyquist, len, 0_u8);
        reset_vec(&mut self.nyquist_refined, len, 0_u8);
    }
}

fn reset_vec<V>(buf: &mut Vec<V>, len: usize, value: V)
where
    V: Copy,
{
    if buf.len() < len {
        buf.resize(len, value);
    }
    buf[..len].fill(value);
}

struct TileOutput<T> {
    row_start: usize,
    row_end: usize,
    col_start: usize,
    col_end: usize,
    rgb: Vec<T>,
}

fn run<T>(
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
    let core_step = tile_size - 2 * BORDER;
    let origins = tile_origins(width, height, core_step);

    let tiles: Vec<TileOutput<T>> = origins
        .into_par_iter()
        .map_init(Workspace::default, |workspace, (row_start, col_start)| {
            process_tile(
                input, width, height, pattern, row_start, col_start, core_step, scale, workspace,
            )
        })
        .collect();

    let mut output = vec![num!(0.0); width * height * 3];
    for tile in tiles {
        let tile_cols = tile.col_end - tile.col_start;
        for row in tile.row_start..tile.row_end {
            let local_row = row - tile.row_start;
            let src_start = local_row * tile_cols * 3;
            let src_end = src_start + tile_cols * 3;
            let dst_start = (row * width + tile.col_start) * 3;
            let dst_end = dst_start + tile_cols * 3;
            output[dst_start..dst_end].copy_from_slice(&tile.rgb[src_start..src_end]);
        }
    }

    // Preserve measured CFA samples bit-exactly.
    for row in 0..height {
        for col in 0..width {
            let measured = input[row * width + col];
            let c = channel(pattern, col as u32, row as u32);
            output[(row * width + col) * 3 + c] = measured;
        }
    }

    output
}

fn tile_origins(width: usize, height: usize, core_step: usize) -> Vec<(usize, usize)> {
    let mut origins = Vec::new();
    for row_start in (0..height).step_by(core_step) {
        for col_start in (0..width).step_by(core_step) {
            origins.push((row_start, col_start));
        }
    }
    origins
}

/// Reflects `idx` into `0..len` while preserving `idx.rem_euclid(2)` parity.
///
/// Every reflection step shifts `idx` by an even integer, keeping CFA phases
/// intact across image boundaries for any `len >= 2`.
const fn reflect_index(mut idx: isize, len: usize) -> usize {
    let limit = len as isize - 1;
    while idx < 0 || idx > limit {
        if idx < 0 {
            idx = -idx;
        }
        if idx > limit {
            idx = 2 * limit - idx;
        }
    }
    idx as usize
}

fn process_tile<T>(
    input: &[T],
    width: usize,
    height: usize,
    pattern: BayerPattern,
    row_start: usize,
    col_start: usize,
    core_step: usize,
    scale: T,
    workspace: &mut Workspace<T>,
) -> TileOutput<T>
where
    T: RawFloat,
{
    let row_end = (row_start + core_step).min(height);
    let col_end = (col_start + core_step).min(width);
    let core_h = row_end - row_start;
    let core_w = col_end - col_start;

    let tile_height = (core_h + 2 * BORDER + (core_h & 1)).max(MIN_TILE_SIZE);
    let tile_width = (core_w + 2 * BORDER + (core_w & 1)).max(MIN_TILE_SIZE);
    workspace.prepare(tile_height, tile_width);

    let ts = workspace.tile_stride;
    let inv_scale = num!(1.0) / scale;
    let mut all_zero = true;
    for tr in 0..tile_height {
        let gy = row_start as isize + tr as isize - BORDER as isize;
        let sy = reflect_index(gy, height);
        for tc in 0..tile_width {
            let gx = col_start as isize + tc as isize - BORDER as isize;
            let sx = reflect_index(gx, width);
            let sample = input[sy * width + sx];
            if sample != num!(0.0) {
                all_zero = false;
            }
            workspace.cfa[tr * ts + tc] = sample * inv_scale;
        }
    }

    let core_len = core_h * core_w * 3;
    if all_zero {
        return TileOutput {
            row_start,
            row_end,
            col_start,
            col_end,
            rgb: vec![num!(0.0); core_len],
        };
    }

    calculate_gradients(workspace, tile_height, tile_width);
    interpolate_differences(workspace, tile_height, tile_width, pattern);
    select_color_differences(workspace, tile_height, tile_width, pattern);
    calculate_horizontal_vertical_weights(workspace, tile_height, tile_width, pattern);
    let nyquist_region = detect_nyquist_texture(workspace, tile_height, tile_width, pattern);
    if let Some(region) = nyquist_region {
        refine_nyquist_texture(workspace, region, pattern);
    }
    interpolate_green_at_red_blue(workspace, tile_height, tile_width, pattern);
    if let Some(region) = nyquist_region {
        refine_nyquist_green(workspace, region, pattern);
    }
    prepare_diagonal_interpolation(workspace, tile_height, tile_width, pattern);
    interpolate_diagonal_red_blue(workspace, tile_height, tile_width, pattern);
    refine_green_from_red_blue(workspace, tile_height, tile_width, pattern);
    split_green_differences(workspace, pattern, tile_height, tile_width);
    interpolate_chroma(workspace, tile_height, tile_width, pattern);

    let mut rgb = vec![num!(0.0); core_len];
    write_tile_rgb(&mut rgb, workspace, core_h, core_w, pattern, scale);

    TileOutput {
        row_start,
        row_end,
        col_start,
        col_end,
        rgb,
    }
}

fn calculate_gradients<T>(workspace: &mut Workspace<T>, tile_height: usize, tile_width: usize)
where
    T: RawFloat,
{
    let ts = workspace.tile_stride;
    for rr in 1..tile_height - 1 {
        for cc in 1..tile_width - 1 {
            let index = rr * ts + cc;
            let delh = (workspace.cfa[index + 1] - workspace.cfa[index - 1]).abs();
            let delv = (workspace.cfa[index + ts] - workspace.cfa[index - ts]).abs();
            workspace.horizontal_weights[index] = workspace.eps + delh;
            workspace.vertical_weights[index] = workspace.eps + delv;
            workspace.gradient_squared_sum[index] = square(delh) + square(delv);
        }
    }
}

fn interpolate_differences<T>(
    workspace: &mut Workspace<T>,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) where
    T: RawFloat,
{
    let ts = workspace.tile_stride;
    for rr in 3..tile_height - 3 {
        for cc in 3..tile_width - 3 {
            let index = rr * ts + cc;
            let center = workspace.cfa[index];
            let up = workspace.cfa[index - ts];
            let down = workspace.cfa[index + ts];
            let left = workspace.cfa[index - 1];
            let right = workspace.cfa[index + 1];

            let ratio_up = color_ratio(
                center,
                up,
                workspace.vertical_weights[index - 2 * ts],
                workspace.cfa[index - 2 * ts],
                workspace.vertical_weights[index],
                workspace.eps,
            );
            let ratio_down = color_ratio(
                center,
                down,
                workspace.vertical_weights[index + 2 * ts],
                workspace.cfa[index + 2 * ts],
                workspace.vertical_weights[index],
                workspace.eps,
            );
            let ratio_left = color_ratio(
                center,
                left,
                workspace.horizontal_weights[index - 2],
                workspace.cfa[index - 2],
                workspace.horizontal_weights[index],
                workspace.eps,
            );
            let ratio_right = color_ratio(
                center,
                right,
                workspace.horizontal_weights[index + 2],
                workspace.cfa[index + 2],
                workspace.horizontal_weights[index],
                workspace.eps,
            );

            let up_ha = up + half(center - workspace.cfa[index - 2 * ts]);
            let down_ha = down + half(center - workspace.cfa[index + 2 * ts]);
            let left_ha = left + half(center - workspace.cfa[index - 2]);
            let right_ha = right + half(center - workspace.cfa[index + 2]);

            let up_adaptive = adaptive_green(center, ratio_up, up_ha);
            let down_adaptive = adaptive_green(center, ratio_down, down_ha);
            let left_adaptive = adaptive_green(center, ratio_left, left_ha);
            let right_adaptive = adaptive_green(center, ratio_right, right_ha);

            let horizontal_weight = directional_weight(
                workspace.horizontal_weights[index - 1],
                workspace.horizontal_weights[index + 1],
            );
            let vertical_weight = directional_weight(
                workspace.vertical_weights[index - ts],
                workspace.vertical_weights[index + ts],
            );
            let vertical_ha = interpolate(vertical_weight, down_ha, up_ha);
            let horizontal_ha = interpolate(horizontal_weight, right_ha, left_ha);
            let vertical_adaptive = interpolate(vertical_weight, down_adaptive, up_adaptive);
            let horizontal_adaptive = interpolate(horizontal_weight, right_adaptive, left_adaptive);

            if is_green(pattern, cc, rr) {
                workspace.vertical_difference[index] = center - vertical_adaptive;
                workspace.horizontal_difference[index] = center - horizontal_adaptive;
                workspace.vertical_alternative[index] = center - vertical_ha;
                workspace.horizontal_alternative[index] = center - horizontal_ha;
            } else {
                workspace.vertical_difference[index] = vertical_adaptive - center;
                workspace.horizontal_difference[index] = horizontal_adaptive - center;
                workspace.vertical_alternative[index] = vertical_ha - center;
                workspace.horizontal_alternative[index] = horizontal_ha - center;
            }

            workspace.vertical_green_delta[index] =
                square(up_ha - down_ha).min(square(up_adaptive - down_adaptive));
            workspace.horizontal_green_delta[index] =
                square(left_ha - right_ha).min(square(left_adaptive - right_adaptive));
        }
    }
}

fn select_color_differences<T>(
    workspace: &mut Workspace<T>,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) where
    T: RawFloat,
{
    let ts = workspace.tile_stride;
    let len = tile_height * tile_width;

    workspace.weight_scratch[..len].copy_from_slice(&workspace.horizontal_difference[..len]);
    for rr in 5..tile_height - 5 {
        for cc in 5..tile_width - 5 {
            let index = rr * ts + cc;
            let horizontal_variance = three_tap_variance(
                workspace.horizontal_difference[index - 2],
                workspace.horizontal_difference[index],
                workspace.horizontal_difference[index + 2],
            );
            let horizontal_alternative_variance = three_tap_variance(
                workspace.horizontal_alternative[index - 2],
                workspace.horizontal_alternative[index],
                workspace.horizontal_alternative[index + 2],
            );
            if horizontal_alternative_variance < horizontal_variance {
                workspace.weight_scratch[index] = workspace.horizontal_alternative[index];
            }
        }
    }
    std::mem::swap(
        &mut workspace.horizontal_difference,
        &mut workspace.weight_scratch,
    );

    workspace.weight_scratch[..len].copy_from_slice(&workspace.vertical_difference[..len]);
    for rr in 5..tile_height - 5 {
        for cc in 5..tile_width - 5 {
            let index = rr * ts + cc;
            let vertical_variance = three_tap_variance(
                workspace.vertical_difference[index - 2 * ts],
                workspace.vertical_difference[index],
                workspace.vertical_difference[index + 2 * ts],
            );
            let vertical_alternative_variance = three_tap_variance(
                workspace.vertical_alternative[index - 2 * ts],
                workspace.vertical_alternative[index],
                workspace.vertical_alternative[index + 2 * ts],
            );
            if vertical_alternative_variance < vertical_variance {
                workspace.weight_scratch[index] = workspace.vertical_alternative[index];
            }
        }
    }
    std::mem::swap(
        &mut workspace.vertical_difference,
        &mut workspace.weight_scratch,
    );

    for rr in 4..tile_height - 4 {
        for cc in 4..tile_width - 4 {
            let index = rr * ts + cc;
            if !is_green(pattern, cc, rr) {
                workspace.difference_squared[index] = square(
                    workspace.vertical_difference[index] - workspace.horizontal_difference[index],
                );
            }
        }
    }
}

fn three_tap_variance<T>(prev: T, curr: T, next: T) -> T
where
    T: RawFloat,
{
    num!(3.0) * (square(prev) + square(curr) + square(next)) - square(prev + curr + next)
}

/// Sign-safe directional color ratio without zero-crossing poles.
///
/// Returns `0.0` outside same-sign regimes so `adaptive_green` falls back
/// to the Hamilton-Adams additive estimate.
fn color_ratio<T>(
    center: T,
    neighbor: T,
    neighbor_weight: T,
    two_away: T,
    center_weight: T,
    eps: T,
) -> T
where
    T: RawFloat,
{
    let same_endpoint_sign = (center > num!(0.0) && two_away > num!(0.0))
        || (center < num!(0.0) && two_away < num!(0.0));
    if !same_endpoint_sign {
        return num!(0.0);
    }
    let denom = neighbor_weight * center + center_weight * two_away;
    if denom.abs() <= eps || (denom * neighbor) <= num!(0.0) {
        return num!(0.0);
    }
    neighbor * (neighbor_weight + center_weight) / denom
}

fn diagonal_ratio<T>(neighbor: T, center: T, two_away: T, eps: T) -> T
where
    T: RawFloat,
{
    let same_endpoint_sign = (center > num!(0.0) && two_away > num!(0.0))
        || (center < num!(0.0) && two_away < num!(0.0));
    if !same_endpoint_sign {
        return num!(0.0);
    }
    let denom = center + two_away;
    if denom.abs() <= eps || (denom * neighbor) <= num!(0.0) {
        return num!(0.0);
    }
    mul_exponent(neighbor) / denom
}

fn interpolate<T>(weight: T, first: T, second: T) -> T
where
    T: RawFloat,
{
    weight * (first - second) + second
}

fn half<T>(value: T) -> T
where
    T: RawFloat,
{
    value * num!(0.5)
}

fn div_exponent<T>(value: T, exponent: i32) -> T
where
    T: RawFloat,
{
    let factor = 0.5_f64.powi(exponent);
    value * num!(factor)
}

fn mul_exponent<T>(value: T) -> T
where
    T: RawFloat,
{
    value * num!(2.0)
}

fn ulim<T>(value: T, left: T, right: T) -> T
where
    T: RawFloat,
{
    let min = left.min(right);
    let max = left.max(right);
    if value < min {
        min
    } else if value > max {
        max
    } else {
        value
    }
}

fn square<T>(value: T) -> T
where
    T: RawFloat,
{
    value * value
}

const fn is_green(pattern: BayerPattern, col: usize, row: usize) -> bool {
    matches!(
        pattern.channel_at(col as u32, row as u32),
        BayerChannel::GRedRow | BayerChannel::GBlueRow
    )
}

#[derive(Clone, Copy)]
struct NyquistRegion {
    row_start: usize,
    row_end: usize,
    col_start: usize,
    col_end: usize,
}

fn calculate_horizontal_vertical_weights<T>(
    workspace: &mut Workspace<T>,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) where
    T: RawFloat,
{
    let ts = workspace.tile_stride;
    for rr in 6..tile_height - 6 {
        let start = 6 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 6).step_by(2) {
            let index = rr * ts + cc;
            let up_average = div_exponent(
                workspace.vertical_difference[index]
                    + workspace.vertical_difference[index - ts]
                    + workspace.vertical_difference[index - 2 * ts]
                    + workspace.vertical_difference[index - 3 * ts],
                2,
            );
            let down_average = div_exponent(
                workspace.vertical_difference[index]
                    + workspace.vertical_difference[index + ts]
                    + workspace.vertical_difference[index + 2 * ts]
                    + workspace.vertical_difference[index + 3 * ts],
                2,
            );
            let left_average = div_exponent(
                workspace.horizontal_difference[index]
                    + workspace.horizontal_difference[index - 1]
                    + workspace.horizontal_difference[index - 2]
                    + workspace.horizontal_difference[index - 3],
                2,
            );
            let right_average = div_exponent(
                workspace.horizontal_difference[index]
                    + workspace.horizontal_difference[index + 1]
                    + workspace.horizontal_difference[index + 2]
                    + workspace.horizontal_difference[index + 3],
                2,
            );

            let up_variance = square(workspace.vertical_difference[index] - up_average)
                + square(workspace.vertical_difference[index - ts] - up_average)
                + square(workspace.vertical_difference[index - 2 * ts] - up_average)
                + square(workspace.vertical_difference[index - 3 * ts] - up_average);
            let down_variance = square(workspace.vertical_difference[index] - down_average)
                + square(workspace.vertical_difference[index + ts] - down_average)
                + square(workspace.vertical_difference[index + 2 * ts] - down_average)
                + square(workspace.vertical_difference[index + 3 * ts] - down_average);
            let left_variance = square(workspace.horizontal_difference[index] - left_average)
                + square(workspace.horizontal_difference[index - 1] - left_average)
                + square(workspace.horizontal_difference[index - 2] - left_average)
                + square(workspace.horizontal_difference[index - 3] - left_average);
            let right_variance = square(workspace.horizontal_difference[index] - right_average)
                + square(workspace.horizontal_difference[index + 1] - right_average)
                + square(workspace.horizontal_difference[index + 2] - right_average)
                + square(workspace.horizontal_difference[index + 3] - right_average);

            let horizontal_weight = directional_weight(
                workspace.horizontal_weights[index - 1],
                workspace.horizontal_weights[index + 1],
            );
            let vertical_weight = directional_weight(
                workspace.vertical_weights[index - ts],
                workspace.vertical_weights[index + ts],
            );
            let vertical_color_variance = workspace.eps_squared
                + vertical_weight * down_variance
                + (num!(1.0) - vertical_weight) * up_variance;
            let horizontal_color_variance = workspace.eps_squared
                + horizontal_weight * right_variance
                + (num!(1.0) - horizontal_weight) * left_variance;

            let up_fluctuation = workspace.vertical_green_delta[index]
                + workspace.vertical_green_delta[index - ts]
                + workspace.vertical_green_delta[index - 2 * ts];
            let down_fluctuation = workspace.vertical_green_delta[index]
                + workspace.vertical_green_delta[index + ts]
                + workspace.vertical_green_delta[index + 2 * ts];
            let left_fluctuation = workspace.horizontal_green_delta[index]
                + workspace.horizontal_green_delta[index - 1]
                + workspace.horizontal_green_delta[index - 2];
            let right_fluctuation = workspace.horizontal_green_delta[index]
                + workspace.horizontal_green_delta[index + 1]
                + workspace.horizontal_green_delta[index + 2];
            let vertical_fluctuation = workspace.eps_squared
                + vertical_weight * down_fluctuation
                + (num!(1.0) - vertical_weight) * up_fluctuation;
            let horizontal_fluctuation = workspace.eps_squared
                + horizontal_weight * right_fluctuation
                + (num!(1.0) - horizontal_weight) * left_fluctuation;

            let variance_weight =
                horizontal_color_variance / (vertical_color_variance + horizontal_color_variance);
            let fluctuation_weight =
                horizontal_fluctuation / (vertical_fluctuation + horizontal_fluctuation);
            workspace.horizontal_vertical_weight[index] =
                if (num!(0.5) - variance_weight) * (num!(0.5) - fluctuation_weight) > num!(0.0)
                    && (num!(0.5) - fluctuation_weight).abs() < (num!(0.5) - variance_weight).abs()
                {
                    variance_weight
                } else {
                    fluctuation_weight
                };
        }
    }
}

fn detect_nyquist_texture<T>(
    workspace: &mut Workspace<T>,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) -> Option<NyquistRegion>
where
    T: RawFloat,
{
    let ts = workspace.tile_stride;
    let m1 = ts + 1;
    let m2 = 2 * ts + 2;
    let p1 = 1_isize - ts as isize;
    let p2 = 2_isize - 2 * ts as isize;
    for rr in 6..tile_height - 6 {
        let start = 6 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 6).step_by(2) {
            let index = rr * ts + cc;
            let index_p1 = offset(index, p1);
            let index_p2 = offset(index, p2);
            let index_m1 = offset(index, -p1);
            let index_m2 = offset(index, -p2);
            workspace.nyquist_test[index] = num!(GAUSS_ODD[0])
                * workspace.difference_squared[index]
                + num!(GAUSS_ODD[1])
                    * (workspace.difference_squared[index - m1]
                        + workspace.difference_squared[index_p1]
                        + workspace.difference_squared[index_m1]
                        + workspace.difference_squared[index + m1])
                + num!(GAUSS_ODD[2])
                    * (workspace.difference_squared[index - 2 * ts]
                        + workspace.difference_squared[index - 2]
                        + workspace.difference_squared[index + 2]
                        + workspace.difference_squared[index + 2 * ts])
                + num!(GAUSS_ODD[3])
                    * (workspace.difference_squared[index - m2]
                        + workspace.difference_squared[index_p2]
                        + workspace.difference_squared[index_m2]
                        + workspace.difference_squared[index + m2])
                - (num!(GAUSS_GRADIENT[0]) * workspace.gradient_squared_sum[index]
                    + num!(GAUSS_GRADIENT[1])
                        * (workspace.gradient_squared_sum[index - ts]
                            + workspace.gradient_squared_sum[index + 1]
                            + workspace.gradient_squared_sum[index - 1]
                            + workspace.gradient_squared_sum[index + ts])
                    + num!(GAUSS_GRADIENT[2])
                        * (workspace.gradient_squared_sum[index - m1]
                            + workspace.gradient_squared_sum[index_p1]
                            + workspace.gradient_squared_sum[index_m1]
                            + workspace.gradient_squared_sum[index + m1])
                    + num!(GAUSS_GRADIENT[3])
                        * (workspace.gradient_squared_sum[index - 2 * ts]
                            + workspace.gradient_squared_sum[index - 2]
                            + workspace.gradient_squared_sum[index + 2]
                            + workspace.gradient_squared_sum[index + 2 * ts])
                    + num!(GAUSS_GRADIENT[4])
                        * (workspace.gradient_squared_sum[index - 2 * ts - 1]
                            + workspace.gradient_squared_sum[index - 2 * ts + 1]
                            + workspace.gradient_squared_sum[index - ts - 2]
                            + workspace.gradient_squared_sum[index - ts + 2]
                            + workspace.gradient_squared_sum[index + ts - 2]
                            + workspace.gradient_squared_sum[index + ts + 2]
                            + workspace.gradient_squared_sum[index + 2 * ts - 1]
                            + workspace.gradient_squared_sum[index + 2 * ts + 1])
                    + num!(GAUSS_GRADIENT[5])
                        * (workspace.gradient_squared_sum[index - m2]
                            + workspace.gradient_squared_sum[index_p2]
                            + workspace.gradient_squared_sum[index_m2]
                            + workspace.gradient_squared_sum[index + m2]));
        }
    }

    let mut start_row = 0;
    let mut end_row = 0;
    let mut start_col = ts + 1;
    let mut end_col = 0;
    for rr in 6..tile_height - 6 {
        let start = 6 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 6).step_by(2) {
            let index = rr * ts + cc;
            if workspace.nyquist_test[index] > num!(0.0) {
                workspace.nyquist[index] = 1;
                if start_row == 0 {
                    start_row = rr;
                }
                end_row = rr;
                start_col = start_col.min(cc);
                end_col = end_col.max(cc);
            }
        }
    }

    if start_row == end_row || start_col == end_col {
        return None;
    }
    let row_start = start_row.max(8);
    let col_start = (start_col & !1).max(8);
    let row_end = (end_row + 1).min(tile_height - 8);
    let col_end = (end_col + 1).min(tile_width - 8);
    if row_start >= row_end || col_start >= col_end {
        return None;
    }
    Some(NyquistRegion {
        row_start,
        row_end,
        col_start,
        col_end,
    })
}

fn refine_nyquist_texture<T>(
    workspace: &mut Workspace<T>,
    region: NyquistRegion,
    pattern: BayerPattern,
) where
    T: RawFloat,
{
    let ts = workspace.tile_stride;
    let m1 = ts + 1;
    let p1 = 1_isize - ts as isize;
    for rr in region.row_start..region.row_end {
        let start = region.col_start + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..region.col_end).step_by(2) {
            let index = rr * ts + cc;
            let count = workspace.nyquist[index - 2 * ts]
                + workspace.nyquist[index - m1]
                + workspace.nyquist[offset(index, p1)]
                + workspace.nyquist[index - 2]
                + workspace.nyquist[index + 2]
                + workspace.nyquist[offset(index, -p1)]
                + workspace.nyquist[index + m1]
                + workspace.nyquist[index + 2 * ts];
            workspace.nyquist_refined[index] = match count {
                0..=3 => 0,
                5..=u8::MAX => 1,
                _ => workspace.nyquist[index],
            };
        }
    }

    for rr in region.row_start..region.row_end {
        let start = region.col_start + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..region.col_end).step_by(2) {
            let index = rr * ts + cc;
            if workspace.nyquist_refined[index] == 0 {
                continue;
            }
            let mut cfa_sum = num!(0.0);
            let mut horizontal_sum = num!(0.0);
            let mut vertical_sum = num!(0.0);
            let mut horizontal_squared_sum = num!(0.0);
            let mut vertical_squared_sum = num!(0.0);
            let mut area_weight = num!(0.0);
            for dy in (-6_isize..=6).step_by(2) {
                for dx in (-6_isize..=6).step_by(2) {
                    let sample = offset(index, dy * ts as isize + dx);
                    if workspace.nyquist_refined[sample] == 0 {
                        continue;
                    }
                    let value = workspace.cfa[sample];
                    cfa_sum = cfa_sum + value;
                    horizontal_sum =
                        horizontal_sum + workspace.cfa[sample - 1] + workspace.cfa[sample + 1];
                    vertical_sum =
                        vertical_sum + workspace.cfa[sample - ts] + workspace.cfa[sample + ts];
                    horizontal_squared_sum = horizontal_squared_sum
                        + square(value - workspace.cfa[sample - 1])
                        + square(value - workspace.cfa[sample + 1]);
                    vertical_squared_sum = vertical_squared_sum
                        + square(value - workspace.cfa[sample - ts])
                        + square(value - workspace.cfa[sample + ts]);
                    area_weight = area_weight + num!(1.0);
                }
            }
            horizontal_sum = cfa_sum - half(horizontal_sum);
            vertical_sum = cfa_sum - half(vertical_sum);
            area_weight = half(area_weight);
            let horizontal_variance = workspace.eps_squared
                + (area_weight * horizontal_squared_sum - horizontal_sum * horizontal_sum).abs();
            let vertical_variance = workspace.eps_squared
                + (area_weight * vertical_squared_sum - vertical_sum * vertical_sum).abs();
            workspace.horizontal_vertical_weight[index] =
                horizontal_variance / (vertical_variance + horizontal_variance);
        }
    }
}

fn interpolate_green_at_red_blue<T>(
    workspace: &mut Workspace<T>,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) where
    T: RawFloat,
{
    let ts = workspace.tile_stride;
    let m1 = ts + 1;
    let p1 = 1_isize - ts as isize;
    let len = tile_height * tile_width;
    workspace.weight_scratch[..len].copy_from_slice(&workspace.horizontal_vertical_weight[..len]);
    for rr in 8..tile_height - 8 {
        let start = 8 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 8).step_by(2) {
            let index = rr * ts + cc;
            let alternate = div_exponent(
                workspace.horizontal_vertical_weight[index - m1]
                    + workspace.horizontal_vertical_weight[offset(index, p1)]
                    + workspace.horizontal_vertical_weight[offset(index, -p1)]
                    + workspace.horizontal_vertical_weight[index + m1],
                2,
            );
            if (num!(0.5) - workspace.horizontal_vertical_weight[index]).abs()
                < (num!(0.5) - alternate).abs()
            {
                workspace.weight_scratch[index] = alternate;
            }
        }
    }
    std::mem::swap(
        &mut workspace.horizontal_vertical_weight,
        &mut workspace.weight_scratch,
    );

    for rr in 8..tile_height - 8 {
        let start = 8 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 8).step_by(2) {
            let index = rr * ts + cc;
            workspace.green_minus_red[index] = interpolate(
                workspace.horizontal_vertical_weight[index],
                workspace.vertical_difference[index],
                workspace.horizontal_difference[index],
            );
            workspace.green[index] = workspace.cfa[index] + workspace.green_minus_red[index];
            if workspace.nyquist_refined[index] != 0 {
                workspace.green_curvature[index][0] = square(
                    workspace.green[index]
                        - half(workspace.green[index - 1] + workspace.green[index + 1]),
                );
                workspace.green_curvature[index][1] = square(
                    workspace.green[index]
                        - half(workspace.green[index - ts] + workspace.green[index + ts]),
                );
            } else {
                workspace.green_curvature[index] = [num!(0.0); 2];
            }
        }
    }
}

fn refine_nyquist_green<T>(
    workspace: &mut Workspace<T>,
    region: NyquistRegion,
    pattern: BayerPattern,
) where
    T: RawFloat,
{
    let ts = workspace.tile_stride as isize;
    let m1 = ts + 1;
    let m2 = 2 * ts + 2;
    for rr in region.row_start..region.row_end {
        let start = region.col_start + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..region.col_end).step_by(2) {
            let index = rr * ts as usize + cc;
            if workspace.nyquist_refined[index] == 0 {
                continue;
            }
            let mut horizontal_variance = workspace.eps_squared;
            let mut vertical_variance = workspace.eps_squared;
            horizontal_variance =
                horizontal_variance + num!(GAUSS_QUINCUNX[0]) * workspace.green_curvature[index][0];
            vertical_variance =
                vertical_variance + num!(GAUSS_QUINCUNX[0]) * workspace.green_curvature[index][1];
            for delta in [-m1, 1 - ts, ts - 1, m1] {
                let sample = offset(index, delta);
                horizontal_variance = horizontal_variance
                    + num!(GAUSS_QUINCUNX[1]) * workspace.green_curvature[sample][0];
                vertical_variance = vertical_variance
                    + num!(GAUSS_QUINCUNX[1]) * workspace.green_curvature[sample][1];
            }
            for delta in [-2 * ts, -2, 2, 2 * ts] {
                let sample = offset(index, delta);
                horizontal_variance = horizontal_variance
                    + num!(GAUSS_QUINCUNX[2]) * workspace.green_curvature[sample][0];
                vertical_variance = vertical_variance
                    + num!(GAUSS_QUINCUNX[2]) * workspace.green_curvature[sample][1];
            }
            for delta in [-m2, 2 - 2 * ts, 2 * ts - 2, m2] {
                let sample = offset(index, delta);
                horizontal_variance = horizontal_variance
                    + num!(GAUSS_QUINCUNX[3]) * workspace.green_curvature[sample][0];
                vertical_variance = vertical_variance
                    + num!(GAUSS_QUINCUNX[3]) * workspace.green_curvature[sample][1];
            }
            workspace.green_minus_red[index] = (workspace.horizontal_difference[index]
                * vertical_variance
                + workspace.vertical_difference[index] * horizontal_variance)
                / (vertical_variance + horizontal_variance);
            workspace.green[index] = workspace.cfa[index] + workspace.green_minus_red[index];
        }
    }
}

const fn offset(index: usize, delta: isize) -> usize { (index as isize + delta) as usize }

fn prepare_diagonal_interpolation<T>(
    workspace: &mut Workspace<T>,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) where
    T: RawFloat,
{
    let ts = workspace.tile_stride;
    let p1 = 1_isize - ts as isize;
    let m1 = ts as isize + 1;
    for rr in 6..tile_height - 6 {
        let green_start = 6 + usize::from(!is_green(pattern, 6, rr));
        for cc in (green_start..tile_width - 6).step_by(2) {
            let index = rr * ts + cc;
            workspace.diagonal_plus_variance[index] =
                square(workspace.cfa[index] - workspace.cfa[offset(index, -p1)])
                    + square(workspace.cfa[index] - workspace.cfa[offset(index, p1)]);
            workspace.diagonal_minus_variance[index] =
                square(workspace.cfa[index] - workspace.cfa[offset(index, -m1)])
                    + square(workspace.cfa[index] - workspace.cfa[offset(index, m1)]);
        }

        let chroma_start = 6 + usize::from(is_green(pattern, 6, rr));
        for cc in (chroma_start..tile_width - 6).step_by(2) {
            let index = rr * ts + cc;
            workspace.plus_gradient[index] =
                (workspace.cfa[offset(index, p1)] - workspace.cfa[offset(index, -p1)]).abs();
            workspace.minus_gradient[index] =
                (workspace.cfa[offset(index, m1)] - workspace.cfa[offset(index, -m1)]).abs();
        }
    }
}

fn interpolate_diagonal_red_blue<T>(
    workspace: &mut Workspace<T>,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) where
    T: RawFloat,
{
    let ts = workspace.tile_stride;
    let p1 = 1_isize - ts as isize;
    let p2 = 2_isize - 2 * ts as isize;
    let m1 = ts as isize + 1;
    let m2 = 2 * ts as isize + 2;
    for rr in 8..tile_height - 8 {
        let start = 8 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 8).step_by(2) {
            let index = rr * ts + cc;
            let center = workspace.cfa[index];
            let ratio_se = diagonal_ratio(
                workspace.cfa[offset(index, m1)],
                center,
                workspace.cfa[offset(index, m2)],
                workspace.eps,
            );
            let ratio_nw = diagonal_ratio(
                workspace.cfa[offset(index, -m1)],
                center,
                workspace.cfa[offset(index, -m2)],
                workspace.eps,
            );
            let ratio_ne = diagonal_ratio(
                workspace.cfa[offset(index, p1)],
                center,
                workspace.cfa[offset(index, p2)],
                workspace.eps,
            );
            let ratio_sw = diagonal_ratio(
                workspace.cfa[offset(index, -p1)],
                center,
                workspace.cfa[offset(index, -p2)],
                workspace.eps,
            );

            let rb_se = adaptive_green(
                center,
                ratio_se,
                workspace.cfa[offset(index, m1)] + half(center - workspace.cfa[offset(index, m2)]),
            );
            let rb_nw = adaptive_green(
                center,
                ratio_nw,
                workspace.cfa[offset(index, -m1)]
                    + half(center - workspace.cfa[offset(index, -m2)]),
            );
            let rb_ne = adaptive_green(
                center,
                ratio_ne,
                workspace.cfa[offset(index, p1)] + half(center - workspace.cfa[offset(index, p2)]),
            );
            let rb_sw = adaptive_green(
                center,
                ratio_sw,
                workspace.cfa[offset(index, -p1)]
                    + half(center - workspace.cfa[offset(index, -p2)]),
            );

            let weight_se = workspace.eps
                + workspace.minus_gradient[index]
                + workspace.minus_gradient[offset(index, m1)]
                + workspace.minus_gradient[offset(index, m2)];
            let weight_nw = workspace.eps
                + workspace.minus_gradient[index]
                + workspace.minus_gradient[offset(index, -m1)]
                + workspace.minus_gradient[offset(index, -m2)];
            let weight_ne = workspace.eps
                + workspace.plus_gradient[index]
                + workspace.plus_gradient[offset(index, p1)]
                + workspace.plus_gradient[offset(index, p2)];
            let weight_sw = workspace.eps
                + workspace.plus_gradient[index]
                + workspace.plus_gradient[offset(index, -p1)]
                + workspace.plus_gradient[offset(index, -p2)];
            workspace.red_blue_minus[index] =
                (weight_se * rb_nw + weight_nw * rb_se) / (weight_se + weight_nw);
            workspace.red_blue_plus[index] =
                (weight_ne * rb_sw + weight_sw * rb_ne) / (weight_ne + weight_sw);

            let minus_variance = workspace.eps_squared
                + diagonal_variance(&workspace.diagonal_minus_variance, index, ts);
            let plus_variance = workspace.eps_squared
                + diagonal_variance(&workspace.diagonal_plus_variance, index, ts);

            clamp_diagonal(workspace, index, center, true);
            clamp_diagonal(workspace, index, center, false);

            let rb_variance_weight = minus_variance / (plus_variance + minus_variance);
            let gradient_weight = (num!(0.1) + square(weight_se - weight_nw))
                / (num!(0.2) + square(weight_se - weight_nw) + square(weight_ne - weight_sw));
            workspace.plus_minus_weight[index] =
                if (num!(0.5) - gradient_weight) * (num!(0.5) - rb_variance_weight) > num!(0.0)
                    && (num!(0.5) - gradient_weight).abs() < (num!(0.5) - rb_variance_weight).abs()
                {
                    rb_variance_weight
                } else {
                    gradient_weight
                };
        }
    }
}

fn diagonal_variance<T>(plane: &[T], index: usize, ts: usize) -> T
where
    T: RawFloat,
{
    plane[index - 1]
        + plane[index + 1]
        + plane[index - ts]
        + plane[index + ts]
        + plane[index - 2 * ts - 1]
        + plane[index - 2 * ts + 1]
        + plane[index - ts - 2]
        + plane[index - ts + 2]
        + plane[index + ts - 2]
        + plane[index + ts + 2]
        + plane[index + 2 * ts - 1]
        + plane[index + 2 * ts + 1]
}

/// Sign-safe, unclamped neighbor overshoot limiter shared by diagonal and
/// green refinement steps.
fn clamp_toward_neighbors<T>(value: T, reference: T, first: T, second: T, eps: T) -> T
where
    T: RawFloat,
{
    let same_sign = (value > num!(0.0) && reference > num!(0.0))
        || (value < num!(0.0) && reference < num!(0.0));
    if !same_sign || value.abs() >= reference.abs() {
        return value;
    }

    let bounded = ulim(value, first, second);
    if mul_exponent(value.abs()) < reference.abs() {
        return bounded;
    }
    let sum_abs = (value + reference).abs();
    if sum_abs <= eps {
        return bounded;
    }
    let weight = (mul_exponent((reference - value).abs()) / sum_abs).min(num!(1.0));
    weight * value + (num!(1.0) - weight) * bounded
}

fn clamp_diagonal<T>(workspace: &mut Workspace<T>, index: usize, center: T, plus: bool)
where
    T: RawFloat,
{
    let ts = workspace.tile_stride as isize;
    let p1 = 1 - ts;
    let m1 = ts + 1;
    let (value, first, second) = if plus {
        (
            workspace.red_blue_plus[index],
            workspace.cfa[offset(index, p1)],
            workspace.cfa[offset(index, -p1)],
        )
    } else {
        (
            workspace.red_blue_minus[index],
            workspace.cfa[offset(index, m1)],
            workspace.cfa[offset(index, -m1)],
        )
    };
    let result = clamp_toward_neighbors(value, center, first, second, workspace.eps);
    if plus {
        workspace.red_blue_plus[index] = result;
    } else {
        workspace.red_blue_minus[index] = result;
    }
}

fn refine_green_from_red_blue<T>(
    workspace: &mut Workspace<T>,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) where
    T: RawFloat,
{
    let ts = workspace.tile_stride;
    let p1 = 1_isize - ts as isize;
    let m1 = ts as isize + 1;
    let len = tile_height * tile_width;
    workspace.weight_scratch[..len].copy_from_slice(&workspace.plus_minus_weight[..len]);
    for rr in 10..tile_height - 10 {
        let start = 10 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 10).step_by(2) {
            let index = rr * ts + cc;
            let alternate = div_exponent(
                workspace.plus_minus_weight[offset(index, -m1)]
                    + workspace.plus_minus_weight[offset(index, p1)]
                    + workspace.plus_minus_weight[offset(index, -p1)]
                    + workspace.plus_minus_weight[offset(index, m1)],
                2,
            );
            if (num!(0.5) - workspace.plus_minus_weight[index]).abs()
                < (num!(0.5) - alternate).abs()
            {
                workspace.weight_scratch[index] = alternate;
            }
        }
    }
    std::mem::swap(
        &mut workspace.plus_minus_weight,
        &mut workspace.weight_scratch,
    );

    for rr in 10..tile_height - 10 {
        let start = 10 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 10).step_by(2) {
            let index = rr * ts + cc;
            workspace.red_blue_interpolated[index] = half(
                workspace.cfa[index]
                    + workspace.red_blue_minus[index]
                        * (num!(1.0) - workspace.plus_minus_weight[index])
                    + workspace.red_blue_plus[index] * workspace.plus_minus_weight[index],
            );
        }
    }

    for rr in 12..tile_height - 12 {
        let start = 12 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 12).step_by(2) {
            let index = rr * ts + cc;
            if (num!(0.5) - workspace.plus_minus_weight[index]).abs()
                < (num!(0.5) - workspace.horizontal_vertical_weight[index]).abs()
            {
                continue;
            }
            let rb = workspace.red_blue_interpolated[index];
            let ratio_up = diagonal_ratio(
                workspace.cfa[index - ts],
                rb,
                workspace.red_blue_interpolated[index - 2 * ts],
                workspace.eps,
            );
            let ratio_down = diagonal_ratio(
                workspace.cfa[index + ts],
                rb,
                workspace.red_blue_interpolated[index + 2 * ts],
                workspace.eps,
            );
            let ratio_left = diagonal_ratio(
                workspace.cfa[index - 1],
                rb,
                workspace.red_blue_interpolated[index - 2],
                workspace.eps,
            );
            let ratio_right = diagonal_ratio(
                workspace.cfa[index + 1],
                rb,
                workspace.red_blue_interpolated[index + 2],
                workspace.eps,
            );

            let green_up = adaptive_green(
                rb,
                ratio_up,
                workspace.cfa[index - ts]
                    + half(rb - workspace.red_blue_interpolated[index - 2 * ts]),
            );
            let green_down = adaptive_green(
                rb,
                ratio_down,
                workspace.cfa[index + ts]
                    + half(rb - workspace.red_blue_interpolated[index + 2 * ts]),
            );
            let green_left = adaptive_green(
                rb,
                ratio_left,
                workspace.cfa[index - 1] + half(rb - workspace.red_blue_interpolated[index - 2]),
            );
            let green_right = adaptive_green(
                rb,
                ratio_right,
                workspace.cfa[index + 1] + half(rb - workspace.red_blue_interpolated[index + 2]),
            );

            let vertical_weight = directional_weight(
                workspace.vertical_weights[index - ts],
                workspace.vertical_weights[index + ts],
            );
            let horizontal_weight = directional_weight(
                workspace.horizontal_weights[index - 1],
                workspace.horizontal_weights[index + 1],
            );
            let green_vertical = interpolate(vertical_weight, green_down, green_up);
            let green_horizontal = interpolate(horizontal_weight, green_right, green_left);

            let green_vertical = clamp_toward_neighbors(
                green_vertical,
                rb,
                workspace.cfa[index - ts],
                workspace.cfa[index + ts],
                workspace.eps,
            );
            let green_horizontal = clamp_toward_neighbors(
                green_horizontal,
                rb,
                workspace.cfa[index - 1],
                workspace.cfa[index + 1],
                workspace.eps,
            );

            workspace.green_minus_red[index] = interpolate(
                workspace.horizontal_vertical_weight[index],
                green_vertical,
                green_horizontal,
            ) - workspace.cfa[index];
        }
    }
}

fn split_green_differences<T>(
    workspace: &mut Workspace<T>,
    pattern: BayerPattern,
    tile_height: usize,
    tile_width: usize,
) where
    T: RawFloat,
{
    let ts = workspace.tile_stride;
    let (ex, ey) = red_phase(pattern);
    let blue_row = 1 - ey;
    let blue_col = 1 - ex;
    let start_row = 11 + ((11 ^ blue_row) & 1);
    let start_col = 11 + ((11 ^ blue_col) & 1);
    for rr in (start_row..tile_height - 11).step_by(2) {
        for cc in (start_col..tile_width - 11).step_by(2) {
            let index = rr * ts + cc;
            workspace.green_minus_blue[index] = workspace.green_minus_red[index];
            workspace.green_minus_red[index] = num!(0.0);
        }
    }
}

fn interpolate_chroma<T>(
    workspace: &mut Workspace<T>,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) where
    T: RawFloat,
{
    let ts = workspace.tile_stride;
    let p1 = 1_isize - ts as isize;
    let p3 = 3_isize - 3 * ts as isize;
    let m1 = ts as isize + 1;
    let m3 = 3 * ts as isize + 3;
    for rr in 14..tile_height - 14 {
        let start = 14 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 14).step_by(2) {
            let index = rr * ts + cc;
            let c = 1 - channel(pattern, cc as u32, rr as u32) / 2;
            let values = if c == 0 {
                &workspace.green_minus_red
            } else {
                &workspace.green_minus_blue
            };
            let weight_nw = num!(1.0)
                / (workspace.eps
                    + (values[offset(index, -m1)] - values[offset(index, m1)]).abs()
                    + (values[offset(index, -m1)] - values[offset(index, -m3)]).abs()
                    + (values[offset(index, m1)] - values[offset(index, -m3)]).abs());
            let weight_ne = num!(1.0)
                / (workspace.eps
                    + (values[offset(index, p1)] - values[offset(index, -p1)]).abs()
                    + (values[offset(index, p1)] - values[offset(index, p3)]).abs()
                    + (values[offset(index, -p1)] - values[offset(index, p3)]).abs());
            let weight_sw = num!(1.0)
                / (workspace.eps
                    + (values[offset(index, -p1)] - values[offset(index, p1)]).abs()
                    + (values[offset(index, -p1)] - values[offset(index, -p3)]).abs()
                    + (values[offset(index, p1)] - values[offset(index, -p3)]).abs());
            let weight_se = num!(1.0)
                / (workspace.eps
                    + (values[offset(index, m1)] - values[offset(index, -m1)]).abs()
                    + (values[offset(index, m1)] - values[offset(index, m3)]).abs()
                    + (values[offset(index, -m1)] - values[offset(index, m3)]).abs());
            let value = (weight_nw
                * (num!(CHROMA_TAP_CENTER) * values[offset(index, -m1)]
                    - num!(CHROMA_TAP_FAR) * values[offset(index, -m3)]
                    - num!(CHROMA_TAP_SIDE) * values[offset(index, -m1 - 2)]
                    - num!(CHROMA_TAP_SIDE) * values[offset(index, -m1 - 2 * ts as isize)])
                + weight_ne
                    * (num!(CHROMA_TAP_CENTER) * values[offset(index, p1)]
                        - num!(CHROMA_TAP_FAR) * values[offset(index, p3)]
                        - num!(CHROMA_TAP_SIDE) * values[offset(index, p1 + 2)]
                        - num!(CHROMA_TAP_SIDE) * values[offset(index, p1 - 2 * ts as isize)])
                + weight_sw
                    * (num!(CHROMA_TAP_CENTER) * values[offset(index, -p1)]
                        - num!(CHROMA_TAP_FAR) * values[offset(index, -p3)]
                        - num!(CHROMA_TAP_SIDE) * values[offset(index, -p1 - 2)]
                        - num!(CHROMA_TAP_SIDE) * values[offset(index, -p1 + 2 * ts as isize)])
                + weight_se
                    * (num!(CHROMA_TAP_CENTER) * values[offset(index, m1)]
                        - num!(CHROMA_TAP_FAR) * values[offset(index, m3)]
                        - num!(CHROMA_TAP_SIDE) * values[offset(index, m1 + 2)]
                        - num!(CHROMA_TAP_SIDE) * values[offset(index, m1 + 2 * ts as isize)]))
                / (weight_nw + weight_ne + weight_sw + weight_se);
            if c == 0 {
                workspace.green_minus_red[index] = value;
            } else {
                workspace.green_minus_blue[index] = value;
            }
        }
    }
}

fn write_tile_rgb<T>(
    output: &mut [T],
    workspace: &Workspace<T>,
    core_h: usize,
    core_w: usize,
    pattern: BayerPattern,
    scale: T,
) where
    T: RawFloat,
{
    let ts = workspace.tile_stride;
    for ly in 0..core_h {
        let rr = BORDER + ly;
        for lx in 0..core_w {
            let cc = BORDER + lx;
            let index = rr * ts + cc;
            let pixel = (ly * core_w + lx) * 3;
            if is_green(pattern, cc, rr) {
                let w_up = workspace.horizontal_vertical_weight[index - ts];
                let w_down = workspace.horizontal_vertical_weight[index + ts];
                let w_left = num!(1.0) - workspace.horizontal_vertical_weight[index - 1];
                let w_right = num!(1.0) - workspace.horizontal_vertical_weight[index + 1];
                let denom = w_up + w_down + w_left + w_right;

                let red_diff = if denom.abs() > workspace.eps {
                    (w_up * workspace.green_minus_red[index - ts]
                        + w_right * workspace.green_minus_red[index + 1]
                        + w_left * workspace.green_minus_red[index - 1]
                        + w_down * workspace.green_minus_red[index + ts])
                        / denom
                } else {
                    div_exponent(
                        workspace.green_minus_red[index - ts]
                            + workspace.green_minus_red[index + 1]
                            + workspace.green_minus_red[index - 1]
                            + workspace.green_minus_red[index + ts],
                        2,
                    )
                };
                let blue_diff = if denom.abs() > workspace.eps {
                    (w_up * workspace.green_minus_blue[index - ts]
                        + w_right * workspace.green_minus_blue[index + 1]
                        + w_left * workspace.green_minus_blue[index - 1]
                        + w_down * workspace.green_minus_blue[index + ts])
                        / denom
                } else {
                    div_exponent(
                        workspace.green_minus_blue[index - ts]
                            + workspace.green_minus_blue[index + 1]
                            + workspace.green_minus_blue[index - 1]
                            + workspace.green_minus_blue[index + ts],
                        2,
                    )
                };
                let green = workspace.cfa[index];
                output[pixel] = (green - red_diff) * scale;
                output[pixel + 1] = green * scale;
                output[pixel + 2] = (green - blue_diff) * scale;
            } else if matches!(pattern.channel_at(cc as u32, rr as u32), BayerChannel::R) {
                let red = workspace.cfa[index];
                let green = red + workspace.green_minus_red[index];
                let blue = green - workspace.green_minus_blue[index];
                output[pixel] = red * scale;
                output[pixel + 1] = green * scale;
                output[pixel + 2] = blue * scale;
            } else {
                let blue = workspace.cfa[index];
                let green = blue + workspace.green_minus_blue[index];
                let red = green - workspace.green_minus_red[index];
                output[pixel] = red * scale;
                output[pixel + 1] = green * scale;
                output[pixel + 2] = blue * scale;
            }
        }
    }
}

const fn red_phase(pattern: BayerPattern) -> (usize, usize) {
    match pattern {
        BayerPattern::RGGB => (0, 0),
        BayerPattern::GRBG => (1, 0),
        BayerPattern::GBRG => (0, 1),
        BayerPattern::BGGR => (1, 1),
    }
}

fn adaptive_green<T>(center: T, ratio: T, additive: T) -> T
where
    T: RawFloat,
{
    if ratio > num!(ADAPTIVE_RATIO_MIN) && ratio < num!(ADAPTIVE_RATIO_MAX) {
        center * ratio
    } else {
        additive
    }
}

fn directional_weight<T>(first: T, second: T) -> T
where
    T: RawFloat,
{
    first / (first + second)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::demosaic::LazyDemosaic;
    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    use crate::demosaic::test_support::format_rgb_3dp;

    fn make_test_bayer(
        width: u32,
        height: u32,
        pattern: BayerPattern,
        scale: f32,
    ) -> MonoImage<f32> {
        let mut data = Vec::with_capacity((width * height) as usize);
        for y in 0..height {
            for x in 0..width {
                let fx = x as f32 / width as f32;
                let fy = y as f32 / height as f32;
                let base =
                    0.15f32.mul_add((fy * 7.0).cos(), 0.25f32.mul_add((fx * 11.0).sin(), 0.35));
                let chroma = match pattern.channel_at(x, y) {
                    BayerChannel::R => 0.25 * fy.mul_add(3.0, fx * 5.0).cos(),
                    BayerChannel::GRedRow | BayerChannel::GBlueRow => 0.05 * (fx * 13.0).sin(),
                    BayerChannel::B => -0.2 * fy.mul_add(-6.0, fx * 4.0).sin(),
                };
                data.push((base + chroma) * scale);
            }
        }
        MonoImage::from_raw(width, height, data).unwrap()
    }

    #[test]
    fn tile_origins_use_the_expected_stride() {
        assert_eq!(
            tile_origins(200, 140, 128),
            vec![(0, 0), (0, 128), (128, 0), (128, 128)]
        );
    }

    #[test]
    fn adaptive_green_uses_ratio_only_inside_threshold() {
        assert!((adaptive_green(0.5_f64, 1.2, 0.9) - 0.6).abs() < 1e-12);
        assert!((adaptive_green(0.5_f64, 2.5, 0.9) - 0.9).abs() < 1e-12);
        assert!((adaptive_green(0.5_f64, 0.0, 0.9) - 0.9).abs() < 1e-12);
    }

    #[test]
    fn directional_weight_prefers_smaller_gradient_neighbor() {
        assert!((directional_weight(0.25_f64, 0.75) - 0.25).abs() < 1e-12);
    }

    #[test]
    fn measured_sites_are_preserved_for_every_bayer_phase() {
        for pattern in [
            BayerPattern::RGGB,
            BayerPattern::BGGR,
            BayerPattern::GRBG,
            BayerPattern::GBRG,
        ] {
            let input = make_test_bayer(48, 48, pattern, 1.0);
            let output = DemosaicAmaze.lazy(&input, pattern).unwrap().collect();
            for y in 0..input.height() {
                for x in 0..input.width() {
                    let c = channel(pattern, x, y);
                    assert_eq!(
                        output.get_pixel(x, y).0[c],
                        input.get_pixel(x, y).0[0],
                        "measured sample changed at ({x}, {y}) for {pattern:?}"
                    );
                }
            }
        }
    }

    #[test]
    fn amaze_accepts_signed_unclamped_and_hdr_inputs() {
        let (w, h) = (48_u32, 48_u32);
        let mut data = Vec::with_capacity((w * h) as usize);
        for y in 0..h {
            for x in 0..w {
                let fx = x as f32 / w as f32 - 0.5;
                let fy = y as f32 / h as f32 - 0.5;
                let v = 1.8f32.mul_add((fy * 8.0).cos(), 3.5 * (fx * 6.0).sin()) - 0.4;
                data.push(v);
            }
        }
        let bayer = MonoImage::from_raw(w, h, data).unwrap();
        let rgb = DemosaicAmaze
            .lazy(&bayer, BayerPattern::RGGB)
            .unwrap()
            .collect();
        let has_negative = rgb.as_raw().iter().any(|&v| v < -0.2);
        let has_hdr = rgb.as_raw().iter().any(|&v| v > 1.5);
        assert!(
            has_negative,
            "expected negative outputs to remain unclamped"
        );
        assert!(has_hdr, "expected HDR outputs > 1.0 to remain unclamped");
        assert!(rgb.as_raw().iter().all(|v| v.is_finite()));
    }

    #[test]
    fn amaze_is_positive_scale_equivariant() {
        let base = make_test_bayer(64, 64, BayerPattern::RGGB, 1.0);
        let scale = 17.25_f32;
        let scaled = MonoImage::from_raw(
            base.width(),
            base.height(),
            base.as_raw().iter().map(|&v| v * scale).collect(),
        )
        .unwrap();

        let out_base = DemosaicAmaze
            .lazy(&base, BayerPattern::RGGB)
            .unwrap()
            .collect();
        let out_scaled = DemosaicAmaze
            .lazy(&scaled, BayerPattern::RGGB)
            .unwrap()
            .collect();

        for (i, (&a, &b)) in out_base
            .as_raw()
            .iter()
            .zip(out_scaled.as_raw().iter())
            .enumerate()
        {
            let diff = f32::mul_add(a, scale, -b).abs();
            assert!(
                diff <= 1e-4 * scale,
                "scale equivariance mismatch at index {i}: {} vs {}",
                a * scale,
                b
            );
        }
    }

    #[test]
    fn amaze_is_sign_symmetric() {
        let mut data = Vec::with_capacity(64 * 64);
        for y in 0..64 {
            for x in 0..64 {
                let fx = x as f32 / 64.0 - 0.5;
                let fy = y as f32 / 64.0 - 0.5;
                data.push(0.6f32.mul_add(-(fy * 7.0).cos(), (fx * 9.0).sin()));
            }
        }
        let pos = MonoImage::from_raw(64, 64, data.clone()).unwrap();
        let neg = MonoImage::from_raw(64, 64, data.iter().map(|&v| -v).collect()).unwrap();

        let out_pos = DemosaicAmaze
            .lazy(&pos, BayerPattern::RGGB)
            .unwrap()
            .collect();
        let out_neg = DemosaicAmaze
            .lazy(&neg, BayerPattern::RGGB)
            .unwrap()
            .collect();

        for (i, (&p, &n)) in out_pos
            .as_raw()
            .iter()
            .zip(out_neg.as_raw().iter())
            .enumerate()
        {
            assert!(
                (p + n).abs() <= 1e-5,
                "sign symmetry mismatch at index {i}: {p} vs {n}"
            );
        }
    }

    #[test]
    fn amaze_hv_flip_equivariance() {
        let (w, h) = (48_usize, 48_usize);
        let img = make_test_bayer(w as u32, h as u32, BayerPattern::RGGB, 1.0);
        let flipped_raw: Vec<f32> = img.as_raw().iter().copied().rev().collect();
        let flipped = MonoImage::from_raw(w as u32, h as u32, flipped_raw).unwrap();

        let out = DemosaicAmaze
            .lazy(&img, BayerPattern::RGGB)
            .unwrap()
            .collect();
        let out_flipped = DemosaicAmaze
            .lazy(&flipped, BayerPattern::BGGR)
            .unwrap()
            .collect();

        for y in 4..h - 4 {
            for x in 4..w - 4 {
                let p1 = out.get_pixel(x as u32, y as u32).0;
                let p2 = out_flipped
                    .get_pixel((w - 1 - x) as u32, (h - 1 - y) as u32)
                    .0;
                for c in 0..3 {
                    assert!(
                        (p1[c] - p2[c]).abs() <= 1e-4,
                        "HV flip mismatch at ({x}, {y}) channel {c}: {} vs {}",
                        p1[c],
                        p2[c]
                    );
                }
            }
        }
    }

    #[test]
    fn amaze_tile_size_consistency() {
        let img = make_test_bayer(96, 96, BayerPattern::RGGB, 1.0);
        let out_small = DemosaicAmaze
            .lazy_with_options(&img, BayerPattern::RGGB, AmazeOptions { tile_size: 36 })
            .unwrap()
            .collect();
        let out_default = DemosaicAmaze
            .lazy_with_options(&img, BayerPattern::RGGB, AmazeOptions::DEFAULT)
            .unwrap()
            .collect();

        for (i, (&a, &b)) in out_small
            .as_raw()
            .iter()
            .zip(out_default.as_raw().iter())
            .enumerate()
        {
            assert!(
                (a - b).abs() <= 1e-4,
                "tile_size mismatch at index {i}: {a} vs {b}"
            );
        }
    }

    #[test]
    fn amaze_supports_f64() {
        let data: Vec<f64> = (0..40 * 40)
            .map(|i| {
                let x = (i % 40) as f64 / 40.0;
                let y = (i / 40) as f64 / 40.0;
                (0.3 * (x * 10.0).sin()).mul_add((y * 8.0).cos(), 0.5)
            })
            .collect();
        let img = MonoImage::from_raw(40, 40, data).unwrap();
        let rgb = DemosaicAmaze
            .lazy(&img, BayerPattern::GRBG)
            .unwrap()
            .collect();
        assert_eq!(rgb.dimensions(), (40, 40));
        assert!(rgb.as_raw().iter().all(|v| v.is_finite()));
    }

    #[test]
    fn amaze_rejects_invalid_options_and_dimensions() {
        let tiny = MonoImage::from_raw(1, 4, vec![0.5_f32; 4]).unwrap();
        assert!(DemosaicAmaze.lazy(&tiny, BayerPattern::RGGB).is_err());

        let img = MonoImage::from_raw(16, 16, vec![0.5_f32; 256]).unwrap();
        assert!(
            DemosaicAmaze
                .lazy_with_options(&img, BayerPattern::RGGB, AmazeOptions { tile_size: 34 })
                .is_err()
        );
        assert!(
            DemosaicAmaze
                .lazy_with_options(&img, BayerPattern::RGGB, AmazeOptions { tile_size: 37 })
                .is_err()
        );

        let mut bad = vec![0.5_f32; 256];
        bad[17] = f32::NAN;
        let bad_img = MonoImage::from_raw(16, 16, bad).unwrap();
        assert!(DemosaicAmaze.lazy(&bad_img, BayerPattern::RGGB).is_err());
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn amaze_matches_24x24_snapshot() {
        let bayer = make_test_bayer(24, 24, BayerPattern::RGGB, 1.0);
        let rgb = DemosaicAmaze
            .lazy(&bayer, BayerPattern::RGGB)
            .unwrap()
            .collect();
        insta::assert_snapshot!(format_rgb_3dp(&rgb));
    }
}
