//! Variable-number-of-gradients Bayer demosaicing.

use fornax_core::error::DemosaicError;
use fornax_core::{BayerChannel, BayerPattern, MonoImage, Primitive};

use super::buffer::{DenseLazy, codes, dense_from_codes};

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
    neighbors: [(i32, i32, i32); 8],
}

#[derive(Debug, Clone)]
struct CodeTerm {
    first: i32,
    second: i32,
    weight: u32,
    gradients: Vec<usize>,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicVng;

impl DemosaicVng {
    pub fn lazy<T>(
        &self,
        bayer: &MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<DenseLazy<T>, DemosaicError>
    where
        T: Primitive + Send + Sync,
    {
        if bayer.width() < 8 || bayer.height() < 8 {
            return Err(DemosaicError::InvalidInput(
                "VNG requires an image at least 8x8".into(),
            ));
        }
        let width = bayer.width() as usize;
        let height = bayer.height() as usize;
        let output = interpolate(&codes(bayer), width, height, pattern);
        Ok(DenseLazy::new(dense_from_codes(
            bayer.width(),
            bayer.height(),
            output,
        )))
    }
}

fn interpolate(input: &[u16], width: usize, height: usize, pattern: BayerPattern) -> Vec<u16> {
    let mut pixels = linear_interpolate(input, width, height, pattern);
    let phases = build_phases(width, pattern);
    let mut rows = [
        vec![[0_u16; 4]; width],
        vec![[0_u16; 4]; width],
        vec![[0_u16; 4]; width],
    ];

    let mut row = 2;
    while row < height - 2 {
        for col in 2..width - 2 {
            let pixel = row * width + col;
            let phase = &phases[row % 8][col % 2];
            let mut gradients = [0_i64; 8];
            for term in &phase.terms {
                let first = sample(&pixels, pixel, term.first) as i64;
                let second = sample(&pixels, pixel, term.second) as i64;
                let difference = (first - second).abs() << term.weight;
                for &gradient in &term.gradients {
                    gradients[gradient] += difference;
                }
            }
            let minimum = gradients.iter().copied().min().unwrap_or(0);
            let maximum = gradients.iter().copied().max().unwrap_or(0);
            if maximum == 0 {
                rows[2][col] = pixels[pixel];
                continue;
            }
            let threshold = minimum + (maximum >> 1);
            let mut sums = [0_i64; 4];
            let color = color_at(pattern, col as i32, row as i32);
            let mut count = 0_i64;
            for (index, (y, x, alternate)) in phase.neighbors.iter().copied().enumerate() {
                if gradients[index] <= threshold {
                    for channel in 0..4 {
                        sums[channel] += if channel == color && alternate != 0 {
                            (pixels[pixel][channel] as i64
                                + sample(&pixels, pixel, alternate) as i64)
                                >> 1
                        } else {
                            sample(&pixels, pixel, (y * width as i32 + x) * 4 + channel as i32)
                                as i64
                        };
                    }
                    count += 1;
                }
            }
            let own = pixels[pixel][color] as i64;
            for channel in 0..4 {
                rows[2][col][channel] = clip(if channel == color {
                    own
                } else {
                    own + (sums[channel] - sums[color]) / count
                });
            }
        }
        if row > 3 {
            pixels[(row - 2) * width + 2..(row - 1) * width - 2]
                .copy_from_slice(&rows[0][2..width - 2]);
        }
        rows.rotate_left(1);
        row += 1;
    }
    pixels[(row - 2) * width + 2..(row - 1) * width - 2].copy_from_slice(&rows[0][2..width - 2]);
    pixels[(row - 1) * width + 2..row * width - 2].copy_from_slice(&rows[1][2..width - 2]);

    let mut rgb = vec![0_u16; width * height * 3];
    for pixel in 0..width * height {
        rgb[pixel * 3] = pixels[pixel][0];
        // darktable keeps G1/G2 separate through VNG and mixes them only here.
        rgb[pixel * 3 + 1] =
            ((u32::from(pixels[pixel][1]) + u32::from(pixels[pixel][3])) / 2) as u16;
        rgb[pixel * 3 + 2] = pixels[pixel][2];
    }
    rgb
}

fn linear_interpolate(
    input: &[u16],
    width: usize,
    height: usize,
    pattern: BayerPattern,
) -> Vec<[u16; 4]> {
    let mut pixels = vec![[0_u16; 4]; width * height];
    for row in 0..height {
        for col in 0..width {
            let pixel = row * width + col;
            let own = color_at(pattern, col as i32, row as i32);
            pixels[pixel][own] = input[pixel];
            for (channel, value) in pixels[pixel].iter_mut().enumerate() {
                if channel == own {
                    continue;
                }
                let mut sum = 0_u32;
                let mut weight = 0_u32;
                for y in -1_i32..=1 {
                    for x in -1_i32..=1 {
                        let ny = row as i32 + y;
                        let nx = col as i32 + x;
                        if nx < 0 || ny < 0 || nx >= width as i32 || ny >= height as i32 {
                            continue;
                        }
                        let ny = ny as usize;
                        let nx = nx as usize;
                        if color_at(pattern, nx as i32, ny as i32) == channel {
                            let neighbor_weight = 1_u32 << (u32::from(y == 0) + u32::from(x == 0));
                            sum += u32::from(input[ny * width + nx]) * neighbor_weight;
                            weight += neighbor_weight;
                        }
                    }
                }
                *value = (sum / weight.max(1)) as u16;
            }
        }
    }
    pixels
}

fn build_phases(width: usize, pattern: BayerPattern) -> Vec<[PhaseCode; 2]> {
    (0..8)
        .map(|row| {
            std::array::from_fn(|col| {
                let terms = TERMS
                    .iter()
                    .filter_map(|term| {
                        let [y1, x1, y2, x2, weight, gradients] = *term;
                        let color = color_at(pattern, col as i32 + x1 as i32, row + y1 as i32);
                        if color_at(pattern, col as i32 + x2 as i32, row + y2 as i32) != color {
                            return None;
                        }
                        let diagonal = if color_at(pattern, col as i32 + 1, row) == color
                            && color_at(pattern, col as i32, row + 1) == color
                        {
                            2
                        } else {
                            1
                        };
                        if (y1 - y2).abs() == diagonal && (x1 - x2).abs() == diagonal {
                            return None;
                        }
                        Some(CodeTerm {
                            first: (y1 as i32 * width as i32 + x1 as i32) * 4 + color as i32,
                            second: (y2 as i32 * width as i32 + x2 as i32) * 4 + color as i32,
                            weight: (weight - 1) as u32,
                            gradients: (0..8)
                                .filter(|gradient| gradients & (1_i16 << gradient) != 0)
                                .collect(),
                        })
                    })
                    .collect();
                let phase = color_at(pattern, col as i32, row);
                let neighbors = NEIGHBORS.map(|(y, x)| {
                    let alternate = if color_at(pattern, col as i32 + x, row + y) != phase
                        && color_at(pattern, col as i32 + x * 2, row + y * 2) == phase
                    {
                        (y * width as i32 + x) * 8 + phase as i32
                    } else {
                        0
                    };
                    (y, x, alternate)
                });
                PhaseCode { terms, neighbors }
            })
        })
        .collect()
}

fn sample(pixels: &[[u16; 4]], pixel: usize, offset: i32) -> u16 {
    let flat = pixel as i64 * 4 + i64::from(offset);
    pixels[(flat / 4) as usize][(flat % 4) as usize]
}

const fn color_at(pattern: BayerPattern, col: i32, row: i32) -> usize {
    match pattern.channel_at(col.rem_euclid(2) as u32, row.rem_euclid(2) as u32) {
        BayerChannel::R => 0,
        BayerChannel::GRedRow => 1,
        BayerChannel::GBlueRow => 3,
        BayerChannel::B => 2,
    }
}

fn clip(value: i64) -> u16 { value.clamp(0, i64::from(u16::MAX)) as u16 }

#[cfg(test)]
mod tests {
    use fornax_core::BayerPattern;

