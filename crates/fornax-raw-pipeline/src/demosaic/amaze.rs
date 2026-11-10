//! AMaZE (Aliasing Minimization and Zipper Elimination) Bayer demosaicing.
//!
//! This independent Rust implementation follows the behavioral conventions of
//! darktable 5.6.2 and RawTherapee 5.13. See the
//! [AMaZE reference notes](https://github.com/Glatzel/fornax/blob/main/docs/references/amaze.md).

#![allow(
    clippy::suboptimal_flops,
    reason = "the reference comparison requires its exact non-fused operation order"
)]

use fornax_raw_core::error::DemosaicError;
use fornax_raw_core::{BayerPattern, MonoImage, Primitive};

use super::buffer::{DenseLazy, channel, dense_from_f32, normalized};
use super::ppg::DemosaicPpg;
use crate::LazyDemosaic;

const RED: usize = 0;
const GREEN: usize = 1;

const TILE_SIZE: i32 = 160;
const TILE_STRIDE: i32 = TILE_SIZE - 32;
const HALO: i32 = 16;
const TILE_PIXELS: usize = TILE_SIZE as usize * TILE_SIZE as usize;
const EPS: f32 = 1.0e-5;
const EPS_SQUARED: f32 = 1.0e-10;
const ADAPTIVE_RATIO_THRESHOLD: f32 = 0.75;

const GAUSS_ODD: [f32; 4] = [0.146_597_28, 0.103_592_716, 0.073_203_616, 0.036_554_355];
const GAUSS_GRADIENT: [f32; 6] = [
    0.036_922_06,
    0.031_037_56,
    0.026_090_91,
    0.018_437_096,
    0.015_498_661,
    0.009_206_597,
];
const GAUSS_EVEN: [f32; 2] = [0.137_194_94, 0.056_402_528];
const GAUSS_QUINCUNX: [f32; 4] = [0.169_917, 0.108_947, 0.069_855, 0.028_718_2];

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicAmaze;

