//! Adaptive homogeneity-directed demosaicing compatible with LibRaw AAHD.
//!
//! The integer pipeline, padded working planes, direction flags, hot-pixel
//! handling, and output truncation follow LibRaw 0.22.3
//! `LibRaw::aahd_interpolate` and its private `AAHD` helper.
//! See the [AAHD reference notes](https://github.com/Glatzel/fornax/blob/main/docs/references/aahd.md)
//! for the pinned source and compatibility conventions.

use std::cmp::Ordering;
use std::sync::LazyLock;

use fornax_core::error::DemosaicError;
use fornax_core::{BayerChannel, BayerPattern, MonoImage, Primitive};

use super::buffer::{DenseLazy, codes, dense_from_codes};

const MARGIN: usize = 4;
const HOT: u8 = 8;
const HVSH: u8 = 1;
const HOR: u8 = 2;
const VER: u8 = 4;
const HORSH: u8 = HOR | HVSH;
const VERSH: u8 = VER | HVSH;

static GAMMA_LUT: LazyLock<[u16; 65_536]> = LazyLock::new(|| {
    std::array::from_fn(|index| {
        let value = index as f32 / 65_536.0;
        let encoded = if value < 0.0181 {
            4.5 * value
        } else {
            1.0993_f32.mul_add(value.powf(0.45), -0.0993)
        };
        (65_536.0 * encoded) as u16
    })
});

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct DemosaicAahd;

impl DemosaicAahd {
    pub fn lazy<T>(
        &self,
        image: &MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<DenseLazy<T>, DemosaicError>
    where
        T: Primitive + Send + Sync,
    {
        let width = image.width() as usize;
        let height = image.height() as usize;
        if width < 5 || height < 5 {
            return Err(DemosaicError::InvalidInput(
                "AAHD requires an image at least 5x5".into(),
            ));
        }
        let stride = width
            .checked_add(2 * MARGIN)
            .ok_or_else(|| DemosaicError::InvalidInput("AAHD image width overflow".into()))?;
        if stride.checked_mul(height + 2 * MARGIN).is_none() {
            return Err(DemosaicError::InvalidInput(
                "AAHD image dimensions overflow".into(),
            ));
        }

        let output = Aahd::new(&codes(image), width, height, pattern).process(pattern);
        Ok(DenseLazy::new(dense_from_codes(
            image.width(),
            image.height(),
            output,
        )))
    }
}

struct Aahd {
    width: usize,
    height: usize,
    stride: usize,
    input: Vec<u16>,
    rgb: [Vec<[u16; 3]>; 2],
    yuv: [Vec<[i32; 3]>; 2],
    ndir: Vec<u8>,
    homo: [Vec<i8>; 2],
    maximum: [u16; 3],
    minimum: [u16; 3],
}

impl Aahd {
    fn new(input: &[u16], width: usize, height: usize, pattern: BayerPattern) -> Self {
        let stride = width + 2 * MARGIN;
        let plane_height = height + 2 * MARGIN;
        let area = stride * plane_height;
        let mut rgb = [vec![[0; 3]; area], vec![[0; 3]; area]];
        let mut maximum = [0; 3];
        let first = input[0];
        let mut minimum = [0; 3];
        minimum[color_index(pattern.channel_at(0, 0))] = first;

        for y in 0..height {
            for x in 0..width {
                let channel = color_index(pattern.channel_at(x as u32, y as u32));
                let value = input[y * width + x];
                if value != 0 {
                    maximum[channel] = maximum[channel].max(value);
                    minimum[channel] = minimum[channel].min(value);
                    let index = (y + MARGIN) * stride + x + MARGIN;
                    rgb[0][index][channel] = value;
                    rgb[1][index][channel] = value;
                }
            }
        }

        Self {
            width,
            height,
            stride,
            input: input.to_vec(),
            rgb,
            yuv: [vec![[0; 3]; area], vec![[0; 3]; area]],
            ndir: vec![0; area],
            homo: [vec![0; area], vec![0; area]],
            maximum,
            minimum,
        }
    }

