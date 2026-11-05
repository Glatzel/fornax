use crate::error::{DecodeError, PostProcessError, PreprocessError};
use crate::metadata::dng::DngMetadata;
use crate::metadata::process::{Levels, ProcessMetadata};
use crate::{
    BayerPattern, CroppedMosaic, MonoImage, NormalizedMosaic, Primitive, RawMosaic, RgbImage,
};
pub trait IDecoder<I, T>: Send + Sync
where
    T: Primitive,
{
    fn decode(&self, input: I)
    -> Result<(RawMosaic, DngMetadata, ProcessMetadata<T>), DecodeError>;
    fn decode_active(
        &self,
        input: I,
    ) -> Result<(CroppedMosaic<T>, DngMetadata, ProcessMetadata<T>), DecodeError>;
}

pub trait IPreprocessor<T>: Sized
where
    T: Primitive,
{
    fn process(
        &self,
        decoded: CroppedMosaic<T>,
        levels: &Levels<T>,
    ) -> Result<NormalizedMosaic<T>, PreprocessError>;
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
