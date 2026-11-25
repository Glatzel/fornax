//! Patterned pixel grouping (PPG) Bayer demosaicing.
//!
//! Estimates green at red/blue sites using horizontal and vertical gradients
//! with a local min/max envelope inside a 3-pixel border, reconstructs red/blue
//! at green sites from cardinal color differences inside a 1-pixel border, and
//! reconstructs the remaining channel at red/blue sites from diagonal color
//! differences inside a 1-pixel border.

use generic_num::num;

use super::LazyDemosaic;
use crate::error::DemosaicError;
use crate::{BayerChannel, BayerPattern, MonoImage, RawFloat};

const RED: usize = 0;
const GREEN: usize = 1;
const BLUE: usize = 2;

pub struct PpgLazy<'a, T> {
    raw: &'a [T],
    width: u32,
    height: u32,
    pattern: BayerPattern,
}

impl<T: RawFloat + Send + Sync> LazyDemosaic<T> for PpgLazy<'_, T> {
    fn dimensions(&self) -> (u32, u32) { (self.width, self.height) }

    fn fill_rows(&self, y0: usize, out: &mut [T]) {
        let width = self.width as usize;
        let height = self.height as usize;
        let row_stride = width * 3;
        let chunk_rows = out.len() / row_stride;
        if chunk_rows == 0 {
            return;
        }

        // Cache green values for the chunk plus a 1-row halo above and below so
        // cardinal/diagonal chroma interpolation never recomputes `green_at`.
        let g_y_start = y0.saturating_sub(1);
        let g_y_end = (y0 + chunk_rows + 1).min(height);
        let mut green_strip = vec![T::zero(); (g_y_end - g_y_start) * width];
        for y in g_y_start..g_y_end {
            let row_offset = (y - g_y_start) * width;
            for x in 0..width {
                green_strip[row_offset + x] = self.green_at(x as u32, y as u32);
            }
        }

        for (dy, row) in out.chunks_exact_mut(row_stride).enumerate() {
            self.fill_row(y0 + dy, g_y_start, &green_strip, row);
        }
    }
}