impl DemosaicAmaze {
    pub fn lazy<T>(
        &self,
        image: &MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<DenseLazy<T>, DemosaicError>
    where
        T: Primitive + Send + Sync,
    {
        if image.width() < 32 || image.height() < 32 {
            let samples = normalized(image);
            let fallback =
                MonoImage::from_raw(image.width(), image.height(), samples).ok_or_else(|| {
                    DemosaicError::InvalidInput("AMaZE could not copy the Bayer mosaic".into())
                })?;
            let rgb: fornax_raw_core::RgbImage<f32> =
                DemosaicPpg.lazy(&fallback, pattern)?.collect();
            return Ok(DenseLazy::new(dense_from_f32(
                image.width(),
                image.height(),
                rgb.as_raw().clone(),
            )));
        }

        let input = normalized(image);
        let output = run(
            &input,
            image.width() as usize,
            image.height() as usize,
            pattern,
        );
        Ok(DenseLazy::new(dense_from_f32(
            image.width(),
            image.height(),
            output,
        )))
    }
}

struct Workspace {
    cfa: Vec<f32>,
    green: Vec<f32>,
    gradient_squared_sum: Vec<f32>,
    vertical_weights: Vec<f32>,
    horizontal_weights: Vec<f32>,
    vertical_difference: Vec<f32>,
    horizontal_difference: Vec<f32>,
    vertical_alternative: Vec<f32>,
    horizontal_alternative: Vec<f32>,
    difference_squared: Vec<f32>,
    horizontal_vertical_weight: Vec<f32>,
    plus_gradient: Vec<f32>,
    minus_gradient: Vec<f32>,
    red_blue_interpolated: Vec<f32>,
    green_curvature: Vec<[f32; 2]>,
    vertical_green_delta: Vec<f32>,
    horizontal_green_delta: Vec<f32>,
    diagonal_minus_variance: Vec<f32>,
    diagonal_plus_variance: Vec<f32>,
    plus_minus_weight: Vec<f32>,
    red_blue_minus: Vec<f32>,
    red_blue_plus: Vec<f32>,
    nyquist: Vec<u8>,
    nyquist_refined: Vec<u8>,
    nyquist_test: Vec<f32>,
    green_minus_red: Vec<f32>,
    green_minus_blue: Vec<f32>,
}

impl Workspace {
    fn new() -> Self {
        Self {
            cfa: vec![0.0; TILE_PIXELS],
            green: vec![0.0; TILE_PIXELS],
            gradient_squared_sum: vec![0.0; TILE_PIXELS],
            vertical_weights: vec![0.0; TILE_PIXELS],
            horizontal_weights: vec![0.0; TILE_PIXELS],
            vertical_difference: vec![0.0; TILE_PIXELS],
            horizontal_difference: vec![0.0; TILE_PIXELS],
            vertical_alternative: vec![0.0; TILE_PIXELS],
            horizontal_alternative: vec![0.0; TILE_PIXELS],
            difference_squared: vec![0.0; TILE_PIXELS],
            horizontal_vertical_weight: vec![0.0; TILE_PIXELS],
            plus_gradient: vec![0.0; TILE_PIXELS],
            minus_gradient: vec![0.0; TILE_PIXELS],
            red_blue_interpolated: vec![0.0; TILE_PIXELS],
            green_curvature: vec![[0.0; 2]; TILE_PIXELS],
            vertical_green_delta: vec![0.0; TILE_PIXELS],
            horizontal_green_delta: vec![0.0; TILE_PIXELS],
            diagonal_minus_variance: vec![0.0; TILE_PIXELS],
            diagonal_plus_variance: vec![0.0; TILE_PIXELS],
            plus_minus_weight: vec![0.0; TILE_PIXELS],
            red_blue_minus: vec![0.0; TILE_PIXELS],
            red_blue_plus: vec![0.0; TILE_PIXELS],
            nyquist: vec![0; TILE_PIXELS],
            nyquist_refined: vec![0; TILE_PIXELS],
            nyquist_test: vec![0.0; TILE_PIXELS],
            green_minus_red: vec![0.0; TILE_PIXELS],
            green_minus_blue: vec![0.0; TILE_PIXELS],
        }
    }
}

fn run(input: &[f32], width: usize, height: usize, pattern: BayerPattern) -> Vec<f32> {
    let mut output = vec![0.0; width * height * 3];
    let mut workspace = Workspace::new();
    for (left, top) in tile_origins(width as u32, height as u32) {
        process_tile(
            input,
            &mut output,
            &mut workspace,
            width,
            height,
            pattern,
            left,
            top,
        );
    }
    output
}

fn process_tile(
    input: &[f32],
    output: &mut [f32],
    workspace: &mut Workspace,
    width: usize,
    height: usize,
    pattern: BayerPattern,
    left: i32,
    top: i32,
) {
    workspace.nyquist.fill(0);
    workspace.nyquist_refined.fill(0);

    let bottom = (top + TILE_SIZE).min(height as i32 + HALO);
    let right = (left + TILE_SIZE).min(width as i32 + HALO);
    let tile_height = (bottom - top) as usize;
    let tile_width = (right - left) as usize;
    let row_min = if top < 0 { HALO as usize } else { 0 };
    let col_min = if left < 0 { HALO as usize } else { 0 };
    let row_max = if bottom > height as i32 {
        (height as i32 - top) as usize
    } else {
        tile_height
    };
    let col_max = if right > width as i32 {
        (width as i32 - left) as usize
    } else {
        tile_width
    };

    fill_tile(
        input,
        workspace,
        width,
        height,
        left,
        top,
        row_min,
        row_max,
        col_min,
        col_max,
        tile_height,
        tile_width,
    );
    calculate_gradients(workspace, tile_height, tile_width);
    interpolate_differences(workspace, tile_height, tile_width, pattern);
    correct_highlight_differences(workspace, tile_height, tile_width, pattern);
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
    write_rgb(
        output,
        workspace,
        width,
        height,
        pattern,
        left,
        top,
        tile_height,
        tile_width,
    );
}

fn fill_tile(
    input: &[f32],
    workspace: &mut Workspace,
    width: usize,
    height: usize,
    left: i32,
    top: i32,
    row_min: usize,
    row_max: usize,
    col_min: usize,
    col_max: usize,
    tile_height: usize,
    tile_width: usize,
) {
    let stride = TILE_SIZE as usize;
    if row_min > 0 {
        for rr in 0..HALO as usize {
            let row = 32 - rr as i32 + top;
            for cc in col_min..col_max {
                let index = rr * stride + cc;
                let value = input[source_index(width, col_left(left, cc), row)];
                workspace.cfa[index] = value;
                workspace.green[index] = value;
            }
        }
    }

    for rr in row_min..row_max {
        let row = rr as i32 + top;
        for cc in col_min..col_max {
            let index = rr * stride + cc;
            let value = input[source_index(width, col_left(left, cc), row)];
            workspace.cfa[index] = value;
            workspace.green[index] = value;
        }
    }

    if row_max < tile_height {
        for rr in 0..(HALO as usize).min(tile_height - row_max) {
            let row = height as i32 - rr as i32 - 2;
            for cc in col_min..col_max {
                let index = (row_max + rr) * stride + cc;
                let value = input[source_index(width, col_left(left, cc), row)];
                workspace.cfa[index] = value;
                workspace.green[index] = value;
            }
        }
    }

    if col_min > 0 {
        for rr in row_min..row_max {
            let row = rr as i32 + top;
            for cc in 0..HALO as usize {
                let index = rr * stride + cc;
                let value = input[source_index(width, 32 - cc as i32 + left, row)];
                workspace.cfa[index] = value;
                workspace.green[index] = value;
            }
        }
    }

    if col_max < tile_width {
        for rr in row_min..row_max {
            let row = rr as i32 + top;
            for cc in 0..(HALO as usize).min(tile_width - col_max) {
                let index = rr * stride + col_max + cc;
                let value = input[source_index(width, width as i32 - cc as i32 - 2, row)];
                workspace.cfa[index] = value;
                workspace.green[index] = value;
            }
        }
    }

    if row_min > 0 && col_min > 0 {
        fill_corner(
            input,
            workspace,
            width,
            true,
            true,
            0,
            0,
            tile_height,
            tile_width,
        );
    }
    if row_max < tile_height && col_max < tile_width {
        fill_corner(
            input,
            workspace,
            width,
            false,
            false,
            row_max,
            col_max,
            tile_height,
            tile_width,
        );
    }
    if row_min > 0 && col_max < tile_width {
        fill_corner(
            input,
            workspace,
            width,
            true,
            false,
            0,
            col_max,
            tile_height,
            tile_width,
        );
    }
    if row_max < tile_height && col_min > 0 {
        fill_corner(
            input,
            workspace,
            width,
            false,
            true,
            row_max,
            0,
            tile_height,
            tile_width,
        );
    }
}

fn fill_corner(
    input: &[f32],
    workspace: &mut Workspace,
    width: usize,
    upper: bool,
    left_edge: bool,
    row_offset: usize,
    col_offset: usize,
    tile_height: usize,
    tile_width: usize,
) {
    let height = input.len() / width;
    let stride = TILE_SIZE as usize;
    for rr in 0..(HALO as usize).min(tile_height - row_offset) {
        for cc in 0..(HALO as usize).min(tile_width - col_offset) {
            let row = if upper {
                32 - rr as i32
            } else {
                height as i32 - rr as i32 - 2
            };
            let col = if left_edge {
                32 - cc as i32
            } else {
                width as i32 - cc as i32 - 2
            };
            let index = (row_offset + rr) * stride + col_offset + cc;
            let value = input[source_index(width, col, row)];
            workspace.cfa[index] = value;
            workspace.green[index] = value;
        }
    }
}

const fn col_left(left: i32, cc: usize) -> i32 { left + cc as i32 }

const fn source_index(width: usize, col: i32, row: i32) -> usize {
    row as usize * width + col as usize
}

fn calculate_gradients(workspace: &mut Workspace, tile_height: usize, tile_width: usize) {
    let ts = TILE_SIZE as usize;
    for rr in 2..tile_height - 2 {
        for cc in 2..tile_width - 2 {
            let index = rr * ts + cc;
            let horizontal = (workspace.cfa[index + 1] - workspace.cfa[index - 1]).abs();
            let vertical = (workspace.cfa[index + ts] - workspace.cfa[index - ts]).abs();
            workspace.vertical_weights[index] = EPS
                + (workspace.cfa[index + 2 * ts] - workspace.cfa[index]).abs()
                + (workspace.cfa[index] - workspace.cfa[index - 2 * ts]).abs()
                + vertical;
            workspace.horizontal_weights[index] = EPS
                + (workspace.cfa[index + 2] - workspace.cfa[index]).abs()
                + (workspace.cfa[index] - workspace.cfa[index - 2]).abs()
                + horizontal;
            workspace.gradient_squared_sum[index] = horizontal * horizontal + vertical * vertical;
        }
    }
}

fn interpolate_differences(
    workspace: &mut Workspace,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) {
    let ts = TILE_SIZE as usize;
    for rr in 4..tile_height - 4 {
        for cc in 4..tile_width - 4 {
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
            );
            let ratio_down = color_ratio(
                center,
                down,
                workspace.vertical_weights[index + 2 * ts],
                workspace.cfa[index + 2 * ts],
                workspace.vertical_weights[index],
            );
            let ratio_left = color_ratio(
                center,
                left,
                workspace.horizontal_weights[index - 2],
                workspace.cfa[index - 2],
                workspace.horizontal_weights[index],
            );
            let ratio_right = color_ratio(
                center,
                right,
                workspace.horizontal_weights[index + 2],
                workspace.cfa[index + 2],
                workspace.horizontal_weights[index],
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

            let highlight = center > 0.8 || vertical_ha > 0.8 || horizontal_ha > 0.8;
            if highlight {
                workspace.vertical_difference[index] = workspace.vertical_alternative[index];
                workspace.horizontal_difference[index] = workspace.horizontal_alternative[index];
            }

            workspace.vertical_green_delta[index] = if highlight {
                square(up_ha - down_ha)
            } else {
                square(up_ha - down_ha).min(square(up_adaptive - down_adaptive))
            };
            workspace.horizontal_green_delta[index] = if highlight {
                square(left_ha - right_ha)
            } else {
                square(left_ha - right_ha).min(square(left_adaptive - right_adaptive))
            };
        }
    }
}

fn correct_highlight_differences(
    workspace: &mut Workspace,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) {
    let ts = TILE_SIZE as usize;
    for rr in 4..tile_height - 4 {
        for cc in 4..tile_width - 4 {
            let index = rr * ts + cc;
            let horizontal_variance = 3.0
                * (square(workspace.horizontal_difference[index - 2])
                    + square(workspace.horizontal_difference[index])
                    + square(workspace.horizontal_difference[index + 2]))
                - square(
                    workspace.horizontal_difference[index - 2]
                        + workspace.horizontal_difference[index]
                        + workspace.horizontal_difference[index + 2],
                );
            let horizontal_alternative_variance = 3.0
                * (square(workspace.horizontal_alternative[index - 2])
                    + square(workspace.horizontal_alternative[index])
                    + square(workspace.horizontal_alternative[index + 2]))
                - square(
                    workspace.horizontal_alternative[index - 2]
                        + workspace.horizontal_alternative[index]
                        + workspace.horizontal_alternative[index + 2],
                );
            let vertical_variance = 3.0
                * (square(workspace.vertical_difference[index - 2 * ts])
                    + square(workspace.vertical_difference[index])
                    + square(workspace.vertical_difference[index + 2 * ts]))
                - square(
                    workspace.vertical_difference[index - 2 * ts]
                        + workspace.vertical_difference[index]
                        + workspace.vertical_difference[index + 2 * ts],
                );
            let vertical_alternative_variance = 3.0
                * (square(workspace.vertical_alternative[index - 2 * ts])
                    + square(workspace.vertical_alternative[index])
                    + square(workspace.vertical_alternative[index + 2 * ts]))
                - square(
                    workspace.vertical_alternative[index - 2 * ts]
                        + workspace.vertical_alternative[index]
                        + workspace.vertical_alternative[index + 2 * ts],
                );

            if horizontal_alternative_variance < horizontal_variance {
                workspace.horizontal_difference[index] = workspace.horizontal_alternative[index];
            }
            if vertical_alternative_variance < vertical_variance {
                workspace.vertical_difference[index] = workspace.vertical_alternative[index];
            }

            if is_green(pattern, cc, rr) {
                let horizontal_green =
                    -workspace.horizontal_difference[index] + workspace.cfa[index];
                let vertical_green = -workspace.vertical_difference[index] + workspace.cfa[index];
                bound_highlight_difference(workspace, index, horizontal_green, true, true);
                bound_highlight_difference(workspace, index, vertical_green, false, true);
            } else {
                let horizontal_green =
                    workspace.horizontal_difference[index] + workspace.cfa[index];
                let vertical_green = workspace.vertical_difference[index] + workspace.cfa[index];
                bound_highlight_difference(workspace, index, horizontal_green, true, false);
                bound_highlight_difference(workspace, index, vertical_green, false, false);
                workspace.difference_squared[index] = square(
                    workspace.vertical_difference[index] - workspace.horizontal_difference[index],
                );
            }
        }
    }
}

fn bound_highlight_difference(
    workspace: &mut Workspace,
    index: usize,
    green: f32,
    horizontal: bool,
    green_site: bool,
) {
    let ts = TILE_SIZE as usize;
    let center = workspace.cfa[index];
    let difference = if horizontal {
        workspace.horizontal_difference[index]
    } else {
        workspace.vertical_difference[index]
    };
    let first = if horizontal {
        workspace.cfa[index - 1]
    } else {
        workspace.cfa[index - ts]
    };
    let second = if horizontal {
        workspace.cfa[index + 1]
    } else {
        workspace.cfa[index + ts]
    };
    let mut corrected = difference;

    if green_site {
        if corrected > 0.0 {
            if 3.0 * corrected > green + center {
                corrected = -ulim(green, first, second) + center;
            } else {
                let weight = 1.0 - 3.0 * corrected / (EPS + green + center);
                corrected =
                    weight * corrected + (1.0 - weight) * (-ulim(green, first, second) + center);
            }
        }
        if green > 1.0 {
            corrected = -ulim(green, first, second) + center;
        }
    } else {
        if corrected < 0.0 {
            if 3.0 * corrected < -(green + center) {
                corrected = ulim(green, first, second) - center;
            } else {
                let weight = 1.0 + 3.0 * corrected / (EPS + green + center);
                corrected =
                    weight * corrected + (1.0 - weight) * (ulim(green, first, second) - center);
            }
        }
        if green > 1.0 {
            corrected = ulim(green, first, second) - center;
        }
    }

    if horizontal {
        workspace.horizontal_difference[index] = corrected;
    } else {
        workspace.vertical_difference[index] = corrected;
    }
}

fn color_ratio(
    center: f32,
    neighbor: f32,
    neighbor_weight: f32,
    two_away: f32,
    center_weight: f32,
) -> f32 {
    neighbor * (neighbor_weight + center_weight)
        / (neighbor_weight * (EPS + center) + center_weight * (EPS + two_away))
}

const fn interpolate(weight: f32, first: f32, second: f32) -> f32 {
    weight * (first - second) + second
}

const fn half(value: f32) -> f32 { 0.5 * value }

fn div_exponent(value: f32, shift: u32) -> f32 {
    if value == 0.0 {
        value
    } else {
        f32::from_bits(value.to_bits() - (shift << 23))
    }
}

fn mul_exponent(value: f32) -> f32 {
    if value == 0.0 {
        value
    } else {
        f32::from_bits(value.to_bits() + (1 << 23))
    }
}

fn ulim(value: f32, first: f32, second: f32) -> f32 {
    if first < second {
        value.clamp(first, second)
    } else {
        value.clamp(second, first)
    }
}

fn square(value: f32) -> f32 { value * value }

const fn is_green(pattern: BayerPattern, x: usize, y: usize) -> bool {
    channel(pattern, x as u32, y as u32) == GREEN
}

const fn is_red(pattern: BayerPattern, x: usize, y: usize) -> bool {
    channel(pattern, x as u32, y as u32) == RED
}

#[derive(Clone, Copy)]
struct NyquistRegion {
    row_start: usize,
    row_end: usize,
    col_start: usize,
    col_end: usize,
}

fn calculate_horizontal_vertical_weights(
    workspace: &mut Workspace,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) {
    let ts = TILE_SIZE as usize;
    for rr in 6..tile_height - 6 {
        let start = 6 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 6).step_by(2) {
            let index = rr * ts + cc;
            let up_average = workspace.vertical_difference[index]
                + workspace.vertical_difference[index - ts]
                + workspace.vertical_difference[index - 2 * ts]
                + workspace.vertical_difference[index - 3 * ts];
            let down_average = workspace.vertical_difference[index]
                + workspace.vertical_difference[index + ts]
                + workspace.vertical_difference[index + 2 * ts]
                + workspace.vertical_difference[index + 3 * ts];
            let left_average = workspace.horizontal_difference[index]
                + workspace.horizontal_difference[index - 1]
                + workspace.horizontal_difference[index - 2]
                + workspace.horizontal_difference[index - 3];
            let right_average = workspace.horizontal_difference[index]
                + workspace.horizontal_difference[index + 1]
                + workspace.horizontal_difference[index + 2]
                + workspace.horizontal_difference[index + 3];

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
            let vertical_color_variance = EPS_SQUARED
                + vertical_weight * down_variance
                + (1.0 - vertical_weight) * up_variance;
            let horizontal_color_variance = EPS_SQUARED
                + horizontal_weight * right_variance
                + (1.0 - horizontal_weight) * left_variance;

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
            let vertical_fluctuation = EPS_SQUARED
                + vertical_weight * down_fluctuation
                + (1.0 - vertical_weight) * up_fluctuation;
            let horizontal_fluctuation = EPS_SQUARED
                + horizontal_weight * right_fluctuation
                + (1.0 - horizontal_weight) * left_fluctuation;

            let variance_weight =
                horizontal_color_variance / (vertical_color_variance + horizontal_color_variance);
            let fluctuation_weight =
                horizontal_fluctuation / (vertical_fluctuation + horizontal_fluctuation);
            workspace.horizontal_vertical_weight[pair(index)] =
                if (0.5 - variance_weight) * (0.5 - fluctuation_weight) > 0.0
                    && (0.5 - fluctuation_weight).abs() < (0.5 - variance_weight).abs()
                {
                    variance_weight
                } else {
                    fluctuation_weight
                };
        }
    }
}

