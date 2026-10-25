use std::ops::Deref;

use image::{GenericImageView, ImageBuffer, Luma, Rgb};
use rayon::prelude::*;

use crate::BayerPattern;

pub type RgbImageF32 = ImageBuffer<Rgb<f32>, Vec<f32>>;

pub fn demosaic<I>(image: I, bayer_pattern: BayerPattern) -> RgbImageF32
where
    I: Deref,
    I::Target: GenericImageView<Pixel = Luma<f32>> + Sync,
{
    let image = &*image;
    let view = image
        .to_pixel_view()
        .expect("demosaic input must expose a pixel view");
    let (width, height) = view.dimensions();
    let mut output = RgbImageF32::new(width, height);
    if width == 0 || height == 0 {
        return output;
    }

    let samples: &[f32] = view.samples();
    let width_stride = view.flat().layout.width_stride;
    let height_stride = view.flat().layout.height_stride;
    let input = Input {
        samples,
        width,
        height,
        width_stride,
        height_stride,
    };
    let output_stride = width as usize * 3;

    output
        .as_mut()
        .par_chunks_mut(output_stride)
        .enumerate()
        .for_each(|(y, row)| input.write_row(bayer_pattern, y as u32, row));

    output
}

#[derive(Clone, Copy)]
struct Input<'a> {
    samples: &'a [f32],
    width: u32,
    height: u32,
    width_stride: usize,
    height_stride: usize,
}

impl Input<'_> {
    #[inline(always)]
    const fn index(&self, x: u32, y: u32) -> usize {
        y as usize * self.height_stride + x as usize * self.width_stride
    }

    #[inline(always)]
    fn centre(&self, x: u32, y: u32) -> f32 { self.samples[self.index(x, y)] }

    fn write_row(self, pattern: BayerPattern, y: u32, output: &mut [f32]) {
        let border_row = y == 0 || y + 1 == self.height;
        for x in 0..self.width {
            let phase = Phase::at(pattern, x, y);
            let rgb = if border_row || x == 0 || x + 1 == self.width {
                self.checked_pixel(x, y, phase)
            } else {
                // SAFETY: `x` and `y` are interior, so every 3x3 neighbor is in
                // the image and therefore maps to an in-bounds view sample.
                unsafe { self.fast_pixel(x, y, phase) }
            };
            let offset = x as usize * 3;
            output[offset] = rgb[0];
            output[offset + 1] = rgb[1];
            output[offset + 2] = rgb[2];
        }
    }

    #[inline(always)]
    unsafe fn fast_pixel(&self, x: u32, y: u32, phase: Phase) -> [f32; 3] {
        // SAFETY: upheld by `write_row`.
        unsafe {
            let centre = *self.samples.get_unchecked(self.index(x, y));
            let left = *self.samples.get_unchecked(self.index(x - 1, y));
            let right = *self.samples.get_unchecked(self.index(x + 1, y));
            let top = *self.samples.get_unchecked(self.index(x, y - 1));
            let bottom = *self.samples.get_unchecked(self.index(x, y + 1));
            let top_left = *self.samples.get_unchecked(self.index(x - 1, y - 1));
            let top_right = *self.samples.get_unchecked(self.index(x + 1, y - 1));
            let bottom_left = *self.samples.get_unchecked(self.index(x - 1, y + 1));
            let bottom_right = *self.samples.get_unchecked(self.index(x + 1, y + 1));
            phase.channels(
                centre,
                (left + right + top + bottom) * 0.25,
                f32::midpoint(left, right),
                f32::midpoint(top, bottom),
                (top_left + top_right + bottom_left + bottom_right) * 0.25,
            )
        }
    }

    fn checked_pixel(&self, x: u32, y: u32, phase: Phase) -> [f32; 3] {
        let left = x.checked_sub(1).map(|x| self.centre(x, y));
        let right = (x + 1 < self.width).then(|| self.centre(x + 1, y));
        let top = y.checked_sub(1).map(|y| self.centre(x, y));
        let bottom = (y + 1 < self.height).then(|| self.centre(x, y + 1));
        let top_left = x
            .checked_sub(1)
            .zip(y.checked_sub(1))
            .map(|(x, y)| self.centre(x, y));
        let top_right = (x + 1 < self.width)
            .then_some(y.checked_sub(1))
            .flatten()
            .map(|y| self.centre(x + 1, y));
        let bottom_left = (y + 1 < self.height)
            .then_some(x.checked_sub(1))
            .flatten()
            .map(|x| self.centre(x, y + 1));
        let bottom_right =
            (x + 1 < self.width && y + 1 < self.height).then(|| self.centre(x + 1, y + 1));
        phase.channels(
            self.centre(x, y),
            average4([left, right, top, bottom]),
            average2(left, right),
            average2(top, bottom),
            average4([top_left, top_right, bottom_left, bottom_right]),
        )
    }
}

#[derive(Clone, Copy)]
enum Phase {
    Red,
    GreenHorizontalRed,
    GreenVerticalRed,
    Blue,
}

