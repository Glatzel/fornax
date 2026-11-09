//! Tiled linear minimum mean-square-error Bayer demosaicing.
//!
//! This path follows the published LMMSE algorithm and darktable 5.6.2's
//! mode conventions. See the
//! [LMMSE reference notes](https://github.com/Glatzel/fornax/blob/main/docs/references/lmmse.md).

use std::sync::OnceLock;

use fornax_core::error::DemosaicError;
use fornax_core::{BayerPattern, MonoImage, Primitive};

use super::LmmseMode;
use super::buffer::{DenseLazy, channel, codes, dense_from_codes};
use super::ppg::DemosaicPpg;
use crate::LazyDemosaic;

const TILE_SIZE: usize = 136;
const BORDER: usize = 4;
const OVERLAP: usize = 8;
const TILE_INTERIOR: usize = TILE_SIZE - 2 * BORDER;
const TILE_VALID: usize = TILE_INTERIOR - 2 * OVERLAP;
const TILE_AREA: usize = TILE_SIZE * TILE_SIZE;

const RED: usize = 0;
const GREEN: usize = 1;
const BLUE: usize = 2;

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
        T: Primitive + Send + Sync,
    {
        self.lazy_with_mode(image, pattern, LmmseMode::Median)
    }

    pub fn lazy_with_mode<T>(
        &self,
        image: &MonoImage<T>,
        pattern: BayerPattern,
        mode: LmmseMode,
    ) -> Result<DenseLazy<T>, DemosaicError>
    where
        T: Primitive + Send + Sync,
    {
        let width = image.width() as usize;
        let height = image.height() as usize;
        let input = codes(image);
        let ppg_samples = input
            .iter()
            .map(|sample| f32::from(*sample) / f32::from(u16::MAX))
            .collect();
        let ppg_input = MonoImage::from_raw(image.width(), image.height(), ppg_samples)
            .ok_or_else(|| {
                DemosaicError::InvalidInput("LMMSE could not copy the Bayer mosaic".into())
            })?;
        let seed: fornax_core::RgbImage<f32> = DemosaicPpg.lazy(&ppg_input, pattern)?.collect();
        let mut output: Vec<u16> = seed
            .as_raw()
            .iter()
            .map(|sample| (sample.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16)
            .collect();

        if width >= 2 * BORDER && height >= 2 * BORDER {
            interpolate(&input, &mut output, width, height, pattern, mode);
        }

        Ok(DenseLazy::new(dense_from_codes(
            image.width(),
            image.height(),
            output,
        )))
    }
}

