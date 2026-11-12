//! Directional color balance demosaicing.
//!
//! This port follows LibRaw 0.22.3's fixed `dcb(0, 1)` path. The original DCB
//! implementation is copyright (C) 2010 Jacek Gozdz and is distributed under
//! the BSD-3-Clause license reproduced in `docs/reference/algorithms/dcb.md`.

use fornax_raw_core::error::DemosaicError;
use fornax_raw_core::{BayerPattern, MonoImage, Primitive};

use super::buffer::{DenseLazy, codes, dense_from_codes};

type DcbPixel = [u16; 4];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicDcb;

impl DemosaicDcb {
    #[cfg(test)]
    pub fn lazy<T>(
        &self,
        image: &MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<DenseLazy<T>, DemosaicError>
    where
        T: Primitive + Send + Sync,
    {
        self.lazy_with_options(image, pattern, 0, true)
    }

    pub fn lazy_with_options<T>(
        &self,
        image: &MonoImage<T>,
        pattern: BayerPattern,
        iterations: u8,
        enhance: bool,
    ) -> Result<DenseLazy<T>, DemosaicError>
    where
        T: Primitive + Send + Sync,
    {
        if image.width() < 3 || image.height() < 3 {
            return Err(DemosaicError::InvalidInput(
                "DCB requires an image at least 3x3".into(),
            ));
        }
        let width = image.width() as usize;
        let height = image.height() as usize;
        let output = run_with_options(&codes(image), width, height, pattern, iterations, enhance);
        Ok(DenseLazy::new(dense_from_codes(
            image.width(),
            image.height(),
            output,
        )))
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

const fn cfa_channel(pattern: BayerPattern, x: isize, y: isize) -> usize {
    match pattern.channel_at(x.rem_euclid(2) as u32, y.rem_euclid(2) as u32) {
        fornax_raw_core::BayerChannel::R => 0,
        fornax_raw_core::BayerChannel::GRedRow | fornax_raw_core::BayerChannel::GBlueRow => 1,
        fornax_raw_core::BayerChannel::B => 2,
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
            let process_col = if col == border
                && width > 2 * border
                && row >= border
                && row < height.saturating_sub(border)
            {
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
                (4.0_f32.mul_add(candidates[index][1], -candidates[index + u + 1][1])
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
                (2.0_f32.mul_add(candidates[index][1], -candidates[index + u][1])
                    - candidates[index - u][1]
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
                (4.0_f32.mul_add(candidates[index][1], -candidates[index + u + 1][1])
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
                (2.0_f32.mul_add(candidates[index][1], -candidates[index + 1][1])
                    - candidates[index - 1][1]
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

fn clip_f32(value: f32) -> u16 { (value as i32).clamp(0, i32::from(u16::MAX)) as u16 }

fn clip_i32(value: i32) -> u16 { value.clamp(0, i32::from(u16::MAX)) as u16 }

fn nyquist_pass(pixels: &mut [DcbPixel], width: usize, height: usize, pattern: BayerPattern) {
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
            pixels[index][1] = ((current as f32).mul_add(
                f32::midpoint(pixels[index - u][1] as f32, pixels[index + u][1] as f32),
                (16 - current) as f32
                    * f32::midpoint(pixels[index - 1][1] as f32, pixels[index + 1][1] as f32),
            ) / 16.0) as u16;
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
                (current as f32).mul_add(
                    f32::midpoint(pixels[index - u][1] as f32, pixels[index + u][1] as f32)
                        + pixels[index][color] as f32
                        - f32::midpoint(
                            pixels[index + v][color] as f32,
                            pixels[index - v][color] as f32,
                        ),
                    (16 - current) as f32
                        * (f32::midpoint(pixels[index - 1][1] as f32, pixels[index + 1][1] as f32)
                            + pixels[index][color] as f32
                            - f32::midpoint(
                                pixels[index + 2][color] as f32,
                                pixels[index - 2][color] as f32,
                            )),
                ) / 16.0,
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
                (4.0_f32.mul_add(pixels[index][1] as f32, -(pixels[index + u + 1][1] as f32))
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
                (2.0_f32.mul_add(pixels[index][1] as f32, -(pixels[index + 1][1] as f32))
                    - pixels[index - 1][1] as f32
                    + pixels[index + 1][color] as f32
                    + pixels[index - 1][color] as f32)
                    / 2.0,
            );
            pixels[index][other] = clip_f32(
                (2.0_f32.mul_add(pixels[index][1] as f32, -(pixels[index + u][1] as f32))
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

fn refinement(pixels: &mut [DcbPixel], width: usize, height: usize, pattern: BayerPattern) {
    let u = width;
    let v = 2 * width;
    let w = 3 * width;
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

            if pixels[index][color] > 1 {
                let center = pixels[index][color] as f32;
                let mut f = [0.0_f32; 5];
                f[0] = (pixels[index - u][1] as f32 + pixels[index + u][1] as f32) / (2.0 * center);
                f[1] = if pixels[index - v][color] > 0 {
                    2.0 * pixels[index - u][1] as f32 / (pixels[index - v][color] as f32 + center)
                } else {
                    f[0]
                };
                f[2] = if pixels[index - v][color] > 0 {
                    (pixels[index - u][1] as f32 + pixels[index - w][1] as f32)
                        / (2.0 * pixels[index - v][color] as f32)
                } else {
                    f[0]
                };
                f[3] = if pixels[index + v][color] > 0 {
                    2.0 * pixels[index + u][1] as f32 / (pixels[index + v][color] as f32 + center)
                } else {
                    f[0]
                };
                f[4] = if pixels[index + v][color] > 0 {
                    (pixels[index + u][1] as f32 + pixels[index + w][1] as f32)
                        / (2.0 * pixels[index + v][color] as f32)
                } else {
                    f[0]
                };
                let vertical_ratio =
                    (3.0_f32.mul_add(f[3], 3.0_f32.mul_add(f[1], 5.0 * f[0]) + f[2]) + f[4]) / 13.0;

                f[0] = (pixels[index - 1][1] as f32 + pixels[index + 1][1] as f32) / (2.0 * center);
                f[1] = if pixels[index - 2][color] > 0 {
                    2.0 * pixels[index - 1][1] as f32 / (pixels[index - 2][color] as f32 + center)
                } else {
                    f[0]
                };
                f[2] = if pixels[index - 2][color] > 0 {
                    (pixels[index - 1][1] as f32 + pixels[index - 3][1] as f32)
                        / (2.0 * pixels[index - 2][color] as f32)
                } else {
                    f[0]
                };
                f[3] = if pixels[index + 2][color] > 0 {
                    2.0 * pixels[index + 1][1] as f32 / (pixels[index + 2][color] as f32 + center)
                } else {
                    f[0]
                };
                f[4] = if pixels[index + 2][color] > 0 {
                    (pixels[index + 1][1] as f32 + pixels[index + 3][1] as f32)
                        / (2.0 * pixels[index + 2][color] as f32)
                } else {
                    f[0]
                };
                let horizontal_ratio =
                    (3.0_f32.mul_add(f[3], 3.0_f32.mul_add(f[1], 5.0 * f[0]) + f[2]) + f[4]) / 13.0;
                pixels[index][1] = clip_f32(
                    center
                        * ((16 - current) as f32)
                            .mul_add(horizontal_ratio, current as f32 * vertical_ratio)
                        / 16.0,
                );
            } else {
                pixels[index][1] = pixels[index][color];
            }

            let neighbors = [
                index + 1 + u,
                index + 1 - u,
                index - 1 + u,
                index - 1 - u,
                index - 1,
                index + 1,
                index - u,
                index + u,
            ];
            let minimum = neighbors
                .iter()
                .map(|neighbor| pixels[*neighbor][1])
                .min()
                .unwrap();
            let maximum = neighbors
                .iter()
                .map(|neighbor| pixels[*neighbor][1])
                .max()
                .unwrap();
            pixels[index][1] =
                upper_limit(pixels[index][1] as f32, maximum as f32, minimum as f32) as u16;
            col += 2;
        }
    }
}

fn color_full(pixels: &mut [DcbPixel], width: usize, height: usize, pattern: BayerPattern) {
    let u = width;
    let w = 3 * width;
    let mut chroma = vec![[0.0_f32; 2]; width * height];

    for row in 1..height.saturating_sub(1) {
        let mut col = 1 + (cfa_channel(pattern, 1, row as isize) & 1);
        while col < width.saturating_sub(1) {
            let index = row * width + col;
            let color = cfa_channel(pattern, col as isize, row as isize);
            chroma[index][color / 2] = pixels[index][color] as f32 - pixels[index][1] as f32;
            col += 2;
        }
    }

    for row in 3..height.saturating_sub(3) {
        let mut col = 3 + (cfa_channel(pattern, 1, row as isize) & 1);
        while col < width.saturating_sub(3) {
            let index = row * width + col;
            let color = 1 - cfa_channel(pattern, col as isize, row as isize) / 2;
            let mut f = [0.0_f32; 4];
            let mut g = [0.0_f32; 4];
            for (direction, weight) in f.iter_mut().enumerate() {
                *weight = diagonal_chroma_weight(&chroma, index, width, color, direction);
            }
            g[0] = 0.075_f32.mul_add(
                -chroma[index - u - 3][color],
                0.075_f32.mul_add(
                    -chroma[index - w - 1][color],
                    0.175_f32.mul_add(
                        -chroma[index - w - 3][color],
                        1.325 * chroma[index - u - 1][color],
                    ),
                ),
            );
            g[1] = 0.075_f32.mul_add(
                -chroma[index - u + 3][color],
                0.075_f32.mul_add(
                    -chroma[index - w + 1][color],
                    0.175_f32.mul_add(
                        -chroma[index - w + 3][color],
                        1.325 * chroma[index - u + 1][color],
                    ),
                ),
            );
            g[2] = 0.075_f32.mul_add(
                -chroma[index + u - 3][color],
                0.075_f32.mul_add(
                    -chroma[index + w - 1][color],
                    0.175_f32.mul_add(
                        -chroma[index + w - 3][color],
                        1.325 * chroma[index + u - 1][color],
                    ),
                ),
            );
            g[3] = 0.075_f32.mul_add(
                -chroma[index + u + 3][color],
                0.075_f32.mul_add(
                    -chroma[index + w + 1][color],
                    0.175_f32.mul_add(
                        -chroma[index + w + 3][color],
                        1.325 * chroma[index + u + 1][color],
                    ),
                ),
            );
            chroma[index][color] = f32::mul_add(
                f[3],
                g[3],
                f32::mul_add(f[2], g[2], f32::mul_add(f[1], g[1], f[0] * g[0])),
            ) / (f[0] + f[1] + f[2] + f[3]);
            col += 2;
        }
    }

    for row in 3..height.saturating_sub(3) {
        let mut col = 3 + (cfa_channel(pattern, 2, row as isize) & 1);
        while col < width.saturating_sub(3) {
            let index = row * width + col;
            let mut color = cfa_channel(pattern, col as isize + 1, row as isize) / 2;
            for _ in 0..2 {
                let mut f = [0.0_f32; 4];
                let mut g = [0.0_f32; 4];
                f[0] = 1.0
                    / (1.0
                        + (chroma[index - u][color] - chroma[index + u][color]).abs()
                        + (chroma[index - u][color] - chroma[index - w][color]).abs()
                        + (chroma[index + u][color] - chroma[index - w][color]).abs());
                f[1] = 1.0
                    / (1.0
                        + (chroma[index + 1][color] - chroma[index - 1][color]).abs()
                        + (chroma[index + 1][color] - chroma[index + 3][color]).abs()
                        + (chroma[index - 1][color] - chroma[index + 3][color]).abs());
                f[2] = 1.0
                    / (1.0
                        + (chroma[index - 1][color] - chroma[index + 1][color]).abs()
                        + (chroma[index - 1][color] - chroma[index - 3][color]).abs()
                        + (chroma[index + 1][color] - chroma[index - 3][color]).abs());
                f[3] = 1.0
                    / (1.0
                        + (chroma[index + u][color] - chroma[index - u][color]).abs()
                        + (chroma[index + u][color] - chroma[index + w][color]).abs()
                        + (chroma[index - u][color] - chroma[index + w][color]).abs());
                g[0] =
                    0.125_f32.mul_add(chroma[index - w][color], 0.875 * chroma[index - u][color]);
                g[1] =
                    0.125_f32.mul_add(chroma[index + 3][color], 0.875 * chroma[index + 1][color]);
                g[2] =
                    0.125_f32.mul_add(chroma[index - 3][color], 0.875 * chroma[index - 1][color]);
                g[3] =
                    0.125_f32.mul_add(chroma[index + w][color], 0.875 * chroma[index + u][color]);
                chroma[index][color] = f32::mul_add(
                    f[3],
                    g[3],
                    f32::mul_add(f[2], g[2], f32::mul_add(f[1], g[1], f[0] * g[0])),
                ) / (f[0] + f[1] + f[2] + f[3]);
                color = 1 - color;
            }
            col += 2;
        }
    }

    for row in 6..height.saturating_sub(6) {
        for col in 6..width.saturating_sub(6) {
            let index = row * width + col;
            pixels[index][0] = clip_f32(chroma[index][0] + pixels[index][1] as f32);
            pixels[index][2] = clip_f32(chroma[index][1] + pixels[index][1] as f32);
            for color in [0, 2] {
                let neighbors = [
                    index + 1 + u,
                    index + 1 - u,
                    index - 1 + u,
                    index - 1 - u,
                    index - 1,
                    index + 1,
                    index - u,
                    index + u,
                ];
                let minimum = neighbors
                    .iter()
                    .map(|neighbor| pixels[*neighbor][color])
                    .min()
                    .unwrap();
                let maximum = neighbors
                    .iter()
                    .map(|neighbor| pixels[*neighbor][color])
                    .max()
                    .unwrap();
                pixels[index][color] =
                    upper_limit(pixels[index][color] as f32, maximum as f32, minimum as f32) as u16;
            }
        }
    }
}

fn diagonal_chroma_weight(
    chroma: &[[f32; 2]],
    index: usize,
    width: usize,
    color: usize,
    direction: usize,
) -> f32 {
    let u = width;
    let w = 3 * width;
    let (near_a, near_b, far_a, far_b) = match direction {
        0 => (index - u - 1, index + u + 1, index - w - 3, index - w - 3),
        1 => (index - u + 1, index + u - 1, index - w + 3, index - w + 3),
        2 => (index + u - 1, index - u + 1, index + w + 3, index + w - 3),
        _ => (index + u + 1, index - u - 1, index + w - 3, index + w + 3),
    };
    1.0 / (1.0
        + (chroma[near_a][color] - chroma[near_b][color]).abs()
        + (chroma[near_a][color] - chroma[far_a][color]).abs()
        + (chroma[near_b][color] - chroma[far_b][color]).abs())
}

fn upper_limit(value: f32, upper: f32, lower: f32) -> f32 {
    if upper < lower {
        value.clamp(upper, lower)
    } else {
        value.clamp(lower, upper)
    }
}

fn apply_post_correction_passes(
    pixels: &mut [DcbPixel],
    width: usize,
    height: usize,
    pattern: BayerPattern,
) {
    for _ in 0..3 {
        direction_map(pixels, width, height);
        correction(pixels, width, height, pattern);
    }
}

#[cfg(test)]
fn run(input: &[u16], width: usize, height: usize, pattern: BayerPattern) -> Vec<u16> {
    run_with_options(input, width, height, pattern, 0, true)
}

fn run_with_options(
    input: &[u16],
    width: usize,
    height: usize,
    pattern: BayerPattern,
    iterations: u8,
    enhance: bool,
) -> Vec<u16> {
    let mut pixels = insert_measured(input, width, height, pattern);
    border_interpolate(&mut pixels, width, height, pattern, 6);
    let horizontal = horizontal_candidates(&pixels, width, height, pattern);
    let vertical = vertical_candidates(&pixels, width, height, pattern);
    apply_direction(&mut pixels, &horizontal, &vertical, width, height, pattern);

    let mut saved = horizontal;
    for (buffer, pixel) in saved.iter_mut().zip(&pixels) {
        buffer[0] = pixel[0] as f32;
        buffer[2] = pixel[2] as f32;
    }

    for _ in 0..iterations {
        for _ in 0..3 {
            nyquist_pass(&mut pixels, width, height, pattern);
        }
        direction_map(&mut pixels, width, height);
        correction(&mut pixels, width, height, pattern);
    }

    color(&mut pixels, width, height, pattern);
    smooth(&mut pixels, width, height);
    direction_map(&mut pixels, width, height);
    correction2(&mut pixels, width, height, pattern);
    apply_post_correction_passes(&mut pixels, width, height, pattern);
    direction_map(&mut pixels, width, height);
    for (buffer, pixel) in saved.iter().zip(&mut pixels) {
        pixel[0] = buffer[0] as u16;
        pixel[2] = buffer[2] as u16;
    }
    color(&mut pixels, width, height, pattern);
    if enhance {
        refinement(&mut pixels, width, height, pattern);
        color_full(&mut pixels, width, height, pattern);
    }

    pack(&pixels)
}

fn pack(pixels: &[DcbPixel]) -> Vec<u16> {
    pixels
        .iter()
        .flat_map(|pixel| [pixel[0], pixel[1], pixel[2]])
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::LazyDemosaic;

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

    #[test]
    fn refinement_preserves_a_uniform_cfa_to_green_ratio() {
        let pattern = BayerPattern::GBRG;
        let mut pixels = vec![[20_000_u16, 10_000, 20_000, 0]; 15 * 15];
        refinement(&mut pixels, 15, 15, pattern);
        for y in 5..10 {
            for x in 5..10 {
                assert_eq!(pixels[y * 15 + x][1], 10_000);
            }
        }
    }

    #[test]
    fn diagonal_chroma_weight_uses_the_matching_far_neighbor() {
        let width = 15;
        let index = 7 * width + 7;
        let mut chroma = vec![[0.0_f32; 2]; 15 * 15];
        chroma[index + width - 1][0] = 10.0;
        chroma[index - width + 1][0] = 20.0;
        chroma[index + 3 * width + 3][0] = 100.0;
        chroma[index + 3 * width - 3][0] = 50.0;

        assert_eq!(
            diagonal_chroma_weight(&chroma, index, width, 0, 2),
            1.0 / 131.0
        );
    }

    #[test]
    fn post_correction_sequence_runs_exactly_three_passes() {
        let pattern = BayerPattern::GBRG;
        let mut source = vec![[1_000_u16, 2_000, 3_000, 0]; 15 * 15];
        source[7 * 15 + 7][1] = 50_000;

        let mut expected = source.clone();
        for _ in 0..3 {
            direction_map(&mut expected, 15, 15);
            correction(&mut expected, 15, 15, pattern);
        }

        let mut actual = source;
        apply_post_correction_passes(&mut actual, 15, 15, pattern);
        assert_eq!(actual, expected);
    }

    #[test]
    fn nyquist_iterations_change_nonuniform_output() {
        let width = 32;
        let height = 32;
        let input = (0..width * height)
            .map(|index| {
                let hash = (index as u32).wrapping_mul(0x9e37_79b9).rotate_left(13)
                    ^ (index as u32).wrapping_mul(0x85eb_ca6b);
                1_000 + (hash ^ (hash >> 16)) as u16 % 50_000
            })
            .collect::<Vec<_>>();

        let baseline = run_with_options(&input, width, height, BayerPattern::GBRG, 0, true);
        let corrected = run_with_options(&input, width, height, BayerPattern::GBRG, 1, true);

        assert_ne!(baseline, corrected);
    }

    #[test]
    fn enhancement_flag_controls_final_refinement() {
        let width = 32;
        let height = 32;
        let input = (0..width * height)
            .map(|index| 2_000_u16.wrapping_add((index as u16).wrapping_mul(97)))
            .collect::<Vec<_>>();

        let enhanced = run_with_options(&input, width, height, BayerPattern::GBRG, 0, true);
        let plain = run_with_options(&input, width, height, BayerPattern::GBRG, 0, false);

        assert_eq!(
            run(&input, width, height, BayerPattern::GBRG),
            enhanced,
            "the legacy convenience path must use dcb(0, true)"
        );
        assert_ne!(enhanced, plain);
    }

    #[test]
    fn color_full_interpolates_uniform_chroma_without_changing_channels() {
        let pattern = BayerPattern::GBRG;
        let mut pixels = vec![[10_000_u16, 20_000, 30_000, 0]; 15 * 15];
        color_full(&mut pixels, 15, 15, pattern);
        for y in 6..9 {
            for x in 6..9 {
                assert_eq!(&pixels[y * 15 + x][..3], &[10_000, 20_000, 30_000]);
            }
        }
    }

    #[test]
    fn uniform_channel_codes_survive_the_public_dcb_path_for_all_patterns() {
        for pattern in BayerPattern::ALL {
            let input = (0..15 * 15)
                .map(|index| {
                    let channel =
                        cfa_channel(pattern, (index % 15) as isize, (index / 15) as isize);
                    [10_000_u16, 20_000, 30_000][channel]
                })
                .collect::<Vec<_>>();
            let output = run(&input, 15, 15, pattern);
            assert!(
                output
                    .as_chunks::<3>()
                    .0
                    .iter()
                    .all(|pixel| pixel == &[10_000, 20_000, 30_000])
            );
        }
    }

    #[test]
    fn full_color_limit_can_modify_a_sampled_value() {
        let pattern = BayerPattern::GBRG;
        let mut input = (0..15 * 15)
            .map(|index| {
                let channel = cfa_channel(pattern, (index % 15) as isize, (index / 15) as isize);
                [10_000_u16, 20_000, 30_000][channel]
            })
            .collect::<Vec<_>>();
        let target = 7 * 15 + 8;
        input[target] = 60_000;
        assert_ne!(run(&input, 15, 15, pattern)[target * 3], 60_000);
    }

    #[test]
    fn public_dcb_rejects_images_smaller_than_three_pixels() {
        for (width, height) in [(2, 3), (3, 2)] {
            let mosaic = MonoImage::from_raw(
                width,
                height,
                vec![0.0_f32; width as usize * height as usize],
            )
            .unwrap();
            assert!(matches!(
                DemosaicDcb.lazy(&mosaic, BayerPattern::GBRG),
                Err(DemosaicError::InvalidInput(_))
            ));
        }
    }

    #[test]
    fn public_dcb_accepts_the_three_pixel_bridge_minimum() {
        let mosaic = MonoImage::from_raw(3_u32, 3_u32, vec![0.0_f32; 9]).unwrap();
        let output = DemosaicDcb
            .lazy(&mosaic, BayerPattern::GBRG)
            .unwrap()
            .collect();
        assert!(output.as_raw().iter().all(|sample| *sample == 0.0));
    }
}
