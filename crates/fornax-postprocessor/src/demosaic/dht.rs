use fornax_core::error::DemosaicError;
use fornax_core::{BayerPattern, MonoImage, Primitive};

use super::buffer::{DenseLazy, codes, dense_from_codes};

const MARGIN: usize = 4;
const HVSH: u8 = 1;
const HOR: u8 = 2;
const VER: u8 = 4;
const HORSH: u8 = HOR | HVSH;
const VERSH: u8 = VER | HVSH;
const DIASH: u8 = 8;
const LURD: u8 = 16;
const RULD: u8 = 32;
const LURDSH: u8 = LURD | DIASH;
const RULDSH: u8 = RULD | DIASH;
const HOT: u8 = 64;
const THOT: f32 = 64.0;
const TG: f32 = 256.0;
const DIAGONAL_THRESHOLD: f32 = 1.4;

fn calc_dist(c1: f32, c2: f32) -> f32 { if c1 > c2 { c1 / c2 } else { c2 / c1 } }

fn scale_over(value: f32, base: f32) -> f32 {
    let scale = base * 0.4;
    let offset = value - base;
    base + (scale * (offset + scale)).sqrt() - scale
}

fn scale_under(value: f32, base: f32) -> f32 {
    let scale = base * 0.6;
    let offset = base - value;
    base - (scale * (offset + scale)).sqrt() + scale
}