fn interpolate(
    input: &[u16],
    output: &mut [u16],
    width: usize,
    height: usize,
    pattern: BayerPattern,
    mode: LmmseMode,
) {
    let (median_passes, refinement_passes) = match mode {
        LmmseMode::Basic => (0, 0),
        LmmseMode::Median => (1, 0),
        LmmseMode::ThreeMedians => (3, 0),
        LmmseMode::Refine(passes) => (3, usize::from(passes.get())),
    };
    let gamma = gamma_tables();
    let h0 = 1.0_f32;
    let h1 = (-1.0_f32 / 8.0).exp();
    let h2 = (-4.0_f32 / 8.0).exp();
    let h3 = (-9.0_f32 / 8.0).exp();
    let h4 = (-16.0_f32 / 8.0).exp();
    let hsum = 2.0_f32.mul_add(h1 + h2 + h3 + h4, h0);
    let h0 = h0 / hsum;
    let h1 = h1 / hsum;
    let h2 = h2 / hsum;
    let h3 = h3 / hsum;
    let h4 = h4 / hsum;

    let num_vertical = 1 + height.saturating_sub(2 * OVERLAP + 1) / TILE_VALID;
    let num_horizontal = 1 + width.saturating_sub(2 * OVERLAP + 1) / TILE_VALID;
    let mut qix = vec![0.0_f32; 6 * TILE_AREA];

    for tile_vertical in 0..num_vertical {
        for tile_horizontal in 0..num_horizontal {
            let row_start = tile_vertical * TILE_VALID;
            let row_end = (row_start + TILE_INTERIOR).min(height);
            let col_start = tile_horizontal * TILE_VALID;
            let col_end = (col_start + TILE_INTERIOR).min(width);
            let tile_rows = (row_end - row_start).min(TILE_INTERIOR);
            let tile_cols = (col_end - col_start).min(TILE_INTERIOR);
            let last_rr = tile_rows + 2 * BORDER;
            let last_cc = tile_cols + 2 * BORDER;

            for (rrr, row) in (BORDER..tile_rows + BORDER).zip(row_start..row_end) {
                for (ccc, col) in (BORDER..tile_cols + BORDER).zip(col_start..col_end) {
                    let value = f32::from(input[row * width + col]) / f32::from(u16::MAX);
                    let encoded = calc_gamma(value, &gamma.input);
                    qix[tile_index(5, rrr, ccc)] = encoded;
                }
            }

            color_differences(&mut qix, last_rr, last_cc, pattern);
            low_pass(&mut qix, last_rr, last_cc, [h0, h1, h2, h3, h4]);
            mmse_interpolate(&mut qix, last_rr, last_cc, pattern);
            copy_cfa_samples(
                &mut qix, row_start, col_start, last_rr, last_cc, width, height, pattern,
            );
            interpolate_red_blue(&mut qix, last_rr, last_cc, pattern);

            let rr_min = if tile_vertical == 0 { 6 } else { 0 };
            let rr_max = last_rr - usize::from(tile_vertical == num_vertical - 1) * 6;
            let cc_min = if tile_horizontal == 0 { 6 } else { 0 };
            let cc_max = last_cc - usize::from(tile_horizontal == num_horizontal - 1) * 6;
            for _ in 0..median_passes {
                median_pass(
                    &mut qix, last_rr, last_cc, rr_min, rr_max, cc_min, cc_max, pattern,
                );
            }

            for rrr in BORDER..last_rr - BORDER {
                for ccc in BORDER..last_cc - BORDER {
                    let c = tile_channel(pattern, rrr, ccc);
                    qix[tile_index(c, rrr, ccc)] = q(&qix, 5, rrr, ccc);
                }
            }

            for _ in 0..refinement_passes {
                refine_tile(&mut qix, rr_min, rr_max, cc_min, cc_max, pattern);
            }

            write_tile(
                &qix,
                output,
                width,
                tile_vertical,
                num_vertical,
                tile_horizontal,
                num_horizontal,
                row_start,
                row_end,
                col_start,
                col_end,
                &gamma.output,
            );
        }
    }
}

