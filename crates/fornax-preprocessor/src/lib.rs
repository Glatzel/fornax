use fornax_core::error::PreprocessError;
use fornax_core::metadata::process::Levels;
use fornax_core::protocol::{IPreprocessor, NormalizedMosaicSource};
use fornax_core::{CroppedMosaic, MonoImage, NormalizedMosaic, Primitive};
use rayon::prelude::*;
pub struct Preprocessor;

/// A deferred normalized view over a cropped mosaic.
pub struct LazyNormalizedMosaic<'a, T: Primitive> {
    image: &'a CroppedMosaic<T>,
    levels: &'a Levels<T>,
}

impl<T> LazyNormalizedMosaic<'_, T>
where
    T: Primitive,
{
    /// Returns the normalized image dimensions.
    pub fn dimensions(&self) -> (u32, u32) { self.image.image().dimensions() }

    /// Normalizes one row into `out`.
    pub fn fill_row(&self, y: usize, out: &mut [T]) {
        let image = self.image.image();
        let width = usize::try_from(image.width()).expect("u32 fits usize");
        let start = y
            .checked_mul(width)
            .expect("normalized row offset does not overflow");
        out[..width].copy_from_slice(&image.as_raw()[start..start + width]);
        normalize_row(y, out, self.levels);
    }

    /// Materializes the normalized mosaic.
    pub fn collect(&self) -> Result<NormalizedMosaic<T>, PreprocessError>
    where
        T: Send + Sync,
    {
        let (width, height) = self.dimensions();
        let width_usize = usize::try_from(width).expect("u32 fits usize");
        let sample_count = width_usize
            .checked_mul(usize::try_from(height).expect("u32 fits usize"))
            .ok_or_else(|| PreprocessError::ImageTooLarge {
                width: width_usize,
                height: usize::try_from(height).expect("u32 fits usize"),
            })?;
        let mut samples = vec![T::zero(); sample_count];
        samples
            .par_chunks_mut(width_usize)
            .enumerate()
            .for_each(|(y, row)| self.fill_row(y, row));
        let image = MonoImage::from_raw(width, height, samples).ok_or_else(|| {
            PreprocessError::ImageTooLarge {
                width: width_usize,
                height: usize::try_from(height).expect("u32 fits usize"),
            }
        })?;
        Ok(NormalizedMosaic::new(image))
    }
}

impl<T> NormalizedMosaicSource<T> for LazyNormalizedMosaic<'_, T>
where
    T: Primitive,
{
    fn dimensions(&self) -> (u32, u32) { self.image.image().dimensions() }

    fn fill_row(&self, y: usize, out: &mut [T]) {
        let image = self.image.image();
        let width = usize::try_from(image.width()).expect("u32 fits usize");
        let start = y
            .checked_mul(width)
            .expect("normalized row offset does not overflow");
        out[..width].copy_from_slice(&image.as_raw()[start..start + width]);
        normalize_row(y, out, self.levels);
    }
}

impl Preprocessor {
    /// Creates a deferred normalized view without allocating a new image.
    pub fn lazy<'a, T>(
        &self,
        decoded: &'a CroppedMosaic<T>,
        levels: &'a Levels<T>,
    ) -> Result<LazyNormalizedMosaic<'a, T>, PreprocessError>
    where
        T: Primitive,
    {
        let (width, height) = decoded.image().dimensions();
        validate_levels(
            usize::try_from(width).expect("u32 fits usize"),
            usize::try_from(height).expect("u32 fits usize"),
            levels,
        )?;
        Ok(LazyNormalizedMosaic {
            image: decoded,
            levels,
        })
    }
}

impl<T> IPreprocessor<T> for Preprocessor
where
    T: Primitive,
{
    fn process(
        &self,
        decoded: CroppedMosaic<T>,
        levels: &Levels<T>,
    ) -> Result<NormalizedMosaic<T>, PreprocessError> {
        let image = decoded.into_image();
        let (width, height) = image.dimensions();
        let w = width as usize;
        let h = height as usize;

        #[cfg(debug_assertions)]
        validate_levels(w, h, levels)?;

        let mut raw = image.into_raw();
        let expected = w.checked_mul(h).ok_or(PreprocessError::ImageTooLarge {
            width: w,
            height: h,
        })?;
        if raw.len() < expected {
            return Err(PreprocessError::InvalidSampleCount {
                expected,
                actual: raw.len(),
            });
        }
        raw.truncate(expected);

        raw.par_chunks_exact_mut(w)
            .enumerate()
            .for_each(|(y, row)| normalize_row(y, row, levels));

        let normalized =
            MonoImage::from_raw(width, height, raw).expect("pixel count matches image dimensions");

        Ok(NormalizedMosaic::new(normalized))
    }
}

fn normalize_row<T>(y: usize, row: &mut [T], levels: &Levels<T>)
where
    T: Primitive,
{
    let pattern_width = levels.black.width() as usize;
    let pattern = levels.black.as_raw();
    let pattern_row = &pattern[(y % levels.black.height() as usize) * pattern_width
        ..(y % levels.black.height() as usize + 1) * pattern_width];
    let delta_v = levels.black_delta_v.get(y).copied().unwrap_or_else(T::zero);
    for (x, pixel) in row.iter_mut().enumerate() {
        let black = pattern_row[x % pattern_width]
            + levels.black_delta_h.get(x).copied().unwrap_or_else(T::zero)
            + delta_v;
        let range = levels.white - black;
        *pixel = if range <= T::zero() {
            T::zero()
        } else {
            let value = (*pixel - black) / range;
            if value > T::zero() { value } else { T::zero() }
        };
    }
}

