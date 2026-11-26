//! Variable-number-of-gradients (VNG and VNG4) Bayer demosaicing.
//!
//! Both selectors share one parameterized linear-light kernel:
//! - [`Demosaic::Vng`](super::Demosaic::Vng) runs 3-color VNG with a unified
//!   green channel, preserving all measured `R`, `G`, and `B` CFA samples
//!   bit-exactly.
//! - [`Demosaic::Vng4`](super::Demosaic::Vng4) runs 4-color split-green VNG4
//!   with separate `G1`/`G2` channels averaged on output as `(G1 + G2) / 2` to
//!   suppress green-imbalance maze artifacts.
//!
//! See the [VNG and VNG4 reference notes](https://github.com/Glatzel/fornax/blob/main/docs/reference/demosaic/vng.md).

use generic_num::num;

use super::LazyDemosaic;
use crate::error::DemosaicError;
use crate::{BayerChannel, BayerPattern, MonoImage, RawFloat};

const TERMS: [[i16; 6]; 64] = [
    [-2, -2, 0, -1, 1, 1],
    [-2, -2, 0, 0, 2, 1],
    [-2, -1, -1, 0, 1, 1],
    [-2, -1, 0, -1, 1, 2],
    [-2, -1, 0, 0, 1, 3],
    [-2, -1, 0, 1, 2, 1],
    [-2, 0, 0, -1, 1, 6],
    [-2, 0, 0, 0, 2, 2],
    [-2, 0, 0, 1, 1, 3],
    [-2, 1, -1, 0, 1, 4],
    [-2, 1, 0, -1, 2, 4],
    [-2, 1, 0, 0, 1, 6],
    [-2, 1, 0, 1, 1, 2],
    [-2, 2, 0, 0, 2, 4],
    [-2, 2, 0, 1, 1, 4],
    [-1, -2, -1, 0, 1, 128],
    [-1, -2, 0, -1, 1, 1],
    [-1, -2, 1, -1, 1, 1],
    [-1, -2, 1, 0, 2, 1],
    [-1, -1, -1, 1, 1, 136],
    [-1, -1, 1, -2, 1, 64],
    [-1, -1, 1, -1, 1, 34],
    [-1, -1, 1, 0, 1, 51],
    [-1, -1, 1, 1, 2, 17],
    [-1, 0, -1, 2, 1, 8],
    [-1, 0, 0, -1, 1, 68],
    [-1, 0, 0, 1, 1, 17],
    [-1, 0, 1, -2, 2, 64],
    [-1, 0, 1, -1, 1, 102],
    [-1, 0, 1, 0, 2, 34],
    [-1, 0, 1, 1, 1, 51],
    [-1, 0, 1, 2, 2, 16],
    [-1, 1, 1, -1, 2, 68],
    [-1, 1, 1, 0, 1, 102],
    [-1, 1, 1, 1, 1, 34],
    [-1, 1, 1, 2, 1, 16],
    [-1, 2, 0, 1, 1, 4],
    [-1, 2, 1, 0, 2, 4],
    [-1, 2, 1, 1, 1, 4],
    [0, -2, 0, 0, 2, 128],
    [0, -1, 0, 1, 2, 136],
    [0, -1, 1, -2, 1, 64],
    [0, -1, 1, 0, 1, 17],
    [0, -1, 2, -2, 1, 64],
    [0, -1, 2, -1, 1, 32],
    [0, -1, 2, 0, 1, 48],
    [0, -1, 2, 1, 2, 16],
    [0, 0, 0, 2, 2, 8],
    [0, 0, 2, -2, 2, 64],
    [0, 0, 2, -1, 1, 96],
    [0, 0, 2, 0, 2, 32],
    [0, 0, 2, 1, 1, 48],
    [0, 0, 2, 2, 2, 16],
    [0, 1, 1, 0, 1, 68],
    [0, 1, 1, 2, 1, 16],
    [0, 1, 2, -1, 2, 64],
    [0, 1, 2, 0, 1, 96],
    [0, 1, 2, 1, 1, 32],
    [0, 1, 2, 2, 1, 16],
    [1, -2, 1, 0, 1, 128],
    [1, -1, 1, 1, 1, 136],
    [1, 0, 1, 2, 1, 8],
    [1, 0, 2, -1, 1, 64],
    [1, 0, 2, 1, 1, 16],
];

