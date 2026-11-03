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

fn clip_i32(value: i32) -> u16 {
    value.clamp(0, i32::from(u16::MAX)) as u16
}

fn nyquist_pass(pixels: &mut [DcbPixel], width: usize, height: usize, pattern: BayerPattern) {
    let u = width;
    let v = 2 * width;
    for row in 2..height.saturating_sub(2) {
        let mut col = 2 + (cfa_channel(pattern, 2, row as isize) & 1);
        while col < width.saturating_sub(2) {
            let index = row * width + col;
            let color = cfa_channel(pattern, col as isize, row as isize);
            pixels[index][1] = clip_f32(
                (pixels[index + v][1] as f32
                    + pixels[index - v][1] as f32
                    + pixels[index - 2][1] as f32
                    + pixels[index + 2][1] as f32)
                    / 4.0
                    + pixels[index][color] as f32
                    - (pixels[index + v][color] as f32
                        + pixels[index - v][color] as f32
                        + pixels[index - 2][color] as f32
                        + pixels[index + 2][color] as f32)
                        / 4.0,
            );
            col += 2;
        }
    }
    let _ = u;
}

fn direction_map(pixels: &mut [DcbPixel], width: usize, height: usize) {
    let u = width;
    for row in 1..height.saturating_sub(1) {
        for col in 1..width.saturating_sub(1) {
            let index = row * width + col;
            let left = pixels[index - 1][1];
            let right = pixels[index + 1][1];
            let up = pixels[index - u][1];
            let down = pixels[index + u][1];
            let horizontal = u32::from(left) + u32::from(right);
            let vertical = u32::from(up) + u32::from(down);
            pixels[index][3] = if u32::from(pixels[index][1]) > (horizontal + vertical) / 4 {
                u16::from(
                    u32::from(left.min(right)) + horizontal < u32::from(up.min(down)) + vertical,
                )
            } else {
                u16::from(
                    u32::from(left.max(right)) + horizontal > u32::from(up.max(down)) + vertical,
                )
            };
        }
    }
}

fn correction(pixels: &mut [DcbPixel], width: usize, height: usize, pattern: BayerPattern) {
    let u = width;
    let v = 2 * width;
    for row in 2..height.saturating_sub(2) {
        let mut col = 2 + (cfa_channel(pattern, 2, row as isize) & 1);
        while col < width.saturating_sub(2) {
            let index = row * width + col;
            let current = 4 * i32::from(pixels[index][3])
                + 2 * (i32::from(pixels[index + u][3])
                    + i32::from(pixels[index - u][3])
                    + i32::from(pixels[index + 1][3])
                    + i32::from(pixels[index - 1][3]))
                + i32::from(pixels[index + v][3])
                + i32::from(pixels[index - v][3])
                + i32::from(pixels[index + 2][3])
                + i32::from(pixels[index - 2][3]);
            pixels[index][1] = (((16 - current) as f32
                * f32::midpoint(pixels[index - 1][1] as f32, pixels[index + 1][1] as f32)
                + current as f32
                    * f32::midpoint(pixels[index - u][1] as f32, pixels[index + u][1] as f32))
                / 16.0) as u16;
            col += 2;
        }
    }
}

fn correction2(pixels: &mut [DcbPixel], width: usize, height: usize, pattern: BayerPattern) {
    let u = width;
    let v = 2 * width;
    for row in 4..height.saturating_sub(4) {
        let mut col = 4 + (cfa_channel(pattern, 2, row as isize) & 1);
        while col < width.saturating_sub(4) {
            let index = row * width + col;
            let color = cfa_channel(pattern, col as isize, row as isize);
            let current = 4 * i32::from(pixels[index][3])
                + 2 * (i32::from(pixels[index + u][3])
                    + i32::from(pixels[index - u][3])
                    + i32::from(pixels[index + 1][3])
                    + i32::from(pixels[index - 1][3]))
                + i32::from(pixels[index + v][3])
                + i32::from(pixels[index - v][3])
                + i32::from(pixels[index + 2][3])
                + i32::from(pixels[index - 2][3]);
            pixels[index][1] = clip_f32(
                ((16 - current) as f32
                    * (f32::midpoint(pixels[index - 1][1] as f32, pixels[index + 1][1] as f32)
                        + pixels[index][color] as f32
                        - f32::midpoint(
                            pixels[index + 2][color] as f32,
                            pixels[index - 2][color] as f32,
                        ))
                    + current as f32
                        * (f32::midpoint(
                            pixels[index - u][1] as f32,
                            pixels[index + u][1] as f32,
                        ) + pixels[index][color] as f32
                            - f32::midpoint(
                                pixels[index + v][color] as f32,
                                pixels[index - v][color] as f32,
                            )))
                    / 16.0,
            );
            col += 2;
        }
    }
}