fn detect_nyquist_texture(
    workspace: &mut Workspace,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) -> Option<NyquistRegion> {
    let ts = TILE_SIZE as usize;
    let m1 = ts + 1;
    let m2 = 2 * ts + 2;
    let p1 = 1_i32 - ts as i32;
    let p2 = 2_i32 - 2 * ts as i32;
    for rr in 6..tile_height - 6 {
        let start = 6 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 6).step_by(2) {
            let index = rr * ts + cc;
            let index_p1 = offset(index, p1);
            let index_p2 = offset(index, p2);
            let index_m1 = offset(index, -p1);
            let index_m2 = offset(index, -p2);
            workspace.nyquist_test[pair(index)] = GAUSS_ODD[0]
                * workspace.difference_squared[index]
                + GAUSS_ODD[1]
                    * (workspace.difference_squared[index - m1]
                        + workspace.difference_squared[index_p1]
                        + workspace.difference_squared[index_m1]
                        + workspace.difference_squared[index + m1])
                + GAUSS_ODD[2]
                    * (workspace.difference_squared[index - 2 * ts]
                        + workspace.difference_squared[index - 2]
                        + workspace.difference_squared[index + 2]
                        + workspace.difference_squared[index + 2 * ts])
                + GAUSS_ODD[3]
                    * (workspace.difference_squared[index - m2]
                        + workspace.difference_squared[index_p2]
                        + workspace.difference_squared[index_m2]
                        + workspace.difference_squared[index + m2])
                - (GAUSS_GRADIENT[0] * workspace.gradient_squared_sum[index]
                    + GAUSS_GRADIENT[1]
                        * (workspace.gradient_squared_sum[index - ts]
                            + workspace.gradient_squared_sum[index + 1]
                            + workspace.gradient_squared_sum[index - 1]
                            + workspace.gradient_squared_sum[index + ts])
                    + GAUSS_GRADIENT[2]
                        * (workspace.gradient_squared_sum[index - m1]
                            + workspace.gradient_squared_sum[index_p1]
                            + workspace.gradient_squared_sum[index_m1]
                            + workspace.gradient_squared_sum[index + m1])
                    + GAUSS_GRADIENT[3]
                        * (workspace.gradient_squared_sum[index - 2 * ts]
                            + workspace.gradient_squared_sum[index - 2]
                            + workspace.gradient_squared_sum[index + 2]
                            + workspace.gradient_squared_sum[index + 2 * ts])
                    + GAUSS_GRADIENT[4]
                        * (workspace.gradient_squared_sum[index - 2 * ts - 1]
                            + workspace.gradient_squared_sum[index - 2 * ts + 1]
                            + workspace.gradient_squared_sum[index - ts - 2]
                            + workspace.gradient_squared_sum[index - ts + 2]
                            + workspace.gradient_squared_sum[index + ts - 2]
                            + workspace.gradient_squared_sum[index + ts + 2]
                            + workspace.gradient_squared_sum[index + 2 * ts - 1]
                            + workspace.gradient_squared_sum[index + 2 * ts + 1])
                    + GAUSS_GRADIENT[5]
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
            if workspace.nyquist_test[pair(index)] > 0.0 {
                workspace.nyquist[pair(index)] = 1;
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
    let mut row_end = end_row + 1;
    let mut col_end = end_col + 1;
    if start_col & 1 == 1 {
        start_col -= 1;
    }
    let row_start = start_row.max(8);
    let col_start = start_col.max(8);
    row_end = row_end.min(tile_height - 8);
    col_end = col_end.min(tile_width - 8);
    Some(NyquistRegion {
        row_start,
        row_end,
        col_start,
        col_end,
    })
}

fn refine_nyquist_texture(workspace: &mut Workspace, region: NyquistRegion, pattern: BayerPattern) {
    let ts = TILE_SIZE as usize;
    let m1 = ts + 1;
    for rr in region.row_start..region.row_end {
        let start = region.col_start + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..region.col_end).step_by(2) {
            let index = rr * ts + cc;
            let count = workspace.nyquist[pair(index - 2 * ts)]
                + workspace.nyquist[pair(index - m1)]
                + workspace.nyquist[pair(offset(index, 1 - ts as i32))]
                + workspace.nyquist[pair(index - 2)]
                + workspace.nyquist[pair(index + 2)]
                + workspace.nyquist[pair(offset(index, ts as i32 - 1))]
                + workspace.nyquist[pair(index + m1)]
                + workspace.nyquist[pair(index + 2 * ts)];
            workspace.nyquist_refined[pair(index)] = match count {
                0..=3 => 0,
                5..=u8::MAX => 1,
                _ => workspace.nyquist[pair(index)],
            };
        }
    }

    for rr in region.row_start..region.row_end {
        let start = region.col_start + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..region.col_end).step_by(2) {
            let index = rr * ts + cc;
            if workspace.nyquist_refined[pair(index)] == 0 {
                continue;
            }
            let mut cfa_sum = 0.0;
            let mut horizontal_sum = 0.0;
            let mut vertical_sum = 0.0;
            let mut horizontal_squared_sum = 0.0;
            let mut vertical_squared_sum = 0.0;
            let mut area_weight = 0.0;
            for dy in (-6..=6).step_by(2) {
                for dx in (-6..=6).step_by(2) {
                    let sample = (index as i32 + dy * TILE_SIZE + dx) as usize;
                    if workspace.nyquist_refined[pair(sample)] == 0 {
                        continue;
                    }
                    let value = workspace.cfa[sample];
                    cfa_sum += value;
                    horizontal_sum += workspace.cfa[sample - 1] + workspace.cfa[sample + 1];
                    vertical_sum += workspace.cfa[sample - ts] + workspace.cfa[sample + ts];
                    horizontal_squared_sum += square(value - workspace.cfa[sample - 1])
                        + square(value - workspace.cfa[sample + 1]);
                    vertical_squared_sum += square(value - workspace.cfa[sample - ts])
                        + square(value - workspace.cfa[sample + ts]);
                    area_weight += 1.0;
                }
            }
            horizontal_sum = cfa_sum - half(horizontal_sum);
            vertical_sum = cfa_sum - half(vertical_sum);
            area_weight = half(area_weight);
            let horizontal_variance = EPS_SQUARED
                + (area_weight * horizontal_squared_sum - horizontal_sum * horizontal_sum).abs();
            let vertical_variance = EPS_SQUARED
                + (area_weight * vertical_squared_sum - vertical_sum * vertical_sum).abs();
            workspace.horizontal_vertical_weight[pair(index)] =
                horizontal_variance / (vertical_variance + horizontal_variance);
        }
    }
}

fn interpolate_green_at_red_blue(
    workspace: &mut Workspace,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) {
    let ts = TILE_SIZE as usize;
    let m1 = ts + 1;
    for rr in 8..tile_height - 8 {
        let start = 8 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 8).step_by(2) {
            let index = rr * ts + cc;
            let index_m1 = index - m1;
            let index_p1 = offset(index, 1 - ts as i32);
            let index_neg_p1 = offset(index, ts as i32 - 1);
            let alternate = div_exponent(
                workspace.horizontal_vertical_weight[pair(index_m1)]
                    + workspace.horizontal_vertical_weight[pair(index_p1)]
                    + workspace.horizontal_vertical_weight[pair(index_neg_p1)]
                    + workspace.horizontal_vertical_weight[pair(index + m1)],
                2,
            );
            if (0.5 - workspace.horizontal_vertical_weight[pair(index)]).abs()
                < (0.5 - alternate).abs()
            {
                workspace.horizontal_vertical_weight[pair(index)] = alternate;
            }
            workspace.green_minus_red[pair(index)] = interpolate(
                workspace.horizontal_vertical_weight[pair(index)],
                workspace.vertical_difference[index],
                workspace.horizontal_difference[index],
            );
            workspace.green[index] = workspace.cfa[index] + workspace.green_minus_red[pair(index)];
            if workspace.nyquist_refined[pair(index)] != 0 {
                workspace.green_curvature[pair(index)][0] = square(
                    workspace.green[index]
                        - half(workspace.green[index - 1] + workspace.green[index + 1]),
                );
                workspace.green_curvature[pair(index)][1] = square(
                    workspace.green[index]
                        - half(workspace.green[index - ts] + workspace.green[index + ts]),
                );
            } else {
                workspace.green_curvature[pair(index)] = [0.0; 2];
            }
        }
    }
}