fn refine_tile(
    qix: &mut [f32],
    rr_min: usize,
    rr_max: usize,
    cc_min: usize,
    cc_max: usize,
    pattern: BayerPattern,
) {
    for rr in rr_min + 2..rr_max - 2 {
        let mut cc = cc_min + 2 + (tile_channel(pattern, rr, 2) & 1);
        let c = tile_channel(pattern, rr, cc);
        while cc < cc_max - 2 {
            let center_color = q(qix, c, rr, cc);
            let horizontal_green = (q(qix, GREEN, rr, cc + 1) - q(qix, GREEN, rr, cc - 1)).abs();
            let vertical_green = (q(qix, GREEN, rr + 1, cc) - q(qix, GREEN, rr - 1, cc)).abs();
            let left =
                1.0 / (1.0 + (q(qix, c, rr, cc - 2) - center_color).abs() + horizontal_green);
            let right =
                1.0 / (1.0 + (q(qix, c, rr, cc + 2) - center_color).abs() + horizontal_green);
            let up = 1.0 / (1.0 + (q(qix, c, rr - 2, cc) - center_color).abs() + vertical_green);
            let down = 1.0 / (1.0 + (q(qix, c, rr + 2, cc) - center_color).abs() + vertical_green);
            let left_difference = q(qix, GREEN, rr, cc - 1) - q(qix, c, rr, cc - 1);
            let right_difference = q(qix, GREEN, rr, cc + 1) - q(qix, c, rr, cc + 1);
            let up_difference = q(qix, GREEN, rr - 1, cc) - q(qix, c, rr - 1, cc);
            let down_difference = q(qix, GREEN, rr + 1, cc) - q(qix, c, rr + 1, cc);
            let numerator = left_difference.mul_add(
                left,
                right_difference.mul_add(right, up_difference.mul_add(up, down_difference * down)),
            );
            qix[tile_index(GREEN, rr, cc)] = center_color + numerator / (left + right + up + down);
            cc += 2;
        }
    }

    for rr in rr_min + 2..rr_max - 2 {
        let mut cc = cc_min + 2 + (tile_channel(pattern, rr, 3) & 1);
        let mut c = tile_channel(pattern, rr, cc + 1);
        while cc < cc_max - 2 {
            for _ in 0..2 {
                let center_green = q(qix, GREEN, rr, cc);
                let left_green = (q(qix, GREEN, rr, cc - 2) - center_green).abs();
                let right_green = (q(qix, GREEN, rr, cc + 2) - center_green).abs();
                let up_green = (q(qix, GREEN, rr - 2, cc) - center_green).abs();
                let down_green = (q(qix, GREEN, rr + 2, cc) - center_green).abs();
                let horizontal_color = (q(qix, c, rr, cc + 1) - q(qix, c, rr, cc - 1)).abs();
                let vertical_color = (q(qix, c, rr + 1, cc) - q(qix, c, rr - 1, cc)).abs();
                let left = 1.0 / (1.0 + left_green + horizontal_color);
                let right = 1.0 / (1.0 + right_green + horizontal_color);
                let up = 1.0 / (1.0 + up_green + vertical_color);
                let down = 1.0 / (1.0 + down_green + vertical_color);
                let left_difference = q(qix, GREEN, rr, cc - 1) - q(qix, c, rr, cc - 1);
                let right_difference = q(qix, GREEN, rr, cc + 1) - q(qix, c, rr, cc + 1);
                let up_difference = q(qix, GREEN, rr - 1, cc) - q(qix, c, rr - 1, cc);
                let down_difference = q(qix, GREEN, rr + 1, cc) - q(qix, c, rr + 1, cc);
                let numerator = left_difference.mul_add(
                    left,
                    right_difference
                        .mul_add(right, up_difference.mul_add(up, down_difference * down)),
                );
                qix[tile_index(c, rr, cc)] = center_green - numerator / (left + right + up + down);
                c = 2 - c;
            }
            cc += 2;
        }
    }

    for rr in rr_min + 2..rr_max - 2 {
        let mut cc = cc_min + 2 + (tile_channel(pattern, rr, 2) & 1);
        while cc < cc_max - 2 {
            let sampled = tile_channel(pattern, rr, cc);
            let c = 2 - sampled;
            let d = sampled;
            let center_green = q(qix, GREEN, rr, cc);
            let center_integrated = q(qix, d, rr, cc);
            let horizontal_green = (q(qix, GREEN, rr, cc + 1) - q(qix, GREEN, rr, cc - 1)).abs();
            let vertical_green = (q(qix, GREEN, rr + 1, cc) - q(qix, GREEN, rr - 1, cc)).abs();
            let left =
                1.0 / (1.0 + (q(qix, d, rr, cc - 2) - center_integrated).abs() + horizontal_green);
            let right =
                1.0 / (1.0 + (q(qix, d, rr, cc + 2) - center_integrated).abs() + horizontal_green);
            let up =
                1.0 / (1.0 + (q(qix, d, rr - 2, cc) - center_integrated).abs() + vertical_green);
            let down =
                1.0 / (1.0 + (q(qix, d, rr + 2, cc) - center_integrated).abs() + vertical_green);
            let left_difference = q(qix, GREEN, rr, cc - 1) - q(qix, c, rr, cc - 1);
            let right_difference = q(qix, GREEN, rr, cc + 1) - q(qix, c, rr, cc + 1);
            let up_difference = q(qix, GREEN, rr - 1, cc) - q(qix, c, rr - 1, cc);
            let down_difference = q(qix, GREEN, rr + 1, cc) - q(qix, c, rr + 1, cc);
            let numerator = left_difference.mul_add(
                left,
                right_difference.mul_add(right, up_difference.mul_add(up, down_difference * down)),
            );
            qix[tile_index(c, rr, cc)] = center_green - numerator / (left + right + up + down);
            cc += 2;
        }
    }
}

