use crate::error::PreprocessError;
use crate::metadata::Levels;
use crate::protocol::IPreprocessor;
use crate::{MonoImage, Primitive, RawImage};

#[derive(Clone, Debug)]
pub struct DecodedRaw<T: Primitive>(MonoImage<T>);

impl<T: Primitive> DecodedRaw<T> {
    pub const fn new(image: MonoImage<T>) -> Self { Self(image) }

    pub const fn image(&self) -> &MonoImage<T> { &self.0 }

    pub fn into_image(self) -> MonoImage<T> { self.0 }

    pub fn process<P>(
        self,
        preprocessor: &P,
        levels: &Levels<T>,
    ) -> Result<RawImage<T>, PreprocessError>
    where
        P: IPreprocessor<T>,
    {
        preprocessor.process(self, levels)
    }
}
