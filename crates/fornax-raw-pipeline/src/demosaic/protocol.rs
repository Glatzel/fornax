use fornax_raw_core::error::DemosaicError;
use fornax_raw_core::{BayerPattern, MonoImage, Primitive, RgbImage};
use num_traits::cast;
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

pub(super) fn linear_f32<T>(image: &MonoImage<T>) -> Result<Vec<f32>, DemosaicError>
where
    T: Primitive,
{
    image
        .as_raw()
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            cast::<T, f32>(*sample).ok_or_else(|| {
                DemosaicError::InvalidInput(
                    format!("demosaic sample {index} does not fit the f32 kernel").into(),
                )
            })
        })
        .collect()
}

pub(super) fn dense_from_f32_workspace<T>(width: u32, height: u32, values: Vec<f32>) -> RgbImage<T>
where
    T: Primitive,
{
    let values = values
        .into_iter()
        .map(|value| cast::<f32, T>(value).expect("f32 fits every supported Primitive"))
        .collect();
    RgbImage::from_raw(width, height, values)
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

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_f32_preserves_signed_hdr_values() {
        let input = vec![-2.0_f64, -0.25, 0.0, 1.0, 4.0, 1.0e6];
        let image = MonoImage::from_raw(6, 1, input.clone()).unwrap();
        let output = linear_f32(&image).unwrap();
        assert_eq!(output.len(), input.len());
        for (actual, expected) in output.iter().zip(input) {
            assert!((f64::from(*actual) - expected).abs() <= expected.abs().max(1.0) * 1.0e-7);
        }
    }

    #[test]
    fn dense_f32_workspace_preserves_signed_hdr_values() {
        let values = vec![
            -2.0_f32, 0.25, 4.0, 1.0e6, 1.0, -0.5, 2.0, 8.0, 16.0, 32.0, 64.0, 128.0,
        ];
        let image = dense_from_f32_workspace::<f64>(4, 1, values.clone());
        assert_eq!(
            image.into_raw(),
            values.into_iter().map(f64::from).collect::<Vec<_>>()
        );
    }
}
