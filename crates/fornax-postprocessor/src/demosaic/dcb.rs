#![allow(dead_code, reason = "DCB stages are wired incrementally")]

use fornax_core::error::DemosaicError;
use fornax_core::{BayerPattern, MonoImage, Primitive};

use super::buffer::DenseLazy;
use super::directional::{self, Profile};

type DcbPixel = [u16; 4];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicDcb;

impl DemosaicDcb {
    pub fn lazy<T>(
        &self,
        image: &MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<DenseLazy<T>, DemosaicError>
    where
        T: Primitive + Send + Sync,
    {
        directional::lazy(image, pattern, Profile::Dcb)
    }
}

fn insert_measured(
    input: &[u16],
    width: usize,
    height: usize,
    pattern: BayerPattern,
) -> Vec<DcbPixel> {
    let mut pixels = vec![[0_u16; 4]; width * height];
    for y in 0..height {
        for x in 0..width {
            let index = y * width + x;
            pixels[index][cfa_channel(pattern, x as isize, y as isize)] = input[index];
        }
    }
    pixels
}

fn cfa_channel(pattern: BayerPattern, x: isize, y: isize) -> usize {
    match pattern.channel_at(x.rem_euclid(2) as u32, y.rem_euclid(2) as u32) {
        fornax_core::BayerChannel::R => 0,
        fornax_core::BayerChannel::GRedRow | fornax_core::BayerChannel::GBlueRow => 1,
        fornax_core::BayerChannel::B => 2,
    }
}

fn border_interpolate(
    pixels: &mut [DcbPixel],
    width: usize,
    height: usize,
    pattern: BayerPattern,
    border: usize,
) {
    for row in 0..height {
        let mut col = 0;
        while col < width {
            let process_col =
                if col == border && row >= border && row < height.saturating_sub(border) {
                    width - border
                } else {
                    col
                };
            let mut sums = [0_u32; 8];
            let y_start = row.saturating_sub(1);
            let y_end = (row + 1).min(height - 1);
            let x_start = process_col.saturating_sub(1);
            let x_end = (process_col + 1).min(width - 1);
            for y in y_start..=y_end {
                for x in x_start..=x_end {
                    let channel = cfa_channel(pattern, x as isize, y as isize);
                    sums[channel] += u32::from(pixels[y * width + x][channel]);
                    sums[channel + 4] += 1;
                }
            }
            let channel = cfa_channel(pattern, process_col as isize, row as isize);
            for color in 0..3 {
                if color != channel && sums[color + 4] != 0 {
                    pixels[row * width + process_col][color] =
                        (sums[color] / sums[color + 4]) as u16;
                }
            }
            col = process_col + 1;
        }
    }
}

fn horizontal_candidates(
    pixels: &[DcbPixel],
    width: usize,
    height: usize,
    pattern: BayerPattern,
) -> Vec<[f32; 3]> {
    let mut candidates = vec![[0.0_f32; 3]; width * height];
    for row in 2..height.saturating_sub(2) {
        let mut col = 2 + (cfa_channel(pattern, 2, row as isize) & 1);
        while col < width.saturating_sub(2) {
            let index = row * width + col;
            candidates[index][1] = clip_f32(f32::midpoint(
                pixels[index - 1][1] as f32,
                pixels[index + 1][1] as f32,
            )) as f32;
            col += 2;
        }
    }
    horizontal_colors(pixels, width, height, pattern, &mut candidates);
    candidates
}

fn horizontal_colors(
    pixels: &[DcbPixel],
    width: usize,
    height: usize,
    pattern: BayerPattern,
    candidates: &mut [[f32; 3]],
) {
    let u = width;
    for row in 1..height.saturating_sub(1) {
        let mut col = 1 + (cfa_channel(pattern, 1, row as isize) & 1);
        while col < width.saturating_sub(1) {
            let index = row * width + col;
            let color = 2 - cfa_channel(pattern, col as isize, row as isize);
            candidates[index][color] = clip_f32(
                (4.0 * candidates[index][1]
                    - candidates[index + u + 1][1]
                    - candidates[index + u - 1][1]
                    - candidates[index - u + 1][1]
                    - candidates[index - u - 1][1]
                    + pixels[index + u + 1][color] as f32
                    + pixels[index + u - 1][color] as f32
                    + pixels[index - u + 1][color] as f32
                    + pixels[index - u - 1][color] as f32)
                    / 4.0,
            ) as f32;
            col += 2;
        }
    }

    for row in 1..height.saturating_sub(1) {
        let mut col = 1 + (cfa_channel(pattern, 2, row as isize) & 1);
        while col < width.saturating_sub(1) {
            let index = row * width + col;
            let color = cfa_channel(pattern, col as isize + 1, row as isize);
            let other = 2 - color;
            candidates[index][color] = clip_f32(f32::midpoint(
                pixels[index - 1][color] as f32,
                pixels[index + 1][color] as f32,
            )) as f32;
            candidates[index][other] = clip_f32(
                (2.0 * candidates[index][1] - candidates[index + u][1] - candidates[index - u][1]
                    + pixels[index + u][other] as f32
                    + pixels[index - u][other] as f32)
                    / 2.0,
            ) as f32;
            col += 2;
        }
    }
}

fn vertical_candidates(
    pixels: &[DcbPixel],
    width: usize,
    height: usize,
    pattern: BayerPattern,
) -> Vec<[f32; 3]> {
    let mut candidates = vec![[0.0_f32; 3]; width * height];
    for row in 2..height.saturating_sub(2) {
        let mut col = 2 + (cfa_channel(pattern, 2, row as isize) & 1);
        while col < width.saturating_sub(2) {
            let index = row * width + col;
            candidates[index][1] = clip_f32(f32::midpoint(
                pixels[index - width][1] as f32,
                pixels[index + width][1] as f32,
            )) as f32;
            col += 2;
        }
    }
    vertical_colors(pixels, width, height, pattern, &mut candidates);
    candidates
}

fn vertical_colors(
    pixels: &[DcbPixel],
    width: usize,
    height: usize,
    pattern: BayerPattern,
    candidates: &mut [[f32; 3]],
) {
    let u = width;
    for row in 1..height.saturating_sub(1) {
        let mut col = 1 + (cfa_channel(pattern, 1, row as isize) & 1);
        while col < width.saturating_sub(1) {
            let index = row * width + col;
            let color = 2 - cfa_channel(pattern, col as isize, row as isize);
            candidates[index][color] = clip_f32(
                (4.0 * candidates[index][1]
                    - candidates[index + u + 1][1]
                    - candidates[index + u - 1][1]
                    - candidates[index - u + 1][1]
                    - candidates[index - u - 1][1]
                    + pixels[index + u + 1][color] as f32
                    + pixels[index + u - 1][color] as f32
                    + pixels[index - u + 1][color] as f32
                    + pixels[index - u - 1][color] as f32)
                    / 4.0,
            ) as f32;
            col += 2;
        }
    }

    for row in 1..height.saturating_sub(1) {
        let mut col = 1 + (cfa_channel(pattern, 2, row as isize) & 1);
        while col < width.saturating_sub(1) {
            let index = row * width + col;
            let color = cfa_channel(pattern, col as isize + 1, row as isize);
            let other = 2 - color;
            candidates[index][color] = clip_f32(
                (2.0 * candidates[index][1] - candidates[index + 1][1] - candidates[index - 1][1]
                    + pixels[index + 1][color] as f32
                    + pixels[index - 1][color] as f32)
                    / 2.0,
            ) as f32;
            candidates[index][other] = clip_f32(f32::midpoint(
                pixels[index - u][other] as f32,
                pixels[index + u][other] as f32,
            )) as f32;
            col += 2;
        }
    }
}

fn apply_direction(
    pixels: &mut [DcbPixel],
    horizontal: &[[f32; 3]],
    vertical: &[[f32; 3]],
    width: usize,
    height: usize,
    pattern: BayerPattern,
) {
    for row in 2..height.saturating_sub(2) {
        let mut col = 2 + (cfa_channel(pattern, 2, row as isize) & 1);
        while col < width.saturating_sub(2) {
            let index = row * width + col;
            pixels[index][1] = if direction_is_vertical(
                pixels, horizontal, vertical, width, height, pattern, index,
            ) {
                vertical[index][1] as u16
            } else {
                horizontal[index][1] as u16
            };
            col += 2;
        }
    }
}

fn direction_is_vertical(
    pixels: &[DcbPixel],
    horizontal: &[[f32; 3]],
    vertical: &[[f32; 3]],
    width: usize,
    _height: usize,
    pattern: BayerPattern,
    index: usize,
) -> bool {
    let row = index / width;
    let col = index % width;
    let color = cfa_channel(pattern, col as isize, row as isize);
    let other = (color as isize - 2).unsigned_abs();
    let u = width;
    let v = 2 * width;
    let axes = [index + v, index - v, index - 2, index + 2];
    let diagonals = [index + 1 + u, index + 1 - u, index - 1 + u, index - 1 - u];
    let range = |values: [f32; 4]| {
        values.into_iter().fold(f32::NEG_INFINITY, f32::max)
            - values.into_iter().fold(f32::INFINITY, f32::min)
    };
    let current = range(axes.map(|sample| pixels[sample][color] as f32))
        + range(diagonals.map(|sample| pixels[sample][other] as f32));
    let current2 = range(axes.map(|sample| horizontal[sample][other]))
        + range(diagonals.map(|sample| horizontal[sample][color]));
    let current3 = range(axes.map(|sample| vertical[sample][other]))
        + range(diagonals.map(|sample| vertical[sample][color]));
    ((current - current2) as i32).abs() >= ((current - current3) as i32).abs()
}

fn clip_f32(value: f32) -> u16 {
    (value as i32).clamp(0, i32::from(u16::MAX)) as u16
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn border_interpolate_fills_corner_channels_from_their_cfa_neighbors() {
        let pattern = BayerPattern::RGGB;
        let mut input = vec![0_u16; 16];
        for (index, sample) in input.iter_mut().enumerate() {
            let channel = cfa_channel(pattern, (index % 4) as isize, (index / 4) as isize);
            *sample = [10, 20, 30, 40][channel] as u16;
        }
        let mut pixels = insert_measured(&input, 4, 4, pattern);
        border_interpolate(&mut pixels, 4, 4, pattern, 1);
        assert_eq!(pixels[0], [10, 20, 30, 0]);
    }

    #[test]
    fn horizontal_and_vertical_green_candidates_use_their_own_axes() {
        let pattern = BayerPattern::GBRG;
        let mut pixels = vec![[0_u16; 4]; 15 * 15];
        let target = 3 * 15 + 2;
        pixels[3 * 15 + 1][1] = 10_000;
        pixels[3 * 15 + 3][1] = 30_000;
        pixels[2 * 15 + 2][1] = 2_000;
        pixels[4 * 15 + 2][1] = 6_000;
        assert_eq!(
            horizontal_candidates(&pixels, 15, 15, pattern)[target][1],
            20_000.0
        );
        assert_eq!(
            vertical_candidates(&pixels, 15, 15, pattern)[target][1],
            4_000.0
        );
    }

    #[test]
    fn direction_decision_prefers_the_candidate_with_matching_local_rhythm() {
        let pattern = BayerPattern::GBRG;
        let mut pixels = vec![[100_u16, 100, 100, 0]; 15 * 15];
        let target = 3 * 15 + 2;
        pixels[target] = [10, 100, 100, 0];
        pixels[target - 2][0] = 100;
        pixels[target + 2][0] = 200;
        let mut horizontal = vec![[100.0_f32; 3]; 15 * 15];
        horizontal[target - 2][2] = 100.0;
        horizontal[target + 2][2] = 200.0;
        let mut vertical = vec![[100.0_f32; 3]; 15 * 15];
        vertical[target - 2][2] = 100.0;
        vertical[target + 2][2] = 300.0;
        assert!(!direction_is_vertical(
            &pixels,
            &horizontal,
            &vertical,
            15,
            15,
            pattern,
            target,
        ));
    }
}