const NEIGHBORS: [(i32, i32); 8] = [
    (-1, -1),
    (-1, 0),
    (-1, 1),
    (0, 1),
    (1, 1),
    (1, 0),
    (1, -1),
    (0, -1),
];

#[derive(Debug, Clone)]
struct PhaseCode {
    terms: Vec<CodeTerm>,
    neighbors: [NeighborStep; 8],
}

#[derive(Debug, Clone, Copy)]
struct CodeTerm {
    dy1: i8,
    dx1: i8,
    dy2: i8,
    dx2: i8,
    channel: u8,
    double_weight: bool,
    grad_mask: u8,
}

#[derive(Debug, Clone, Copy)]
struct NeighborStep {
    dy: i8,
    dx: i8,
    alt_dy: i8,
    alt_dx: i8,
    has_alt: bool,
}

/// VNG / VNG4 Bayer demosaicer.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicVng;

impl DemosaicVng {
    /// Creates a streaming lazy VNG (3-color) or VNG4 (4-color split-green)
    /// demosaicer.
    pub fn lazy<'a, T>(
        &self,
        bayer: &'a MonoImage<T>,
        pattern: BayerPattern,
        split_green: bool,
    ) -> Result<VngLazy<'a, T>, DemosaicError>
    where
        T: RawFloat + Send + Sync,
    {
        if bayer.width() < 2 || bayer.height() < 2 {
            return Err(DemosaicError::InvalidInput(
                "VNG requires an image at least 2x2".into(),
            ));
        }
        Ok(VngLazy {
            raw: bayer.as_raw(),
            width: bayer.width(),
            height: bayer.height(),
            pattern,
            split_green,
            phases: build_phases(pattern, split_green),
        })
    }
}

/// Streaming row-strip VNG / VNG4 demosaicer.
pub struct VngLazy<'a, T> {
    raw: &'a [T],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    split_green: bool,
    phases: [PhaseCode; 4],
}

impl<T: RawFloat + Send + Sync> LazyDemosaic<T> for VngLazy<'_, T> {
    fn dimensions(&self) -> (u32, u32) { (self.width, self.height) }

    fn fill_rows(&self, y0: usize, out: &mut [T]) {
        let width = self.width as usize;
        let height = self.height as usize;
        let row_stride = width * 3;
        let chunk_rows = out.len() / row_stride;
        if chunk_rows == 0 {
            return;
        }

        if self.split_green {
            self.fill_rows_typed::<4>(y0, chunk_rows, width, height, out);
        } else {
            self.fill_rows_typed::<3>(y0, chunk_rows, width, height, out);
        }
    }
}

