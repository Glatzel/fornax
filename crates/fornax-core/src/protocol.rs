use crate::error::{DecodeError, PostProcessError, PreprocessError};
use crate::metadata::dng::DngMetadata;
use crate::metadata::process::{Levels, ProcessMetadata};
use crate::{BayerPattern, DecodedRaw, MonoImage, MosaicData, Primitive, RawImage, RgbImage};
pub trait IDecoder<I, T>: Send + Sync
where
    T: Primitive,
{
    fn decode(
        &self,
        input: I,
    ) -> Result<(MosaicData, DngMetadata, ProcessMetadata<T>), DecodeError>;
    fn decode_active(
        &self,
        input: I,
    ) -> Result<(DecodedRaw<T>, DngMetadata, ProcessMetadata<T>), DecodeError>;
}

pub trait IPreprocessor<T>: Sized
where
    T: Primitive,
{
    fn process(
        &self,
        decoded: DecodedRaw<T>,
        levels: &Levels<T>,
    ) -> Result<RawImage<T>, PreprocessError>;
}
pub trait IPostProcessMetadata<T>: Sync
where
    T: Primitive,
{
    fn bayer_pattern(&self) -> BayerPattern;
    fn wb_coeffs(&self) -> &Option<(T, T)>;
}
pub trait IPostProcessor<T, M>
where
    T: Primitive,
    M: IPostProcessMetadata<T>,
{
    fn process(
        &self,
        bayer_image: &MonoImage<T>,
        metadata: &M,
    ) -> Result<RgbImage<T>, PostProcessError>;
}
