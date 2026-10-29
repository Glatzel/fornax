mod demosaic;
use std::marker::PhantomData;

pub use demosaic::Demosaic;
use fornax_core::Primitive;
use fornax_core::error::PostProcessError;
use fornax_core::protocol::{IDemosaic, IPostProcessMetadata, IPostProcessor};
use rayon::iter::ParallelIterator;

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
        let mut result = self
            .demosaic
            .demosaic(bayer_image, metadata.bayer_pattern())?;
        if let Some(as_shot_neutral) = metadata.as_shot_neutral()
            && self.use_as_shot_neutral
        {
            result.par_enumerate_pixels_mut().for_each(|(_x, _y, rgb)| {
                rgb.0[0] = rgb.0[0] * as_shot_neutral.0;
                rgb.0[2] = rgb.0[2] * as_shot_neutral.1;
            });
        }
        Ok(result)
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
            _mark: Default::default(),
        }
    }
}