impl<T: RawFloat + Send + Sync> VngLazy<'_, T> {
    fn fill_rows_typed<const CHANNELS: usize>(
        &self,
        y0: usize,
        chunk_rows: usize,
        width: usize,
        height: usize,
        out: &mut [T],
    ) {
        let split_green = CHANNELS == 4;
        let pw = width + 4;
        let ph = chunk_rows + 4;
        let mut strip = vec![[T::zero(); 4]; ph * pw];

        let rx = (0..pw + 2)
            .map(|i| reflect_index(i as isize - 3, width))
            .collect::<Vec<_>>();
        let half = num!(0.5);
        let quarter = num!(0.25);

        for sy in 0..ph {
            let gy = y0 as isize + sy as isize - 2;
            let ry_m1 = reflect_index(gy - 1, height) * width;
            let ry_0 = reflect_index(gy, height) * width;
            let ry_p1 = reflect_index(gy + 1, height) * width;
            let row_base = sy * pw;

            for sx in 0..pw {
                let gx = sx as isize - 2;
                let rx_m1 = rx[sx];
                let rx_0 = rx[sx + 1];
                let rx_p1 = rx[sx + 2];
                let px = &mut strip[row_base + sx];

                if split_green {
                    let c0 = color_at(self.pattern, gx as i32, gy as i32, true);
                    let ch = color_at(self.pattern, gx as i32 + 1, gy as i32, true);
                    let cv = color_at(self.pattern, gx as i32, gy as i32 + 1, true);
                    let cd = color_at(self.pattern, gx as i32 + 1, gy as i32 + 1, true);

                    px[c0] = self.raw[ry_0 + rx_0];
                    px[ch] = (self.raw[ry_0 + rx_m1] + self.raw[ry_0 + rx_p1]) * half;
                    px[cv] = (self.raw[ry_m1 + rx_0] + self.raw[ry_p1 + rx_0]) * half;
                    px[cd] = (self.raw[ry_m1 + rx_m1]
                        + self.raw[ry_m1 + rx_p1]
                        + self.raw[ry_p1 + rx_m1]
                        + self.raw[ry_p1 + rx_p1])
                        * quarter;
                } else {
                    let c0 = color_at(self.pattern, gx as i32, gy as i32, false);
                    if c0 == 1 {
                        let ch = color_at(self.pattern, gx as i32 + 1, gy as i32, false);
                        let cv = 2 - ch;
                        px[1] = self.raw[ry_0 + rx_0];
                        px[ch] = (self.raw[ry_0 + rx_m1] + self.raw[ry_0 + rx_p1]) * half;
                        px[cv] = (self.raw[ry_m1 + rx_0] + self.raw[ry_p1 + rx_0]) * half;
                    } else {
                        let opp = 2 - c0;
                        px[c0] = self.raw[ry_0 + rx_0];
                        px[1] = (self.raw[ry_m1 + rx_0]
                            + self.raw[ry_p1 + rx_0]
                            + self.raw[ry_0 + rx_m1]
                            + self.raw[ry_0 + rx_p1])
                            * quarter;
                        px[opp] = (self.raw[ry_m1 + rx_m1]
                            + self.raw[ry_m1 + rx_p1]
                            + self.raw[ry_p1 + rx_m1]
                            + self.raw[ry_p1 + rx_p1])
                            * quarter;
                    }
                }
            }
        }

        let row_stride = width * 3;
        for (dy, out_row) in out.chunks_exact_mut(row_stride).enumerate() {
            let gy = y0 + dy;
            let sy = dy + 2;
            let gy_parity = (gy & 1) * 2;
            for (x, pixel) in out_row.as_chunks_mut::<3>().0.iter_mut().enumerate() {
                let sx = x + 2;
                let phase = &self.phases[gy_parity + (x & 1)];
                let center_idx = sy * pw + sx;

                let mut gradients = [T::zero(); 8];
                for term in &phase.terms {
                    let i1 = ((sy as isize + term.dy1 as isize) as usize) * pw
                        + ((sx as isize + term.dx1 as isize) as usize);
                    let i2 = ((sy as isize + term.dy2 as isize) as usize) * pw
                        + ((sx as isize + term.dx2 as isize) as usize);
                    let ch = term.channel as usize;
                    let mut diff = (strip[i1][ch] - strip[i2][ch]).abs();
                    if term.double_weight {
                        diff = diff + diff;
                    }
                    let mut mask = term.grad_mask;
                    while mask != 0 {
                        let g = mask.trailing_zeros() as usize;
                        mask &= mask - 1;
                        gradients[g] = gradients[g] + diff;
                    }
                }

                let mut minimum = gradients[0];
                let mut maximum = gradients[0];
                for &g in &gradients[1..] {
                    if g < minimum {
                        minimum = g;
                    }
                    if g > maximum {
                        maximum = g;
                    }
                }

                if maximum == T::zero() {
                    let c = &strip[center_idx];
                    pixel[0] = c[0];
                    pixel[1] = if split_green {
                        (c[1] + c[3]) * half
                    } else {
                        c[1]
                    };
                    pixel[2] = c[2];
                    continue;
                }

                let threshold = minimum + maximum * half;
                let color = color_at(self.pattern, x as i32, gy as i32, split_green);
                let mut sums = [T::zero(); 4];
                let mut count = 0_usize;

                for (index, step) in phase.neighbors.iter().enumerate() {
                    if gradients[index] <= threshold {
                        let n_idx = ((sy as isize + step.dy as isize) as usize) * pw
                            + ((sx as isize + step.dx as isize) as usize);
                        for (c, sum) in sums.iter_mut().take(CHANNELS).enumerate() {
                            *sum = *sum
                                + if c == color && step.has_alt {
                                    let alt_idx = ((sy as isize + step.alt_dy as isize) as usize)
                                        * pw
                                        + ((sx as isize + step.alt_dx as isize) as usize);
                                    (strip[center_idx][c] + strip[alt_idx][c]) * half
                                } else {
                                    strip[n_idx][c]
                                };
                        }
                        count += 1;
                    }
                }

                let own = strip[center_idx][color];
                let inv_count = T::one() / num!(count);
                let mut out_c = [T::zero(); 4];
                for c in 0..CHANNELS {
                    out_c[c] = if c == color {
                        own
                    } else {
                        own + (sums[c] - sums[color]) * inv_count
                    };
                }

                pixel[0] = out_c[0];
                pixel[1] = if split_green {
                    (out_c[1] + out_c[3]) * half
                } else {
                    out_c[1]
                };
                pixel[2] = out_c[2];
            }
        }
    }
}

