use crate::error::{DecodeError, DemosaicError, PostProcessError, PreprocessError};
use crate::metadata::{Levels, Metadata, WhiteBalance};
use crate::{
    BayerPattern, DecodedRawActive, DecodedRawFull, MonoImage, Primitive, RawImage, RgbImage,
};
pub trait IDecoder<I: ?Sized, T>: Send + Sync
where
    T: Primitive,
{
    fn decode_full(&self, input: &I) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError>;
    fn decode_active(&self, input: &I) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError>;
}

pub trait IPreprocessor<T>: Sized
where
    T: Primitive,
{
    fn process(
        &self,
        decoded: DecodedRawActive<T>,
        levels: &Levels<T>,
    ) -> Result<RawImage<T>, PreprocessError>;
}
pub trait IPostProcessMetadata<T>: Sync
where
    T: Primitive,
{
    fn bayer_pattern(&self) -> BayerPattern;
    fn white_balance(&self) -> &Option<WhiteBalance<T>>;
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

pub trait IDemosaic<T>
where
    T: Primitive,
{
    fn demosaic(
        &self,
        bayer_image: &MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<RgbImage<T>, DemosaicError>;
}
