//! Patterned pixel grouping interpolation derived from LibRaw and darktable.
//!
//! The algorithm performs a three-pixel border fill, estimates green with
//! horizontal and vertical gradients, reconstructs red/blue at green sites
//! from local color differences, and then reconstructs the remaining channel
//! from diagonal color differences.

use fornax_core::error::DemosaicError;
use fornax_core::protocol::LazyDemosaic;
use fornax_core::{BayerChannel, BayerPattern, MonoImage, Primitive};
use generic_num::num;

const RED: usize = 0;
const GREEN: usize = 1;
const BLUE: usize = 2;

pub struct PpgLazy<'a, T> {
    raw: &'a [T],
    width: u32,
    height: u32,
    pattern: BayerPattern,
}

impl<T: Primitive + Send + Sync> LazyDemosaic<T> for PpgLazy<'_, T> {
    fn dimensions(&self) -> (u32, u32) { (self.width, self.height) }

    fn fill_rows(&self, y0: usize, out: &mut [T]) {
        let row_stride = self.width as usize * 3;
        for (dy, row) in out.chunks_exact_mut(row_stride).enumerate() {
            self.fill_row(y0 + dy, row);
        }
    }
}

impl<T: Primitive + Send + Sync> PpgLazy<'_, T> {
    fn fill_row(&self, y: usize, out: &mut [T]) {
        for (x, pixel) in out.as_chunks_mut::<3>().0.iter_mut().enumerate() {
            pixel.copy_from_slice(&self.pixel(x as u32, y as u32));
        }
    }

    fn pixel(&self, x: u32, y: u32) -> [T; 3] {
        let sensor = self.pattern.channel_at(x, y);

        match sensor {
            BayerChannel::R | BayerChannel::B => {
                let current = sensor_color(sensor);
                let missing = BLUE - current;
                let mut rgb = [T::zero(); 3];
                rgb[current] = self.raw_at(x, y);
                rgb[GREEN] = self.green_at(x, y);
                rgb[missing] = self.opposite_at_non_green(x, y, current);
                rgb
            }
            BayerChannel::GRedRow | BayerChannel::GBlueRow => {
                let horizontal = sensor_color(self.pattern.channel_at(x + 1, y));
                let mut rgb = [T::zero(); 3];
                rgb[GREEN] = self.raw_at(x, y);
                rgb[horizontal] = self.opposite_at_green(x, y, horizontal);
                rgb[BLUE - horizontal] = self.opposite_at_green(x, y, BLUE - horizontal);
                rgb
            }
        }
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
        non_negative(
            (guess / num!(4))
                .max(first.min(second))
                .min(first.max(second)),
        )
    }

    fn opposite_at_green(&self, x: u32, y: u32, color: usize) -> T {
        if x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height {
            return self.border_channel(x, y, color);
        }

        let horizontal = sensor_color(self.pattern.channel_at(x + 1, y));
        let (first, second, green) = if color == horizontal {
            (
                self.raw_at(x - 1, y),
                self.raw_at(x + 1, y),
                [
                    self.green_at(x - 1, y),
                    self.green_at(x, y),
                    self.green_at(x + 1, y),
                ],
            )
        } else {
            (
                self.raw_at(x, y - 1),
                self.raw_at(x, y + 1),
                [
                    self.green_at(x, y - 1),
                    self.green_at(x, y),
                    self.green_at(x, y + 1),
                ],
            )
        };
        non_negative((first + second + green[1] * num!(2) - green[0] - green[2]) / num!(2))
    }

    fn opposite_at_non_green(&self, x: u32, y: u32, current: usize) -> T {
        let missing = BLUE - current;
        if x == 0 || y == 0 || x + 1 == self.width || y + 1 == self.height {
            return self.border_channel(x, y, missing);
        }

        let center = self.green_at(x, y);
        let ne = self.raw_at(x + 1, y - 1);
        let sw = self.raw_at(x - 1, y + 1);
        let nw = self.raw_at(x - 1, y - 1);
        let se = self.raw_at(x + 1, y + 1);
        let g_ne = self.green_at(x + 1, y - 1);
        let g_sw = self.green_at(x - 1, y + 1);
        let g_nw = self.green_at(x - 1, y - 1);
        let g_se = self.green_at(x + 1, y + 1);
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
        non_negative(guess / num!(2))
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
        non_negative(sum / num!(count))
    }

    fn in_three_pixel_border(&self, x: u32, y: u32) -> bool {
        x < 3 || y < 3 || x + 3 >= self.width || y + 3 >= self.height
    }

    fn raw_at(&self, x: u32, y: u32) -> T {
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
fn non_negative<T: Primitive>(value: T) -> T { value.max(T::zero()) }

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicPpg;

impl DemosaicPpg {
    pub fn lazy<'a, T>(
        &self,
        bayer: &'a MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<PpgLazy<'a, T>, DemosaicError>
    where
        T: Primitive + Send + Sync,
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
    use fornax_core::MonoImage;

    use super::*;

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
}