#[cfg(debug_assertions)]
fn validate_levels<T>(
    width: usize,
    height: usize,
    levels: &Levels<T>,
) -> Result<(), PreprocessError>
where
    T: Primitive,
{
    if width == 0 || height == 0 {
        return Err(PreprocessError::EmptyImage);
    }
    if levels.black.width() == 0 || levels.black.height() == 0 {
        return Err(PreprocessError::InvalidBlackPattern);
    }
    if !levels.black_delta_h.is_empty() && levels.black_delta_h.len() != width {
        return Err(PreprocessError::BlackDeltaWidthMismatch {
            expected: width,
            actual: levels.black_delta_h.len(),
        });
    }
    if !levels.black_delta_v.is_empty() && levels.black_delta_v.len() != height {
        return Err(PreprocessError::BlackDeltaHeightMismatch {
            expected: height,
            actual: levels.black_delta_v.len(),
        });
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn levels(black: MonoImage<f32>) -> Levels<f32> {
        Levels {
            black,
            black_delta_h: Vec::new(),
            black_delta_v: Vec::new(),
            white: 1.0,
        }
    }

    #[test]
    fn normalizes_with_repeated_black_pattern() -> Result<(), PreprocessError> {
        let image = MonoImage::from_raw(3, 2, vec![0.1, 0.2, 0.3, 0.4, 0.5, 0.6])
            .expect("image dimensions");
        let black =
            MonoImage::from_raw(2, 2, vec![0.0, 0.1, 0.2, 0.3]).expect("black-level dimensions");
        let output = Preprocessor.process(CroppedMosaic::new(image), &levels(black))?;
        assert_eq!(output.image().dimensions(), (3, 2));
        assert!(output.image().as_raw().iter().all(|value| *value > 0.0));
        Ok(())
    }

    #[cfg(debug_assertions)]
    #[test]
    fn empty_black_pattern_returns_error() {
        let image = MonoImage::from_raw(1, 1, vec![1.0]).expect("image dimensions");
        let black = MonoImage::new(0, 0);
        let error = Preprocessor
            .process(CroppedMosaic::new(image), &levels(black))
            .expect_err("empty black pattern must fail");
        assert_eq!(error, PreprocessError::InvalidBlackPattern);
    }

    #[cfg(debug_assertions)]
    #[test]
    fn black_delta_lengths_are_checked() {
        let image = MonoImage::from_raw(2, 2, vec![1.0; 4]).expect("image dimensions");
        let black = MonoImage::from_raw(2, 2, vec![0.0; 4]).expect("black-level dimensions");
        let mut level_data = levels(black);
        level_data.black_delta_h = vec![0.0];
        let error = Preprocessor
            .process(CroppedMosaic::new(image), &level_data)
            .expect_err("mismatched horizontal delta must fail");
        assert_eq!(
            error,
            PreprocessError::BlackDeltaWidthMismatch {
                expected: 2,
                actual: 1
            }
        );

        let image = MonoImage::from_raw(2, 2, vec![1.0; 4]).expect("image dimensions");
        let black = MonoImage::from_raw(2, 2, vec![0.0; 4]).expect("black-level dimensions");
        let mut level_data = levels(black);
        level_data.black_delta_v = vec![0.0];
        let error = Preprocessor
            .process(CroppedMosaic::new(image), &level_data)
            .expect_err("mismatched vertical delta must fail");
        assert_eq!(
            error,
            PreprocessError::BlackDeltaHeightMismatch {
                expected: 2,
                actual: 1
            }
        );
    }

    #[test]
    fn cropped_image_ignores_retained_source_samples() -> Result<(), PreprocessError> {
        let mut image = MonoImage::from_raw(4, 2, (0..8).map(|value| value as f32).collect())
            .expect("image dimensions");
        image.crop_in_place(image::math::Rect {
            x: 1,
            y: 0,
            width: 3,
            height: 2,
        });
        let black = MonoImage::from_raw(2, 2, vec![0.0; 4]).expect("black-level dimensions");
        let output = Preprocessor.process(CroppedMosaic::new(image), &levels(black))?;
        assert_eq!(output.image().dimensions(), (3, 2));
        Ok(())
    }

    #[test]
    fn lazy_preprocessing_matches_eager_rows() -> Result<(), PreprocessError> {
        let image = MonoImage::from_raw(4, 3, (0_u16..12).map(f32::from).collect())
            .expect("image dimensions");
        let black = MonoImage::from_raw(2, 2, vec![0.0, 0.1, 0.2, 0.3]).expect("black dimensions");
        let level_data = Levels {
            black,
            black_delta_h: vec![0.01, 0.02, 0.03, 0.04],
            black_delta_v: vec![0.05, 0.06, 0.07],
            white: 12.0,
        };
        let eager = Preprocessor.process(CroppedMosaic::new(image.clone()), &level_data)?;
        let cropped = CroppedMosaic::new(image);
        let lazy = Preprocessor.lazy(&cropped, &level_data)?;

        for y in 0..3 {
            let mut row = vec![0.0; 4];
            lazy.fill_row(y, &mut row);
            assert_eq!(row, eager.image().as_raw()[y * 4..(y + 1) * 4].to_vec());
        }
        Ok(())
    }
}