impl<T: RawFloat + Send + Sync> PpgLazy<'_, T> {
    fn fill_row(&self, y: usize, g_y_start: usize, green_strip: &[T], out: &mut [T]) {
        let yu = y as u32;
        for (x, pixel) in out.as_chunks_mut::<3>().0.iter_mut().enumerate() {
            let xu = x as u32;
            let green = self.green_from_strip(green_strip, g_y_start, xu, yu);
            let sensor = self.pattern.channel_at(xu, yu);

            match sensor {
                BayerChannel::R | BayerChannel::B => {
                    let current = sensor_color(sensor);
                    let missing = BLUE - current;
                    pixel[current] = self.raw_at(xu, yu);
                    pixel[GREEN] = green;
                    pixel[missing] =
                        self.opposite_at_non_green(xu, yu, current, green, green_strip, g_y_start);
                }
                BayerChannel::GRedRow | BayerChannel::GBlueRow => {
                    let horizontal = horizontal_color(sensor);
                    pixel[GREEN] = green;
                    pixel[horizontal] =
                        self.opposite_at_green(xu, yu, horizontal, green, green_strip, g_y_start);
                    pixel[BLUE - horizontal] = self.opposite_at_green(
                        xu,
                        yu,
                        BLUE - horizontal,
                        green,
                        green_strip,
                        g_y_start,
                    );
                }
            }
        }
    }

    #[inline(always)]
    const fn green_from_strip(&self, strip: &[T], g_y_start: usize, x: u32, y: u32) -> T {
        strip[(y as usize - g_y_start) * self.width as usize + x as usize]
    }

    fn green_at(&self, x: u32, y: u32) -> T {
        let sensor = self.pattern.channel_at(x, y);
        if sensor.is_green() {
            return self.raw_at(x, y);
        }
        if self.in_three_pixel_border(x, y) {
            return self.border_channel(x, y, GREEN);
        }

        let center = self.raw_at(x, y);
        let north = self.raw_at(x, y - 1);
        let east = self.raw_at(x + 1, y);
        let south = self.raw_at(x, y + 1);
        let west = self.raw_at(x - 1, y);
        let north2 = self.raw_at(x, y - 2);
        let east2 = self.raw_at(x + 2, y);
        let south2 = self.raw_at(x, y + 2);
        let west2 = self.raw_at(x - 2, y);
        let north3 = self.raw_at(x, y - 3);
        let east3 = self.raw_at(x + 3, y);
        let south3 = self.raw_at(x, y + 3);
        let west3 = self.raw_at(x - 3, y);
        debug_assert_eq!(self.pattern.channel_at(x - 2, y), sensor);
        debug_assert_eq!(self.pattern.channel_at(x + 2, y), sensor);
        debug_assert_eq!(self.pattern.channel_at(x, y - 2), sensor);
        debug_assert_eq!(self.pattern.channel_at(x, y + 2), sensor);
        let horizontal_guess = (west + center + east) * num!(2) - east2 - west2;
        let horizontal_diff =
            ((west2 - center).abs() + (east2 - center).abs() + (west - east).abs()) * num!(3)
                + ((east3 - east).abs() + (west3 - west).abs()) * num!(2);
        let vertical_guess = (north + center + south) * num!(2) - south2 - north2;
        let vertical_diff =
            ((north2 - center).abs() + (south2 - center).abs() + (north - south).abs()) * num!(3)
                + ((south3 - south).abs() + (north3 - north).abs()) * num!(2);

        let (guess, first, second) = if horizontal_diff > vertical_diff {
            (vertical_guess, north, south)
        } else {
            (horizontal_guess, west, east)
        };

        (guess / num!(4))
            .max(first.min(second))
            .min(first.max(second))
    }

    fn opposite_at_green(
        &self,
        x: u32,
        y: u32,
        color: usize,
        center_green: T,
        green_strip: &[T],
        g_y_start: usize,
    ) -> T {
        if x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height {
            return self.border_channel(x, y, color);
        }

        let horizontal = horizontal_color(self.pattern.channel_at(x, y));
        let (first, second, green) = if color == horizontal {
            (
                self.raw_at(x - 1, y),
                self.raw_at(x + 1, y),
                [
                    self.green_from_strip(green_strip, g_y_start, x - 1, y),
                    center_green,
                    self.green_from_strip(green_strip, g_y_start, x + 1, y),
                ],
            )
        } else {
            (
                self.raw_at(x, y - 1),
                self.raw_at(x, y + 1),
                [
                    self.green_from_strip(green_strip, g_y_start, x, y - 1),
                    center_green,
                    self.green_from_strip(green_strip, g_y_start, x, y + 1),
                ],
            )
        };

        (first + second + green[1] * num!(2) - green[0] - green[2]) / num!(2)
    }

    fn opposite_at_non_green(
        &self,
        x: u32,
        y: u32,
        current: usize,
        center: T,
        green_strip: &[T],
        g_y_start: usize,
    ) -> T {
        let missing = BLUE - current;
        if x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height {
            return self.border_channel(x, y, missing);
        }

        let ne = self.raw_at(x + 1, y - 1);
        let sw = self.raw_at(x - 1, y + 1);
        let nw = self.raw_at(x - 1, y - 1);
        let se = self.raw_at(x + 1, y + 1);
        let g_ne = self.green_from_strip(green_strip, g_y_start, x + 1, y - 1);
        let g_sw = self.green_from_strip(green_strip, g_y_start, x - 1, y + 1);
        let g_nw = self.green_from_strip(green_strip, g_y_start, x - 1, y - 1);
        let g_se = self.green_from_strip(green_strip, g_y_start, x + 1, y + 1);
        let ne_diff = (ne - sw).abs() + (g_ne - center).abs() + (center - g_sw).abs();
        let nw_diff = (nw - se).abs() + (g_nw - center).abs() + (center - g_se).abs();
        let ne_guess = ne + sw + center * num!(2) - g_ne - g_sw;
        let nw_guess = nw + se + center * num!(2) - g_nw - g_se;
        let guess = if ne_diff > nw_diff {
            nw_guess
        } else if ne_diff < nw_diff {
            ne_guess
        } else {
            (ne_guess + nw_guess) / num!(2)
        };
        guess / num!(2)
    }

    fn border_channel(&self, x: u32, y: u32, color: usize) -> T {
        if sensor_color(self.pattern.channel_at(x, y)) == color {
            return self.raw_at(x, y);
        }

        let mut sum = T::zero();
        let mut count = 0_u32;
        for dy in -1_i32..=1 {
            for dx in -1_i32..=1 {
                let nx = x as i32 + dx;
                let ny = y as i32 + dy;
                if nx < 0 || ny < 0 || nx >= self.width as i32 || ny >= self.height as i32 {
                    continue;
                }
                let nx = nx as u32;
                let ny = ny as u32;
                if sensor_color(self.pattern.channel_at(nx, ny)) == color {
                    sum = sum + self.raw_at(nx, ny);
                    count += 1;
                }
            }
        }
        sum / num!(count)
    }

    const fn in_three_pixel_border(&self, x: u32, y: u32) -> bool {
        x < 3 || y < 3 || x + 3 >= self.width || y + 3 >= self.height
    }

    const fn raw_at(&self, x: u32, y: u32) -> T {
        self.raw[y as usize * self.width as usize + x as usize]
    }
}

