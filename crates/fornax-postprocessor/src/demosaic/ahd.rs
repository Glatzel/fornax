//! Adaptive homogeneity-directed demosaicing.
//!
//! This independently structured implementation follows the published AHD
//! algorithm by Hirakawa, Parks, and Lee. LibRaw is used only as the
//! behavioral reference for comparison tests.

use std::sync::LazyLock;

use fornax_core::error::DemosaicError;
use fornax_core::protocol::LazyDemosaic;
use fornax_core::{BayerChannel, BayerPattern, MonoImage, Primitive, RgbImage};
use generic_num::num;

use super::linear::DemosaicLinear;

const TILE_SIZE: usize = 512;
const TILE_STEP: usize = TILE_SIZE - 6;
const BORDER: usize = 5;
const GREEN: usize = 1;

const XYZ_CAMERA: [[f32; 3]; 3] = [
    [0.433_949_46, 0.376_220_94, 0.189_829_6],
    [0.212_672_9, 0.715_152_2, 0.072_175],
    [0.017_757_48, 0.109_475_98, 0.872_766_55],
];

static CBRT: LazyLock<[f32; 65_536]> = LazyLock::new(|| {
    std::array::from_fn(|index| {
        let value = index as f32 / 65_535.0;
        if value > 0.008_856 {
            value.cbrt()
        } else {
            7.787_f32.mul_add(value, 16.0 / 116.0)
        }
    })
});

pub struct AhdLazy<T: Primitive> {
    image: RgbImage<T>,
}

