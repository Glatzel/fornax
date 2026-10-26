use image::math::Rect;
use rayon::prelude::*;

use crate::alias::MonoImage;
use crate::metadata::Levels;
use crate::{Primitive, RawImage};
#[derive(Clone, Debug)]
pub struct DecodedRawFull<T>(MonoImage<T>)
where
    T: Primitive;

impl<T> DecodedRawFull<T>
where
    T: Primitive,
{
    pub fn new(image: MonoImage<T>) -> Self { Self(image) }

    pub const fn image(&self) -> &MonoImage<T> { &self.0 }

    pub fn active_area(mut self, active_area: Rect) -> DecodedRawActive<T> {
        self.0.crop_in_place(active_area);
        DecodedRawActive(self.0)
    }
}

#[derive(Clone, Debug)]
pub struct DecodedRawActive<T>(MonoImage<T>)
where
    T: Primitive;
impl<T> DecodedRawActive<T>
where
    T: Primitive,
{
    pub fn new(image: MonoImage<T>) -> Self { Self(image) }

    pub const fn image(&self) -> &MonoImage<T> { &self.0 }

    pub fn preprocess(self, levels: Levels<T>) -> RawImage<T> {
        let Levels {
            black,
            black_delta_h,
            black_delta_v,
            white,
        } = levels;

        let image = self.0;
        let (width, height) = image.dimensions();
        let w = width as usize;
        let h = height as usize;

        let pattern_w = black.width() as usize;
        let pattern_h = black.height() as usize;
        debug_assert!(
            pattern_w > 0 && pattern_h > 0,
            "black pattern must not be empty"
        );
        let pattern = black.as_raw();

        // Empty delta vectors mean "no adjustment".
        let zeros = |n: usize| vec![T::zero(); n];
        let delta_h = if black_delta_h.is_empty() {
            zeros(w)
        } else {
            black_delta_h
        };
        let delta_v = if black_delta_v.is_empty() {
            zeros(h)
        } else {
            black_delta_v
        };
        debug_assert_eq!(delta_h.len(), w, "black_delta_h must match image width");
        debug_assert_eq!(delta_v.len(), h, "black_delta_v must match image height");

        let mut raw = image.into_raw();

        raw.par_chunks_exact_mut(w)
            .enumerate()
            .for_each(|(y, row)| {
                let py = y % pattern_h;
                let pattern_row = &pattern[py * pattern_w..(py + 1) * pattern_w];
                let dv = delta_v[y];

                for ((px, &dh), &pat) in row
                    .iter_mut()
                    .zip(delta_h.iter())
                    .zip(pattern_row.iter().cycle())
                {
                    let black_level = pat + dh + dv;
                    let range = white - black_level;

                    *px = if range <= T::zero() {
                        T::zero()
                    } else {
                        let value = (*px - black_level) / range;
                        if value > T::zero() { value } else { T::zero() }
                    };
                }
            });

        let normalized =
            MonoImage::from_raw(width, height, raw).expect("pixel count matches image dimensions");

        RawImage::new(normalized)
    }
}