fn refine_nyquist_green(workspace: &mut Workspace, region: NyquistRegion, pattern: BayerPattern) {
    let ts = TILE_SIZE as usize;
    let m1 = ts + 1;
    let m2 = 2 * ts + 2;
    for rr in region.row_start..region.row_end {
        let start = region.col_start + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..region.col_end).step_by(2) {
            let index = rr * ts + cc;
            if workspace.nyquist_refined[pair(index)] == 0 {
                continue;
            }
            let mut horizontal_variance = EPS_SQUARED;
            let mut vertical_variance = EPS_SQUARED;
            horizontal_variance += GAUSS_QUINCUNX[0] * workspace.green_curvature[pair(index)][0];
            vertical_variance += GAUSS_QUINCUNX[0] * workspace.green_curvature[pair(index)][1];
            for offset in [-(m1 as i32), 1 - ts as i32, ts as i32 - 1, m1 as i32] {
                let sample = offset_index(index, offset);
                horizontal_variance +=
                    GAUSS_QUINCUNX[1] * workspace.green_curvature[pair(sample)][0];
                vertical_variance += GAUSS_QUINCUNX[1] * workspace.green_curvature[pair(sample)][1];
            }
            for offset in [-(2 * ts as i32), -2, 2, 2 * ts as i32] {
                let sample = offset_index(index, offset);
                horizontal_variance +=
                    GAUSS_QUINCUNX[2] * workspace.green_curvature[pair(sample)][0];
                vertical_variance += GAUSS_QUINCUNX[2] * workspace.green_curvature[pair(sample)][1];
            }
            for offset in [
                -(m2 as i32),
                2 - 2 * ts as i32,
                2 * ts as i32 - 2,
                m2 as i32,
            ] {
                let sample = offset_index(index, offset);
                horizontal_variance +=
                    GAUSS_QUINCUNX[3] * workspace.green_curvature[pair(sample)][0];
                vertical_variance += GAUSS_QUINCUNX[3] * workspace.green_curvature[pair(sample)][1];
            }
            workspace.green_minus_red[pair(index)] = (workspace.horizontal_difference[index]
                * vertical_variance
                + workspace.vertical_difference[index] * horizontal_variance)
                / (vertical_variance + horizontal_variance);
            workspace.green[index] = workspace.cfa[index] + workspace.green_minus_red[pair(index)];
        }
    }
}

