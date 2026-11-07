use crate::error::{DecodeError, PostProcessError, PreprocessError};
use crate::metadata::{ColorCalibration, ColorTransform, Levels, ProcessMetadata};
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
    fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError>;
}

pub trait IPreprocessor<T>: Sized
where
    T: Primitive,
{
    fn process(
        &self,
        decoded: CroppedMosaic<T>,
        levels: &Levels,
    ) -> Result<NormalizedMosaic<T>, PreprocessError>;
}
pub trait IPostProcessMetadata: Sync {
    fn bayer_pattern(&self) -> BayerPattern;

    /// Normalized color calibration (white balance, profiles, camera -> XYZ
    /// D50).
    fn color_calibration(&self) -> &ColorCalibration;

    /// Black/white levels; drop this method (and `T`) if the trait is
    /// color-only.
    fn levels(&self) -> &Levels;

    /// Convenience: build the transforms for a white point.
    fn resolve_color(&self) -> ColorTransform {
        self.color_calibration().resolve()
    }
}
pub trait IPostProcessor<T, M>
where
    T: Primitive,
    M: IPostProcessMetadata,
{
    fn process(
        &self,
        bayer_image: &MonoImage<T>,
        metadata: &M,
    ) -> Result<RgbImage<T>, PostProcessError>;
}