fn color_differences(qix: &mut [f32], last_rr: usize, last_cc: usize, pattern: BayerPattern) {
    for rr in 2..last_rr - 2 {
        let mut cc = 2 + (tile_channel(pattern, rr, 2) & 1);
        while cc < last_cc - 2 {
            let center = q(qix, 5, rr, cc);
            let value = 0.25_f32.mul_add(
                center,
                0.0625
                    * (q(qix, 5, rr - 1, cc - 1)
                        + q(qix, 5, rr - 1, cc + 1)
                        + q(qix, 5, rr + 1, cc - 1)
                        + q(qix, 5, rr + 1, cc + 1)),
            );

            let mut horizontal = 0.5_f32.mul_add(
                q(qix, 5, rr, cc - 1) + center + q(qix, 5, rr, cc + 1),
                -0.25 * (q(qix, 5, rr, cc - 2) + q(qix, 5, rr, cc + 2)),
            );
            let horizontal_luma = 0.5_f32.mul_add(horizontal, value);
            horizontal = if center > 1.75 * horizontal_luma {
                median3(horizontal, q(qix, 5, rr, cc - 1), q(qix, 5, rr, cc + 1))
            } else {
                horizontal.clamp(0.0, 1.0)
            } - center;

            let mut vertical = 0.5_f32.mul_add(
                q(qix, 5, rr - 1, cc) + center + q(qix, 5, rr + 1, cc),
                -0.25 * (q(qix, 5, rr - 2, cc) + q(qix, 5, rr + 2, cc)),
            );
            let vertical_luma = 0.5_f32.mul_add(vertical, value);
            vertical = if center > 1.75 * vertical_luma {
                median3(vertical, q(qix, 5, rr - 1, cc), q(qix, 5, rr + 1, cc))
            } else {
                vertical.clamp(0.0, 1.0)
            } - center;

            qix[tile_index(0, rr, cc)] = horizontal;
            qix[tile_index(1, rr, cc)] = vertical;
            cc += 2;
        }

        let mut cc = 2 + (tile_channel(pattern, rr, 3) & 1);
        while cc < last_cc - 2 {
            let center = q(qix, 5, rr, cc);
            let horizontal = 0.5_f32
                .mul_add(
                    -(q(qix, 5, rr, cc - 1) + center + q(qix, 5, rr, cc + 1)),
                    0.25 * (q(qix, 5, rr, cc - 2) + q(qix, 5, rr, cc + 2)),
                )
                .clamp(-1.0, 0.0)
                + center;
            let vertical = 0.5_f32
                .mul_add(
                    -(q(qix, 5, rr - 1, cc) + center + q(qix, 5, rr + 1, cc)),
                    0.25 * (q(qix, 5, rr - 2, cc) + q(qix, 5, rr + 2, cc)),
                )
                .clamp(-1.0, 0.0)
                + center;
            qix[tile_index(0, rr, cc)] = horizontal;
            qix[tile_index(1, rr, cc)] = vertical;
            cc += 2;
        }
    }
}

fn low_pass(qix: &mut [f32], last_rr: usize, last_cc: usize, h: [f32; 5]) {
    let [h0, h1, h2, h3, h4] = h;
    for rr in 4..last_rr - 4 {
        for cc in 4..last_cc - 4 {
            let horizontal = f32::mul_add(
                h4,
                q(qix, 0, rr, cc - 4) + q(qix, 0, rr, cc + 4),
                f32::mul_add(
                    h3,
                    q(qix, 0, rr, cc - 3) + q(qix, 0, rr, cc + 3),
                    f32::mul_add(
                        h2,
                        q(qix, 0, rr, cc - 2) + q(qix, 0, rr, cc + 2),
                        f32::mul_add(
                            h1,
                            q(qix, 0, rr, cc - 1) + q(qix, 0, rr, cc + 1),
                            h0 * q(qix, 0, rr, cc),
                        ),
                    ),
                ),
            );
            let vertical = f32::mul_add(
                h4,
                q(qix, 1, rr - 4, cc) + q(qix, 1, rr + 4, cc),
                f32::mul_add(
                    h3,
                    q(qix, 1, rr - 3, cc) + q(qix, 1, rr + 3, cc),
                    f32::mul_add(
                        h2,
                        q(qix, 1, rr - 2, cc) + q(qix, 1, rr + 2, cc),
                        f32::mul_add(
                            h1,
                            q(qix, 1, rr - 1, cc) + q(qix, 1, rr + 1, cc),
                            h0 * q(qix, 1, rr, cc),
                        ),
                    ),
                ),
            );
            qix[tile_index(2, rr, cc)] = horizontal;
            qix[tile_index(3, rr, cc)] = vertical;
        }
    }
}

