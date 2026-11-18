//! Tiled linear minimum mean-square-error Bayer demosaicing.
//!
//! This path follows the published LMMSE algorithm and darktable 5.6.2's
//! mode conventions. See the
//! [LMMSE reference notes](https://github.com/Glatzel/fornax/blob/main/docs/reference/algorithms/lmmse.md).

use fornax_raw_core::error::DemosaicError;
use fornax_raw_core::{BayerPattern, MonoImage, Primitive};
use generic_num::num;

use super::LmmseMode;
use super::buffer::{DenseLazy, channel, dense_from_vec};
use super::numerics::normalization_scale;
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

const GAMMA_INPUT_THRESHOLD: f64 = 0.001_867;
const GAMMA_OUTPUT_THRESHOLD: f64 = 0.031_746;
const GAMMA_SLOPE: f64 = 17.0;
const GAMMA_ALPHA: f64 = 1.044_445;
const GAMMA_OFFSET: f64 = 0.044_445;
const GAMMA_ENCODE_POWER: f64 = 1.0 / 2.4;
const GAMMA_DECODE_POWER: f64 = 2.4;

struct LmmseTransfer<T: Primitive> {
    input_threshold: T,
    output_threshold: T,
    slope: T,
    alpha: T,
    offset: T,
    encode_power: T,
    decode_power: T,
}

impl<T: Primitive> LmmseTransfer<T> {
    fn new() -> Self {
        Self {
            input_threshold: num!(GAMMA_INPUT_THRESHOLD),
            output_threshold: num!(GAMMA_OUTPUT_THRESHOLD),
            slope: num!(GAMMA_SLOPE),
            alpha: num!(GAMMA_ALPHA),
            offset: num!(GAMMA_OFFSET),
            encode_power: num!(GAMMA_ENCODE_POWER),
            decode_power: num!(GAMMA_DECODE_POWER),
        }
    }

    fn encode(&self, value: T) -> T {
        if value.abs() > T::one() {
            return value.signum() * value.abs().powf(self.encode_power);
        }
        let sign = if value < T::zero() {
            -T::one()
        } else {
            T::one()
        };
        let magnitude = value.abs();
        sign * if magnitude <= self.input_threshold {
            magnitude * self.slope
        } else {
            self.alpha
                .mul_add(magnitude.powf(self.encode_power), -self.offset)
        }
    }

    fn decode(&self, value: T) -> T {
        if value.abs() > T::one() {
            return value.signum() * value.abs().powf(self.decode_power);
        }
        let sign = if value < T::zero() {
            -T::one()
        } else {
            T::one()
        };
        let magnitude = value.abs();
        sign * if magnitude <= self.output_threshold {
            magnitude / self.slope
        } else {
            ((magnitude + self.offset) / self.alpha).powf(self.decode_power)
        }
    }
}

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
        let scale = normalization_scale(image.as_raw());
        let ppg_samples = image
            .as_raw()
            .iter()
            .map(|sample| *sample / scale)
            .collect();
        let ppg_input = MonoImage::from_raw(image.width(), image.height(), ppg_samples)
            .ok_or_else(|| {
                DemosaicError::InvalidInput("LMMSE could not copy the Bayer mosaic".into())
            })?;
        let seed: fornax_raw_core::RgbImage<T> = DemosaicPpg.lazy(&ppg_input, pattern)?.collect();
        let mut output = seed.into_raw();

        if width >= 2 * BORDER && height >= 2 * BORDER {
            interpolate(
                ppg_input.as_raw(),
                &mut output,
                width,
                height,
                pattern,
                mode,
            );
        }
        for value in &mut output {
            *value = *value * scale;
        }

        Ok(DenseLazy::new(dense_from_vec(
            image.width(),
            image.height(),
            output,
        )))
    }
}