impl<T> LazyDemosaic<T> for AhdLazy<T>
where
    T: Primitive + Send + Sync,
{
    fn dimensions(&self) -> (u32, u32) { self.image.dimensions() }

    fn fill_rows(&self, y0: usize, out: &mut [T]) {
        let row_stride = self.image.width() as usize * 3;
        let start = y0 * row_stride;
        let end = start + out.len();
        out.copy_from_slice(&self.image.as_raw()[start..end]);
    }

    fn collect(&self) -> RgbImage<T> { self.image.clone() }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicAhd;

impl DemosaicAhd {
    pub fn lazy<T>(
        &self,
        bayer: &MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<AhdLazy<T>, DemosaicError>
    where
        T: Primitive + Send + Sync,
    {
        let linear = DemosaicLinear.lazy(bayer, pattern)?.collect();
        let width = bayer.width() as usize;
        let height = bayer.height() as usize;
        let raw = bayer
            .as_raw()
            .iter()
            .map(|sample| sample.to_f32().unwrap_or_default() * 65_535.0)
            .collect::<Vec<_>>();
        let mut output = linear
            .as_raw()
            .iter()
            .map(|sample| sample.to_f32().unwrap_or_default() * 65_535.0)
            .collect::<Vec<_>>();

        process_ahd(&raw, &mut output, width, height, pattern);

        let image = RgbImage::from_raw(
            bayer.width(),
            bayer.height(),
            output
                .into_iter()
                .map(|code| num!((code / 65_535.0).clamp(0.0, 1.0)))
                .collect(),
        )
        .expect("AHD output dimensions match the source image");
        Ok(AhdLazy { image })
    }
}

fn process_ahd(
    raw: &[f32],
    output: &mut [f32],
    width: usize,
    height: usize,
    pattern: BayerPattern,
) {
    if width <= 2 * BORDER || height <= 2 * BORDER {
        return;
    }

    let area = TILE_SIZE * TILE_SIZE;
    let mut rgb = vec![[0.0_f32; 3]; 2 * area];
    let mut lab = vec![[0_i32; 3]; 2 * area];
    let mut homogeneity = vec![[0_u8; 2]; area];

    for top in (2..height - BORDER).step_by(TILE_STEP) {
        for left in (2..width - BORDER).step_by(TILE_STEP) {
            let tile_height = TILE_SIZE.min(height - top);
            let tile_width = TILE_SIZE.min(width - left);
            rgb.fill([0.0; 3]);
            lab.fill([0; 3]);
            homogeneity.fill([0; 2]);

            green_candidates(
                raw,
                width,
                height,
                pattern,
                &mut rgb,
                top,
                left,
                tile_height,
                tile_width,
            );
            directional_rgb_and_lab(
                raw,
                width,
                height,
                pattern,
                &mut rgb,
                &mut lab,
                top,
                left,
                tile_height,
                tile_width,
            );
            build_homogeneity(
                &lab,
                &mut homogeneity,
                top,
                left,
                tile_height,
                tile_width,
                height,
                width,
            );
            combine_pixels(
                output,
                width,
                &rgb,
                &homogeneity,
                top,
                left,
                tile_height,
                tile_width,
                height,
            );
        }
    }
}

#[allow(clippy::too_many_arguments)]
fn green_candidates(
    raw: &[f32],
    width: usize,
    height: usize,
    pattern: BayerPattern,
    rgb: &mut [[f32; 3]],
    top: usize,
    left: usize,
    _tile_height: usize,
    _tile_width: usize,
) {
    let row_limit = (top + TILE_SIZE).min(height.saturating_sub(2));
    let col_limit = (left + TILE_SIZE).min(width.saturating_sub(2));
    for row in top..row_limit {
        let first = color_index(pattern.channel_at(left as u32, row as u32));
        let start = left + (first & 1);
        for col in (start..col_limit).step_by(2) {
            let local = (row - top) * TILE_SIZE + col - left;
            let center = raw_at(raw, width, col, row);
            let left_green = raw_at(raw, width, col - 1, row);
            let right_green = raw_at(raw, width, col + 1, row);
            let horizontal = (((left_green + center + right_green)
                .mul_add(2.0, -raw_at(raw, width, col - 2, row))
                - raw_at(raw, width, col + 2, row))
                / 4.0)
                .floor();
            rgb[local][GREEN] = clamp_between(horizontal, left_green, right_green);

            let above_green = raw_at(raw, width, col, row - 1);
            let below_green = raw_at(raw, width, col, row + 1);
            let vertical = (((above_green + center + below_green)
                .mul_add(2.0, -raw_at(raw, width, col, row - 2))
                - raw_at(raw, width, col, row + 2))
                / 4.0)
                .floor();
            rgb[TILE_SIZE * TILE_SIZE + local][GREEN] =
                clamp_between(vertical, above_green, below_green);
        }
    }
}

fn directional_rgb_and_lab(
    raw: &[f32],
    width: usize,
    height: usize,
    pattern: BayerPattern,
    rgb: &mut [[f32; 3]],
    lab: &mut [[i32; 3]],
    top: usize,
    left: usize,
    _tile_height: usize,
    _tile_width: usize,
) {
    let row_limit = (top + TILE_SIZE - 1).min(height.saturating_sub(3));
    let col_limit = (left + TILE_SIZE - 1).min(width.saturating_sub(3));
    for row in top + 1..row_limit {
        for col in left + 1..col_limit {
            for direction in 0..2 {
                let local = (row - top) * TILE_SIZE + col - left;
                let candidate_index = direction * TILE_SIZE * TILE_SIZE + local;
                let mut candidate = rgb[candidate_index];
                let center = color_index(pattern.channel_at(col as u32, row as u32));
                if center == GREEN {
                    let center_green = raw_at(raw, width, col, row);
                    let down = color_index(pattern.channel_at(col as u32, (row + 1) as u32));
                    let down = if down == GREEN { 2 } else { down };
                    let other = 2 - down;
                    candidate[other] = clamp_code(
                        (raw_channel(raw, width, pattern, col - 1, row, other)
                            + raw_channel(raw, width, pattern, col + 1, row, other)
                            - rgb[candidate_index - 1][GREEN]
                            - rgb[candidate_index + 1][GREEN])
                            .mul_add(0.5, center_green)
                            .floor(),
                    );
                    candidate[down] = clamp_code(
                        (raw_channel(raw, width, pattern, col, row - 1, down)
                            + raw_channel(raw, width, pattern, col, row + 1, down)
                            - rgb[candidate_index - TILE_SIZE][GREEN]
                            - rgb[candidate_index + TILE_SIZE][GREEN])
                            .mul_add(0.5, center_green)
                            .floor(),
                    );
                    candidate[GREEN] = center_green;
                } else {
                    let missing = 2 - center;
                    let above_left = raw_channel(raw, width, pattern, col - 1, row - 1, missing);
                    let above_right = raw_channel(raw, width, pattern, col + 1, row - 1, missing);
                    let below_left = raw_channel(raw, width, pattern, col - 1, row + 1, missing);
                    let below_right = raw_channel(raw, width, pattern, col + 1, row + 1, missing);
                    let above_left_green = rgb[candidate_index - TILE_SIZE - 1][GREEN];
                    let above_right_green = rgb[candidate_index - TILE_SIZE + 1][GREEN];
                    let below_left_green = rgb[candidate_index + TILE_SIZE - 1][GREEN];
                    let below_right_green = rgb[candidate_index + TILE_SIZE + 1][GREEN];
                    candidate[missing] = clamp_code(
                        (candidate[GREEN]
                            + (above_left + above_right + below_left + below_right
                                - above_left_green
                                - above_right_green
                                - below_left_green
                                - below_right_green
                                + 1.0)
                                / 4.0)
                            .floor(),
                    );
                    candidate[center] = raw_at(raw, width, col, row);
                }
                rgb[candidate_index] = candidate;
                lab[candidate_index] = to_lab(candidate);
            }
        }
    }
}

fn build_homogeneity(
    lab: &[[i32; 3]],
    homogeneity: &mut [[u8; 2]],
    top: usize,
    left: usize,
    _tile_height: usize,
    _tile_width: usize,
    height: usize,
    width: usize,
) {
    let row_limit = (top + TILE_SIZE - 2).min(height.saturating_sub(4));
    let col_limit = (left + TILE_SIZE - 2).min(width.saturating_sub(4));
    for row in top + 2..row_limit {
        for col in left + 2..col_limit {
            let local = (row - top) * TILE_SIZE + col - left;
            let neighbors = [local - 1, local + 1, local - TILE_SIZE, local + TILE_SIZE];
            let mut l_diff = [[0_u32; 4]; 2];
            let mut ab_diff = [[0_u32; 4]; 2];
            for direction in 0..2 {
                let current = lab[direction * TILE_SIZE * TILE_SIZE + local];
                for (index, neighbor) in neighbors.iter().copied().enumerate() {
                    let adjacent = lab[direction * TILE_SIZE * TILE_SIZE + neighbor];
                    l_diff[direction][index] = current[0].abs_diff(adjacent[0]);
                    ab_diff[direction][index] = current[1].abs_diff(adjacent[1]).pow(2)
                        + current[2].abs_diff(adjacent[2]).pow(2);
                }
            }
            let l_epsilon = l_diff[0][0]
                .max(l_diff[0][1])
                .min(l_diff[1][2].max(l_diff[1][3]));
            let ab_epsilon = ab_diff[0][0]
                .max(ab_diff[0][1])
                .min(ab_diff[1][2].max(ab_diff[1][3]));
            for direction in 0..2 {
                homogeneity[local][direction] = (0..4)
                    .filter(|&index| {
                        l_diff[direction][index] <= l_epsilon
                            && ab_diff[direction][index] <= ab_epsilon
                    })
                    .count() as u8;
            }
        }
    }
}

fn combine_pixels(
    output: &mut [f32],
    width: usize,
    rgb: &[[f32; 3]],
    homogeneity: &[[u8; 2]],
    top: usize,
    left: usize,
    _tile_height: usize,
    _tile_width: usize,
    height: usize,
) {
    let row_limit = (top + TILE_SIZE - 3).min(height.saturating_sub(5));
    let col_limit = (left + TILE_SIZE - 3).min(width.saturating_sub(5));
    for row in top + 3..row_limit {
        for col in left + 3..col_limit {
            let local = (row - top) * TILE_SIZE + col - left;
            let mut score = [0_u32; 2];
            let tile_row = local / TILE_SIZE;
            let tile_col = local % TILE_SIZE;
            for row_offset in -1_i32..=1 {
                for col_offset in -1_i32..=1 {
                    let neighbor = (tile_row as i32 + row_offset) as usize * TILE_SIZE
                        + (tile_col as i32 + col_offset) as usize;
                    for direction in 0..2 {
                        score[direction] += u32::from(homogeneity[neighbor][direction]);
                    }
                }
            }
            let selected = if score[0] == score[1] {
                (0..3)
                    .map(|channel| {
                        f32::midpoint(
                            rgb[local][channel],
                            rgb[TILE_SIZE * TILE_SIZE + local][channel],
                        )
                    })
                    .collect::<Vec<_>>()
            } else {
                let direction = usize::from(score[1] > score[0]);
                rgb[direction * TILE_SIZE * TILE_SIZE + local].to_vec()
            };
            let output_index = (row * width + col) * 3;
            output[output_index..output_index + 3].copy_from_slice(&selected[..3]);
        }
    }
}

fn to_lab(rgb: [f32; 3]) -> [i32; 3] {
    let mut xyz = [0.5_f32; 3];
    for channel in 0..3 {
        for component in 0..3 {
            xyz[component] = XYZ_CAMERA[component][channel].mul_add(rgb[channel], xyz[component]);
        }
    }
    let x = CBRT[xyz[0].clamp(0.0, 65_535.0) as usize];
    let y = CBRT[xyz[1].clamp(0.0, 65_535.0) as usize];
    let z = CBRT[xyz[2].clamp(0.0, 65_535.0) as usize];
    [
        ((64.0 * 116.0_f32.mul_add(y, -16.0)) as i32) as i16 as i32,
        ((64.0 * 500.0 * (x - y)) as i32) as i16 as i32,
        ((64.0 * 200.0 * (y - z)) as i32) as i16 as i32,
    ]
}

const fn raw_channel(
    raw: &[f32],
    width: usize,
    pattern: BayerPattern,
    x: usize,
    y: usize,
    channel: usize,
) -> f32 {
    if color_index(pattern.channel_at(x as u32, y as u32)) == channel {
        raw_at(raw, width, x, y)
    } else {
        0.0
    }
}

#[inline(always)]
const fn raw_at(raw: &[f32], width: usize, x: usize, y: usize) -> f32 { raw[y * width + x] }

const fn clamp_code(value: f32) -> f32 { value.clamp(0.0, 65_535.0).floor() }

const fn clamp_between(value: f32, first: f32, second: f32) -> f32 {
    value.clamp(first.min(second), first.max(second))
}

const fn color_index(channel: BayerChannel) -> usize {
    match channel {
        BayerChannel::R => 0,
        BayerChannel::GRedRow | BayerChannel::GBlueRow => 1,
        BayerChannel::B => 2,
    }
}

#[cfg(test)]
mod tests {
    use fornax_core::BayerPattern;

    use super::*;

    #[test]
    fn uniform_mosaic_has_neutral_rgb_output() -> mischief::Result<()> {
        let image = MonoImage::from_raw(32, 32, vec![0.25_f32; 32 * 32]).unwrap();
        for pattern in BayerPattern::ALL {
            let output = DemosaicAhd.lazy(&image, pattern)?.collect();
            let minimum = output
                .as_raw()
                .iter()
                .copied()
                .fold(f32::INFINITY, f32::min);
            let maximum = output
                .as_raw()
                .iter()
                .copied()
                .fold(f32::NEG_INFINITY, f32::max);
            let zero = output.enumerate_pixels().find_map(|(x, y, pixel)| {
                pixel
                    .0
                    .iter()
                    .any(|sample| sample.abs() < f32::EPSILON)
                    .then_some((x, y, pixel.0))
            });
            assert!(
                output
                    .as_raw()
                    .iter()
                    .all(|sample| (*sample - 0.25).abs() < 2.0e-5),
                "{pattern}: AHD output range [{minimum}, {maximum}], first zero: {zero:?}"
            );
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
            let output = DemosaicAhd.lazy(&image, pattern)?.collect();
            for y in 0..image.height() {
                for x in 0..image.width() {
                    let channel = color_index(pattern.channel_at(x, y));
                    let actual = output.get_pixel(x, y)[channel];
                    let expected = image.get_pixel(x, y)[0];
                    assert!(
                        (actual - expected).abs() < 1.0e-6,
                        "{pattern} at ({x}, {y}): {actual} != {expected}"
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
        let lazy = DemosaicAhd.lazy(&image, BayerPattern::GRBG)?;
        let whole = lazy.collect();

        let mut rows = vec![0.0; 40 * 3 * 40];
        lazy.fill_rows(0, &mut rows);
        assert_eq!(whole.as_raw(), &rows);
        Ok(())
    }
}
