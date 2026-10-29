mod demosaic;

use std::marker::PhantomData;

pub use demosaic::Demosaic;
use fornax_core::Primitive;
use fornax_core::error::PostProcessError;
use fornax_core::protocol::{IDemosaic, IPostProcessMetadata, IPostProcessor, LazyDemosaic};

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
                for px in row.chunks_exact_mut(3) {
                    px[0] = px[0] * r_mul;
                    px[2] = px[0] * b_mul;
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