fn interpolate<T: Primitive>(
    input: &[T],
    output: &mut [T],
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
    let transfer = LmmseTransfer::<T>::new();
    let h0 = T::one();
    let h1 = (-T::one() / num!(8.0)).exp();
    let h2 = (-num!(4.0) / num!(8.0)).exp();
    let h3 = (-num!(9.0) / num!(8.0)).exp();
    let h4 = (-num!(16.0) / num!(8.0)).exp();
    let hsum = num!(2.0) * (h1 + h2 + h3 + h4) + h0;
    let h0 = h0 / hsum;
    let h1 = h1 / hsum;
    let h2 = h2 / hsum;
    let h3 = h3 / hsum;
    let h4 = h4 / hsum;

    let num_vertical = 1 + height.saturating_sub(2 * OVERLAP + 1) / TILE_VALID;
    let num_horizontal = 1 + width.saturating_sub(2 * OVERLAP + 1) / TILE_VALID;
    let mut qix = vec![T::zero(); 6 * TILE_AREA];

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

            for rr in 0..last_rr {
                let row = reflect_index(row_start as isize + rr as isize - BORDER as isize, height);
                for cc in 0..last_cc {
                    let col =
                        reflect_index(col_start as isize + cc as isize - BORDER as isize, width);
                    qix[tile_index(5, rr, cc)] = transfer.encode(input[row * width + col]);
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
                &transfer,
            );
        }
    }
}