const fn offset(index: usize, delta: i32) -> usize { offset_index(index, delta) }

const fn offset_index(index: usize, delta: i32) -> usize { (index as i32 + delta) as usize }

const fn pair(index: usize) -> usize { index / 2 }

fn prepare_diagonal_interpolation(
    workspace: &mut Workspace,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) {
    let ts = TILE_SIZE as usize;
    let p1 = 1 - ts as i32;
    let m1 = ts as i32 + 1;
    for rr in 6..tile_height - 6 {
        if is_green(pattern, 2, rr) {
            for cc in (6..tile_width - 6).step_by(2) {
                let index = rr * ts + cc;
                workspace.diagonal_plus_variance[pair(index)] =
                    square(workspace.cfa[index] - workspace.cfa[offset(index, -p1)])
                        + square(workspace.cfa[index] - workspace.cfa[offset(index, p1)]);
                workspace.diagonal_minus_variance[pair(index)] =
                    square(workspace.cfa[index] - workspace.cfa[offset(index, -m1)])
                        + square(workspace.cfa[index] - workspace.cfa[offset(index, m1)]);
                workspace.plus_gradient[pair(index)] = (workspace.cfa[offset(index + 1, p1)]
                    - workspace.cfa[offset(index + 1, -p1)])
                .abs();
                workspace.minus_gradient[pair(index)] = (workspace.cfa[offset(index + 1, m1)]
                    - workspace.cfa[offset(index + 1, -m1)])
                .abs();
            }
        } else {
            for cc in (6..tile_width - 6).step_by(2) {
                let index = rr * ts + cc;
                workspace.plus_gradient[pair(index)] =
                    (workspace.cfa[offset(index, p1)] - workspace.cfa[offset(index, -p1)]).abs();
                workspace.minus_gradient[pair(index)] =
                    (workspace.cfa[offset(index, m1)] - workspace.cfa[offset(index, -m1)]).abs();
                workspace.diagonal_plus_variance[pair(index)] =
                    square(workspace.cfa[index + 1] - workspace.cfa[offset(index + 1, -p1)])
                        + square(workspace.cfa[index + 1] - workspace.cfa[offset(index + 1, p1)]);
                workspace.diagonal_minus_variance[pair(index)] =
                    square(workspace.cfa[index + 1] - workspace.cfa[offset(index + 1, -m1)])
                        + square(workspace.cfa[index + 1] - workspace.cfa[offset(index + 1, m1)]);
            }
        }
    }
}