    use super::*;

    #[test]
    fn phase_terms_use_shift_weights() {
        for pattern in BayerPattern::ALL {
            let phases = build_phases(16, pattern);
            for row in &phases {
                for phase in row {
                    assert!(
                        phase.terms.iter().all(|term| term.weight <= 1),
                        "{pattern} phase contains a multiplicative weight"
                    );
                }
            }
        }
    }

    #[test]
    fn phase_plans_keep_split_greens() {
        for pattern in BayerPattern::ALL {
            let phases = build_phases(16, pattern);
            let terms = phases
                .iter()
                .flatten()
                .map(|phase| phase.terms.len())
                .sum::<usize>();
            let alternate_neighbors = phases
                .iter()
                .flatten()
                .flat_map(|phase| phase.neighbors)
                .filter(|(_, _, alternate)| *alternate != 0)
                .count();

            assert_eq!(terms, 512, "{pattern} phase terms differ from darktable");
            assert_eq!(
                alternate_neighbors, 128,
                "{pattern} special green neighbors differ from darktable"
            );
        }
    }

    #[test]
    fn alternate_neighbor_offsets_address_four_channel_pixels() {
        let width = 16;
        for pattern in BayerPattern::ALL {
            let phases = build_phases(width, pattern);
            for (row_index, row) in phases.iter().enumerate() {
                for (col, phase) in row.iter().enumerate() {
                    let phase_color = color_at(pattern, col as i32, row_index as i32) as i32;
                    for (y, x, alternate) in phase.neighbors {
                        if alternate == 0 {
                            continue;
                        }
                        assert_eq!(
                            alternate,
                            (y * width as i32 + x) * 8 + phase_color,
                            "{pattern} neighbor ({y}, {x}) with color {phase_color}"
                        );
                    }
                }
            }
        }
    }