#[inline(always)]
const fn sensor_color(channel: BayerChannel) -> usize {
    match channel {
        BayerChannel::R => RED,
        BayerChannel::GRedRow | BayerChannel::GBlueRow => GREEN,
        BayerChannel::B => BLUE,
    }
}

#[inline(always)]
const fn horizontal_color(green_channel: BayerChannel) -> usize {
    match green_channel {
        BayerChannel::GRedRow => RED,
        BayerChannel::GBlueRow => BLUE,
        BayerChannel::R | BayerChannel::B => unreachable!(),
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicPpg;

impl DemosaicPpg {
    pub fn lazy<'a, T>(
        &self,
        bayer: &'a MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<PpgLazy<'a, T>, DemosaicError>
    where
        T: RawFloat + Send + Sync,
    {
        if bayer.width() < 3 || bayer.height() < 3 {
            return Err(DemosaicError::InvalidInput(
                "PPG requires an image at least 3x3".into(),
            ));
        }
        Ok(PpgLazy {
            raw: bayer.as_raw(),
            width: bayer.width(),
            height: bayer.height(),
            pattern,
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::MonoImage;

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn fixed_9x9_rggb_mosaic_matches_reference() -> mischief::Result<()> {
        // PPG reads up to three pixels in each direction. A 9x9 fixture is the
        // smallest practical fixed oracle with a broad interior region rather
        // than only the border fallback.
        let input = [
            [
                0.0_f32, 204.0, 60.0, 264.0, 120.0, 324.0, 180.0, 36.0, 240.0,
            ],
            [24.0, 312.0, 252.0, 192.0, 132.0, 72.0, 12.0, 300.0, 240.0],
            [48.0, 72.0, 96.0, 120.0, 144.0, 168.0, 192.0, 216.0, 240.0],
            [72.0, 180.0, 288.0, 48.0, 156.0, 264.0, 24.0, 132.0, 240.0],
            [96.0, 288.0, 132.0, 324.0, 168.0, 12.0, 204.0, 48.0, 240.0],
            [120.0, 48.0, 324.0, 252.0, 180.0, 108.0, 36.0, 312.0, 240.0],
            [
                144.0, 156.0, 168.0, 180.0, 192.0, 204.0, 216.0, 228.0, 240.0,
            ],
            [168.0, 264.0, 12.0, 108.0, 204.0, 300.0, 48.0, 144.0, 240.0],
            [192.0, 24.0, 204.0, 36.0, 216.0, 48.0, 228.0, 60.0, 240.0],
        ];
        let image = MonoImage::from_raw(9, 9, input.into_iter().flatten().collect()).unwrap();
        let output = DemosaicPpg.lazy(&image, BayerPattern::RGGB)?.collect();
        insta::assert_snapshot!(crate::demosaic::test_support::format_rgb_3dp(&output));
        Ok(())
    }

    #[test]
    fn uniform_mosaic_has_neutral_rgb_output() -> mischief::Result<()> {
        let image = MonoImage::from_raw(8, 8, vec![0.25_f32; 64]).unwrap();
        for pattern in BayerPattern::ALL {
            let output = DemosaicPpg.lazy(&image, pattern)?.collect();
            assert!(
                output
                    .as_raw()
                    .iter()
                    .all(|sample| (*sample - 0.25).abs() < 1.0e-6)
            );
        }
        Ok(())
    }

    #[test]
    fn measured_bayer_samples_are_preserved() -> mischief::Result<()> {
        let samples = (0..81)
            .map(|sample| sample as f64 / 80.0)
            .collect::<Vec<_>>();
        let image = MonoImage::from_raw(9, 9, samples).unwrap();
        for pattern in BayerPattern::ALL {
            let output = DemosaicPpg.lazy(&image, pattern)?.collect();
            for y in 0..image.height() {
                for x in 0..image.width() {
                    let color = sensor_color(pattern.channel_at(x, y));
                    assert_eq!(
                        output.get_pixel(x, y)[color],
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
        let samples = (0..72)
            .map(|sample| sample as f32 / 71.0)
            .collect::<Vec<_>>();
        let image = MonoImage::from_raw(8, 9, samples).unwrap();
        let lazy = DemosaicPpg.lazy(&image, BayerPattern::GRBG)?;
        let whole = lazy.collect();

        let mut rows = vec![0.0; 8 * 3 * 9];
        lazy.fill_rows(0, &mut rows);
        assert_eq!(whole.as_raw(), &rows);

        let mut single_rows = vec![0.0; 8 * 3 * 9];
        let row_stride = 8 * 3;
        for y in 0..9 {
            lazy.fill_rows(y, &mut single_rows[y * row_stride..(y + 1) * row_stride]);
        }
        assert_eq!(whole.as_raw(), &single_rows);
        Ok(())
    }

    #[test]
    fn images_smaller_than_three_pixels_are_rejected() {
        for (width, height) in [(1, 1), (2, 2), (2, 3), (3, 2)] {
            let image = MonoImage::from_raw(
                width,
                height,
                vec![0.0_f32; width as usize * height as usize],
            )
            .unwrap();
            assert!(matches!(
                DemosaicPpg.lazy(&image, BayerPattern::RGGB),
                Err(DemosaicError::InvalidInput(_))
            ));
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

    #[test]
    fn signed_and_zero_constants_are_preserved() -> mischief::Result<()> {
        for pattern in BayerPattern::ALL {
            for value in [-0.25_f32, 0.0_f32, 4.0_f32] {
                let image = MonoImage::from_raw(16, 16, vec![value; 256]).unwrap();
                let output = DemosaicPpg.lazy(&image, pattern)?.collect();
                assert!(
                    output
                        .as_raw()
                        .iter()
                        .all(|sample| (*sample - value).abs() < 1.0e-6)
                );
            }
            for value in [-0.25_f64, 0.0_f64, 1.0e4_f64] {
                let image = MonoImage::from_raw(16, 16, vec![value; 256]).unwrap();
                let output = DemosaicPpg.lazy(&image, pattern)?.collect();
                assert!(
                    output
                        .as_raw()
                        .iter()
                        .all(|sample| (*sample - value).abs() < 1.0e-11)
                );
            }
        }
        Ok(())
    }

    #[test]
    fn signed_zero_crossing_ramp_is_finite_and_exact() -> mischief::Result<()> {
        let width = 16_u32;
        let height = 16_u32;
        let samples = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| {
                    let t = (x + y) as f64 / f64::from(width + height - 2);
                    0.5f64.mul_add(t, -0.25)
                })
            })
            .collect();
        let image = MonoImage::from_raw(width, height, samples).unwrap();
        let output = DemosaicPpg.lazy(&image, BayerPattern::RGGB)?.collect();
        for y in 0..height {
            for x in 0..width {
                for ch in output.get_pixel(x, y).0 {
                    assert!(ch.is_finite());
                }
            }
        }
        for y in 4..height - 4 {
            for x in 4..width - 4 {
                let expected = -0.25 + 0.5 * (x + y) as f64 / f64::from(width + height - 2);
                for ch in output.get_pixel(x, y).0 {
                    assert!(
                        (ch - expected).abs() < 1.0e-12,
                        "at ({x},{y}): {ch} vs {expected}"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn positive_scale_equivariance_holds_in_f32_and_f64() -> mischief::Result<()> {
        let base_f32: MonoImage<f32> = seeded_mosaic(24, 24, -0.5, 2.5);
        let base_out_f32 = DemosaicPpg.lazy(&base_f32, BayerPattern::GBRG)?.collect();
        for scale in [1.0e-4_f32, 1.0e-1, 10.0, 1.0e4] {
            let scaled = MonoImage::from_raw(
                24,
                24,
                base_f32.as_raw().iter().map(|v| *v * scale).collect(),
            )
            .unwrap();
            let out = DemosaicPpg.lazy(&scaled, BayerPattern::GBRG)?.collect();
            for (a, b) in out.as_raw().iter().zip(base_out_f32.as_raw()) {
                let expected = *b * scale;
                let tol = 1.0e-5 * expected.abs().max(scale);
                assert!(
                    (*a - expected).abs() <= tol,
                    "f32 scale {scale}: {a} vs {expected}"
                );
            }
        }

        let base_f64: MonoImage<f64> = seeded_mosaic(24, 24, -0.5, 2.5);
        let base_out_f64 = DemosaicPpg.lazy(&base_f64, BayerPattern::GBRG)?.collect();
        for scale in [1.0e-6_f64, 1.0e6] {
            let scaled = MonoImage::from_raw(
                24,
                24,
                base_f64.as_raw().iter().map(|v| *v * scale).collect(),
            )
            .unwrap();
            let out = DemosaicPpg.lazy(&scaled, BayerPattern::GBRG)?.collect();
            for (a, b) in out.as_raw().iter().zip(base_out_f64.as_raw()) {
                let expected = *b * scale;
                let tol = 1.0e-11 * expected.abs().max(scale);
                assert!(
                    (*a - expected).abs() <= tol,
                    "f64 scale {scale}: {a} vs {expected}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn shift_invariance_holds_in_f32_and_f64() -> mischief::Result<()> {
        let base_f32: MonoImage<f32> = seeded_mosaic(24, 24, -0.25, 1.5);
        let base_out_f32 = DemosaicPpg.lazy(&base_f32, BayerPattern::BGGR)?.collect();
        for shift in [-0.37_f32, 12.5_f32] {
            let shifted = MonoImage::from_raw(
                24,
                24,
                base_f32.as_raw().iter().map(|v| *v + shift).collect(),
            )
            .unwrap();
            let out = DemosaicPpg.lazy(&shifted, BayerPattern::BGGR)?.collect();
            for (a, b) in out.as_raw().iter().zip(base_out_f32.as_raw()) {
                let expected = *b + shift;
                let tol = 2.0e-5 * expected.abs().max(1.0);
                assert!(
                    (*a - expected).abs() <= tol,
                    "f32 shift {shift}: {a} vs {expected}"
                );
            }
        }

        let base_f64: MonoImage<f64> = seeded_mosaic(24, 24, -0.25, 1.5);
        let base_out_f64 = DemosaicPpg.lazy(&base_f64, BayerPattern::BGGR)?.collect();
        for shift in [-0.37_f64, 12.5_f64] {
            let shifted = MonoImage::from_raw(
                24,
                24,
                base_f64.as_raw().iter().map(|v| *v + shift).collect(),
            )
            .unwrap();
            let out = DemosaicPpg.lazy(&shifted, BayerPattern::BGGR)?.collect();
            for (a, b) in out.as_raw().iter().zip(base_out_f64.as_raw()) {
                let expected = *b + shift;
                assert!(
                    (*a - expected).abs() <= 1.0e-11,
                    "f64 shift {shift}: {a} vs {expected}"
                );
            }
        }
        Ok(())
    }

    #[test]
    fn vertical_flip_symmetry_holds() -> mischief::Result<()> {
        let width = 24_u32;
        let height = 24_u32;
        let base: MonoImage<f64> = seeded_mosaic(width, height, -0.5, 2.0);
        let mut flipped_samples = Vec::with_capacity((width * height) as usize);
        for y in (0..height).rev() {
            for x in 0..width {
                flipped_samples.push(base.get_pixel(x, y)[0]);
            }
        }
        let flipped = MonoImage::from_raw(width, height, flipped_samples).unwrap();

        for (pattern, flipped_pattern) in [
            (BayerPattern::RGGB, BayerPattern::GBRG),
            (BayerPattern::GBRG, BayerPattern::RGGB),
            (BayerPattern::BGGR, BayerPattern::GRBG),
            (BayerPattern::GRBG, BayerPattern::BGGR),
        ] {
            let out = DemosaicPpg.lazy(&base, pattern)?.collect();
            let out_flip = DemosaicPpg.lazy(&flipped, flipped_pattern)?.collect();
            for y in 0..height {
                for x in 0..width {
                    let a = out.get_pixel(x, y).0;
                    let b = out_flip.get_pixel(x, height - 1 - y).0;
                    for c in 0..3 {
                        assert!(
                            (a[c] - b[c]).abs() < 1.0e-12,
                            "{pattern} at ({x},{y}) ch {c}"
                        );
                    }
                }
            }
        }
        Ok(())
    }
}