fn mmse_interpolate(qix: &mut [f32], last_rr: usize, last_cc: usize, pattern: BayerPattern) {
    for rr in 4..last_rr - 4 {
        let mut cc = 4 + (tile_channel(pattern, rr, 4) & 1);
        while cc < last_cc - 4 {
            let horizontal_filtered = [
                q(qix, 2, rr, cc - 4),
                q(qix, 2, rr, cc - 3),
                q(qix, 2, rr, cc - 2),
                q(qix, 2, rr, cc - 1),
                q(qix, 2, rr, cc),
                q(qix, 2, rr, cc + 1),
                q(qix, 2, rr, cc + 2),
                q(qix, 2, rr, cc + 3),
                q(qix, 2, rr, cc + 4),
            ];
            let horizontal_raw = [
                q(qix, 0, rr, cc - 4),
                q(qix, 0, rr, cc - 3),
                q(qix, 0, rr, cc - 2),
                q(qix, 0, rr, cc - 1),
                q(qix, 0, rr, cc),
                q(qix, 0, rr, cc + 1),
                q(qix, 0, rr, cc + 2),
                q(qix, 0, rr, cc + 3),
                q(qix, 0, rr, cc + 4),
            ];
            let (xh, vh) = mmse_value(horizontal_filtered, horizontal_raw);

            let vertical_filtered = [
                q(qix, 3, rr - 4, cc),
                q(qix, 3, rr - 3, cc),
                q(qix, 3, rr - 2, cc),
                q(qix, 3, rr - 1, cc),
                q(qix, 3, rr, cc),
                q(qix, 3, rr + 1, cc),
                q(qix, 3, rr + 2, cc),
                q(qix, 3, rr + 3, cc),
                q(qix, 3, rr + 4, cc),
            ];
            let vertical_raw = [
                q(qix, 1, rr - 4, cc),
                q(qix, 1, rr - 3, cc),
                q(qix, 1, rr - 2, cc),
                q(qix, 1, rr - 1, cc),
                q(qix, 1, rr, cc),
                q(qix, 1, rr + 1, cc),
                q(qix, 1, rr + 2, cc),
                q(qix, 1, rr + 3, cc),
                q(qix, 1, rr + 4, cc),
            ];
            let (xv, vv) = mmse_value(vertical_filtered, vertical_raw);

            qix[tile_index(4, rr, cc)] = f32::mul_add(xv, vh, xh * vv) / (vh + vv);
            cc += 2;
        }
    }
}

fn mmse_value(mut filtered: [f32; 9], raw: [f32; 9]) -> (f32, f32) {
    let mean = filtered.iter().sum::<f32>() / 9.0;
    let filtered_center = filtered[4];
    let variance_filtered = 1.0e-7
        + filtered
            .iter()
            .map(|sample| square(*sample - mean))
            .sum::<f32>();
    for (filtered, raw) in filtered.iter_mut().zip(raw) {
        *filtered -= raw;
    }
    let variance_raw = 1.0e-7 + filtered.iter().map(|sample| square(*sample)).sum::<f32>();
    let value = filtered_center.mul_add(variance_raw, raw[4] * variance_filtered)
        / (variance_filtered + variance_raw);
    (
        value,
        variance_filtered * variance_raw / (variance_filtered + variance_raw),
    )
}

fn copy_cfa_samples(
    qix: &mut [f32],
    row_start: usize,
    col_start: usize,
    last_rr: usize,
    last_cc: usize,
    width: usize,
    height: usize,
    pattern: BayerPattern,
) {
    for rr in 0..last_rr {
        let row = row_start as isize - BORDER as isize + rr as isize;
        for cc in 0..last_cc {
            let col = col_start as isize - BORDER as isize + cc as isize;
            let inside = row >= 0 && row < height as isize && col >= 0 && col < width as isize;
            let sample = if inside { q(qix, 5, rr, cc) } else { 0.0 };
            let c = tile_channel(pattern, rr, cc);
            qix[tile_index(c, rr, cc)] = sample;
            if c != GREEN {
                qix[tile_index(GREEN, rr, cc)] = if inside {
                    sample + q(qix, 4, rr, cc)
                } else {
                    0.0
                };
            }
        }
    }
}

