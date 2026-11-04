mod demosaic;
pub mod protocol;

use std::marker::PhantomData;

pub use demosaic::{Demosaic, LmmseMode};
use fornax_core::Primitive;
use fornax_core::error::PostProcessError;
use fornax_core::protocol::{IPostProcessMetadata, IPostProcessor};

use crate::protocol::{IDemosaic, LazyDemosaic};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PostProcessor<T, D>
where
    T: Primitive,
    D: IDemosaic<T>,
{
    pub demosaic: D,
    pub use_as_shot_neutral: bool,
    pub _mark: PhantomData<T>,
}
impl<T, D, M> IPostProcessor<T, M> for PostProcessor<T, D>
where
    T: Primitive,
    M: IPostProcessMetadata<T>,
    D: IDemosaic<T>,
{
    fn process(
        &self,
        bayer_image: &fornax_core::MonoImage<T>,
        metadata: &M,
    ) -> Result<fornax_core::RgbImage<T>, PostProcessError> {
        let result = self
            .demosaic
            .demosaic(bayer_image, metadata.bayer_pattern())?;
        if let Some(wb) = metadata.white_balance()
            && self.use_as_shot_neutral
        {
            let (r_mul, b_mul) = wb.multipliers();
            return Ok(result.collect_with(|_y, row| {
                for px in row.as_chunks_mut::<3>().0 {
                    px[0] = px[0] * r_mul;
                    px[2] = px[2] * b_mul;
                }
            }));
        }
        Ok(result.collect())
    }
}

impl<T> Default for PostProcessor<T, Demosaic>
where
    T: Primitive,
{
    fn default() -> Self {
        Self {
            demosaic: Demosaic::Linear,
            use_as_shot_neutral: true,
            _mark: PhantomData,
        }
    }
}

#[cfg(test)]
mod tests {
    use fornax_core::metadata::{Levels, Metadata, WhiteBalance};
    use fornax_core::{BayerPattern, MonoImage};
    use image::math::Rect;

    use super::*;

    #[test]
    fn white_balance_multiplies_blue_from_its_own_sample() -> mischief::Result<()> {
        let image = MonoImage::from_raw(8, 8, vec![0.1_f32; 8 * 8]).unwrap();
        let metadata = Metadata {
            active_area: Rect {
                x: 0,
                y: 0,
                width: 8,
                height: 8,
            },
            default_crop: Rect {
                x: 0,
                y: 0,
                width: 8,
                height: 8,
            },
            levels: Levels {
                black: MonoImage::from_raw(1, 1, vec![0.0_f32]).unwrap(),
                black_delta_h: Vec::new(),
                black_delta_v: Vec::new(),
                white: 1.0,
            },
            bayer_pattern: BayerPattern::RGGB,
            white_balance: Some(WhiteBalance::AsShotNeutral(0.5, 1.0, 0.25)),
        };
        let processor = PostProcessor::<f32, _> {
            demosaic: Demosaic::Linear,
            use_as_shot_neutral: true,
            _mark: PhantomData,
        };

        let output = processor.process(&image, &metadata)?;
        let pixel = output.get_pixel(4, 4);

        assert!((pixel[0] - 0.2).abs() < 1.0e-6, "{pixel:?}");
        assert!((pixel[1] - 0.1).abs() < 1.0e-6, "{pixel:?}");
        assert!((pixel[2] - 0.4).abs() < 1.0e-6, "{pixel:?}");
        Ok(())
    }
}