    fn process(mut self, pattern: BayerPattern) -> Vec<u16> {
        self.hide_hots(pattern);
        self.make_greens(pattern);
        self.make_rb(pattern);
        self.evaluate();
        self.refine_hv_dirs();

        let mut output = Vec::with_capacity(self.width * self.height * 3);
        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.offset(x, y);
                if self.ndir[index] & HOT != 0 {
                    let channel = color_index(pattern.channel_at(x as u32, y as u32));
                    let value = self.known_input(x, y);
                    self.rgb[0][index][channel] = value;
                    self.rgb[1][index][channel] = value;
                }
                let candidate = usize::from(self.ndir[index] & VER != 0);
                output.extend_from_slice(&self.rgb[candidate][index]);
            }
        }
        output
    }

    fn hide_hots(&mut self, pattern: BayerPattern) {
        let pe = 1;
        let pw = -1;
        let pn = -(self.stride as isize);
        let ps = self.stride as isize;
        for y in 0..self.height {
            let js = color_index(pattern.channel_at(0, y as u32)) & 1;
            let known = color_index(pattern.channel_at(js as u32, y as u32));
            let mut x = js;
            while x < self.width {
                let index = self.offset(x, y);
                let value = self.rgb[0][index][known];
                let neighbors = [
                    self.rgb[0][add(index, 2 * pe)][known],
                    self.rgb[0][add(index, 2 * pw)][known],
                    self.rgb[0][add(index, 2 * pn)][known],
                    self.rgb[0][add(index, 2 * ps)][known],
                    self.rgb[0][add(index, pe)][1],
                    self.rgb[0][add(index, pw)][1],
                    self.rgb[0][add(index, pn)][1],
                    self.rgb[0][add(index, ps)][1],
                ];
                self.hide_hot_sample(index, known, known, value, &neighbors);
                x += 2;
            }

            let js = js ^ 1;
            let mut x = js;
            while x < self.width {
                let index = self.offset(x, y);
                let value = self.rgb[0][index][1];
                let neighbors = [
                    self.rgb[0][add(index, 2 * pe)][1],
                    self.rgb[0][add(index, 2 * pw)][1],
                    self.rgb[0][add(index, 2 * pn)][1],
                    self.rgb[0][add(index, 2 * ps)][1],
                    self.rgb[0][add(index, pe)][known],
                    self.rgb[0][add(index, pw)][known],
                    self.rgb[0][add(index, pn)][known ^ 2],
                    self.rgb[0][add(index, ps)][known ^ 2],
                ];
                self.hide_hot_sample(index, known, 1, value, &neighbors);
                x += 2;
            }
        }
    }

    fn hide_hot_sample(
        &mut self,
        index: usize,
        adjacent_channel: usize,
        channel: usize,
        value: u16,
        neighbors: &[u16; 8],
    ) {
        if !(neighbors.iter().all(|neighbor| value > *neighbor)
            || neighbors.iter().all(|neighbor| value < *neighbor))
        {
            return;
        }

        let hot = i32::from(value) >> 4;
        let dead = i32::from(value) << 4;
        let pn = -(self.stride as isize);
        let ps = self.stride as isize;
        let average = [
            add(index, 2 * pn - 2),
            add(index, 2 * pn + 2),
            add(index, 2 * ps - 2),
            add(index, 2 * ps + 2),
        ]
        .iter()
        .map(|sample| i32::from(self.rgb[0][*sample][channel]))
        .sum::<i32>()
        .wrapping_div(8);
        if hot <= average && dead >= average {
            return;
        }

        let pe = 1;
        let pw = -1;
        let sample = |offset: isize| i32::from(self.rgb[0][add(index, offset)][channel]);

        let (dh, dv) = if channel == 1 {
            let row_left = i32::from(self.rgb[0][add(index, pw)][adjacent_channel]);
            let row_right = i32::from(self.rgb[0][add(index, pe)][adjacent_channel]);
            let column_up = i32::from(self.rgb[0][add(index, pn)][adjacent_channel ^ 2]);
            let column_down = i32::from(self.rgb[0][add(index, ps)][adjacent_channel ^ 2]);
            (
                (sample(2 * pw) - sample(2 * pe)).abs()
                    + (row_left - row_right).abs()
                    + (row_left - row_right + sample(2 * pe) - sample(2 * pw)).abs(),
                (sample(2 * pn) - sample(2 * ps)).abs()
                    + (column_up - column_down).abs()
                    + (column_up - column_down + sample(2 * ps) - sample(2 * pn)).abs(),
            )
        } else {
            let left_green = i32::from(self.rgb[0][add(index, pw)][1]);
            let right_green = i32::from(self.rgb[0][add(index, pe)][1]);
            let north_green = i32::from(self.rgb[0][add(index, pn)][1]);
            let south_green = i32::from(self.rgb[0][add(index, ps)][1]);
            (
                (sample(2 * pw) - sample(2 * pe)).abs()
                    + (left_green - right_green).abs()
                    + (left_green - right_green + sample(2 * pe) - sample(2 * pw)).abs(),
                (sample(2 * pn) - sample(2 * ps)).abs()
                    + (north_green - south_green).abs()
                    + (north_green - south_green + sample(2 * ps) - sample(2 * pn)).abs(),
            )
        };

        self.ndir[index] |= HOT;
        let direction = if dv > dh { pw } else { pn };
        let replacement = i32::midpoint(sample(2 * direction), sample(-2 * direction));
        let replacement = replacement as u16;
        self.rgb[0][index][channel] = replacement;
        self.rgb[1][index][channel] = replacement;
    }

    fn make_greens(&mut self, pattern: BayerPattern) {
        let pe = 1;
        let ps = self.stride as isize;
        let offsets = [pe, ps];
        for y in 0..self.height {
            let js = color_index(pattern.channel_at(0, y as u32)) & 1;
            let known = color_index(pattern.channel_at(js as u32, y as u32));
            let mut x = js;
            while x < self.width {
                let index = self.offset(x, y);
                for (direction, offset) in offsets.into_iter().enumerate() {
                    let h1 = 2 * i32::from(self.rgb[direction][add(index, -offset)][1])
                        - (i32::from(self.rgb[direction][add(index, -2 * offset)][known])
                            + i32::from(self.rgb[direction][index][known]));
                    let h2 = 2 * i32::from(self.rgb[direction][add(index, offset)][1])
                        - (i32::from(self.rgb[direction][add(index, 2 * offset)][known])
                            + i32::from(self.rgb[direction][index][known]));
                    let h0 = (h1 + h2) / 4;
                    let mut green = i32::from(self.rgb[direction][index][known]) + h0;
                    let mut minimum = i32::from(self.rgb[direction][add(index, -offset)][1])
                        .min(i32::from(self.rgb[direction][add(index, offset)][1]));
                    let mut maximum = i32::from(self.rgb[direction][add(index, -offset)][1])
                        .max(i32::from(self.rgb[direction][add(index, offset)][1]));
                    minimum -= minimum / 8;
                    maximum += maximum / 8;
                    if green < minimum {
                        green = minimum - ((minimum - green) as f32).sqrt() as i32;
                    } else if green > maximum {
                        green = maximum + ((green - maximum) as f32).sqrt() as i32;
                    }
                    self.rgb[direction][index][1] = clip(green, self.minimum[1], self.maximum[1]);
                }
                x += 2;
            }
        }
    }

    fn make_rb(&mut self, pattern: BayerPattern) {
        self.make_rb_hv(pattern);
        self.make_rb_last(pattern);
    }

    fn make_rb_hv(&mut self, pattern: BayerPattern) {
        let pe = 1;
        let ps = self.stride as isize;
        for y in 0..self.height {
            let js = color_index(pattern.channel_at(0, y as u32)) & 1;
            let known = color_index(pattern.channel_at(js as u32, y as u32));
            let mut x = js ^ 1;
            while x < self.width {
                let index = self.offset(x, y);
                for direction in 0..2 {
                    let offset = if direction == 0 { pe } else { ps };
                    let channel = known ^ (direction << 1);
                    let h1 = i32::from(self.rgb[direction][add(index, -offset)][channel])
                        - i32::from(self.rgb[direction][add(index, -offset)][1]);
                    let h2 = i32::from(self.rgb[direction][add(index, offset)][channel])
                        - i32::from(self.rgb[direction][add(index, offset)][1]);
                    let value = i32::from(self.rgb[direction][index][1]) + i32::midpoint(h1, h2);
                    self.rgb[direction][index][channel] =
                        clip(value, self.minimum[channel], self.maximum[channel]);
                }
                x += 2;
            }
        }
    }

    fn make_rb_last(&mut self, pattern: BayerPattern) {
        for y in 0..self.height {
            let js = color_index(pattern.channel_at(0, y as u32)) & 1;
            let known = color_index(pattern.channel_at(js as u32, y as u32));
            for x in 0..self.width {
                let index = self.offset(x, y);
                for direction in 0..2 {
                    let mut channel = known ^ 2;
                    if x & 1 != js {
                        channel ^= direction << 1;
                    }
                    let directions: [isize; 3] = if direction == 0 {
                        [
                            -1 - self.stride as isize,
                            -(self.stride as isize),
                            1 - self.stride as isize,
                        ]
                    } else {
                        [-1 - self.stride as isize, -1, -1 + self.stride as isize]
                    };
                    let mut best_gradient = 0;
                    let mut best_forward = 0;
                    let mut best_backward = 0;
                    for forward in 0..3 {
                        for backward in 0..3 {
                            let forward_index = add(index, directions[forward]);
                            let backward_index = add(index, -directions[backward]);
                            let gradient = (2 * i32::from(self.rgb[direction][index][1])
                                - (i32::from(self.rgb[direction][forward_index][1])
                                    + i32::from(self.rgb[direction][backward_index][1])))
                            .abs()
                                + (i32::from(self.rgb[direction][forward_index][channel])
                                    - i32::from(self.rgb[direction][backward_index][channel]))
                                .abs()
                                    / 4
                                + (i32::from(self.rgb[direction][forward_index][channel])
                                    - i32::from(self.rgb[direction][forward_index][1])
                                    + i32::from(self.rgb[direction][backward_index][1])
                                    - i32::from(self.rgb[direction][backward_index][channel]))
                                .abs()
                                    / 4;
                            if best_gradient == 0 || gradient < best_gradient {
                                best_gradient = gradient;
                                best_forward = forward;
                                best_backward = backward;
                            }
                        }
                    }
                    let forward_index = add(index, directions[best_forward]);
                    let backward_index = add(index, -directions[best_backward]);
                    let h1 = i32::from(self.rgb[direction][forward_index][channel])
                        - i32::from(self.rgb[direction][forward_index][1]);
                    let h2 = i32::from(self.rgb[direction][backward_index][channel])
                        - i32::from(self.rgb[direction][backward_index][1]);
                    let value = i32::from(self.rgb[direction][index][1]) + i32::midpoint(h1, h2);
                    self.rgb[direction][index][channel] =
                        clip(value, self.minimum[channel], self.maximum[channel]);
                }
            }
        }
    }

    fn evaluate(&mut self) {
        for direction in 0..2 {
            for index in 0..self.rgb[direction].len() {
                let rgb = self.rgb[direction][index].map(|value| GAMMA_LUT[value as usize]);
                self.yuv[direction][index] = [
                    0.0593_f32.mul_add(
                        f32::from(rgb[2]),
                        0.6780_f32.mul_add(f32::from(rgb[1]), 0.2627 * f32::from(rgb[0])),
                    ) as i32,
                    0.5_f32.mul_add(
                        f32::from(rgb[2]),
                        0.36037_f32.mul_add(-f32::from(rgb[1]), -0.13963 * f32::from(rgb[0])),
                    ) as i32,
                    0.0405_f32.mul_add(
                        -f32::from(rgb[2]),
                        0.4629_f32.mul_add(-f32::from(rgb[1]), 0.5034 * f32::from(rgb[0])),
                    ) as i32,
                ];
            }
        }

        let pe = 1;
        let pw = -1;
        let pn = -(self.stride as isize);
        let ps = self.stride as isize;
        let directions = [pw, pe, pn, ps];

        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.offset(x, y);
                let mut y_difference = [[0.0_f32; 4]; 2];
                let mut uv_difference = [[0_i32; 4]; 2];
                for direction in 0..2 {
                    for (slot, offset) in directions.iter().copied().enumerate() {
                        let neighbor = self.yuv[direction][add(index, offset)];
                        let current = self.yuv[direction][index];
                        y_difference[direction][slot] = current[0].abs_diff(neighbor[0]) as f32;
                        uv_difference[direction][slot] = square_diff(current[1], neighbor[1])
                            .wrapping_add(square_diff(current[2], neighbor[2]));
                    }
                }
                let y_epsilon = y_difference[0][0]
                    .max(y_difference[0][1])
                    .min(y_difference[1][2].max(y_difference[1][3]));
                let uv_epsilon = uv_difference[0][0]
                    .max(uv_difference[0][1])
                    .min(uv_difference[1][2].max(uv_difference[1][3]));
                for direction in 0..2 {
                    for (slot, offset) in directions.iter().copied().enumerate() {
                        if y_difference[direction][slot] <= y_epsilon
                            && uv_difference[direction][slot] <= uv_epsilon
                        {
                            self.homo[direction][add(index, offset)] += 1;
                            if slot / 2 == direction {
                                for multiple in 2..4 {
                                    let doubled = multiple as isize * offset;
                                    let adjacent = self.yuv[direction][add(index, doubled)];
                                    let current = self.yuv[direction][index];
                                    if (current[0].abs_diff(adjacent[0]) as f32) < y_epsilon
                                        && square_diff(current[1], adjacent[1])
                                            .wrapping_add(square_diff(current[2], adjacent[2]))
                                            < uv_epsilon
                                    {
                                        self.homo[direction][add(index, doubled)] += 1;
                                    } else {
                                        break;
                                    }
                                }
                            }
                        }
                    }
                }
            }
        }

        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.offset(x, y);
                let mut scores = [0_i16; 2];
                for (direction, score) in scores.iter_mut().enumerate() {
                    for hx in -1..2 {
                        for hy in -1..2 {
                            *score += i16::from(
                                self.homo[direction][add(index, hy * self.stride as isize + hx)],
                            );
                        }
                    }
                }
                let direction = match scores[0].cmp(&scores[1]) {
                    Ordering::Equal => {
                        let vertical = interp_error(
                            self.yuv[1][index],
                            self.yuv[1][add(index, pn)],
                            self.yuv[1][add(index, ps)],
                        )
                        .wrapping_add(
                            interp_error(
                                self.yuv[1][add(index, pn)],
                                self.yuv[1][add(index, pn + pn)],
                                self.yuv[1][add(index, pn + ps)],
                            )
                            .wrapping_div(2),
                        )
                        .wrapping_add(
                            interp_error(
                                self.yuv[1][add(index, ps)],
                                self.yuv[1][add(index, ps + pn)],
                                self.yuv[1][add(index, ps + ps)],
                            )
                            .wrapping_div(2),
                        );

                        let horizontal = interp_error(
                            self.yuv[0][index],
                            self.yuv[0][add(index, pw)],
                            self.yuv[0][add(index, pe)],
                        )
                        .wrapping_add(
                            interp_error(
                                self.yuv[0][add(index, pw)],
                                self.yuv[0][add(index, pw + pw)],
                                self.yuv[0][add(index, pw + pe)],
                            )
                            .wrapping_div(2),
                        )
                        .wrapping_add(
                            interp_error(
                                self.yuv[0][add(index, pe)],
                                self.yuv[0][add(index, pe + pw)],
                                self.yuv[0][add(index, pe + pe)],
                            )
                            .wrapping_div(2),
                        );

                        if vertical > horizontal { HOR } else { VER }
                    }
                    Ordering::Greater => HORSH,
                    Ordering::Less => VERSH,
                };
                self.ndir[index] |= direction;
            }
        }
    }

    fn refine_hv_dirs(&mut self) {
        for parity in 0..2 {
            for y in 0..self.height {
                let mut x = parity;
                while x < self.width {
                    self.refine_hv_at(x, y);
                    x += 2;
                }
            }
        }
        let pe = 1;
        let pw = -1;
        let pn = -(self.stride as isize);
        let ps = self.stride as isize;
        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.offset(x, y);
                if self.ndir[index] & HVSH != 0 {
                    continue;
                }
                let vertical = self.direction_count(index, VER, [pn, ps, pw, pe]);
                let horizontal = self.direction_count(index, HOR, [pn, ps, pw, pe]);
                if self.ndir[index] & VER != 0 && horizontal > 3 {
                    self.ndir[index] &= !VER;
                    self.ndir[index] |= HOR;
                }
                if self.ndir[index] & HOR != 0 && vertical > 3 {
                    self.ndir[index] &= !HOR;
                    self.ndir[index] |= VER;
                }
            }
        }
    }

    fn refine_hv_at(&mut self, x: usize, y: usize) {
        let pe = 1;
        let pw = -1;
        let pn = -(self.stride as isize);
        let ps = self.stride as isize;
        let index = self.offset(x, y);
        let vertical = self.direction_count(index, VER, [pn, ps, pw, pe]);
        let horizontal = self.direction_count(index, HOR, [pn, ps, pw, pe]);
        let codir = if self.ndir[index] & VER != 0 {
            self.ndir[add(index, pn)] & VER != 0 || self.ndir[add(index, ps)] & VER != 0
        } else {
            self.ndir[add(index, pw)] & HOR != 0 || self.ndir[add(index, pe)] & HOR != 0
        };
        if self.ndir[index] & VER != 0 && horizontal > 2 && !codir {
            self.ndir[index] &= !VER;
            self.ndir[index] |= HOR;
        }
        if self.ndir[index] & HOR != 0 && vertical > 2 && !codir {
            self.ndir[index] &= !HOR;
            self.ndir[index] |= VER;
        }
    }

    fn direction_count(&self, index: usize, direction: u8, offsets: [isize; 4]) -> i32 {
        offsets
            .into_iter()
            .filter(|offset| self.ndir[add(index, *offset)] & direction != 0)
            .count() as i32
    }

    const fn offset(&self, x: usize, y: usize) -> usize { (y + MARGIN) * self.stride + x + MARGIN }

    fn known_input(&self, x: usize, y: usize) -> u16 { self.input[y * self.width + x] }
}