fn interpolate_red_blue(qix: &mut [f32], last_rr: usize, last_cc: usize, pattern: BayerPattern) {
    for rr in 1..last_rr - 1 {
        let mut cc = 1 + (tile_channel(pattern, rr, 2) & 1);
        while cc < last_cc - 1 {
            let mut c = tile_channel(pattern, rr, cc + 1);
            let green = q(qix, 1, rr, cc);
            let value = 0.5_f32.mul_add(
                q(qix, c, rr, cc - 1) - q(qix, 1, rr, cc - 1) + q(qix, c, rr, cc + 1)
                    - q(qix, 1, rr, cc + 1),
                green,
            );
            qix[tile_index(c, rr, cc)] = value;
            c = 2 - c;
            let value = 0.5_f32.mul_add(
                q(qix, c, rr - 1, cc) - q(qix, 1, rr - 1, cc) + q(qix, c, rr + 1, cc)
                    - q(qix, 1, rr + 1, cc),
                green,
            );
            qix[tile_index(c, rr, cc)] = value;
            cc += 2;
        }
    }

    for rr in 1..last_rr - 1 {
        let mut cc = 1 + (tile_channel(pattern, rr, 1) & 1);
        while cc < last_cc - 1 {
            let c = 2 - tile_channel(pattern, rr, cc);
            let green = q(qix, 1, rr, cc);
            let value = 0.25_f32.mul_add(
                q(qix, c, rr - 1, cc) - q(qix, 1, rr - 1, cc) + q(qix, c, rr, cc - 1)
                    - q(qix, 1, rr, cc - 1)
                    + q(qix, c, rr, cc + 1)
                    - q(qix, 1, rr, cc + 1)
                    + q(qix, c, rr + 1, cc)
                    - q(qix, 1, rr + 1, cc),
                green,
            );
            qix[tile_index(c, rr, cc)] = value;
            cc += 2;
        }
    }
}

fn median_pass(
    qix: &mut [f32],
    last_rr: usize,
    last_cc: usize,
    rr_min: usize,
    rr_max: usize,
    cc_min: usize,
    cc_max: usize,
    pattern: BayerPattern,
) {
    for rr in 1..last_rr - 1 {
        for color in [RED, BLUE] {
            let correction_channel = if color == RED { 3 } else { 4 };
            for cc in 1..last_cc - 1 {
                let mut samples = [0.0_f32; 9];
                let mut slot = 0;
                for dy in -1_i32..=1 {
                    for dx in -1_i32..=1 {
                        let ny = (rr as i32 + dy) as usize;
                        let nx = (cc as i32 + dx) as usize;
                        samples[slot] = q(qix, color, ny, nx) - q(qix, GREEN, ny, nx);
                        slot += 1;
                    }
                }
                qix[tile_index(correction_channel, rr, cc)] = median9(&samples);
            }
        }
    }

    for rr in rr_min..rr_max - 1 {
        if tile_channel(pattern, rr, 0) == GREEN {
            let mut cc = cc_min;
            while cc < cc_max - 1 {
                let green = q(qix, GREEN, rr, cc);
                qix[tile_index(RED, rr, cc)] = green + q(qix, 3, rr, cc);
                qix[tile_index(BLUE, rr, cc)] = green + q(qix, 4, rr, cc);
                cc += 1;

                let missing = 2 - tile_channel(pattern, rr, cc);
                let correction_channel = if missing == RED { 3 } else { 4 };
                qix[tile_index(missing, rr, cc)] =
                    q(qix, GREEN, rr, cc) + q(qix, correction_channel, rr, cc);
                qix[tile_index(GREEN, rr, cc)] = 0.5
                    * (q(qix, RED, rr, cc) - q(qix, 3, rr, cc) + q(qix, BLUE, rr, cc)
                        - q(qix, 4, rr, cc));
                cc += 1;
            }
            if cc < cc_max {
                let green = q(qix, GREEN, rr, cc);
                qix[tile_index(RED, rr, cc)] = green + q(qix, 3, rr, cc);
                qix[tile_index(BLUE, rr, cc)] = green + q(qix, 4, rr, cc);
            }
        } else {
            let opposite = 2 - tile_channel(pattern, rr, 0);
            let correction_channel = if opposite == RED { 3 } else { 4 };
            let mut cc = cc_min;
            while cc < cc_max - 1 {
                qix[tile_index(opposite, rr, cc)] =
                    q(qix, GREEN, rr, cc) + q(qix, correction_channel, rr, cc);
                qix[tile_index(GREEN, rr, cc)] = 0.5
                    * (q(qix, RED, rr, cc) - q(qix, 3, rr, cc) + q(qix, BLUE, rr, cc)
                        - q(qix, 4, rr, cc));
                cc += 1;
                let green = q(qix, GREEN, rr, cc);
                qix[tile_index(RED, rr, cc)] = green + q(qix, 3, rr, cc);
                qix[tile_index(BLUE, rr, cc)] = green + q(qix, 4, rr, cc);
                cc += 1;
            }
            if cc < cc_max {
                qix[tile_index(opposite, rr, cc)] =
                    q(qix, GREEN, rr, cc) + q(qix, correction_channel, rr, cc);
                qix[tile_index(GREEN, rr, cc)] = 0.5
                    * (q(qix, RED, rr, cc) - q(qix, 3, rr, cc) + q(qix, BLUE, rr, cc)
                        - q(qix, 4, rr, cc));
            }
        }
    }
}