fn diagonal_ratio(neighbor: f32, center: f32, two_away: f32) -> f32 {
    mul_exponent(neighbor) / (EPS + center + two_away)
}

fn interpolate_diagonal_red_blue(
    workspace: &mut Workspace,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) {
    let ts = TILE_SIZE as usize;
    let p1 = 1 - ts as i32;
    let p2 = 2 - 2 * ts as i32;
    let m1 = ts as i32 + 1;
    let m2 = 2 * ts as i32 + 2;
    for rr in 8..tile_height - 8 {
        let start = 8 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 8).step_by(2) {
            let index = rr * ts + cc;
            let center = workspace.cfa[index];
            let ratio_se = diagonal_ratio(
                workspace.cfa[offset(index, m1)],
                center,
                workspace.cfa[offset(index, m2)],
            );
            let ratio_nw = diagonal_ratio(
                workspace.cfa[offset(index, -m1)],
                center,
                workspace.cfa[offset(index, -m2)],
            );
            let ratio_ne = diagonal_ratio(
                workspace.cfa[offset(index, p1)],
                center,
                workspace.cfa[offset(index, p2)],
            );
            let ratio_sw = diagonal_ratio(
                workspace.cfa[offset(index, -p1)],
                center,
                workspace.cfa[offset(index, -p2)],
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

            let weight_se = EPS
                + workspace.minus_gradient[pair(index)]
                + workspace.minus_gradient[pair(offset(index, m1))]
                + workspace.minus_gradient[pair(offset(index, m2))];
            let weight_nw = EPS
                + workspace.minus_gradient[pair(index)]
                + workspace.minus_gradient[pair(offset(index, -m1))]
                + workspace.minus_gradient[pair(offset(index, -m2))];
            let weight_ne = EPS
                + workspace.plus_gradient[pair(index)]
                + workspace.plus_gradient[pair(offset(index, p1))]
                + workspace.plus_gradient[pair(offset(index, p2))];
            let weight_sw = EPS
                + workspace.plus_gradient[pair(index)]
                + workspace.plus_gradient[pair(offset(index, -p1))]
                + workspace.plus_gradient[pair(offset(index, -p2))];
            workspace.red_blue_minus[pair(index)] =
                (weight_se * rb_nw + weight_nw * rb_se) / (weight_se + weight_nw);
            workspace.red_blue_plus[pair(index)] =
                (weight_ne * rb_sw + weight_sw * rb_ne) / (weight_ne + weight_sw);

            let minus_variance =
                EPS_SQUARED + diagonal_variance(&workspace.diagonal_minus_variance, index, ts);
            let plus_variance =
                EPS_SQUARED + diagonal_variance(&workspace.diagonal_plus_variance, index, ts);
            workspace.plus_minus_weight[pair(index)] =
                minus_variance / (plus_variance + minus_variance);

            clamp_diagonal(workspace, index, center, true);
            clamp_diagonal(workspace, index, center, false);
        }
    }
}

fn diagonal_variance(values: &[f32], index: usize, ts: usize) -> f32 {
    GAUSS_EVEN[0]
        * (values[pair(index - ts)]
            + values[pair(index - 1)]
            + values[pair(index + 1)]
            + values[pair(index + ts)])
        + GAUSS_EVEN[1]
            * (values[pair(offset(index, -2 * ts as i32 - 1))]
                + values[pair(offset(index, -2 * ts as i32 + 1))]
                + values[pair(offset(index, -2 - ts as i32))]
                + values[pair(offset(index, 2 - ts as i32))]
                + values[pair(offset(index, -2 + ts as i32))]
                + values[pair(offset(index, 2 + ts as i32))]
                + values[pair(offset(index, 2 * ts as i32 - 1))]
                + values[pair(offset(index, 2 * ts as i32 + 1))])
}

fn clamp_diagonal(workspace: &mut Workspace, index: usize, center: f32, plus: bool) {
    let ts = TILE_SIZE as usize;
    let p1 = 1 - ts as i32;
    let m1 = ts as i32 + 1;
    let (value, first, second) = if plus {
        (
            workspace.red_blue_plus[pair(index)],
            workspace.cfa[offset(index, p1)],
            workspace.cfa[offset(index, -p1)],
        )
    } else {
        (
            workspace.red_blue_minus[pair(index)],
            workspace.cfa[offset(index, m1)],
            workspace.cfa[offset(index, -m1)],
        )
    };
    let mut result = value;
    if result < center {
        if mul_exponent(result) < center {
            result = ulim(result, first, second);
        } else {
            let weight = mul_exponent(center - result) / (EPS + result + center);
            result = weight * result + (1.0 - weight) * ulim(result, first, second);
        }
    }
    if result > 1.0 {
        result = ulim(result, first, second);
    }
    if plus {
        workspace.red_blue_plus[pair(index)] = result;
    } else {
        workspace.red_blue_minus[pair(index)] = result;
    }
}