impl Phase {
    const fn at(pattern: BayerPattern, x: u32, y: u32) -> Self {
        let slot = (y & 1) * 2 + (x & 1);
        match (pattern, slot) {
            (BayerPattern::Rggb, 0) | (BayerPattern::Bggr, 3) => Self::Red,
            (BayerPattern::Grbg, 1) | (BayerPattern::Gbrg, 2) => Self::Red,
            (BayerPattern::Rggb, 1)
            | (BayerPattern::Bggr, 2)
            | (BayerPattern::Grbg, 0)
            | (BayerPattern::Gbrg, 3) => Self::GreenHorizontalRed,
            (BayerPattern::Rggb, 2)
            | (BayerPattern::Bggr, 1)
            | (BayerPattern::Grbg, 3)
            | (BayerPattern::Gbrg, 0) => Self::GreenVerticalRed,
            (BayerPattern::Rggb, 3) | (BayerPattern::Bggr, 0) => Self::Blue,
            (BayerPattern::Grbg, 2) | (BayerPattern::Gbrg, 1) => Self::Blue,
            (BayerPattern::Rggb, _)
            | (BayerPattern::Bggr, _)
            | (BayerPattern::Grbg, _)
            | (BayerPattern::Gbrg, _) => Self::Red,
        }
    }

    const fn channels(
        self,
        centre: f32,
        cardinal: f32,
        left_right: f32,
        top_down: f32,
        diagonal: f32,
    ) -> [f32; 3] {
        match self {
            Self::Red => [centre, cardinal, diagonal],
            Self::GreenHorizontalRed => [left_right, centre, top_down],
            Self::GreenVerticalRed => [top_down, centre, left_right],
            Self::Blue => [diagonal, cardinal, centre],
        }
    }
}

fn average2(a: Option<f32>, b: Option<f32>) -> f32 {
    match (a, b) {
        (Some(a), Some(b)) => f32::midpoint(a, b),
        (Some(value), None) | (None, Some(value)) => value,
        (None, None) => 0.0,
    }
}

fn average4(values: [Option<f32>; 4]) -> f32 {
    let (sum, count) = values
        .into_iter()
        .flatten()
        .fold((0.0, 0_u8), |(sum, count), value| (sum + value, count + 1));
    if count == 0 {
        0.0
    } else {
        sum / f32::from(count)
    }
}

#[cfg(test)]
mod tests {
    use image::ImageBuffer;

    use super::*;

    fn run(pattern: BayerPattern) -> Vec<[f32; 3]> {
        let samples: Vec<f32> = (1_u16..=9).map(|value| f32::from(value) / 16.0).collect();
        let image = ImageBuffer::from_raw(3, 3, samples).unwrap();
        let output = demosaic(&image, pattern);
        output.as_raw().as_chunks::<3>().0.to_vec()
    }

    #[test]
    fn matches_bilinear_results_for_every_pattern() {
        assert_eq!(
            run(BayerPattern::Rggb),
            vec![
                scaled([1, 3, 5]),
                scaled([2, 2, 5]),
                scaled([3, 4, 5]),
                scaled([4, 4, 5]),
                scaled([5, 5, 5]),
                scaled([6, 6, 5]),
                scaled([7, 6, 5]),
                scaled([8, 8, 5]),
                scaled([9, 7, 5]),
            ]
        );
        assert_close(
            BayerPattern::Bggr,
            [
                scaled([5, 3, 1]),
                scaled([5, 2, 2]),
                scaled([5, 4, 3]),
                scaled([5, 4, 4]),
                scaled([5, 5, 5]),
                scaled([5, 6, 6]),
                scaled([5, 6, 7]),
                scaled([5, 8, 8]),
                scaled([5, 7, 9]),
            ],
        );
        assert_close(
            BayerPattern::Grbg,
            [
                scaled([2, 1, 4]),
                scaled([2, 3, 5]),
                scaled([2, 3, 6]),
                scaled_f32([5.0, 13.0 / 3.0, 4.0]),
                scaled([5, 5, 5]),
                scaled_f32([5.0, 17.0 / 3.0, 6.0]),
                scaled([8, 7, 4]),
                scaled([8, 7, 5]),
                scaled([8, 9, 6]),
            ],
        );
        assert_close(
            BayerPattern::Gbrg,
            [
                scaled([4, 1, 2]),
                scaled([5, 3, 2]),
                scaled([6, 3, 2]),
                scaled_f32([4.0, 13.0 / 3.0, 5.0]),
                scaled([5, 5, 5]),
                scaled_f32([6.0, 17.0 / 3.0, 5.0]),
                scaled([4, 7, 8]),
                scaled([5, 7, 8]),
                scaled([6, 9, 8]),
            ],
        );
    }

    #[test]
    fn handles_one_pixel_borders() {
        let image = ImageBuffer::from_raw(1, 1, vec![0.25]).unwrap();
        assert_eq!(
            demosaic(&image, BayerPattern::Rggb).get_pixel(0, 0).0,
            [0.25, 0.0, 0.0]
        );
    }

    fn scaled(rgb: [u8; 3]) -> [f32; 3] { rgb.map(|value| f32::from(value) / 16.0) }

    fn scaled_f32(rgb: [f32; 3]) -> [f32; 3] { rgb.map(|value| value / 16.0) }

    fn assert_close(pattern: BayerPattern, expected: [[f32; 3]; 9]) {
        for (actual, expected) in run(pattern).into_iter().zip(expected) {
            for (actual, expected) in actual.into_iter().zip(expected) {
                assert!(
                    (actual - expected).abs() < 1e-6,
                    "{pattern:?}: {actual} != {expected}"
                );
            }
        }
    }
}
