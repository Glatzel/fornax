use fornax_core::error::DemosaicError;
use fornax_core::{BayerPattern, MonoImage, Primitive};

use super::buffer::DenseLazy;
use super::directional::{self, Profile};

const MARGIN: usize = 4;
const HOT: u8 = 64;
const THOT: f32 = 64.0;

fn calc_dist(c1: f32, c2: f32) -> f32 {
    if c1 > c2 { c1 / c2 } else { c2 / c1 }
}

const fn cfa_channel(pattern: BayerPattern, x: isize, y: isize) -> usize {
    match pattern.channel_at(x.rem_euclid(2) as u32, y.rem_euclid(2) as u32) {
        fornax_core::BayerChannel::R => 0,
        fornax_core::BayerChannel::GRedRow | fornax_core::BayerChannel::GBlueRow => 1,
        fornax_core::BayerChannel::B => 2,
    }
}

struct Dht {
    width: usize,
    height: usize,
    stride: usize,
    pattern: BayerPattern,
    nraw: Vec<[f32; 3]>,
    ndir: Vec<u8>,
    channel_maximum: [u16; 3],
    channel_minimum: [f32; 3],
}

impl Dht {
    fn new(input: &[u16], width: usize, height: usize, pattern: BayerPattern) -> Self {
        let stride = width + 2 * MARGIN;
        let area = stride * (height + 2 * MARGIN);
        let mut nraw = vec![[0.5_f32; 3]; area];
        let mut channel_maximum = [0_u16; 3];
        let first = cfa_channel(pattern, 0, 0);
        let mut channel_minimum = [0.0_f32; 3];
        channel_minimum[first] = f32::from(input[0]);

        for y in 0..height {
            for x in 0..width {
                let channel = cfa_channel(pattern, x as isize, y as isize);
                let value = input[y * width + x];
                if value != 0 {
                    channel_maximum[channel] = channel_maximum[channel].max(value);
                    channel_minimum[channel] = channel_minimum[channel].min(f32::from(value));
                    let index = (y + MARGIN) * stride + x + MARGIN;
                    nraw[index][channel] = f32::from(value);
                }
            }
        }
        for minimum in &mut channel_minimum {
            *minimum += 0.5;
        }

        Self {
            width,
            height,
            stride,
            pattern,
            nraw,
            ndir: vec![0; area],
            channel_maximum,
            channel_minimum,
        }
    }

    const fn offset(&self, x: usize, y: usize) -> usize {
        (y + MARGIN) * self.stride + x + MARGIN
    }

    fn hide_hots(&mut self) {
        for y in 0..self.height {
            let js = cfa_channel(self.pattern, 0, y as isize) & 1;
            let known = cfa_channel(self.pattern, js as isize, y as isize);
            let mut x = js;
            while x < self.width {
                let index = self.offset(x, y);
                let value = self.nraw[index][known];
                let neighbors = [
                    self.nraw[index + 2][known],
                    self.nraw[index - 2][known],
                    self.nraw[index - 2 * self.stride][known],
                    self.nraw[index + 2 * self.stride][known],
                    self.nraw[index + 1][1],
                    self.nraw[index - 1][1],
                    self.nraw[index - self.stride][1],
                    self.nraw[index + self.stride][1],
                ];
                self.hide_hot_sample_at(index, known, known, value, neighbors);
                x += 2;
            }

            let mut x = js ^ 1;
            while x < self.width {
                let index = self.offset(x, y);
                let value = self.nraw[index][1];
                let neighbors = [
                    self.nraw[index + 2][1],
                    self.nraw[index - 2][1],
                    self.nraw[index - 2 * self.stride][1],
                    self.nraw[index + 2 * self.stride][1],
                    self.nraw[index + 1][known],
                    self.nraw[index - 1][known],
                    self.nraw[index - self.stride][known ^ 2],
                    self.nraw[index + self.stride][known ^ 2],
                ];
                self.hide_hot_sample_at(index, known, 1, value, neighbors);
                x += 2;
            }
        }
    }