fn reflect_index(index: isize, length: usize) -> usize {
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

fn refine_tile<T: Primitive>(
    qix: &mut [T],
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
            let left = num!(1.0)
                / (num!(1.0) + (q(qix, c, rr, cc - 2) - center_color).abs() + horizontal_green);
            let right = num!(1.0)
                / (num!(1.0) + (q(qix, c, rr, cc + 2) - center_color).abs() + horizontal_green);
            let up = num!(1.0)
                / (num!(1.0) + (q(qix, c, rr - 2, cc) - center_color).abs() + vertical_green);
            let down = num!(1.0)
                / (num!(1.0) + (q(qix, c, rr + 2, cc) - center_color).abs() + vertical_green);
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
                let left = num!(1.0) / (num!(1.0) + left_green + horizontal_color);
                let right = num!(1.0) / (num!(1.0) + right_green + horizontal_color);
                let up = num!(1.0) / (num!(1.0) + up_green + vertical_color);
                let down = num!(1.0) / (num!(1.0) + down_green + vertical_color);
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
            let left = num!(1.0)
                / (num!(1.0)
                    + (q(qix, d, rr, cc - 2) - center_integrated).abs()
                    + horizontal_green);
            let right = num!(1.0)
                / (num!(1.0)
                    + (q(qix, d, rr, cc + 2) - center_integrated).abs()
                    + horizontal_green);
            let up = num!(1.0)
                / (num!(1.0) + (q(qix, d, rr - 2, cc) - center_integrated).abs() + vertical_green);
            let down = num!(1.0)
                / (num!(1.0) + (q(qix, d, rr + 2, cc) - center_integrated).abs() + vertical_green);
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

fn color_differences<T: Primitive>(
    qix: &mut [T],
    last_rr: usize,
    last_cc: usize,
    pattern: BayerPattern,
) {
    for rr in 2..last_rr - 2 {
        let mut cc = 2 + (tile_channel(pattern, rr, 2) & 1);
        while cc < last_cc - 2 {
            let center = q(qix, 5, rr, cc);
            let value = num!(0.25).mul_add(
                center,
                num!(0.0625)
                    * (q(qix, 5, rr - 1, cc - 1)
                        + q(qix, 5, rr - 1, cc + 1)
                        + q(qix, 5, rr + 1, cc - 1)
                        + q(qix, 5, rr + 1, cc + 1)),
            );

            let mut horizontal = num!(0.5).mul_add(
                q(qix, 5, rr, cc - 1) + center + q(qix, 5, rr, cc + 1),
                -num!(0.25) * (q(qix, 5, rr, cc - 2) + q(qix, 5, rr, cc + 2)),
            );
            let horizontal_luma = num!(0.5).mul_add(horizontal, value);
            horizontal = if center > num!(1.75) * horizontal_luma {
                median3(horizontal, q(qix, 5, rr, cc - 1), q(qix, 5, rr, cc + 1))
            } else {
                horizontal
            } - center;

            let mut vertical = num!(0.5).mul_add(
                q(qix, 5, rr - 1, cc) + center + q(qix, 5, rr + 1, cc),
                -num!(0.25) * (q(qix, 5, rr - 2, cc) + q(qix, 5, rr + 2, cc)),
            );
            let vertical_luma = num!(0.5).mul_add(vertical, value);
            vertical = if center > num!(1.75) * vertical_luma {
                median3(vertical, q(qix, 5, rr - 1, cc), q(qix, 5, rr + 1, cc))
            } else {
                vertical
            } - center;

            qix[tile_index(0, rr, cc)] = horizontal;
            qix[tile_index(1, rr, cc)] = vertical;
            cc += 2;
        }

        let mut cc = 2 + (tile_channel(pattern, rr, 3) & 1);
        while cc < last_cc - 2 {
            let center = q(qix, 5, rr, cc);
            let horizontal = num!(0.5).mul_add(
                -(q(qix, 5, rr, cc - 1) + center + q(qix, 5, rr, cc + 1)),
                num!(0.25) * (q(qix, 5, rr, cc - 2) + q(qix, 5, rr, cc + 2)),
            ) + center;
            let vertical = num!(0.5).mul_add(
                -(q(qix, 5, rr - 1, cc) + center + q(qix, 5, rr + 1, cc)),
                num!(0.25) * (q(qix, 5, rr - 2, cc) + q(qix, 5, rr + 2, cc)),
            ) + center;
            qix[tile_index(0, rr, cc)] = horizontal;
            qix[tile_index(1, rr, cc)] = vertical;
            cc += 2;
        }
    }
}

fn low_pass<T: Primitive>(qix: &mut [T], last_rr: usize, last_cc: usize, h: [T; 5]) {
    let [h0, h1, h2, h3, h4] = h;
    for rr in 4..last_rr - 4 {
        for cc in 4..last_cc - 4 {
            let horizontal = T::mul_add(
                h4,
                q(qix, 0, rr, cc - 4) + q(qix, 0, rr, cc + 4),
                T::mul_add(
                    h3,
                    q(qix, 0, rr, cc - 3) + q(qix, 0, rr, cc + 3),
                    T::mul_add(
                        h2,
                        q(qix, 0, rr, cc - 2) + q(qix, 0, rr, cc + 2),
                        T::mul_add(
                            h1,
                            q(qix, 0, rr, cc - 1) + q(qix, 0, rr, cc + 1),
                            h0 * q(qix, 0, rr, cc),
                        ),
                    ),
                ),
            );
            let vertical = T::mul_add(
                h4,
                q(qix, 1, rr - 4, cc) + q(qix, 1, rr + 4, cc),
                T::mul_add(
                    h3,
                    q(qix, 1, rr - 3, cc) + q(qix, 1, rr + 3, cc),
                    T::mul_add(
                        h2,
                        q(qix, 1, rr - 2, cc) + q(qix, 1, rr + 2, cc),
                        T::mul_add(
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

fn mmse_interpolate<T: Primitive>(
    qix: &mut [T],
    last_rr: usize,
    last_cc: usize,
    pattern: BayerPattern,
) {
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

            qix[tile_index(4, rr, cc)] = T::mul_add(xv, vh, xh * vv) / (vh + vv);
            cc += 2;
        }
    }
}

fn mmse_value<T: Primitive>(mut filtered: [T; 9], raw: [T; 9]) -> (T, T) {
    let mean = filtered.iter().fold(T::zero(), |sum, value| sum + *value) / num!(9.0);
    let filtered_center = filtered[4];
    let variance_filtered = num!(1.0e-7)
        + filtered
            .iter()
            .map(|sample| square(*sample - mean))
            .fold(T::zero(), |sum, value| sum + value);
    for (filtered, raw) in filtered.iter_mut().zip(raw) {
        *filtered = *filtered - raw;
    }
    let variance_raw = num!(1.0e-7)
        + filtered
            .iter()
            .map(|sample| square(*sample))
            .fold(T::zero(), |sum, value| sum + value);
    let value = filtered_center.mul_add(variance_raw, raw[4] * variance_filtered)
        / (variance_filtered + variance_raw);
    (
        value,
        variance_filtered * variance_raw / (variance_filtered + variance_raw),
    )
}

fn copy_cfa_samples<T: Primitive>(
    qix: &mut [T],
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
            let sample = if inside { q(qix, 5, rr, cc) } else { num!(0.0) };
            let c = tile_channel(pattern, rr, cc);
            qix[tile_index(c, rr, cc)] = sample;
            if c != GREEN {
                qix[tile_index(GREEN, rr, cc)] = if inside {
                    sample + q(qix, 4, rr, cc)
                } else {
                    num!(0.0)
                };
            }
        }
    }
}

fn interpolate_red_blue<T: Primitive>(
    qix: &mut [T],
    last_rr: usize,
    last_cc: usize,
    pattern: BayerPattern,
) {
    for rr in 1..last_rr - 1 {
        let mut cc = 1 + (tile_channel(pattern, rr, 2) & 1);
        while cc < last_cc - 1 {
            let mut c = tile_channel(pattern, rr, cc + 1);
            let green = q(qix, 1, rr, cc);
            let value = num!(0.5).mul_add(
                q(qix, c, rr, cc - 1) - q(qix, 1, rr, cc - 1) + q(qix, c, rr, cc + 1)
                    - q(qix, 1, rr, cc + 1),
                green,
            );
            qix[tile_index(c, rr, cc)] = value;
            c = 2 - c;
            let value = num!(0.5).mul_add(
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
            let value = num!(0.25).mul_add(
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

fn median_pass<T: Primitive>(
    qix: &mut [T],
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
                let mut samples = [num!(0.0); 9];
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
                qix[tile_index(GREEN, rr, cc)] = num!(0.5)
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
                qix[tile_index(GREEN, rr, cc)] = num!(0.5)
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
                qix[tile_index(GREEN, rr, cc)] = num!(0.5)
                    * (q(qix, RED, rr, cc) - q(qix, 3, rr, cc) + q(qix, BLUE, rr, cc)
                        - q(qix, 4, rr, cc));
            }
        }
    }
}

fn write_tile<T: Primitive>(
    qix: &[T],
    output: &mut [T],
    width: usize,
    tile_vertical: usize,
    num_vertical: usize,
    tile_horizontal: usize,
    num_horizontal: usize,
    row_start: usize,
    row_end: usize,
    col_start: usize,
    col_end: usize,
    transfer: &LmmseTransfer<T>,
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
                output[target + color] = transfer.decode(q(qix, color, rr, cc));
            }
        }
    }
}

#[inline]
const fn tile_index(channel: usize, rr: usize, cc: usize) -> usize {
    channel * TILE_AREA + rr * TILE_SIZE + cc
}

#[inline]
const fn q<T: Primitive>(qix: &[T], channel: usize, rr: usize, cc: usize) -> T {
    qix[tile_index(channel, rr, cc)]
}

#[inline]
const fn tile_channel(pattern: BayerPattern, rr: usize, cc: usize) -> usize {
    channel(pattern, cc as u32, rr as u32)
}

#[inline]
fn median3<T: Primitive>(a: T, b: T, c: T) -> T { a.min(b).max(c.min(a.max(b))) }

#[inline]
fn median9<T: Primitive>(pixels: &[T; 9]) -> T {
    let mut sorted = *pixels;
    sorted.sort_unstable_by(|left, right| {
        left.partial_cmp(right).unwrap_or(std::cmp::Ordering::Equal)
    });
    sorted[4]
}

#[inline]
fn square<T: Primitive>(value: T) -> T { value * value }

#[cfg(test)]
mod tests {
    use std::num::NonZeroU8;

    use fornax_raw_core::MonoImage;

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
    fn analytic_transfer_round_trips_signed_hdr_values() {
        let transfer = LmmseTransfer::<f64>::new();
        for value in [-4.0_f64, -1.0, -0.1, 0.0, 0.001, 0.1, 1.0, 4.0, 1.0e4] {
            let encoded = transfer.encode(value);
            let decoded = transfer.decode(encoded);
            assert!(
                (decoded - value).abs() <= 2.0e-12,
                "{value} round-tripped to {decoded}"
            );
        }
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