fn to_code(value: f32) -> u16 { value.clamp(0.0, f32::from(u16::MAX)) as u16 }

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

    const fn offset(&self, x: usize, y: usize) -> usize { (y + MARGIN) * self.stride + x + MARGIN }

    fn sample_at(&self, x: isize, y: isize, channel: usize) -> f32 {
        let x = (x + MARGIN as isize) as usize;
        let y = (y + MARGIN as isize) as usize;
        self.nraw[y * self.stride + x][channel]
    }

    fn get_hv_grb(&self, x: usize, y: usize, known: usize) -> u8 {
        let x = x as isize;
        let y = y as isize;
        let sample = |x: isize, y: isize, channel| self.sample_at(x, y, channel);
        let center = sample(x, y, known);
        let hv1 = 2.0 * sample(x, y - 1, 1) / (sample(x, y - 2, known) + center);
        let hv2 = 2.0 * sample(x, y + 1, 1) / (sample(x, y + 2, known) + center);
        let mut kv = calc_dist(hv1, hv2)
            * calc_dist(
                center * center,
                sample(x, y - 2, known) * sample(x, y + 2, known),
            );
        kv *= kv;
        kv *= kv;
        kv *= kv;
        let dv = kv
            * calc_dist(
                sample(x, y - 3, 1) * sample(x, y + 3, 1),
                sample(x, y - 1, 1) * sample(x, y + 1, 1),
            );

        let hh1 = 2.0 * sample(x - 1, y, 1) / (sample(x - 2, y, known) + center);
        let hh2 = 2.0 * sample(x + 1, y, 1) / (sample(x + 2, y, known) + center);
        let mut kh = calc_dist(hh1, hh2)
            * calc_dist(
                center * center,
                sample(x - 2, y, known) * sample(x + 2, y, known),
            );
        kh *= kh;
        kh *= kh;
        kh *= kh;
        let dh = kh
            * calc_dist(
                sample(x - 3, y, 1) * sample(x + 3, y, 1),
                sample(x - 1, y, 1) * sample(x + 1, y, 1),
            );

        let strength = calc_dist(dh, dv);
        if dh < dv {
            if strength > TG { HORSH } else { HOR }
        } else if strength > TG {
            VERSH
        } else {
            VER
        }
    }

    fn get_hv_rbg(&self, x: usize, y: usize, known: usize) -> u8 {
        let x = x as isize;
        let y = y as isize;
        let sample = |x: isize, y: isize, channel| self.sample_at(x, y, channel);
        let center = sample(x, y, 1);
        let opposite = known ^ 2;
        let hv1 = 2.0 * sample(x, y - 1, opposite) / (sample(x, y - 2, 1) + sample(x, y, 1));
        let hv2 = 2.0 * sample(x, y + 1, opposite) / (sample(x, y + 2, 1) + sample(x, y, 1));
        let mut kv = calc_dist(hv1, hv2)
            * calc_dist(center * center, sample(x, y - 2, 1) * sample(x, y + 2, 1));
        kv *= kv;
        kv *= kv;
        kv *= kv;
        let dv = kv
            * calc_dist(
                sample(x, y - 3, opposite) * sample(x, y + 3, opposite),
                sample(x, y - 1, opposite) * sample(x, y + 1, opposite),
            );

        let hh1 = 2.0 * sample(x - 1, y, known) / (sample(x - 2, y, 1) + center);
        let hh2 = 2.0 * sample(x + 1, y, known) / (sample(x + 2, y, 1) + center);
        let mut kh = calc_dist(hh1, hh2)
            * calc_dist(center * center, sample(x - 2, y, 1) * sample(x + 2, y, 1));
        kh *= kh;
        kh *= kh;
        kh *= kh;
        let dh = kh
            * calc_dist(
                sample(x - 3, y, known) * sample(x + 3, y, known),
                sample(x - 1, y, known) * sample(x + 1, y, known),
            );

        let strength = calc_dist(dh, dv);
        if dh < dv {
            if strength > TG { HORSH } else { HOR }
        } else if strength > TG {
            VERSH
        } else {
            VER
        }
    }

    fn get_diag_grb(&self, x: usize, y: usize, known: usize) -> u8 {
        let x = x as isize;
        let y = y as isize;
        let sample = |x: isize, y: isize, channel| self.sample_at(x, y, channel);
        let center = sample(x, y, 1) * sample(x, y, 1);
        let hlu = sample(x - 1, y - 1, 1) / sample(x - 1, y - 1, known);
        let hrd = sample(x + 1, y + 1, 1) / sample(x + 1, y + 1, known);
        let lurd = calc_dist(hlu, hrd)
            * calc_dist(sample(x - 1, y - 1, 1) * sample(x + 1, y + 1, 1), center);
        let ruld = calc_dist(hlu, hrd)
            * calc_dist(sample(x + 1, y - 1, 1) * sample(x - 1, y + 1, 1), center);
        let strength = calc_dist(lurd, ruld);
        if ruld < lurd {
            if strength > DIAGONAL_THRESHOLD {
                RULDSH
            } else {
                RULD
            }
        } else if strength > DIAGONAL_THRESHOLD {
            LURDSH
        } else {
            LURD
        }
    }

    fn get_diag_rbg(&self, x: usize, y: usize) -> u8 {
        let x = x as isize;
        let y = y as isize;
        let sample = |x: isize, y: isize, channel| self.sample_at(x, y, channel);
        let center = sample(x, y, 1) * sample(x, y, 1);
        let lurd = calc_dist(sample(x - 1, y - 1, 1) * sample(x + 1, y + 1, 1), center);
        let ruld = calc_dist(sample(x + 1, y - 1, 1) * sample(x - 1, y + 1, 1), center);
        let strength = calc_dist(lurd, ruld);
        if ruld < lurd {
            if strength > DIAGONAL_THRESHOLD {
                RULDSH
            } else {
                RULD
            }
        } else if strength > DIAGONAL_THRESHOLD {
            LURDSH
        } else {
            LURD
        }
    }

    fn make_hv_dirs(&mut self) {
        for y in 0..self.height {
            let js = cfa_channel(self.pattern, 0, y as isize) & 1;
            let known = cfa_channel(self.pattern, js as isize, y as isize);
            for x in 0..self.width {
                let direction = if x & 1 == js {
                    self.get_hv_grb(x, y, known)
                } else {
                    self.get_hv_rbg(x, y, known)
                };
                let index = self.offset(x, y);
                self.ndir[index] |= direction;
            }
        }
        for y in 0..self.height {
            self.refine_hv_row(y, y & 1);
        }
        for y in 0..self.height {
            self.refine_hv_row(y, (y & 1) ^ 1);
        }
        for y in 0..self.height {
            self.refine_hv_all_columns(y);
        }
    }

    fn refine_hv_row(&mut self, y: usize, parity: usize) {
        for x in (parity..self.width).step_by(2) {
            let index = self.offset(x, y);
            if self.ndir[index] & HVSH != 0 {
                continue;
            }
            let vertical = (self.ndir[index - self.stride] & VER)
                + (self.ndir[index + self.stride] & VER)
                + (self.ndir[index - 1] & VER)
                + (self.ndir[index + 1] & VER);
            let horizontal = (self.ndir[index - self.stride] & HOR)
                + (self.ndir[index + self.stride] & HOR)
                + (self.ndir[index - 1] & HOR)
                + (self.ndir[index + 1] & HOR);
            let codirectional = if self.ndir[index] & VER != 0 {
                self.ndir[index - self.stride] & VER != 0
                    || self.ndir[index + self.stride] & VER != 0
            } else {
                self.ndir[index - 1] & HOR != 0 || self.ndir[index + 1] & HOR != 0
            };
            let vertical = vertical / VER;
            let horizontal = horizontal / HOR;
            if self.ndir[index] & VER != 0 && horizontal > 2 && !codirectional {
                self.ndir[index] &= !VER;
                self.ndir[index] |= HOR;
            }
            if self.ndir[index] & HOR != 0 && vertical > 2 && !codirectional {
                self.ndir[index] &= !HOR;
                self.ndir[index] |= VER;
            }
        }
    }

    fn refine_hv_all_columns(&mut self, y: usize) {
        for x in 0..self.width {
            let index = self.offset(x, y);
            if self.ndir[index] & HVSH != 0 {
                continue;
            }
            let vertical = (self.ndir[index - self.stride] & VER)
                + (self.ndir[index + self.stride] & VER)
                + (self.ndir[index - 1] & VER)
                + (self.ndir[index + 1] & VER);
            let horizontal = (self.ndir[index - self.stride] & HOR)
                + (self.ndir[index + self.stride] & HOR)
                + (self.ndir[index - 1] & HOR)
                + (self.ndir[index + 1] & HOR);
            let vertical = vertical / VER;
            let horizontal = horizontal / HOR;
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

    fn make_diag_dirs(&mut self) {
        for y in 0..self.height {
            let js = cfa_channel(self.pattern, 0, y as isize) & 1;
            let known = cfa_channel(self.pattern, js as isize, y as isize);
            for x in 0..self.width {
                let direction = if x & 1 == js {
                    self.get_diag_grb(x, y, known)
                } else {
                    self.get_diag_rbg(x, y)
                };
                let index = self.offset(x, y);
                self.ndir[index] |= direction;
            }
        }
        for y in 0..self.height {
            self.refine_diag_all_columns(y);
        }
    }

    fn refine_diag_all_columns(&mut self, y: usize) {
        for x in 0..self.width {
            let index = self.offset(x, y);
            if self.ndir[index] & DIASH != 0 {
                continue;
            }
            let neighbors = [
                index - self.stride,
                index + self.stride,
                index - 1,
                index + 1,
                index - self.stride - 1,
                index - self.stride + 1,
                index + self.stride - 1,
                index + self.stride + 1,
            ];
            let lurd = neighbors
                .iter()
                .map(|neighbor| usize::from(self.ndir[*neighbor] & LURD))
                .sum::<usize>()
                / usize::from(LURD);
            let ruld = neighbors
                .iter()
                .map(|neighbor| usize::from(self.ndir[*neighbor] & RULD))
                .sum::<usize>()
                / usize::from(RULD);
            if self.ndir[index] & LURD != 0 && ruld > 7 {
                self.ndir[index] &= !LURD;
                self.ndir[index] |= RULD;
            }
            if self.ndir[index] & RULD != 0 && lurd > 7 {
                self.ndir[index] &= !RULD;
                self.ndir[index] |= LURD;
            }
        }
    }

    #[allow(
        clippy::suboptimal_flops,
        reason = "preserve LibRaw's separate multiply-add evaluation for reference fidelity"
    )]
    fn make_greens(&mut self) {
        for y in 0..self.height {
            let js = cfa_channel(self.pattern, 0, y as isize) & 1;
            let known = cfa_channel(self.pattern, js as isize, y as isize);
            let stride = self.stride as isize;
            for x in (js..self.width).step_by(2) {
                let index = self.offset(x, y);
                let (dx, dy, dx2, dy2) = if self.ndir[index] & VER != 0 {
                    (0, -1, 0, 1)
                } else {
                    (1, 0, -1, 0)
                };
                let h1 = 2.0 * self.sample_add(index, dy * stride + dx, 1)
                    / (self.sample_add(index, 2 * dy * stride + 2 * dx, known)
                        + self.nraw[index][known]);
                let h2 = 2.0 * self.sample_add(index, dy2 * stride + dx2, 1)
                    / (self.sample_add(index, 2 * dy2 * stride + 2 * dx2, known)
                        + self.nraw[index][known]);
                let mut b1 = 1.0
                    / calc_dist(
                        self.nraw[index][known],
                        self.sample_add(index, 2 * dy * stride + 2 * dx, known),
                    );
                let mut b2 = 1.0
                    / calc_dist(
                        self.nraw[index][known],
                        self.sample_add(index, 2 * dy2 * stride + 2 * dx2, known),
                    );
                b1 *= b1;
                b2 *= b2;
                let mut green = self.nraw[index][known] * (b1 * h1 + b2 * h2) / (b1 + b2);
                let mut minimum = self
                    .sample_add(index, dy * stride + dx, 1)
                    .min(self.sample_add(index, dy2 * stride + dx2, 1));
                let mut maximum = self
                    .sample_add(index, dy * stride + dx, 1)
                    .max(self.sample_add(index, dy2 * stride + dx2, 1));
                minimum /= 1.2;
                maximum *= 1.2;
                if green < minimum {
                    green = scale_under(green, minimum);
                } else if green > maximum {
                    green = scale_over(green, maximum);
                }
                if green > f32::from(self.channel_maximum[1]) {
                    green = f32::from(self.channel_maximum[1]);
                } else if green < self.channel_minimum[1] {
                    green = self.channel_minimum[1];
                }
                self.nraw[index][1] = green;
            }
        }
    }

    fn make_rb(&mut self) {
        for y in 0..self.height {
            self.make_rb_diag_row(y);
        }
        for y in 0..self.height {
            self.make_rb_hv_row(y);
        }
    }

    fn make_rb_diag_row(&mut self, y: usize) {
        let js = cfa_channel(self.pattern, 0, y as isize) & 1;
        let known = cfa_channel(self.pattern, js as isize, y as isize);
        let missing = known ^ 2;
        let stride = self.stride as isize;
        for x in (js..self.width).step_by(2) {
            let index = self.offset(x, y);
            let (dx, dy, dx2, dy2) = if self.ndir[index] & LURD != 0 {
                (-1, -1, 1, 1)
            } else {
                (-1, 1, 1, -1)
            };
            let first = dy * stride + dx;
            let second = dy2 * stride + dx2;
            let mut g1 = 1.0 / calc_dist(self.nraw[index][1], self.sample_add(index, first, 1));
            let mut g2 = 1.0 / calc_dist(self.nraw[index][1], self.sample_add(index, second, 1));
            g1 = g1 * (g1 * g1);
            g2 = g2 * (g2 * g2);
            let mut color = self.nraw[index][1]
                * (g1 * self.sample_add(index, first, missing) / self.sample_add(index, first, 1)
                    + g2 * self.sample_add(index, second, missing)
                        / self.sample_add(index, second, 1))
                / (g1 + g2);
            let mut minimum = self
                .sample_add(index, first, missing)
                .min(self.sample_add(index, second, missing));
            let mut maximum = self
                .sample_add(index, first, missing)
                .max(self.sample_add(index, second, missing));
            minimum /= 1.2;
            maximum *= 1.2;
            if color < minimum {
                color = scale_under(color, minimum);
            } else if color > maximum {
                color = scale_over(color, maximum);
            }
            if color > f32::from(self.channel_maximum[missing]) {
                color = f32::from(self.channel_maximum[missing]);
            } else if color < self.channel_minimum[missing] {
                color = self.channel_minimum[missing];
            }
            self.nraw[index][missing] = color;
        }
    }

    fn make_rb_hv_row(&mut self, y: usize) {
        let js = (cfa_channel(self.pattern, 0, y as isize) & 1) ^ 1;
        let stride = self.stride as isize;
        for x in (js..self.width).step_by(2) {
            let index = self.offset(x, y);
            let (dx, dy, dx2, dy2) = if self.ndir[index] & VER != 0 {
                (0, -1, 0, 1)
            } else {
                (1, 0, -1, 0)
            };
            let first = dy * stride + dx;
            let second = dy2 * stride + dx2;
            let mut g1 = 1.0 / calc_dist(self.nraw[index][1], self.sample_add(index, first, 1));
            let mut g2 = 1.0 / calc_dist(self.nraw[index][1], self.sample_add(index, second, 1));
            g1 *= g1;
            g2 *= g2;
            for channel in [0, 2] {
                let mut color = self.nraw[index][1]
                    * (g1 * self.sample_add(index, first, channel)
                        / self.sample_add(index, first, 1)
                        + g2 * self.sample_add(index, second, channel)
                            / self.sample_add(index, second, 1))
                    / (g1 + g2);
                let mut minimum = self
                    .sample_add(index, first, channel)
                    .min(self.sample_add(index, second, channel));
                let mut maximum = self
                    .sample_add(index, first, channel)
                    .max(self.sample_add(index, second, channel));
                minimum /= 1.2;
                maximum *= 1.2;
                if color < minimum {
                    color = scale_under(color, minimum);
                } else if color > maximum {
                    color = scale_over(color, maximum);
                }
                if color > f32::from(self.channel_maximum[channel]) {
                    color = f32::from(self.channel_maximum[channel]);
                } else if color < self.channel_minimum[channel] {
                    color = self.channel_minimum[channel];
                }
                self.nraw[index][channel] = color;
            }
        }
    }

    fn process(&mut self, input: &[u16]) -> Vec<u16> {
        self.hide_hots();
        self.make_hv_dirs();
        self.make_greens();
        self.make_diag_dirs();
        self.make_rb();
        self.restore_hots(input);

        let mut output = Vec::with_capacity(self.width * self.height * 3);
        for y in 0..self.height {
            for x in 0..self.width {
                let index = self.offset(x, y);
                output.extend(self.nraw[index].map(to_code));
            }
        }
        output
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
            -2 * self.stride as isize,
            -2 * self.stride as isize + 2,
            -2,
            2,
            2 * self.stride as isize - 2,
            2 * self.stride as isize,
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
        let width = image.width() as usize;
        let height = image.height() as usize;
        if width < 3 || height < 3 {
            return Err(DemosaicError::InvalidInput(
                "DHT requires an image at least 3x3".into(),
            ));
        }
        let stride = width
            .checked_add(2 * MARGIN)
            .ok_or_else(|| DemosaicError::InvalidInput("DHT image width overflow".into()))?;
        let plane_height = height
            .checked_add(2 * MARGIN)
            .ok_or_else(|| DemosaicError::InvalidInput("DHT image height overflow".into()))?;
        if stride.checked_mul(plane_height).is_none() {
            return Err(DemosaicError::InvalidInput(
                "DHT image dimensions overflow".into(),
            ));
        }

        let input = codes(image);
        let output = Dht::new(&input, width, height, pattern).process(&input);
        Ok(DenseLazy::new(dense_from_codes(
            image.width(),
            image.height(),
            output,
        )))
    }
}