fn refine_green_from_red_blue(
    workspace: &mut Workspace,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) {
    let ts = TILE_SIZE as usize;
    let p1 = 1 - ts as i32;
    let m1 = ts as i32 + 1;
    for rr in 10..tile_height - 10 {
        let start = 10 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 10).step_by(2) {
            let index = rr * ts + cc;
            let alternate = div_exponent(
                workspace.plus_minus_weight[pair(offset(index, -m1))]
                    + workspace.plus_minus_weight[pair(offset(index, p1))]
                    + workspace.plus_minus_weight[pair(offset(index, -p1))]
                    + workspace.plus_minus_weight[pair(offset(index, m1))],
                2,
            );
            if (0.5 - workspace.plus_minus_weight[pair(index)]).abs() < (0.5 - alternate).abs() {
                workspace.plus_minus_weight[pair(index)] = alternate;
            }
            workspace.red_blue_interpolated[pair(index)] = half(
                workspace.cfa[index]
                    + workspace.red_blue_minus[pair(index)]
                        * (1.0 - workspace.plus_minus_weight[pair(index)])
                    + workspace.red_blue_plus[pair(index)]
                        * workspace.plus_minus_weight[pair(index)],
            );
        }
    }

    for rr in 12..tile_height - 12 {
        let start = 12 + (channel(pattern, 2, rr as u32) & 1);
        for cc in (start..tile_width - 12).step_by(2) {
            let index = rr * ts + cc;
            if (0.5 - workspace.plus_minus_weight[pair(index)]).abs()
                < (0.5 - workspace.horizontal_vertical_weight[pair(index)]).abs()
            {
                continue;
            }
            let pair_index = pair(index);
            let rb = workspace.red_blue_interpolated[pair_index];
            let ratio_up = workspace.cfa[index - ts] * 2.0
                / (EPS + rb + workspace.red_blue_interpolated[pair_index - ts]);
            let ratio_down = workspace.cfa[index + ts] * 2.0
                / (EPS + rb + workspace.red_blue_interpolated[pair_index + ts]);
            let ratio_left = workspace.cfa[index - 1] * 2.0
                / (EPS + rb + workspace.red_blue_interpolated[pair_index - 1]);
            let ratio_right = workspace.cfa[index + 1] * 2.0
                / (EPS + rb + workspace.red_blue_interpolated[pair_index + 1]);

            let green_up = adaptive_green(
                rb,
                ratio_up,
                workspace.cfa[index - ts]
                    + half(rb - workspace.red_blue_interpolated[pair_index - ts]),
            );
            let green_down = adaptive_green(
                rb,
                ratio_down,
                workspace.cfa[index + ts]
                    + half(rb - workspace.red_blue_interpolated[pair_index + ts]),
            );
            let green_left = adaptive_green(
                rb,
                ratio_left,
                workspace.cfa[index - 1]
                    + half(rb - workspace.red_blue_interpolated[pair_index - 1]),
            );
            let green_right = adaptive_green(
                rb,
                ratio_right,
                workspace.cfa[index + 1]
                    + half(rb - workspace.red_blue_interpolated[pair_index + 1]),
            );

            let mut green_vertical = (workspace.vertical_weights[index - ts] * green_down
                + workspace.vertical_weights[index + ts] * green_up)
                / (workspace.vertical_weights[index + ts] + workspace.vertical_weights[index - ts]);
            let mut green_horizontal = (workspace.horizontal_weights[index - 1] * green_right
                + workspace.horizontal_weights[index + 1] * green_left)
                / (workspace.horizontal_weights[index - 1]
                    + workspace.horizontal_weights[index + 1]);

            green_vertical = clamp_green_against_neighbors(
                green_vertical,
                rb,
                workspace.cfa[index - ts],
                workspace.cfa[index + ts],
            );
            green_horizontal = clamp_green_against_neighbors(
                green_horizontal,
                rb,
                workspace.cfa[index - 1],
                workspace.cfa[index + 1],
            );
            workspace.green[index] = green_horizontal
                * (1.0 - workspace.horizontal_vertical_weight[pair(index)])
                + green_vertical * workspace.horizontal_vertical_weight[pair(index)];
            workspace.green_minus_red[pair(index)] = workspace.green[index] - workspace.cfa[index];
        }
    }
}

fn clamp_green_against_neighbors(green: f32, rb: f32, first: f32, second: f32) -> f32 {
    let mut result = green;
    if result < rb {
        if 2.0 * result < rb {
            result = ulim(result, first, second);
        } else {
            let weight = 2.0 * (rb - result) / (EPS + result + rb);
            result = weight * result + (1.0 - weight) * ulim(result, first, second);
        }
    }
    if result > 1.0 {
        result = ulim(result, first, second);
    }
    result
}

fn split_green_differences(
    workspace: &mut Workspace,
    pattern: BayerPattern,
    tile_height: usize,
    tile_width: usize,
) {
    let (ex, ey) = red_phase(pattern);
    let mut rr = 13 - ey as i32;
    while rr < tile_height as i32 - 12 {
        let mut cc = 13 - ex as i32;
        while cc < tile_width as i32 - 12 {
            let index = pair((rr * TILE_SIZE + cc) as usize);
            workspace.green_minus_blue[index] = workspace.green_minus_red[index];
            workspace.green_minus_red[index] = 0.0;
            cc += 2;
        }
        rr += 2;
    }
}

fn interpolate_chroma(
    workspace: &mut Workspace,
    tile_height: usize,
    tile_width: usize,
    pattern: BayerPattern,
) {
    let ts = TILE_SIZE as usize;
    let p1 = 1 - ts as i32;
    let p3 = 3 - 3 * ts as i32;
    let m1 = ts as i32 + 1;
    let m3 = 3 * ts as i32 + 3;
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
            let weight_nw = 1.0
                / (EPS
                    + (values[pair(offset(index, -m1))] - values[pair(offset(index, m1))]).abs()
                    + (values[pair(offset(index, -m1))] - values[pair(offset(index, -m3))]).abs()
                    + (values[pair(offset(index, m1))] - values[pair(offset(index, -m3))]).abs());
            let weight_ne = 1.0
                / (EPS
                    + (values[pair(offset(index, p1))] - values[pair(offset(index, -p1))]).abs()
                    + (values[pair(offset(index, p1))] - values[pair(offset(index, p3))]).abs()
                    + (values[pair(offset(index, -p1))] - values[pair(offset(index, p3))]).abs());
            let weight_sw = 1.0
                / (EPS
                    + (values[pair(offset(index, -p1))] - values[pair(offset(index, p1))]).abs()
                    + (values[pair(offset(index, -p1))] - values[pair(offset(index, m3))]).abs()
                    + (values[pair(offset(index, p1))] - values[pair(offset(index, -p3))]).abs());
            let weight_se = 1.0
                / (EPS
                    + (values[pair(offset(index, m1))] - values[pair(offset(index, -m1))]).abs()
                    + (values[pair(offset(index, m1))] - values[pair(offset(index, -p3))]).abs()
                    + (values[pair(offset(index, -m1))] - values[pair(offset(index, m3))]).abs());
            let value = (weight_nw
                * (1.325 * values[pair(offset(index, -m1))]
                    - 0.175 * values[pair(offset(index, -m3))]
                    - 0.075 * values[pair(offset(index, -m1 - 2))]
                    - 0.075 * values[pair(offset(index, -m1 + -2 * ts as i32))])
                + weight_ne
                    * (1.325 * values[pair(offset(index, p1))]
                        - 0.175 * values[pair(offset(index, p3))]
                        - 0.075 * values[pair(offset(index, p1 + 2))]
                        - 0.075 * values[pair(offset(index, p1 + 2 * ts as i32))])
                + weight_sw
                    * (1.325 * values[pair(offset(index, -p1))]
                        - 0.175 * values[pair(offset(index, -p3))]
                        - 0.075 * values[pair(offset(index, -p1 - 2))]
                        - 0.075 * values[pair(offset(index, -p1 + -2 * ts as i32))])
                + weight_se
                    * (1.325 * values[pair(offset(index, m1))]
                        - 0.175 * values[pair(offset(index, m3))]
                        - 0.075 * values[pair(offset(index, m1 + 2))]
                        - 0.075 * values[pair(offset(index, m1 + 2 * ts as i32))]))
                / (weight_nw + weight_ne + weight_sw + weight_se);
            if c == 0 {
                workspace.green_minus_red[pair(index)] = value;
            } else {
                workspace.green_minus_blue[pair(index)] = value;
            }
        }
    }
}

