use fornax_core::error::DemosaicError;
use fornax_core::{BayerPattern, MonoImage, Primitive, RgbImage};
use rayon::prelude::*;

/// Exposes demosaiced RGB rows without requiring a full output allocation.
///
/// # Examples
///
/// ```
/// use fornax_core::{BayerPattern, MonoImage};
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
/// use fornax_core::{BayerPattern, MonoImage};
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
