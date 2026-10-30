//! Ratio-corrected demosaicing.
//!
//! This independently structured implementation follows the published RCD 2.3
//! formulas by Luis Sanz Rodríguez. darktable 5.6.2 is used as the behavioral
//! reference; no darktable source code is copied into this module.

use fornax_core::error::DemosaicError;
use fornax_core::protocol::LazyDemosaic;
use fornax_core::{BayerPattern, MonoImage, Primitive, RgbImage};
use generic_num::num;

use super::ppg::DemosaicPpg;

const TILE_SIZE: usize = 112;
const TILE_CORE: usize = 92;
const BORDER: usize = 10;
const MARGIN: usize = 9;
const HALF_RANGE: usize = TILE_SIZE / 2;

pub struct RcdLazy<T: Primitive> {
    image: RgbImage<T>,
}

impl<T> LazyDemosaic<T> for RcdLazy<T>
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
pub struct DemosaicRcd;

impl DemosaicRcd {
    pub fn lazy<T>(
        &self,
        bayer: &MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<RcdLazy<T>, DemosaicError>
    where
        T: Primitive + Send + Sync,
    {
        let mut image = DemosaicPpg.lazy(bayer, pattern)?.collect();
        if image.width() >= (2 * BORDER) as u32 && image.height() >= (2 * BORDER) as u32 {
            process_image(
                bayer.as_raw(),
                image.as_mut(),
                bayer.width(),
                bayer.height(),
                pattern,
            );
        }
        Ok(RcdLazy { image })
    }
}

fn process_image<T>(raw: &[T], output: &mut [T], width: u32, height: u32, pattern: BayerPattern)
where
    T: Primitive,
{
    let width = width as usize;
    let height = height as usize;
    let vertical_tiles = 1 + height.saturating_sub(2 * BORDER + 1) / TILE_CORE;
    let horizontal_tiles = 1 + width.saturating_sub(2 * BORDER + 1) / TILE_CORE;

    let tile_area = TILE_SIZE * TILE_SIZE;
    let mut cfa = vec![T::zero(); tile_area];
    let mut vh = vec![T::zero(); tile_area];
    let mut pq = [T::zero(); HALF_RANGE * TILE_SIZE];
    let mut p_hpf = [T::zero(); HALF_RANGE * TILE_SIZE];
    let mut q_hpf = [T::zero(); HALF_RANGE * TILE_SIZE];
    let mut rgb = vec![[T::zero(); 3]; tile_area];
    let mut vertical_hp = [T::zero(); 3 * (TILE_SIZE - 8)];
    let mut horizontal_hp = [T::zero(); TILE_SIZE];

    let eps: T = num!(1.0e-5);
    let epssq: T = num!(1.0e-10);
    let half: T = num!(0.5);
    let quarter: T = num!(0.25);
    let three: T = num!(3);
    let six: T = num!(6);

    for tile_y in 0..vertical_tiles {
        for tile_x in 0..horizontal_tiles {
            let row_start = tile_y * TILE_CORE;
            let col_start = tile_x * TILE_CORE;
            let row_end = (row_start + TILE_SIZE).min(height);
            let col_end = (col_start + TILE_SIZE).min(width);
            let tile_rows = row_end - row_start;
            let tile_cols = col_end - col_start;

            cfa.fill(T::zero());
            vh.fill(T::zero());
            pq.fill(T::zero());
            p_hpf.fill(T::zero());
            q_hpf.fill(T::zero());
            rgb.fill([T::zero(); 3]);
            vertical_hp.fill(T::zero());
            horizontal_hp.fill(T::zero());

            for row in 0..tile_rows {
                let global_y = row_start + row;
                let first = color_index(pattern.channel_at(col_start as u32, global_y as u32));
                let second =
                    color_index(pattern.channel_at((col_start + 1) as u32, global_y as u32));
                for col in 0..tile_cols {
                    let index = row * TILE_SIZE + col;
                    let value = raw[(global_y) * width + col_start + col].max(T::zero());
                    cfa[index] = value;
                    rgb[index][first] = value;
                    rgb[index][second] = value;
                }
            }

            for row in 3..tile_rows.saturating_sub(3).min(5) {
                let base = (row - 3) * (TILE_SIZE - 8);
                for col in 4..tile_cols.saturating_sub(4) {
                    let index = row * TILE_SIZE + col;
                    vertical_hp[base + col - 4] = square(
                        (cfa[index - 3 * TILE_SIZE]
                            - cfa[index - TILE_SIZE]
                            - cfa[index + TILE_SIZE]
                            + cfa[index + 3 * TILE_SIZE])
                            - three * (cfa[index - 2 * TILE_SIZE] + cfa[index + 2 * TILE_SIZE])
                            + six * cfa[index],
                    );
                }
            }

            for row in 4..tile_rows.saturating_sub(4) {
                for col in 3..tile_cols.saturating_sub(3) {
                    let index = row * TILE_SIZE + col;
                    horizontal_hp[col - 3] = square(
                        (cfa[index - 3] - cfa[index - 1] - cfa[index + 1] + cfa[index + 3])
                            - three * (cfa[index - 2] + cfa[index + 2])
                            + six * cfa[index],
                    );
                }
                for col in 4..tile_cols.saturating_sub(4) {
                    let index = (row + 1) * TILE_SIZE + col;
                    let slot = (row + 1 - 3) % 3;
                    vertical_hp[slot * (TILE_SIZE - 8) + col - 4] = square(
                        (cfa[index - 3 * TILE_SIZE]
                            - cfa[index - TILE_SIZE]
                            - cfa[index + TILE_SIZE]
                            + cfa[index + 3 * TILE_SIZE])
                            - three * (cfa[index - 2 * TILE_SIZE] + cfa[index + 2 * TILE_SIZE])
                            + six * cfa[index],
                    );
                }
                let v0 = (row - 4) % 3 * (TILE_SIZE - 8);
                let v1 = (row - 3) % 3 * (TILE_SIZE - 8);
                let v2 = (row - 2) % 3 * (TILE_SIZE - 8);
                for col in 4..tile_cols.saturating_sub(4) {
                    let index = row * TILE_SIZE + col;
                    let vertical = eps.max(
                        vertical_hp[v0 + col - 4]
                            + vertical_hp[v1 + col - 4]
                            + vertical_hp[v2 + col - 4],
                    );
                    let horizontal = epssq.max(
                        horizontal_hp[col - 4] + horizontal_hp[col - 3] + horizontal_hp[col - 2],
                    );
                    vh[index] = vertical / (vertical + horizontal);
                }
            }

            for row in 2..tile_rows.saturating_sub(2) {
                let global_y = row_start + row;
                let start = 2
                    + (pattern
                        .channel_at(col_start as u32, global_y as u32)
                        .index()
                        & 1);
                for col in (start..tile_cols.saturating_sub(2)).step_by(2) {
                    let index = row * TILE_SIZE + col;
                    pq[index / 2] = cfa[index]
                        + half
                            * (cfa[index - TILE_SIZE]
                                + cfa[index + TILE_SIZE]
                                + cfa[index - 1]
                                + cfa[index + 1])
                        + quarter
                            * (cfa[index - TILE_SIZE - 1]
                                + cfa[index - TILE_SIZE + 1]
                                + cfa[index + TILE_SIZE - 1]
                                + cfa[index + TILE_SIZE + 1]);
                }
            }

            for row in 4..tile_rows.saturating_sub(4) {
                let global_y = row_start + row;
                let start = 4
                    + (pattern
                        .channel_at(col_start as u32, global_y as u32)
                        .index()
                        & 1);
                for col in (start..tile_cols.saturating_sub(4)).step_by(2) {
                    let index = row * TILE_SIZE + col;
                    let low = index / 2;
                    let center = cfa[index];
                    let north_gradient = eps
                        + (cfa[index - TILE_SIZE] - cfa[index + TILE_SIZE]).abs()
                        + (center - cfa[index - 2 * TILE_SIZE]).abs()
                        + (cfa[index - TILE_SIZE] - cfa[index - 3 * TILE_SIZE]).abs()
                        + (cfa[index - 2 * TILE_SIZE] - cfa[index - 4 * TILE_SIZE]).abs();
                    let south_gradient = eps
                        + (cfa[index + TILE_SIZE] - cfa[index - TILE_SIZE]).abs()
                        + (center - cfa[index + 2 * TILE_SIZE]).abs()
                        + (cfa[index + TILE_SIZE] - cfa[index + 3 * TILE_SIZE]).abs()
                        + (cfa[index + 2 * TILE_SIZE] - cfa[index + 4 * TILE_SIZE]).abs();
                    let west_gradient = eps
                        + (cfa[index - 1] - cfa[index + 1]).abs()
                        + (center - cfa[index - 2]).abs()
                        + (cfa[index - 1] - cfa[index - 3]).abs()
                        + (cfa[index - 2] - cfa[index - 4]).abs();
                    let east_gradient = eps
                        + (cfa[index + 1] - cfa[index - 1]).abs()
                        + (center - cfa[index + 2]).abs()
                        + (cfa[index + 1] - cfa[index + 3]).abs()
                        + (cfa[index + 2] - cfa[index + 4]).abs();

                    let local_low = pq[low];
                    let north = cfa[index - TILE_SIZE] * (local_low + local_low)
                        / (eps + local_low + pq[low - TILE_SIZE]);
                    let south = cfa[index + TILE_SIZE] * (local_low + local_low)
                        / (eps + local_low + pq[low + TILE_SIZE]);
                    let west =
                        cfa[index - 1] * (local_low + local_low) / (eps + local_low + pq[low - 1]);
                    let east =
                        cfa[index + 1] * (local_low + local_low) / (eps + local_low + pq[low + 1]);

                    let vertical = (south_gradient * north + north_gradient * south)
                        / (north_gradient + south_gradient);
                    let horizontal = (west_gradient * east + east_gradient * west)
                        / (east_gradient + west_gradient);
                    let central = vh[index];
                    let neighbours = quarter
                        * (vh[index - TILE_SIZE - 1]
                            + vh[index - TILE_SIZE + 1]
                            + vh[index + TILE_SIZE - 1]
                            + vh[index + TILE_SIZE + 1]);
                    let discrimination = if (half - central).abs() < (half - neighbours).abs() {
                        neighbours
                    } else {
                        central
                    };
                    rgb[index][1] = interpolate(
                        discrimination.max(T::zero()).min(T::one()),
                        horizontal,
                        vertical,
                    );
                }
            }

            for row in 3..tile_rows.saturating_sub(3) {
                for col in (3..tile_cols.saturating_sub(3)).step_by(2) {
                    let index = row * TILE_SIZE + col;
                    let half_index = index / 2;
                    p_hpf[half_index] = square(
                        (cfa[index - 3 * TILE_SIZE - 3]
                            - cfa[index - TILE_SIZE - 1]
                            - cfa[index + TILE_SIZE + 1]
                            + cfa[index + 3 * TILE_SIZE + 3])
                            - three
                                * (cfa[index - 2 * TILE_SIZE - 2] + cfa[index + 2 * TILE_SIZE + 2])
                            + six * cfa[index],
                    );
                    q_hpf[half_index] = square(
                        (cfa[index - 3 * TILE_SIZE + 3]
                            - cfa[index - TILE_SIZE + 1]
                            - cfa[index + TILE_SIZE - 1]
                            + cfa[index + 3 * TILE_SIZE - 3])
                            - three
                                * (cfa[index - 2 * TILE_SIZE + 2] + cfa[index + 2 * TILE_SIZE - 2])
                            + six * cfa[index],
                    );
                }
            }

            for row in 4..tile_rows.saturating_sub(4) {
                let global_y = row_start + row;
                let start = 4
                    + (pattern
                        .channel_at(col_start as u32, global_y as u32)
                        .index()
                        & 1);
                for col in (start..tile_cols.saturating_sub(4)).step_by(2) {
                    let index = row * TILE_SIZE + col;
                    let half_index = index / 2;
                    let p_start = (index - TILE_SIZE - 1) / 2;
                    let q_start = (index + TILE_SIZE - 1) / 2;
                    let p_stat = epssq.max(p_hpf[p_start] + p_hpf[half_index] + p_hpf[q_start + 1]);
                    let q_stat = epssq.max(q_hpf[p_start + 1] + q_hpf[half_index] + q_hpf[q_start]);
                    pq[half_index] = p_stat / (p_stat + q_stat);
                }
            }

            for row in 4..tile_rows.saturating_sub(4) {
                let global_y = row_start + row;
                let start = 4
                    + (pattern
                        .channel_at(col_start as u32, global_y as u32)
                        .index()
                        & 1);
                for col in (start..tile_cols.saturating_sub(4)).step_by(2) {
                    let index = row * TILE_SIZE + col;
                    let c = 2 - pattern
                        .channel_at((col_start + col) as u32, global_y as u32)
                        .index();
                    let half_index = index / 2;
                    let northwest = index - TILE_SIZE - 1;
                    let northeast = index - TILE_SIZE + 1;
                    let southwest = index + TILE_SIZE - 1;
                    let southeast = index + TILE_SIZE + 1;
                    let pq_central = pq[half_index];
                    let pq_neighbours = quarter
                        * (pq[(index - TILE_SIZE - 1) / 2]
                            + pq[(index - TILE_SIZE - 1) / 2 + 1]
                            + pq[(index + TILE_SIZE - 1) / 2]
                            + pq[(index + TILE_SIZE - 1) / 2 + 1]);
                    let discrimination = if (half - pq_central).abs() < (half - pq_neighbours).abs()
                    {
                        pq_neighbours
                    } else {
                        pq_central
                    };

                    let nw_gradient = eps
                        + (rgb[northwest][c] - rgb[southeast][c]).abs()
                        + (rgb[northwest][c] - rgb[index - 3 * TILE_SIZE - 3][c]).abs()
                        + (rgb[index][1] - rgb[index - 2 * TILE_SIZE - 2][1]).abs();
                    let ne_gradient = eps
                        + (rgb[northeast][c] - rgb[southwest][c]).abs()
                        + (rgb[northeast][c] - rgb[index - 3 * TILE_SIZE + 3][c]).abs()
                        + (rgb[index][1] - rgb[index - 2 * TILE_SIZE + 2][1]).abs();
                    let sw_gradient = eps
                        + (rgb[northeast][c] - rgb[southwest][c]).abs()
                        + (rgb[southwest][c] - rgb[index + 3 * TILE_SIZE - 3][c]).abs()
                        + (rgb[index][1] - rgb[index + 2 * TILE_SIZE - 2][1]).abs();
                    let se_gradient = eps
                        + (rgb[northwest][c] - rgb[southeast][c]).abs()
                        + (rgb[southeast][c] - rgb[index + 3 * TILE_SIZE + 3][c]).abs()
                        + (rgb[index][1] - rgb[index + 2 * TILE_SIZE + 2][1]).abs();
                    let nw_estimate = rgb[northwest][c] - rgb[northwest][1];
                    let ne_estimate = rgb[northeast][c] - rgb[northeast][1];
                    let sw_estimate = rgb[southwest][c] - rgb[southwest][1];
                    let se_estimate = rgb[southeast][c] - rgb[southeast][1];
                    let p_estimate = (nw_gradient * se_estimate + se_gradient * nw_estimate)
                        / (nw_gradient + se_gradient);
                    let q_estimate = (ne_gradient * sw_estimate + sw_gradient * ne_estimate)
                        / (ne_gradient + sw_gradient);
                    rgb[index][c] = rgb[index][1]
                        + interpolate(
                            discrimination.max(T::zero()).min(T::one()),
                            q_estimate,
                            p_estimate,
                        );
                }
            }

            for row in 4..tile_rows.saturating_sub(4) {
                let global_y = row_start + row;
                let start = 4
                    + (pattern
                        .channel_at((col_start + 1) as u32, global_y as u32)
                        .index()
                        & 1);
                for col in (start..tile_cols.saturating_sub(4)).step_by(2) {
                    let index = row * TILE_SIZE + col;
                    let central = vh[index];
                    let neighbours = quarter
                        * (vh[index - TILE_SIZE - 1]
                            + vh[index - TILE_SIZE + 1]
                            + vh[index + TILE_SIZE - 1]
                            + vh[index + TILE_SIZE + 1]);
                    let discrimination = if (half - central).abs() < (half - neighbours).abs() {
                        neighbours
                    } else {
                        central
                    }
                    .max(T::zero())
                    .min(T::one());
                    let green = rgb[index][1];
                    let north = eps + (green - rgb[index - 2 * TILE_SIZE][1]).abs();
                    let south = eps + (green - rgb[index + 2 * TILE_SIZE][1]).abs();
                    let west = eps + (green - rgb[index - 2][1]).abs();
                    let east = eps + (green - rgb[index + 2][1]).abs();
                    for channel in [0, 2] {
                        let north_south = (rgb[index - TILE_SIZE][channel]
                            - rgb[index + TILE_SIZE][channel])
                            .abs();
                        let east_west = (rgb[index - 1][channel] - rgb[index + 1][channel]).abs();
                        let north_gradient = north
                            + north_south
                            + (rgb[index - TILE_SIZE][channel]
                                - rgb[index - 3 * TILE_SIZE][channel])
                                .abs();
                        let south_gradient = south
                            + north_south
                            + (rgb[index + TILE_SIZE][channel]
                                - rgb[index + 3 * TILE_SIZE][channel])
                                .abs();
                        let west_gradient = west
                            + east_west
                            + (rgb[index - 1][channel] - rgb[index - 3][channel]).abs();
                        let east_gradient = east
                            + east_west
                            + (rgb[index + 1][channel] - rgb[index + 3][channel]).abs();
                        let north_estimate =
                            rgb[index - TILE_SIZE][channel] - rgb[index - TILE_SIZE][1];
                        let south_estimate =
                            rgb[index + TILE_SIZE][channel] - rgb[index + TILE_SIZE][1];
                        let west_estimate = rgb[index - 1][channel] - rgb[index - 1][1];
                        let east_estimate = rgb[index + 1][channel] - rgb[index + 1][1];
                        let vertical = (north_gradient * south_estimate
                            + south_gradient * north_estimate)
                            / (north_gradient + south_gradient);
                        let horizontal = (east_gradient * west_estimate
                            + west_gradient * east_estimate)
                            / (east_gradient + west_gradient);
                        rgb[index][channel] =
                            green + interpolate(discrimination, horizontal, vertical);
                    }
                }
            }

            let first_row = row_start + if tile_y == 0 { MARGIN } else { BORDER };
            let last_row = row_end
                - if tile_y + 1 == vertical_tiles {
                    MARGIN
                } else {
                    BORDER
                };
            let first_col = col_start + if tile_x == 0 { MARGIN } else { BORDER };
            let last_col = col_end
                - if tile_x + 1 == horizontal_tiles {
                    MARGIN
                } else {
                    BORDER
                };
            for row in first_row..last_row {
                for col in first_col..last_col {
                    let index = (row - row_start) * TILE_SIZE + col - col_start;
                    let output_index = (row * width + col) * 3;
                    for channel in 0..3 {
                        output[output_index + channel] = rgb[index][channel].max(T::zero());
                    }
                }
            }
        }
    }
}

#[inline(always)]
fn square<T>(value: T) -> T where T: Primitive { value * value }

const fn color_index(channel: fornax_core::BayerChannel) -> usize {
    match channel {
        fornax_core::BayerChannel::R => 0,
        fornax_core::BayerChannel::GRedRow | fornax_core::BayerChannel::GBlueRow => 1,
        fornax_core::BayerChannel::B => 2,
    }
}

#[inline(always)]
fn interpolate<T>(weight: T, first: T, second: T) -> T where T: Primitive {
    weight * (first - second) + second
}

#[cfg(test)]
mod tests {
    use fornax_core::{BayerChannel, BayerPattern};