    #[test]
    fn edge_green_interpolation_ignores_clamped_neighbors() {
        let width = 9_usize;
        let height = 9;
        let mut input = vec![0_u16; width * height];
        input[1] = 10_000;
        input[3] = 30_000;
        input[width + 2] = 50_000;
        input[8 * width + 1] = 20_000;
        input[8 * width + 3] = 40_000;
        input[7 * width + 2] = 60_000;
        input[width] = 30_000;
        input[3 * width] = 50_000;
        input[2 * width + 1] = 40_000;
        input[width + 8] = 20_000;
        input[3 * width + 8] = 40_000;
        input[2 * width + 7] = 60_000;

        let pixels = linear_interpolate(&input, width, height, BayerPattern::RGGB);

        assert_eq!(pixels[2][1], 20_000);
        assert_eq!(pixels[8 * width + 2][1], 30_000);
        assert_eq!(pixels[2 * width][1], 40_000);
        assert_eq!(pixels[2 * width + 8][1], 60_000);
    }

    #[test]
    fn final_green_averages_both_green_phases() {
        let width = 12;
        let height = 12;
        let pattern = BayerPattern::GBRG;
        let input = (0..width * height)
            .map(|pixel| {
                let x = (pixel % width) as u32;
                let y = (pixel / width) as u32;
                match pattern.channel_at(x, y) {
                    BayerChannel::R => 1_000,
                    BayerChannel::GRedRow => 10_000,
                    BayerChannel::B => 2_000,
                    BayerChannel::GBlueRow => 30_000,
                }
            })
            .collect::<Vec<_>>();

        let output = interpolate(&input, width, height, pattern);

        for pixel in output.as_chunks::<3>().0 {
            assert_eq!(pixel, &[1_000, 20_000, 2_000]);
        }
    }

    #[test]
    fn uniform_channel_codes_are_reconstructed_exactly() {
        let width = 12;
        let height = 12;
        let pattern = BayerPattern::GBRG;
        let input = (0..width * height)
            .map(|pixel| {
                let x = (pixel % width) as i32;
                let y = (pixel / width) as i32;
                match color_at(pattern, x, y) {
                    0 => 10_000,
                    1 | 3 => 20_000,
                    2 => 30_000,
                    _ => unreachable!(),
                }
            })
            .collect::<Vec<_>>();

        let output = interpolate(&input, width, height, pattern);

        for pixel in output.as_chunks::<3>().0 {
            assert_eq!(pixel, &[10_000, 20_000, 30_000]);
        }
    }
}