fn interp_error(center: [i32; 3], first: [i32; 3], second: [i32; 3]) -> i32 {
    (0..3).fold(0, |sum, channel| {
        sum.wrapping_add(square_diff(
            2_i32
                .wrapping_mul(center[channel])
                .wrapping_sub(first[channel]),
            second[channel],
        ))
    })
}

const fn square_diff(first: i32, second: i32) -> i32 {
    let difference = first.wrapping_sub(second);
    difference.wrapping_mul(difference)
}

const fn clip(value: i32, minimum: u16, maximum: u16) -> u16 {
    if value > maximum as i32 {
        maximum
    } else if value < minimum as i32 {
        minimum
    } else {
        value as u16
    }
}

const fn add(index: usize, offset: isize) -> usize { (index as isize + offset) as usize }

const fn color_index(channel: BayerChannel) -> usize {
    match channel {
        BayerChannel::R => 0,
        BayerChannel::GRedRow | BayerChannel::GBlueRow => 1,
        BayerChannel::B => 2,
    }
}

#[cfg(test)]
mod tests {
    use fornax_core::{BayerChannel, BayerPattern, MonoImage};

    use super::*;
    use crate::LazyDemosaic;

    fn normalize(code: u16) -> f32 { f32::from(code) / f32::from(u16::MAX) }