    use super::*;

    fn color(channel: BayerChannel) -> usize {
        match channel {
            BayerChannel::R => 0,
            BayerChannel::GRedRow | BayerChannel::GBlueRow => 1,
            BayerChannel::B => 2,
        }
    }

    #[test]
    fn uniform_mosaic_has_neutral_rgb_output() -> mischief::Result<()> {
        let image = MonoImage::from_raw(32, 32, vec![0.25_f32; 32 * 32]).unwrap();
        for pattern in BayerPattern::ALL {
            let output = DemosaicRcd.lazy(&image, pattern)?.collect();
            assert!(
                output
                    .as_raw()
                    .iter()
                    .all(|sample| (*sample - 0.25).abs() < 1.0e-5)
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
            let output = DemosaicRcd.lazy(&image, pattern)?.collect();
            for y in 0..image.height() {
                for x in 0..image.width() {
                    let channel = color(pattern.channel_at(x, y));
                    assert_eq!(
                        output.get_pixel(x, y)[channel],
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
        let samples = (0..40 * 40)
            .map(|sample| sample as f32 / (40.0 * 40.0))
            .collect::<Vec<_>>();
        let image = MonoImage::from_raw(40, 40, samples).unwrap();
        let lazy = DemosaicRcd.lazy(&image, BayerPattern::GRBG)?;
        let whole = lazy.collect();

        let mut rows = vec![0.0; 40 * 3 * 40];
        lazy.fill_rows(0, &mut rows);
        assert_eq!(whole.as_raw(), &rows);
        Ok(())
    }

    #[test]
    fn small_images_fall_back_to_ppg() -> mischief::Result<()> {
        let image = MonoImage::from_raw(8, 8, vec![0.5_f32; 64]).unwrap();
        let rcd = DemosaicRcd.lazy(&image, BayerPattern::RGGB)?.collect();
        let ppg = DemosaicPpg.lazy(&image, BayerPattern::RGGB)?.collect();
        assert_eq!(rcd, ppg);
        Ok(())
    }
}
