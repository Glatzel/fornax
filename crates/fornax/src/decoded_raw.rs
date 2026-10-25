use image::ImageBuffer;
use rayon::prelude::*;

use crate::alias::LumaImage;
use crate::metadata::{Metadata, WhiteLevels};
use crate::{Primitive, RawImage};
#[derive(Clone, Debug, PartialEq)]
pub struct DecodedRaw<T>
where
    T: Primitive,
{
    image: LumaImage<T>,
    metadata: Metadata,
}

impl<T> DecodedRaw<T>
where
    T: Primitive,
{
    pub fn new(image: LumaImage<T>, metadata: Metadata) -> Self { Self { image, metadata } }

    pub const fn image(&self) -> &LumaImage<T> { &self.image }

    pub const fn metadata(&self) -> &Metadata { &self.metadata }

    pub fn into_parts(self) -> (LumaImage<T>, Metadata) { (self.image, self.metadata) }

    pub fn preprocess(self) -> RawImage<T> {
        let Self { image, metadata } = self;

        let area = metadata.preprocess.active_area;
        let (left, top) = (area.x as usize, area.y as usize);
        let (width, height) = (area.width, area.height);
        let w = width as usize;

        let black = metadata.preprocess.levels.black;
        let white = metadata.preprocess.levels.white;

        // Per-CFA-slot (black, range), computed once instead of per pixel.
        let slots: [(T, T); 4] = std::array::from_fn(|slot| {
            let black_level = black.common.saturating_add(black.per_slot[slot]);
            let white_level = match white {
                WhiteLevels::Uniform(value) => value,
                WhiteLevels::PerSlot(values) => values[slot],
            };
            let black_t = T::from(black_level).unwrap_or_else(T::zero);
            let white_t = T::from(white_level).unwrap_or_else(T::zero);
            (black_t, white_t - black_t)
        });

        let src_stride = image.width() as usize;
        let src = image.as_raw();

        let mut out = vec![T::zero(); w * height as usize];

        out.par_chunks_exact_mut(w)
            .enumerate()
            .for_each(|(y, dst_row)| {
                let sy = y + top;
                let start = sy * src_stride + left;
                let src_row = &src[start..start + w];
                let row_slot = (sy & 1) * 2;

                for (x, (dst, &pixel)) in dst_row.iter_mut().zip(src_row).enumerate() {
                    // CFA slot in the original, uncropped sensor image.
                    let slot = ((x + left) & 1) + row_slot;
                    let (black_level, range) = slots[slot];

                    *dst = if range <= T::zero() {
                        T::zero()
                    } else {
                        let pixel = T::from(pixel).unwrap_or_else(T::zero);
                        ((pixel - black_level) / range).max(T::zero())
                    };
                }
            });

        let normalized = ImageBuffer::from_raw(width, height, out)
            .expect("pixel count matches image dimensions");

        RawImage::new(normalized)
    }
}
