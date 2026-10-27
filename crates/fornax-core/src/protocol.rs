use crate::error::{DecodeError, DemosaicError, PreprocessError};
use crate::metadata::{Levels, Metadata};
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
    fn preprocess(
        &self,
        decoded: DecodedRawActive<T>,
        levels: &Levels<T>,
    ) -> Result<RawImage<T>, PreprocessError>;
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