const fn reflect_index(index: isize, length: usize) -> usize {
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

fn build_phases(pattern: BayerPattern, split_green: bool) -> [PhaseCode; 4] {
    std::array::from_fn(|idx| {
        let row = (idx / 2) as i32;
        let col = (idx % 2) as i32;
        let terms = TERMS
            .iter()
            .filter_map(|&[y1, x1, y2, x2, weight, gradients]| {
                let color = color_at(pattern, col + x1 as i32, row + y1 as i32, split_green);
                if color_at(pattern, col + x2 as i32, row + y2 as i32, split_green) != color {
                    return None;
                }
                let diagonal = if color_at(pattern, col + 1, row, split_green) == color
                    && color_at(pattern, col, row + 1, split_green) == color
                {
                    2
                } else {
                    1
                };
                if (y1 - y2).abs() == diagonal && (x1 - x2).abs() == diagonal {
                    return None;
                }
                Some(CodeTerm {
                    dy1: y1 as i8,
                    dx1: x1 as i8,
                    dy2: y2 as i8,
                    dx2: x2 as i8,
                    channel: color as u8,
                    double_weight: weight == 2,
                    grad_mask: gradients as u8,
                })
            })
            .collect();

        let phase = color_at(pattern, col, row, split_green);
        let neighbors = NEIGHBORS.map(|(y, x)| {
            let has_alt = color_at(pattern, col + x, row + y, split_green) != phase
                && color_at(pattern, col + x * 2, row + y * 2, split_green) == phase;
            NeighborStep {
                dy: y as i8,
                dx: x as i8,
                alt_dy: (y * 2) as i8,
                alt_dx: (x * 2) as i8,
                has_alt,
            }
        });
        PhaseCode { terms, neighbors }
    })
}

const fn color_at(pattern: BayerPattern, col: i32, row: i32, split_green: bool) -> usize {
    match pattern.channel_at(col.rem_euclid(2) as u32, row.rem_euclid(2) as u32) {
        BayerChannel::R => 0,
        BayerChannel::GRedRow => 1,
        BayerChannel::B => 2,
        BayerChannel::GBlueRow => {
            if split_green {
                3
            } else {
                1
            }
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::BayerPattern;

    fn seeded_mosaic<T>(width: usize, height: usize, seed: u64) -> Vec<T>
    where
        T: RawFloat,
    {
        let mut state = seed;
        (0..width * height)
            .map(|_| {
                state = state.wrapping_mul(6364136223846793005).wrapping_add(1);
                let unit = ((state >> 32) as f64) / f64::from(u32::MAX);
                num!(unit.mul_add(2.0, -0.5))
            })
            .collect()
    }

    fn flip_h(samples: &[f32], width: usize, height: usize) -> Vec<f32> {
        let mut out = vec![0.0_f32; width * height];
        for y in 0..height {
            for x in 0..width {
                out[y * width + (width - 1 - x)] = samples[y * width + x];
            }
        }
        out
    }

    fn flip_v(samples: &[f32], width: usize, height: usize) -> Vec<f32> {
        let mut out = vec![0.0_f32; width * height];
        for y in 0..height {
            for x in 0..width {
                out[(height - 1 - y) * width + x] = samples[y * width + x];
            }
        }
        out
    }

    fn flip_pattern_h(pattern: BayerPattern) -> BayerPattern {
        match pattern {
            BayerPattern::RGGB => BayerPattern::GRBG,
            BayerPattern::GRBG => BayerPattern::RGGB,
            BayerPattern::GBRG => BayerPattern::BGGR,
            BayerPattern::BGGR => BayerPattern::GBRG,
        }
    }

    fn flip_pattern_v(pattern: BayerPattern) -> BayerPattern {
        match pattern {
            BayerPattern::RGGB => BayerPattern::GBRG,
            BayerPattern::GBRG => BayerPattern::RGGB,
            BayerPattern::GRBG => BayerPattern::BGGR,
            BayerPattern::BGGR => BayerPattern::GRBG,
        }
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn fixed_16x16_rggb_vng_and_vng4_match_snapshots() {
        let width = 16_usize;
        let height = 16_usize;
        let samples = seeded_mosaic::<f32>(width, height, 0x564E_4733_4334_5F31);
        let mosaic = MonoImage::from_raw(width as u32, height as u32, samples).unwrap();

        let vng = DemosaicVng
            .lazy(&mosaic, BayerPattern::RGGB, false)
            .unwrap()
            .collect();
        insta::assert_snapshot!(
            "vng_3color_16x16_rggb_matches_snapshot",
            super::super::test_support::format_rgb_3dp(&vng)
        );

        let vng4 = DemosaicVng
            .lazy(&mosaic, BayerPattern::RGGB, true)
            .unwrap()
            .collect();
        insta::assert_snapshot!(
            "vng4_split_green_16x16_rggb_matches_snapshot",
            super::super::test_support::format_rgb_3dp(&vng4)
        );
    }

    #[test]
    fn phase_plans_match_3color_and_4color_counts() {
        for pattern in BayerPattern::ALL {
            let p4 = build_phases(pattern, true);
            let terms4: usize = p4.iter().map(|p| p.terms.len()).sum::<usize>() * 4;
            let alt4: usize = p4
                .iter()
                .flat_map(|p| p.neighbors)
                .filter(|n| n.has_alt)
                .count()
                * 4;
            assert_eq!(terms4, 512, "{pattern} 4-color term count");
            assert_eq!(alt4, 128, "{pattern} 4-color alternate count");

            let p3 = build_phases(pattern, false);
            let terms3: usize = p3.iter().map(|p| p.terms.len()).sum::<usize>() * 4;
            let alt3: usize = p3
                .iter()
                .flat_map(|p| p.neighbors)
                .filter(|n| n.has_alt)
                .count()
                * 4;
            assert_eq!(terms3, 544, "{pattern} 3-color term count");
            assert_eq!(alt3, 96, "{pattern} 3-color alternate count");
        }
    }

    #[test]
    fn vng_and_vng4_differ_on_unbalanced_green_and_preserve_measured_samples() {
        let width = 16_u32;
        let height = 16_u32;
        let pattern = BayerPattern::GBRG;
        let mut samples = vec![0.0_f32; (width * height) as usize];
        for y in 0..height {
            for x in 0..width {
                samples[(y * width + x) as usize] = match pattern.channel_at(x, y) {
                    BayerChannel::R => 0.25,
                    BayerChannel::GRedRow => 1.0,
                    BayerChannel::GBlueRow => 3.0,
                    BayerChannel::B => 0.75,
                };
            }
        }
        let mosaic = MonoImage::from_raw(width, height, samples).unwrap();

        let vng = DemosaicVng.lazy(&mosaic, pattern, false).unwrap().collect();
        let vng4 = DemosaicVng.lazy(&mosaic, pattern, true).unwrap().collect();

        // 4-color VNG4 averages G1 and G2 cleanly to 2.0 everywhere.
        for y in 0..height {
            for x in 0..width {
                let p4 = vng4.get_pixel(x, y).0;
                assert!((p4[0] - 0.25).abs() < 1e-6);
                assert!((p4[1] - 2.0).abs() < 1e-6);
                assert!((p4[2] - 0.75).abs() < 1e-6);
            }
        }

        // 3-color VNG preserves measured G1 (1.0) and G2 (3.0) bit-exactly.
        let mut saw_difference = false;
        for y in 0..height {
            for x in 0..width {
                let p3 = vng.get_pixel(x, y).0;
                let raw = mosaic.get_pixel(x, y)[0];
                match pattern.channel_at(x, y) {
                    BayerChannel::R => assert_eq!(p3[0].to_bits(), raw.to_bits()),
                    BayerChannel::GRedRow | BayerChannel::GBlueRow => {
                        assert_eq!(p3[1].to_bits(), raw.to_bits());
                        saw_difference = true;
                    }
                    BayerChannel::B => assert_eq!(p3[2].to_bits(), raw.to_bits()),
                }
            }
        }
        assert!(saw_difference);
        assert_ne!(vng, vng4);
    }

    #[test]
    fn uniform_and_colored_constants_are_preserved_everywhere() {
        let width = 16_u32;
        let height = 16_u32;
        for split_green in [false, true] {
            for pattern in BayerPattern::ALL {
                for &(r, g, b) in &[
                    (0.25_f32, 0.25, 0.25),
                    (0.0, 0.0, 0.0),
                    (-0.75, -0.75, -0.75),
                    (16.0, 16.0, 16.0),
                    (-0.5, 1.25, 4.5),
                ] {
                    let samples = (0..width * height)
                        .map(|idx| {
                            let x = idx % width;
                            let y = idx / width;
                            match pattern.channel_at(x, y) {
                                BayerChannel::R => r,
                                BayerChannel::GRedRow | BayerChannel::GBlueRow => g,
                                BayerChannel::B => b,
                            }
                        })
                        .collect::<Vec<_>>();
                    let mosaic = MonoImage::from_raw(width, height, samples).unwrap();
                    let out = DemosaicVng
                        .lazy(&mosaic, pattern, split_green)
                        .unwrap()
                        .collect();
                    for y in 0..height {
                        for x in 0..width {
                            let [ro, go, bo] = out.get_pixel(x, y).0;
                            assert!(
                                (ro - r).abs() < 1e-5
                                    && (go - g).abs() < 1e-5
                                    && (bo - b).abs() < 1e-5,
                                "split={split_green} {pattern} at ({x},{y}): got [{ro}, {go}, {bo}] expected [{r}, {g}, {b}]"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn signed_zero_crossing_ramp_is_exact() {
        let width = 16_u32;
        let height = 16_u32;
        let value =
            |x: u32, y: u32| -> f32 { (x as f32).mul_add(0.03, (y as f32).mul_add(0.02, -0.35)) };
        let samples = (0..width * height)
            .map(|i| value(i % width, i / width))
            .collect::<Vec<_>>();
        let mosaic = MonoImage::from_raw(width, height, samples).unwrap();
        for split_green in [false, true] {
            for pattern in BayerPattern::ALL {
                let out = DemosaicVng
                    .lazy(&mosaic, pattern, split_green)
                    .unwrap()
                    .collect();
                for y in 2..height - 2 {
                    for x in 2..width - 2 {
                        let expected = value(x, y);
                        for (ch, &actual) in out.get_pixel(x, y).0.iter().enumerate() {
                            assert!(
                                (actual - expected).abs() < 1e-5,
                                "split={split_green} {pattern} ch={ch} at ({x},{y}): {actual} vs {expected}"
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn positive_scale_shift_and_sign_invariance_hold_in_f32_and_f64() {
        let width = 16_u32;
        let height = 16_u32;
        let base_f32 = seeded_mosaic::<f32>(width as usize, height as usize, 0xABCD_1234);
        let base_f64 = seeded_mosaic::<f64>(width as usize, height as usize, 0xABCD_1234);

        for split_green in [false, true] {
            let m32 = MonoImage::from_raw(width, height, base_f32.clone()).unwrap();
            let o32 = DemosaicVng
                .lazy(&m32, BayerPattern::RGGB, split_green)
                .unwrap()
                .collect();

            for &scale in &[1.0e-12_f32, 1.0e-4, 1.0e4, 1.0e12] {
                let scaled = base_f32.iter().map(|&v| v * scale).collect();
                let ms = MonoImage::from_raw(width, height, scaled).unwrap();
                let os = DemosaicVng
                    .lazy(&ms, BayerPattern::RGGB, split_green)
                    .unwrap()
                    .collect();
                for (a, b) in os.as_raw().iter().zip(o32.as_raw().iter()) {
                    let expected = *b * scale;
                    let tol = 1.0e-5 * expected.abs().max(scale);
                    assert!(
                        (*a - expected).abs() <= tol,
                        "f32 scale {scale}: {a} vs {expected}"
                    );
                }
            }

            let negated = base_f32.iter().map(|&v| -v).collect();
            let mn = MonoImage::from_raw(width, height, negated).unwrap();
            let on = DemosaicVng
                .lazy(&mn, BayerPattern::RGGB, split_green)
                .unwrap()
                .collect();
            for (a, b) in on.as_raw().iter().zip(o32.as_raw().iter()) {
                assert!((*a + *b).abs() <= 1.0e-5, "sign symmetry: {a} vs -{b}");
            }

            let shift = 1.5_f32;
            let shifted = base_f32.iter().map(|&v| v + shift).collect();
            let msh = MonoImage::from_raw(width, height, shifted).unwrap();
            let osh = DemosaicVng
                .lazy(&msh, BayerPattern::RGGB, split_green)
                .unwrap()
                .collect();
            for (a, b) in osh.as_raw().iter().zip(o32.as_raw().iter()) {
                let expected = *b + shift;
                assert!((*a - expected).abs() <= 2.0e-5, "shift: {a} vs {expected}");
            }

            let m64 = MonoImage::from_raw(width, height, base_f64.clone()).unwrap();
            let o64 = DemosaicVng
                .lazy(&m64, BayerPattern::GBRG, split_green)
                .unwrap()
                .collect();
            let scale64 = 1.0e12_f64;
            let ms64 = MonoImage::from_raw(
                width,
                height,
                base_f64.iter().map(|&v| v * scale64).collect(),
            )
            .unwrap();
            let os64 = DemosaicVng
                .lazy(&ms64, BayerPattern::GBRG, split_green)
                .unwrap()
                .collect();
            for (a, b) in os64.as_raw().iter().zip(o64.as_raw().iter()) {
                let expected = *b * scale64;
                let tol = 1.0e-11 * expected.abs().max(scale64);
                assert!((*a - expected).abs() <= tol, "f64 scale: {a} vs {expected}");
            }
        }
    }

    #[test]
    fn horizontal_and_vertical_flip_symmetry_hold() {
        let width = 20_usize;
        let height = 18_usize;
        let base = seeded_mosaic::<f32>(width, height, 0x464C_4950_5F56);
        let h_in = flip_h(&base, width, height);
        let v_in = flip_v(&base, width, height);
        let m_base = MonoImage::from_raw(width as u32, height as u32, base).unwrap();
        let m_h = MonoImage::from_raw(width as u32, height as u32, h_in).unwrap();
        let m_v = MonoImage::from_raw(width as u32, height as u32, v_in).unwrap();

        for split_green in [false, true] {
            for pattern in BayerPattern::ALL {
                let o_base = DemosaicVng
                    .lazy(&m_base, pattern, split_green)
                    .unwrap()
                    .collect();
                let o_h = DemosaicVng
                    .lazy(&m_h, flip_pattern_h(pattern), split_green)
                    .unwrap()
                    .collect();
                let o_v = DemosaicVng
                    .lazy(&m_v, flip_pattern_v(pattern), split_green)
                    .unwrap()
                    .collect();

                for y in 0..height as u32 {
                    for x in 0..width as u32 {
                        let pb = o_base.get_pixel(x, y).0;
                        let ph = o_h.get_pixel(width as u32 - 1 - x, y).0;
                        let pv = o_v.get_pixel(x, height as u32 - 1 - y).0;
                        for ch in 0..3 {
                            assert!(
                                (pb[ch] - ph[ch]).abs() < 2.0e-4,
                                "H-flip split={split_green} {pattern} ({x},{y}) ch={ch}: {} vs {}",
                                pb[ch],
                                ph[ch]
                            );
                            assert!(
                                (pb[ch] - pv[ch]).abs() < 2.0e-4,
                                "V-flip split={split_green} {pattern} ({x},{y}) ch={ch}: {} vs {}",
                                pb[ch],
                                pv[ch]
                            );
                        }
                    }
                }
            }
        }
    }

    #[test]
    fn tiny_images_and_row_chunking_are_consistent() {
        for split_green in [false, true] {
            for (w, h) in [(2_u32, 2_u32), (3, 3), (5, 7), (15, 11)] {
                let samples = seeded_mosaic::<f32>(w as usize, h as usize, 0x5449_4E59);
                let mosaic = MonoImage::from_raw(w, h, samples).unwrap();
                let lazy = DemosaicVng
                    .lazy(&mosaic, BayerPattern::GRBG, split_green)
                    .unwrap();
                let full = lazy.collect();
                assert!(full.as_raw().iter().all(|v| v.is_finite()));

                let stride = w as usize * 3;
                let mut chunked = vec![0.0_f32; full.as_raw().len()];
                for y0 in (0..h as usize).step_by(2) {
                    let rows = 2.min(h as usize - y0);
                    lazy.fill_rows(y0, &mut chunked[y0 * stride..(y0 + rows) * stride]);
                }
                assert_eq!(full.as_raw(), &chunked);
            }

            for (w, h) in [(1_u32, 8_u32), (8, 1)] {
                let mosaic = MonoImage::from_raw(w, h, vec![0.0_f32; (w * h) as usize]).unwrap();
                assert!(matches!(
                    DemosaicVng.lazy(&mosaic, BayerPattern::GBRG, split_green),
                    Err(DemosaicError::InvalidInput(_))
                ));
            }
        }
    }
}
