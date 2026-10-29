use crate::error::PostProcessError;
use crate::protocol::{IPostProcessMetadata, IPostProcessor};
use crate::{MonoImage, Primitive, RgbImage};

#[derive(Clone, Debug, PartialEq)]
pub struct RawImage<T>(MonoImage<T>)
where
    T: Primitive;

impl<T> RawImage<T>
where
    T: Primitive,
{
    pub const fn new(plane: MonoImage<T>) -> Self { Self(plane) }
    pub const fn image(&self) -> &MonoImage<T> { &self.0 }
    pub fn into_image(self) -> MonoImage<T> { self.0 }
    pub fn process<P, M>(self, processor: &P, metadata: &M) -> Result<RgbImage<T>, PostProcessError>
    where
        P: IPostProcessor<T, M>,
        M: IPostProcessMetadata<T>,
    {
        processor.process(self.image(), metadata)
    }
}