#[cfg(test)]
mod tests {
    use fornax_core::BayerPattern;

    use super::*;
    use crate::protocol::LazyDemosaic;

    const HOT: u8 = 64;
    const HVSH: u8 = 1;
    const HOR: u8 = 2;
    const VER: u8 = 4;
    const HORSH: u8 = HOR | HVSH;
    const LURD: u8 = 16;
    const RULD: u8 = 32;

    fn measured_mosaic(width: usize, height: usize, pattern: BayerPattern, value: u16) -> Vec<u16> {
        let mut samples = vec![value; width * height];
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

    #[test]
    fn hide_hots_uses_axis_and_diagonal_neighbors_for_its_average() {
        let pattern = BayerPattern::GBRG;
        let width = 9;
        let height = 9;
        let input = measured_mosaic(width, height, pattern, 500);
        let mut dht = Dht::new(&input, width, height, pattern);
        let center = dht.offset(4, 4);
        let stride = dht.stride as isize;
        dht.nraw[center][1] = 65_535.0;
        for offset in [
            -2 * stride - 2,
            -2 * stride + 2,
            2 * stride - 2,
            2 * stride + 2,
        ] {
            dht.nraw[center.wrapping_add_signed(offset)][1] = 1_000.0;
        }
        for offset in [-2 * stride, -2, 2, 2 * stride] {
            dht.nraw[center.wrapping_add_signed(offset)][1] = 6_000.0;
        }

        dht.hide_hots();

        assert_eq!(dht.ndir[center] & HOT, 0);
        assert_eq!(dht.nraw[center][1], 65_535.0);
    }

    fn uniform_dht(width: usize, height: usize, pattern: BayerPattern) -> Dht {
        let mut dht = Dht::new(&[1_000; 81], width, height, pattern);
        dht.nraw.fill([1.0; 3]);
        dht
    }

    #[test]
    fn hv_direction_marks_strong_and_regular_horizontal_edges() {
        let pattern = BayerPattern::GBRG;
        let mut regular = uniform_dht(9, 9, pattern);
        let index = regular.offset(4, 2);
        regular.nraw[index][1] = 4.0;
        assert_eq!(regular.get_hv_grb(4, 5, 0), HOR);

        let mut strong = uniform_dht(9, 9, pattern);
        let index = strong.offset(4, 2);
        strong.nraw[index][1] = 1_024.0;
        assert_eq!(strong.get_hv_grb(4, 5, 0), HORSH);
    }

    #[test]
    fn diag_direction_marks_strong_and_regular_diagonal_edges() {
        let pattern = BayerPattern::GBRG;
        let mut regular = uniform_dht(9, 9, pattern);
        let index = regular.offset(3, 2);
        regular.nraw[index][1] = 0.91;
        assert_eq!(regular.get_diag_grb(4, 3, 0), RULD);

        let mut strong = uniform_dht(9, 9, pattern);
        let index = strong.offset(5, 2);
        strong.nraw[index][1] = 0.5;
        assert_eq!(strong.get_diag_grb(4, 3, 0), LURDSH);
    }

    #[test]
    fn hv_refinement_flips_an_unsupported_direction() {
        let pattern = BayerPattern::GBRG;
        let mut dht = uniform_dht(9, 9, pattern);
        let center = dht.offset(4, 4);
        dht.ndir[center] = VER;
        dht.ndir[center - 1] = HOR;
        dht.ndir[center + 1] = HOR;
        dht.ndir[center - dht.stride] = HOR;
        dht.ndir[center + dht.stride] = HOR;

        dht.refine_hv_row(4, 0);

        assert_eq!(dht.ndir[center] & (HOR | VER), HOR);
    }

    fn unbounded(dht: &mut Dht) {
        dht.channel_minimum = [0.5; 3];
        dht.channel_maximum = [1_000; 3];
    }

    #[test]
    fn make_greens_uses_the_selected_axis_and_channel_bounds() {
        let pattern = BayerPattern::GBRG;
        let mut horizontal = uniform_dht(9, 9, pattern);
        unbounded(&mut horizontal);
        horizontal.nraw.fill([100.0; 3]);
        let center = horizontal.offset(4, 5);
        horizontal.ndir[center] = HOR;
        let north = horizontal.offset(4, 4);
        let south = horizontal.offset(4, 6);
        horizontal.nraw[north][1] = 1_000.0;
        horizontal.nraw[south][1] = 1_000.0;
        horizontal.make_greens();
        assert_eq!(horizontal.nraw[center][1], 100.0);

        let mut vertical = uniform_dht(9, 9, pattern);
        unbounded(&mut vertical);
        vertical.nraw.fill([100.0; 3]);
        let center = vertical.offset(3, 6);
        vertical.ndir[center] = VER;
        let west = vertical.offset(2, 6);
        let east = vertical.offset(4, 6);
        vertical.nraw[west][1] = 1_000.0;
        vertical.nraw[east][1] = 1_000.0;
        vertical.make_greens();
        assert_eq!(vertical.nraw[center][1], 100.0);

        let center = horizontal.offset(4, 5);
        horizontal.nraw[center - 1][1] = 5_000.0;
        horizontal.nraw[center + 1][1] = 5_000.0;
        horizontal.make_greens();
        assert_eq!(horizontal.nraw[center][1], 1_000.0);
    }

    #[test]
    fn make_rb_reconstructs_diagonal_and_hv_missing_samples() {
        let pattern = BayerPattern::GBRG;
        let mut dht = uniform_dht(9, 9, pattern);
        unbounded(&mut dht);
        dht.nraw.fill([100.0; 3]);

        let red = dht.offset(4, 5);
        dht.ndir[red] = LURD;
        let northwest = dht.offset(3, 4);
        let southeast = dht.offset(5, 6);
        dht.nraw[northwest] = [100.0, 100.0, 200.0];
        dht.nraw[southeast] = [100.0, 100.0, 300.0];
        dht.make_rb_diag_row(5);
        assert_eq!(dht.nraw[red][2], 250.0);

        let green = dht.offset(4, 4);
        dht.ndir[green] = HOR;
        dht.nraw[green - 1] = [200.0, 100.0, 400.0];
        dht.nraw[green + 1] = [300.0, 100.0, 500.0];
        dht.make_rb_hv_row(4);
        assert_eq!(dht.nraw[green][0], 250.0);
        assert_eq!(dht.nraw[green][2], 450.0);
    }

    #[test]
    fn output_codes_truncate_toward_zero_and_clip() {
        assert_eq!(to_code(100.9), 100);
        assert_eq!(to_code(-1.0), 0);
        assert_eq!(to_code(70_000.0), u16::MAX);
    }

    #[test]
    fn process_restores_hot_samples_and_preserves_measured_sites() {
        let pattern = BayerPattern::GBRG;
        let width = 9;
        let height = 9;
        let mut input = vec![1_000; width * height];
        let hot = 5 * width + 5;
        input[hot] = 60_000;
        let mut dht = Dht::new(&input, width, height, pattern);

        let output = dht.process(&input);

        assert_eq!(output.len(), width * height * 3);
        assert_eq!(output[hot * 3 + 1], 60_000);
        for y in 0..height {
            for x in 0..width {
                let input_index = y * width + x;
                let output_index = input_index * 3 + cfa_channel(pattern, x as isize, y as isize);
                assert_eq!(output[output_index], input[input_index], "({x}, {y})");
            }
        }
    }

    #[test]
    fn lazy_accepts_the_comparison_bridge_minimum_dimensions() -> mischief::Result<()> {
        let image = MonoImage::from_raw(3, 3, vec![0.25_f32; 9]).unwrap();
        let output = DemosaicDht.lazy(&image, BayerPattern::GBRG)?.collect();
        assert_eq!(output.dimensions(), (3, 3));
        Ok(())
    }
}
