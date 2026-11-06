use crate::error::{DecodeError, PostProcessError, PreprocessError};
use crate::metadata::{ Levels, Matrix3x3, ProcessMetadata};
use crate::{
    BayerPattern, CroppedMosaic, MonoImage, NormalizedMosaic, Primitive, RawMosaic, RgbImage,
};
pub trait IDecoder<I, T>: Send + Sync
where
    T: Primitive,
    Self: Sized,
{
    fn decode(input: I) -> Result<Self, DecodeError>;
    fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError>;
    fn cropped_mosaic(&self) -> Result<CroppedMosaic<T>, DecodeError>;
    fn process_metadata(&self) -> Result<ProcessMetadata<T>, DecodeError>;
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
    fn color_matrix_1(&self) -> Option<&Matrix3x3> { None }
    fn color_matrix_2(&self) -> Option<&Matrix3x3> { None }
    fn forward_matrix_1(&self) -> Option<&Matrix3x3> { None }
    fn forward_matrix_2(&self) -> Option<&Matrix3x3> { None }
    fn camera_calibration_1(&self) -> Option<&Matrix3x3> { None }
    fn camera_calibration_2(&self) -> Option<&Matrix3x3> { None }
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