fn write_rgb(
    output: &mut [f32],
    workspace: &Workspace,
    width: usize,
    height: usize,
    pattern: BayerPattern,
    left: i32,
    top: i32,
    tile_height: usize,
    tile_width: usize,
) {
    let ts = TILE_SIZE as usize;
    for rr in 16..tile_height - 16 {
        let row = rr as i32 + top;
        if row < 0 || row >= height as i32 {
            continue;
        }
        for cc in 16..tile_width - 16 {
            let col = cc as i32 + left;
            if col < 0 || col >= width as i32 {
                continue;
            }
            let index = rr * ts + cc;
            let pixel = (row as usize * width + col as usize) * 3;
            if is_green(pattern, cc, rr) {
                let temp = 1.0
                    / (workspace.horizontal_vertical_weight[pair(index - ts)] + 2.0
                        - workspace.horizontal_vertical_weight[pair(index + 1)]
                        - workspace.horizontal_vertical_weight[pair(index - 1)]
                        + workspace.horizontal_vertical_weight[pair(index + ts)]);
                let red_sum = workspace.horizontal_vertical_weight[pair(index - ts)]
                    * workspace.green_minus_red[pair(index - ts)]
                    + (1.0 - workspace.horizontal_vertical_weight[pair(index + 1)])
                        * workspace.green_minus_red[pair(index + 1)]
                    + (1.0 - workspace.horizontal_vertical_weight[pair(index - 1)])
                        * workspace.green_minus_red[pair(index - 1)]
                    + workspace.horizontal_vertical_weight[pair(index + ts)]
                        * workspace.green_minus_red[pair(index + ts)];
                let blue_sum = workspace.horizontal_vertical_weight[pair(index - ts)]
                    * workspace.green_minus_blue[pair(index - ts)]
                    + (1.0 - workspace.horizontal_vertical_weight[pair(index + 1)])
                        * workspace.green_minus_blue[pair(index + 1)]
                    + (1.0 - workspace.horizontal_vertical_weight[pair(index - 1)])
                        * workspace.green_minus_blue[pair(index - 1)]
                    + workspace.horizontal_vertical_weight[pair(index + ts)]
                        * workspace.green_minus_blue[pair(index + ts)];
                output[pixel] = clamp_nan(workspace.green[index] - red_sum * temp);
                output[pixel + 2] = clamp_nan(workspace.green[index] - blue_sum * temp);
            } else {
                output[pixel] =
                    clamp_nan(workspace.green[index] - workspace.green_minus_red[pair(index)]);
                output[pixel + 2] =
                    clamp_nan(workspace.green[index] - workspace.green_minus_blue[pair(index)]);
            }
            output[pixel + 1] = clamp_nan(workspace.green[index]);
        }
    }
}

const fn clamp_nan(value: f32) -> f32 {
    if value.is_nan() {
        0.5
    } else {
        value.clamp(0.0, 1.0)
    }
}

const fn red_phase(pattern: BayerPattern) -> (usize, usize) {
    if is_green(pattern, 0, 0) {
        if is_red(pattern, 1, 0) {
            (1, 0)
        } else {
            (0, 1)
        }
    } else if is_red(pattern, 0, 0) {
        (0, 0)
    } else {
        (1, 1)
    }
}

fn tile_origins(width: u32, height: u32) -> Vec<(i32, i32)> {
    let width = i32::try_from(width).expect("image width fits an i32");
    let height = i32::try_from(height).expect("image height fits an i32");
    let mut origins = Vec::new();
    let mut top = -HALO;
    while top < height {
        let mut left = -HALO;
        while left < width {
            origins.push((left, top));
            left += TILE_STRIDE;
        }
        top += TILE_STRIDE;
    }
    origins
}

fn adaptive_green(center: f32, ratio: f32, hamilton_adams: f32) -> f32 {
    if (1.0 - ratio).abs() < ADAPTIVE_RATIO_THRESHOLD {
        center * ratio
    } else {
        hamilton_adams
    }
}

fn directional_weight(first: f32, second: f32) -> f32 { first / (first + second) }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn tile_origins_use_the_reference_halo_and_stride() {
        let origins = tile_origins(5202, 3464);
        assert_eq!(origins.first(), Some(&(-16, -16)));
        assert!(origins.contains(&(-16, 1264)));
        assert!(origins.contains(&(5104, -16)));
        assert_eq!(origins.last(), Some(&(5104, 3440)));
    }

    #[test]
    fn adaptive_green_uses_ratio_only_inside_the_reference_threshold() {
        assert_eq!(adaptive_green(0.5, 0.8, 0.9), 0.4);
        assert_eq!(adaptive_green(0.5, 2.0, 0.9), 0.9);
    }

    #[test]
    fn directional_weight_prefers_the_neighbor_with_the_smaller_gradient() {
        assert_eq!(directional_weight(1.0, 3.0), 0.25);
        assert_eq!(directional_weight(3.0, 1.0), 0.75);
    }

    #[test]
    fn measured_red_site_is_preserved() {
        let input = (0..64 * 64)
            .map(|sample| sample as f32 / (64.0 * 64.0 - 1.0))
            .collect::<Vec<_>>();
        let output = run(&input, 64, 64, BayerPattern::GRBG);
        let pixel = (16 * 64 + 17) * 3;
        assert!((output[pixel] - input[16 * 64 + 17]).abs() < 2.0e-5);
    }

    #[test]
    fn measured_blue_site_is_preserved() {
        let input = (0..64 * 64)
            .map(|sample| sample as f32 / (64.0 * 64.0 - 1.0))
            .collect::<Vec<_>>();
        let output = run(&input, 64, 64, BayerPattern::GRBG);
        let pixel = (17 * 64 + 16) * 3;
        assert!((output[pixel + 2] - input[17 * 64 + 16]).abs() < 2.0e-5);
    }

    #[test]
    fn measured_sites_are_preserved_for_every_bayer_phase() {
        let input = (0..64 * 64)
            .map(|sample| sample as f32 / (64.0 * 64.0 - 1.0))
            .collect::<Vec<_>>();
        for pattern in BayerPattern::ALL {
            let output = run(&input, 64, 64, pattern);
            for (x, y) in [(16, 17), (17, 16)] {
                let color = channel(pattern, x, y);
                let actual = output[(y as usize * 64 + x as usize) * 3 + color];
                let expected = input[y as usize * 64 + x as usize];
                assert!(
                    (actual - expected).abs() < 2.0e-5,
                    "{pattern} at ({x}, {y}), channel {color}: {actual} != {expected}"
                );
            }
        }
    }
}