fn write_tile(
    qix: &[f32],
    output: &mut [u16],
    width: usize,
    tile_vertical: usize,
    num_vertical: usize,
    tile_horizontal: usize,
    num_horizontal: usize,
    row_start: usize,
    row_end: usize,
    col_start: usize,
    col_end: usize,
    gamma_output: &[f32],
) {
    let first_vertical = row_start + if tile_vertical == 0 { BORDER } else { OVERLAP };
    let last_vertical = row_end
        - if tile_vertical == num_vertical - 1 {
            BORDER
        } else {
            OVERLAP
        };
    let first_horizontal = col_start
        + if tile_horizontal == 0 {
            BORDER
        } else {
            OVERLAP
        };
    let last_horizontal = col_end
        - if tile_horizontal == num_horizontal - 1 {
            BORDER
        } else {
            OVERLAP
        };

    for row in first_vertical..last_vertical {
        let rr = row - row_start + BORDER;
        for col in first_horizontal..last_horizontal {
            let cc = col - col_start + BORDER;
            let target = (row * width + col) * 3;
            for color in [RED, GREEN, BLUE] {
                let value = calc_gamma(q(qix, color, rr, cc), gamma_output).clamp(0.0, 1.0);
                output[target + color] = (value * f32::from(u16::MAX)).round() as u16;
            }
        }
    }
}

fn gamma_tables() -> &'static GammaTables {
    static TABLES: OnceLock<GammaTables> = OnceLock::new();
    TABLES.get_or_init(|| {
        let mut input = Vec::with_capacity(65_536);
        let mut output = Vec::with_capacity(65_536);
        for code in 0..=u16::MAX {
            let value = f64::from(code) / 65_535.0;
            let encoded = if value <= 0.001867 {
                value * 17.0
            } else {
                1.044_445_f64.mul_add((value.ln() / 2.4).exp(), -0.044_445)
            };
            let decoded = if value <= 0.031_746 {
                value / 17.0
            } else {
                (((value + 0.044_445) / 1.044_445).ln() * 2.4).exp()
            };
            input.push(encoded as f32);
            output.push(decoded as f32);
        }
        GammaTables { input, output }
    })
}

fn calc_gamma(value: f32, table: &[f32]) -> f32 {
    let index = value * f32::from(u16::MAX);
    if index < 0.0 {
        return 0.0;
    }
    if index > 65_534.99 {
        return 1.0;
    }
    let index = index as usize;
    let diff = value.mul_add(f32::from(u16::MAX), -(index as f32));
    let first = table[index];
    let second = table[index + 1] - first;
    second.mul_add(diff, first)
}

struct GammaTables {
    input: Vec<f32>,
    output: Vec<f32>,
}

