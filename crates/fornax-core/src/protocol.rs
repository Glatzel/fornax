use crate::error::{DecodeError, PostProcessError, PreprocessError};
use crate::metadata::process::{Levels, ProcessMetadata};
use crate::{
    BayerPattern, CroppedMosaic, MonoImage, NormalizedMosaic, Primitive, RawMosaic, RgbImage,
};
pub trait IDecoder<I, T>: Send + Sync
where
    T: Primitive,
    Self: Sized,
{
    type Metadata;

    fn decode(input: I) -> Result<Self, DecodeError>;
    fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError>;
    fn cropped_mosaic(&self) -> Result<CroppedMosaic<T>, DecodeError>;
    fn dng_metadata(&self) -> Result<Self::Metadata, DecodeError>;
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

/// A normalized mosaic that can be consumed one row at a time.
pub trait NormalizedMosaicSource<T>: Sync
where
    T: Primitive,
{
    fn dimensions(&self) -> (u32, u32);
    fn fill_row(&self, y: usize, out: &mut [T]);

    /// Materializes the source when a consumer needs random access.
    fn collect_source(&self) -> Result<NormalizedMosaic<T>, PreprocessError> {
        let (width, height) = self.dimensions();
        let width_usize = usize::try_from(width).expect("u32 fits usize");
        let height_usize = usize::try_from(height).expect("u32 fits usize");
        let sample_count =
            width_usize
                .checked_mul(height_usize)
                .ok_or(PreprocessError::ImageTooLarge {
                    width: width_usize,
                    height: height_usize,
                })?;
        let mut samples = vec![T::zero(); sample_count];
        for (y, row) in samples.chunks_exact_mut(width_usize).enumerate() {
            self.fill_row(y, row);
        }
        let image =
            MonoImage::from_raw(width, height, samples).ok_or(PreprocessError::ImageTooLarge {
                width: width_usize,
                height: height_usize,
            })?;
        Ok(NormalizedMosaic::new(image))
    }
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
