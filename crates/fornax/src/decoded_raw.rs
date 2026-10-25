use rayon::prelude::*;

use crate::alias::LumaImage;
use crate::error::{PreprocessError, RawValidationError};
use crate::metadata::{LevelsMetadata, Metadata, PreprocessMetadata, WhiteLevels};
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
        let left = area.x;
        let top = area.y;
        let width = area.width;
        let height = area.height;

        let pattern = metadata.preprocess.cfa.bayer_pattern;
        let black = metadata.preprocess.levels.black;
        let white = metadata.preprocess.levels.white;

        let cropped = image::imageops::crop_imm(&image, left, top, width, height).to_image();

        let pixels = cropped.into_raw();

        let white_at = |slot: usize| -> u32 {
            match white {
                WhiteLevels::Uniform(value) => value,
                WhiteLevels::PerSlot(values) => values[slot],
            }
        };

        let pixels: Vec<T> = pixels
            .into_par_iter()
            .enumerate()
            .map(|(index, pixel)| {
                let x = index as u32 % width;
                let y = index as u32 / width;

                // CFA slot in the original, uncropped sensor image.
                let slot = ((x + left) & 1) as usize + (((y + top) & 1) as usize * 2);

                let black_level = black.common.saturating_add(black.per_slot[slot]);
                let white_level = white_at(slot);

                let pixel = T::from(pixel).unwrap_or_else(T::zero);
                let black_level = T::from(black_level).unwrap_or_else(T::zero);
                let white_level = T::from(white_level).unwrap_or_else(T::zero);

                let range = white_level - black_level;

                if range <= T::zero() {
                    T::zero()
                } else {
                    ((pixel - black_level) / range).max(T::zero())
                }
            })
            .collect();

        let normalized = ImageBuffer::from_raw(width, height, pixels)
            .expect("pixel count matches image dimensions");

        RawImage::new(normalized)
    }
}
