use rayon::prelude::*;

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
pub trait LazyDemosaic<T: Primitive + Send + Sync>: Sync {
    fn dimensions(&self) -> (u32, u32);
    fn fill_rows(&self, y0: usize, out: &mut [T]);
    /// Shared by every algorithm: one parallel pass, optional fused per-row
    /// stage.
    fn collect_with(&self, post: impl Fn(usize, &mut [T]) + Sync) -> RgbImage<T>
    where
        Self: Sized,
    {
        const ROWS_PER_TASK: usize = 16;
        let (width, height) = self.dimensions();
        let mut img = RgbImage::new(width, height);
        let row_stride = width as usize * 3;
        let raw_out: &mut [T] = &mut img;

        raw_out
            .par_chunks_mut(row_stride * ROWS_PER_TASK)
            .enumerate()
            .for_each(|(i, chunk)| {
                let y0 = i * ROWS_PER_TASK;
                self.fill_rows(y0, chunk);
                for (dy, row) in chunk.chunks_exact_mut(row_stride).enumerate() {
                    post(y0 + dy, row);
                }
            });
        img
    }

    fn collect(&self) -> RgbImage<T>
    where
        Self: Sized,
    {
        self.collect_with(|_, _| {})
    }
}
pub trait IDemosaic<T: Primitive + Send + Sync> {
    fn demosaic<'a>(
        &self,
        bayer: &'a MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<impl LazyDemosaic<T> + 'a, DemosaicError>;
}
