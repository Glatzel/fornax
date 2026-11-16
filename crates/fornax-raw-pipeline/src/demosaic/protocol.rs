use std::any::Any;
use std::borrow::Cow;

use fornax_raw_core::error::DemosaicError;
use fornax_raw_core::{BayerPattern, MonoImage, Primitive, RgbImage};
use rayon::prelude::*;

/// Exposes demosaiced RGB rows without requiring a full output allocation.
///
/// # Examples
///
/// ```
/// use fornax_raw_core::{BayerPattern, MonoImage};
/// use fornax_raw_pipeline::{Demosaic, IDemosaic, LazyDemosaic};
///
/// let mosaic = MonoImage::from_raw(2, 2, vec![0.25_f32; 4]).unwrap();
/// let lazy = Demosaic::Linear
///     .demosaic(&mosaic, BayerPattern::RGGB)
///     .unwrap();
/// assert_eq!(lazy.dimensions(), (2, 2));
/// ```
pub trait LazyDemosaic<T: Primitive + Send + Sync>: Sync {
    /// Returns the RGB output dimensions in pixels.
    fn dimensions(&self) -> (u32, u32);

    /// Fills `out` with RGB rows starting at row `y0`.
    fn fill_rows(&self, y0: usize, out: &mut [T]);

    /// Shared by every algorithm: one parallel pass, optional fused per-row
    /// stage.
    fn collect_with(&self, post: impl Fn(usize, &mut [T]) + Sync) -> RgbImage<T>
    where
        Self: Sized,
    {
        let (width, height) = self.dimensions();
        let threads = rayon::current_num_threads();
        let rows_per_task = (height as usize / (threads * 4)).clamp(8, 128);

        let mut img = RgbImage::new(width, height);
        let row_stride = width as usize * 3;
        let raw_out: &mut [T] = &mut img;

        raw_out
            .par_chunks_mut(row_stride * rows_per_task)
            .enumerate()
            .for_each(|(i, chunk)| {
                let y0 = i * rows_per_task;
                self.fill_rows(y0, chunk);
                for (dy, row) in chunk.chunks_exact_mut(row_stride).enumerate() {
                    post(y0 + dy, row);
                }
            });
        img
    }

    /// Collects all rows into an RGB image.
    fn collect(&self) -> RgbImage<T>
    where
        Self: Sized,
    {
        self.collect_with(|_, _| {})
    }
}

pub(super) fn apply_post_in_place<T>(image: &mut RgbImage<T>, post: impl Fn(usize, &mut [T]) + Sync)
where
    T: Primitive + Send + Sync,
{
    let width = image.width() as usize;
    let height = image.height() as usize;
    let threads = rayon::current_num_threads();
    let rows_per_task = (height / (threads * 4)).clamp(8, 128);
    let row_stride = width * 3;

    image
        .as_mut()
        .par_chunks_mut(row_stride * rows_per_task)
        .enumerate()
        .for_each(|(i, chunk)| {
            let y0 = i * rows_per_task;
            for (dy, row) in chunk.chunks_exact_mut(row_stride).enumerate() {
                post(y0 + dy, row);
            }
        });
}

pub(super) fn normalized_f32<'a, T>(image: &'a MonoImage<T>) -> Cow<'a, [f32]>
where
    T: Primitive,
{
    if let Some(samples) = (image.as_raw() as &dyn Any).downcast_ref::<Vec<f32>>() {
        Cow::Borrowed(samples)
    } else {
        Cow::Owned(
            image
                .as_raw()
                .iter()
                .map(|sample| sample.to_f32().unwrap_or_default().clamp(0.0, 1.0))
                .collect(),
        )
    }
}

pub(super) fn f32_codes_into<T>(image: RgbImage<T>) -> Vec<f32>
where
    T: Primitive,
{
    let values: Box<dyn Any> = Box::new(image.into_raw());
    if std::any::TypeId::of::<T>() == std::any::TypeId::of::<f32>() {
        let mut values = values
            .downcast::<Vec<f32>>()
            .expect("the image payload is f32");
        values.par_iter_mut().for_each(|value| *value *= 65_535.0);
        return *values;
    }
    let source = values
        .downcast::<Vec<T>>()
        .expect("the image payload retains its generic sample type");
    source
        .into_iter()
        .map(|sample| sample.to_f32().unwrap_or_default() * 65_535.0)
        .collect()
}

pub(super) fn dense_from_f32_owned<T>(width: u32, height: u32, mut values: Vec<f32>) -> RgbImage<T>
where
    T: Primitive,
{
    values
        .par_iter_mut()
        .for_each(|value| *value = value.clamp(0.0, 1.0));
    let boxed: Box<dyn Any> = Box::new(values);
    if std::any::TypeId::of::<T>() == std::any::TypeId::of::<f32>() {
        let values = *boxed.downcast::<Vec<f32>>().expect("the workspace is f32");
        let cast = Box::new(values) as Box<dyn Any>;
        let values = *cast
            .downcast::<Vec<T>>()
            .expect("the f32 workspace matches the requested sample type");
        return RgbImage::from_raw(width, height, values)
            .expect("demosaic output dimensions match the source image");
    }
    let values = boxed.downcast::<Vec<f32>>().expect("the workspace is f32");
    RgbImage::from_raw(
        width,
        height,
        values
            .into_iter()
            .map(|value| generic_num::num!(value))
            .collect(),
    )
    .expect("demosaic output dimensions match the source image")
}

/// Builds a lazy demosaic result from a normalized Bayer mosaic.
///
/// # Errors
///
/// Returns [`DemosaicError`] when the mosaic geometry or Bayer pattern cannot
/// be processed by the implementation.
///
/// # Examples
///
/// ```
/// use fornax_raw_core::{BayerPattern, MonoImage};
/// use fornax_raw_pipeline::{Demosaic, IDemosaic, LazyDemosaic};
///
/// let mosaic = MonoImage::from_raw(2, 2, vec![0.25_f32; 4]).unwrap();
/// let lazy = Demosaic::Linear
///     .demosaic(&mosaic, BayerPattern::RGGB)
///     .unwrap();
/// assert_eq!(lazy.dimensions(), (2, 2));
/// ```
pub trait IDemosaic<T: Primitive + Send + Sync> {
    /// Demosaics `bayer` using the channel layout described by `pattern`.
    fn demosaic<'a>(
        &self,
        bayer: &'a MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<impl LazyDemosaic<T> + 'a, DemosaicError>;
}