fn color(pixels: &mut [DcbPixel], width: usize, height: usize, pattern: BayerPattern) {
    let u = width;
    for row in 1..height.saturating_sub(1) {
        let mut col = 1 + (cfa_channel(pattern, 1, row as isize) & 1);
        while col < width.saturating_sub(1) {
            let index = row * width + col;
            let color = 2 - cfa_channel(pattern, col as isize, row as isize);
            pixels[index][color] = clip_f32(
                (4.0 * pixels[index][1] as f32
                    - pixels[index + u + 1][1] as f32
                    - pixels[index + u - 1][1] as f32
                    - pixels[index - u + 1][1] as f32
                    - pixels[index - u - 1][1] as f32
                    + pixels[index + u + 1][color] as f32
                    + pixels[index + u - 1][color] as f32
                    + pixels[index - u + 1][color] as f32
                    + pixels[index - u - 1][color] as f32)
                    / 4.0,
            );
            col += 2;
        }
    }
    for row in 1..height.saturating_sub(1) {
        let mut col = 1 + (cfa_channel(pattern, 2, row as isize) & 1);
        while col < width.saturating_sub(1) {
            let index = row * width + col;
            let color = cfa_channel(pattern, col as isize + 1, row as isize);
            let other = 2 - color;
            pixels[index][color] = clip_f32(
                (2.0 * pixels[index][1] as f32
                    - pixels[index + 1][1] as f32
                    - pixels[index - 1][1] as f32
                    + pixels[index + 1][color] as f32
                    + pixels[index - 1][color] as f32)
                    / 2.0,
            );
            pixels[index][other] = clip_f32(
                (2.0 * pixels[index][1] as f32
                    - pixels[index + u][1] as f32
                    - pixels[index - u][1] as f32
                    + pixels[index + u][other] as f32
                    + pixels[index - u][other] as f32)
                    / 2.0,
            );
            col += 2;
        }
    }
}

fn smooth(pixels: &mut [DcbPixel], width: usize, height: usize) {
    let u = width;
    for row in 2..height.saturating_sub(2) {
        for col in 2..width.saturating_sub(2) {
            let index = row * width + col;
            let neighbors = [
                index - 1,
                index + 1,
                index - u,
                index + u,
                index - u - 1,
                index + u + 1,
                index - u + 1,
                index + u - 1,
            ];
            let mean = |channel: usize| {
                (neighbors
                    .iter()
                    .map(|neighbor| pixels[*neighbor][channel] as f32)
                    .sum::<f32>()
                    / 8.0) as i32
            };
            let red = mean(0);
            let green = mean(1);
            let blue = mean(2);
            pixels[index][0] = clip_i32(red + i32::from(pixels[index][1]) - green);
            pixels[index][2] = clip_i32(blue + i32::from(pixels[index][1]) - green);
        }
    }
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

    #[test]
    fn nyquist_pass_mixes_green_neighbors_with_the_current_cfa_color() {
        let pattern = BayerPattern::GBRG;
        let mut pixels = vec![[0_u16; 4]; 15 * 15];
        let target = 3 * 15 + 2;
        pixels[target] = [10, 100, 0, 0];
        for (offset, green, red) in [
            (-2_isize, 10_u16, 2_u16),
            (-2 * 15, 20, 4),
            (2, 30, 6),
            (2 * 15, 40, 8),
        ] {
            let index = target.checked_add_signed(offset).unwrap();
            pixels[index][1] = green;
            pixels[index][0] = red;
        }
        nyquist_pass(&mut pixels, 15, 15, pattern);
        assert_eq!(pixels[target][1], 30);
    }

    #[test]
    fn map_and_correction_mix_horizontal_and_vertical_green() {
        let pattern = BayerPattern::GBRG;
        let mut pixels = vec![[0_u16; 4]; 15 * 15];
        let target = 2 * 15 + 3;
        pixels[target - 1][1] = 100;
        pixels[target + 1][1] = 100;
        pixels[target - 15][1] = 300;
        pixels[target + 15][1] = 300;
        pixels[target][1] = 500;
        direction_map(&mut pixels, 15, 15);
        assert_eq!(pixels[target][3], 1);
        for pixel in &mut pixels {
            pixel[3] = 0;
        }
        pixels[target][3] = 1;
        pixels[target][1] = 200;
        correction(&mut pixels, 15, 15, pattern);
        assert_eq!(pixels[target][1], 150);
    }

    #[test]
    fn correction2_uses_cfa_contrast_on_the_selected_axis() {
        let pattern = BayerPattern::GBRG;
        let mut pixels = vec![[100_u16, 100, 100, 0]; 15 * 15];
        let target = 4 * 15 + 5;
        pixels[target][3] = 1;
        correction2(&mut pixels, 15, 15, pattern);
        assert_eq!(pixels[target][1], 100);
    }

    #[test]
    fn color_reconstructs_the_missing_diagonal_channel() {
        let pattern = BayerPattern::GBRG;
        let mut pixels = vec![[100_u16, 100, 200, 0]; 15 * 15];
        color(&mut pixels, 15, 15, pattern);
        assert_eq!(pixels[15 + 2][2], 200);
    }

    #[test]
    fn smooth_preserves_green_contrast_over_eight_neighbors() {
        let mut pixels = vec![[100_u16, 100, 100, 0]; 15 * 15];
        let target = 2 * 15 + 2;
        for y in 1..=3 {
            for x in 1..=3 {
                if y == 2 && x == 2 {
                    continue;
                }
                pixels[y * 15 + x] = [200, 100, 300, 0];
            }
        }
        smooth(&mut pixels, 15, 15);
        assert_eq!(pixels[target], [200, 100, 300, 0]);
    }
}