    #[test]
    fn gamma_lut_matches_libraw_thresholds() {
        for (index, expected) in [
            (0, 0),
            (1, 4),
            (1_185, 5_332),
            (1_186, 5_337),
            (32_768, 46_231),
            (65_534, 65_535),
            (65_535, 65_535),
        ] {
            assert_eq!(GAMMA_LUT[index], expected, "gamma LUT at {index}");
        }
    }

    #[test]
    fn uniform_field_uses_libraw_16_bit_quantization() -> mischief::Result<()> {
        let input_code = 32_768_u16;
        let image = MonoImage::from_raw(32, 32, vec![normalize(input_code); 32 * 32]).unwrap();
        let expected = normalize(input_code);
        for pattern in BayerPattern::ALL {
            let output = DemosaicAahd.lazy(&image, pattern)?.collect();
            for y in 4..28 {
                for x in 4..28 {
                    let pixel = output.get_pixel(x, y);
                    assert!(
                        pixel.0.iter().all(|sample| *sample == expected),
                        "{pattern} AAHD at ({x}, {y}) produced {pixel:?}, expected {expected}"
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn measured_bayer_samples_are_preserved() -> mischief::Result<()> {
        let codes = (0..40 * 40)
            .map(|sample| (sample as u16).wrapping_mul(257))
            .collect::<Vec<_>>();
        let image = MonoImage::from_raw(
            40,
            40,
            codes.iter().copied().map(normalize).collect::<Vec<_>>(),
        )
        .unwrap();
        for pattern in BayerPattern::ALL {
            let output = DemosaicAahd.lazy(&image, pattern)?.collect();
            for y in 0..image.height() {
                for x in 0..image.width() {
                    let channel = color_index(pattern.channel_at(x, y));
                    let actual = output.get_pixel(x, y)[channel];
                    let expected = normalize(codes[(y * image.width() + x) as usize]);
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
        let image = MonoImage::from_raw(
            40,
            40,
            (0..40 * 40)
                .map(|sample| sample as f32 / 1_600.0)
                .collect::<Vec<_>>(),
        )
        .unwrap();
        let lazy = DemosaicAahd.lazy(&image, BayerPattern::GRBG)?;
        let whole = lazy.collect();

        let mut rows = vec![0.0; 40 * 3 * 40];
        lazy.fill_rows(0, &mut rows);
        assert_eq!(whole.as_raw(), &rows);
        Ok(())
    }

    #[test]
    fn output_clipping_handles_negative_and_over_white_values() {
        assert_eq!(clip(-1, 10, 20), 10);
        assert_eq!(clip(25, 10, 20), 20);
        assert_eq!(clip(15, 10, 20), 15);
    }

    #[test]
    fn textured_fixture_exercises_horizontal_and_vertical_homogeneity() {
        let width = 32;
        let height = 32;
        let input = (0..height)
            .flat_map(|y| {
                (0..width).map(move |x| {
                    if y < height / 2 {
                        (x as u16).wrapping_mul(1_500)
                    } else {
                        (y as u16).wrapping_mul(1_500).wrapping_add(4_000)
                    }
                })
            })
            .collect::<Vec<_>>();
        let mut aahd = Aahd::new(&input, width, height, BayerPattern::GBRG);
        aahd.hide_hots(BayerPattern::GBRG);
        aahd.make_greens(BayerPattern::GBRG);

        let differing_candidates = (0..height)
            .flat_map(|y| (0..width).map(move |x| (x, y)))
            .filter(|&(x, y)| {
                let index = aahd.offset(x, y);
                aahd.rgb[0][index][1] != aahd.rgb[1][index][1]
            })
            .count();
        assert!(differing_candidates > 0);

        aahd.make_rb(BayerPattern::GBRG);
        aahd.evaluate();
        let mut horizontal = 0;
        let mut vertical = 0;
        let mut homogeneity_choice = 0;
        for y in 4..height - 4 {
            for x in 4..width - 4 {
                let direction = aahd.ndir[aahd.offset(x, y)];
                horizontal += usize::from(direction & HOR != 0);
                vertical += usize::from(direction & VER != 0);
                homogeneity_choice += usize::from(direction & HVSH != 0);
            }
        }
        assert!(horizontal > 0, "horizontal={horizontal}");
        assert!(vertical > 0, "vertical={vertical}");
        assert!(homogeneity_choice > 0, "homogeneity={homogeneity_choice}");
    }

    #[test]
    fn hot_pixel_is_corrected_for_neighbors_and_restored_at_its_site() -> mischief::Result<()> {
        let normal = [1_000_u16, 1_200, 900].map(normalize);
        for pattern in BayerPattern::ALL {
            let mut samples = (0..32 * 32)
                .map(|index| {
                    let x = (index % 32) as u32;
                    let y = (index / 32) as u32;
                    normal[color_index(pattern.channel_at(x, y))]
                })
                .collect::<Vec<_>>();
            let (hot_x, hot_y) = match pattern {
                BayerPattern::RGGB => (8, 8),
                BayerPattern::BGGR => (9, 9),
                BayerPattern::GRBG => (9, 8),
                BayerPattern::GBRG => (8, 9),
            };
            samples[hot_y * 32 + hot_x] = 1.0;
            let image = MonoImage::from_raw(32, 32, samples).unwrap();
            let output = DemosaicAahd.lazy(&image, pattern)?.collect();

            assert!((output.get_pixel(hot_x as u32, hot_y as u32)[0] - 1.0).abs() < 1.0e-6);
            for y in 6..14 {
                for x in 6..14 {
                    if (x, y) == (hot_x as u32, hot_y as u32) {
                        continue;
                    }
                    let pixel = output.get_pixel(x, y);
                    for (channel, expected) in normal.into_iter().enumerate() {
                        assert!(
                            (pixel[channel] - expected).abs() < 2.0e-5,
                            "{pattern} at ({x}, {y}) channel {channel}: {pixel:?}"
                        );
                    }
                }
            }
        }
        Ok(())
    }

    const fn color_index(channel: BayerChannel) -> usize {
        match channel {
            BayerChannel::R => 0,
            BayerChannel::GRedRow | BayerChannel::GBlueRow => 1,
            BayerChannel::B => 2,
        }
    }
}