#[inline]
const fn tile_index(channel: usize, rr: usize, cc: usize) -> usize {
    channel * TILE_AREA + rr * TILE_SIZE + cc
}

#[inline]
const fn q(qix: &[f32], channel: usize, rr: usize, cc: usize) -> f32 {
    qix[tile_index(channel, rr, cc)]
}

#[inline]
const fn tile_channel(pattern: BayerPattern, rr: usize, cc: usize) -> usize {
    channel(pattern, cc as u32, rr as u32)
}

#[inline]
const fn median3(a: f32, b: f32, c: f32) -> f32 { a.min(b).max(c.min(a.max(b))) }

#[inline]
fn median9(pixels: &[f32; 9]) -> f32 {
    let mut sorted = *pixels;
    sorted.sort_unstable_by(f32::total_cmp);
    sorted[4]
}

#[inline]
fn square(value: f32) -> f32 { value * value }

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;

    use fornax_core::MonoImage;

    use super::*;
    use crate::LazyDemosaic;

    #[test]
    fn images_smaller_than_the_lmmse_border_still_return_ppg_output() -> mischief::Result<()> {
        let samples = (0..16).map(|sample| sample as f64 / 15.0).collect();
        let image = MonoImage::from_raw(4, 4, samples).unwrap();
        let output = DemosaicLmmse.lazy(&image, BayerPattern::RGGB)?.collect();
        assert_eq!(output.dimensions(), (4, 4));
        Ok(())
    }

    #[test]
    fn lmmse_modes_select_median_and_refinement_passes() -> mischief::Result<()> {
        let width = 64;
        let height = 64;
        let samples = (0..width * height)
            .map(|index| {
                let hash = (index as u32).wrapping_mul(0x9e37_79b9).rotate_left(13)
                    ^ (index as u32).wrapping_mul(0x85eb_ca6b);
                f64::from((hash ^ (hash >> 16)) as u16) / f64::from(u16::MAX)
            })
            .collect();
        let image = MonoImage::from_raw(width as u32, height as u32, samples).unwrap();

        let legacy = DemosaicLmmse.lazy(&image, BayerPattern::GBRG)?.collect();
        let basic = DemosaicLmmse
            .lazy_with_mode(&image, BayerPattern::GBRG, LmmseMode::Basic)?
            .collect();
        let median = DemosaicLmmse
            .lazy_with_mode(&image, BayerPattern::GBRG, LmmseMode::Median)?
            .collect();
        let three_medians = DemosaicLmmse
            .lazy_with_mode(&image, BayerPattern::GBRG, LmmseMode::ThreeMedians)?
            .collect();
        let refined = DemosaicLmmse
            .lazy_with_mode(
                &image,
                BayerPattern::GBRG,
                LmmseMode::Refine(NonZeroU8::new(1).unwrap()),
            )?
            .collect();

        assert_eq!(median, legacy);
        assert_ne!(median, basic);
        assert_ne!(median, three_medians);
        assert_ne!(three_medians, refined);
        Ok(())
    }

    #[test]
    #[allow(
        clippy::suboptimal_flops,
        reason = "the inverse-gamma check must retain the same separate operations as the reference"
    )]
    fn gamma_tables_are_inverse_at_table_endpoints_and_interior_codes() {
        let gamma = gamma_tables();
        for code in [0_u16, 1, 122, 123, 2_080, 2_081, 32_768, 65_534, 65_535] {
            let normalized = f32::from(code) / f32::from(u16::MAX);
            let encoded = calc_gamma(normalized, &gamma.input);
            let decoded = calc_gamma(encoded, &gamma.output);
            assert!(
                (decoded * f32::from(u16::MAX) - f32::from(code)).abs() <= 2.0,
                "code {code} round-tripped to {}",
                decoded * f32::from(u16::MAX)
            );
        }
        assert_eq!(calc_gamma(-0.1, &gamma.input), 0.0);
        assert_eq!(calc_gamma(1.0, &gamma.output), 1.0);
    }

    #[test]
    #[allow(
        clippy::suboptimal_flops,
        reason = "the synthetic tile pattern must retain the same floating-point construction"
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

        for boundary in [120, 232] {
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
        }

        for boundary in [120, 232] {
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