    fn hide_hot_sample_at(
        &mut self,
        index: usize,
        adjacent_channel: usize,
        channel: usize,
        value: f32,
        neighbors: [f32; 8],
    ) {
        if !(neighbors.iter().all(|neighbor| value > *neighbor)
            || neighbors.iter().all(|neighbor| value < *neighbor))
        {
            return;
        }

        let offsets = [
            -2 * self.stride as isize - 2,
            -2 * self.stride as isize + 2,
            2 * self.stride as isize - 2,
            2 * self.stride as isize + 2,
        ];
        let average = offsets
            .into_iter()
            .map(|offset| self.sample_add(index, offset, channel))
            .sum::<f32>()
            / 8.0;
        if calc_dist(value, average) <= THOT {
            return;
        }

        self.ndir[index] |= HOT;
        let (vertical, horizontal) = if channel == 1 {
            let vertical = calc_dist(
                self.sample_add(index, -2 * self.stride as isize, 1)
                    * self.sample_add(index, -(self.stride as isize), adjacent_channel ^ 2),
                self.sample_add(index, 2 * self.stride as isize, 1)
                    * self.sample_add(index, self.stride as isize, adjacent_channel ^ 2),
            );
            let horizontal = calc_dist(
                self.sample_add(index, -2, 1) * self.sample_add(index, -1, adjacent_channel),
                self.sample_add(index, 2, 1) * self.sample_add(index, 1, adjacent_channel),
            );
            (vertical, horizontal)
        } else {
            let vertical = calc_dist(
                self.sample_add(index, -2 * self.stride as isize, channel)
                    * self.sample_add(index, -(self.stride as isize), 1),
                self.sample_add(index, 2 * self.stride as isize, channel)
                    * self.sample_add(index, self.stride as isize, 1),
            );
            let horizontal = calc_dist(
                self.sample_add(index, -2, channel) * self.sample_add(index, -1, 1),
                self.sample_add(index, 2, channel) * self.sample_add(index, 1, 1),
            );
            (vertical, horizontal)
        };
        self.nraw[index][channel] = if vertical > horizontal {
            f32::midpoint(
                self.sample_add(index, 2, channel),
                self.sample_add(index, -2, channel),
            )
        } else {
            f32::midpoint(
                self.sample_add(index, 2 * self.stride as isize, channel),
                self.sample_add(index, -2 * self.stride as isize, channel),
            )
        };
    }

    fn sample_add(&self, index: usize, offset: isize, channel: usize) -> f32 {
        self.nraw[index.wrapping_add_signed(offset)][channel]
    }

    fn restore_hots(&mut self, input: &[u16]) {
        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.offset(x, y);
                if self.ndir[index] & HOT != 0 {
                    let channel = cfa_channel(self.pattern, x as isize, y as isize);
                    self.nraw[index][channel] = f32::from(input[y * self.width + x]);
                }
            }
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicDht;

impl DemosaicDht {
    pub fn lazy<T>(
        &self,
        image: &MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<DenseLazy<T>, DemosaicError>
    where
        T: Primitive + Send + Sync,
    {
        directional::lazy(image, pattern, Profile::Dht)
    }
}

#[cfg(test)]
mod tests {
    use fornax_core::BayerPattern;

    use super::*;

    const HOT: u8 = 64;

    fn measured_mosaic(width: usize, height: usize, pattern: BayerPattern, value: u16) -> Vec<u16> {
        let mut samples = vec![value; width * height];
        samples[0] = value.saturating_sub(1);
        samples[0] = match cfa_channel(pattern, 0, 0) {
            0 => value,
            1 => value - 1,
            _ => value - 2,
        };
        samples
    }

    #[test]
    fn calc_dist_is_invariant_to_one_stop_scaling() {
        assert_eq!(calc_dist(10.0, 20.0), 2.0);
        assert_eq!(calc_dist(100.0, 200.0), 2.0);
        assert_eq!(calc_dist(200.0, 100.0), 2.0);
    }

    #[test]
    fn new_preserves_measured_samples_in_a_padded_neutral_plane() {
        let pattern = BayerPattern::GBRG;
        let width = 5;
        let height = 5;
        let input = (0..width * height)
            .map(|index| 1_000 + index as u16)
            .collect::<Vec<_>>();
        let dht = Dht::new(&input, width, height, pattern);

        assert_eq!(dht.nraw.len(), (width + 8) * (height + 8));
        assert!(dht.ndir.iter().all(|direction| *direction == 0));
        for y in 0..height {
            for x in 0..width {
                let index = dht.offset(x, y);
                let measured = cfa_channel(pattern, x as isize, y as isize);
                for channel in 0..3 {
                    let expected = if channel == measured {
                        f32::from(input[y * width + x])
                    } else {
                        0.5
                    };
                    assert_eq!(
                        dht.nraw[index][channel], expected,
                        "({x}, {y}) ch {channel}"
                    );
                }
            }
        }
    }

    #[test]
    fn hide_hots_marks_and_repairs_an_isolated_sample() {
        let pattern = BayerPattern::GBRG;
        let width = 9;
        let height = 9;
        let mut input = measured_mosaic(width, height, pattern, 500);
        let hot = 5 * width + 5;
        input[hot] = 60_000;
        let mut dht = Dht::new(&input, width, height, pattern);

        dht.hide_hots();

        let index = dht.offset(5, 5);
        assert_ne!(dht.ndir[index] & HOT, 0);
        assert_eq!(dht.nraw[index][1], 500.0);
        dht.restore_hots(&input);
        assert_eq!(dht.nraw[index][1], 60_000.0);
    }
}
