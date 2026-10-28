use std::marker::PhantomData;

use fornax_core::Primitive;
use fornax_core::error::PostProcessError;
use fornax_core::protocol::{IDemosaic, IPostProcessMetadata, IPostProcessor};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct PostProcessor<T, D>
where
    T: Primitive,
    D: IDemosaic<T>,
{
    pub demosaic: D,
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
        Ok(self
            .demosaic
            .demosaic(bayer_image, metadata.bayer_pattern())?)
    }
}
