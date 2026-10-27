use fornax_core::error::PreprocessError;
use fornax_core::metadata::Levels;
use fornax_core::{DecodedRawActive, IPreprocessor, MonoImage, Primitive, RawImage};
use rayon::prelude::*;
pub struct Preprocessor;
impl<T> IPreprocessor<T> for Preprocessor
where
    T: Primitive,
{
    fn preprocess(
        &self,
        decoded: DecodedRawActive<T>,
        levels: &Levels<T>,
    ) -> Result<RawImage<T>, PreprocessError> {
        let image = decoded.into_image();
        let (width, height) = image.dimensions();
        let w = width as usize;
        let h = height as usize;

        #[cfg(debug_assertions)]
        validate_levels(w, h, levels)?;

        let pattern_w = levels.black.width() as usize;
        let pattern_h = levels.black.height() as usize;
        let pattern = levels.black.as_raw();

        let zeros = |n: usize| vec![T::zero(); n];
        let delta_h = if levels.black_delta_h.is_empty() {
            &zeros(w)
        } else {
            &levels.black_delta_h
        };
        let delta_v = if levels.black_delta_v.is_empty() {
            &zeros(h)
        } else {
            &levels.black_delta_v
        };
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
            .for_each(|(y, row)| {
                let py = y % pattern_h;
                let pattern_row = &pattern[py * pattern_w..(py + 1) * pattern_w];
                let dv = delta_v[y];

                for ((px, &dh), &pat) in row
                    .iter_mut()
                    .zip(delta_h.iter())
                    .zip(pattern_row.iter().cycle())
                {
                    let black_level = pat + dh + dv;
                    let range = levels.white - black_level;

                    *px = if range <= T::zero() {
                        T::zero()
                    } else {
                        let value = (*px - black_level) / range;
                        if value > T::zero() { value } else { T::zero() }
                    };
                }
            });

        let normalized =
            MonoImage::from_raw(width, height, raw).expect("pixel count matches image dimensions");

        Ok(RawImage::new(normalized))
    }
}

#[cfg(debug_assertions)]
fn validate_levels<T: Primitive>(
    width: usize,
    height: usize,
    levels: &Levels<T>,
) -> Result<(), PreprocessError> {
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
        let output = Preprocessor.preprocess(DecodedRawActive::new(image), &levels(black))?;
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
            .preprocess(DecodedRawActive::new(image), &levels(black))
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
            .preprocess(DecodedRawActive::new(image), &level_data)
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
            .preprocess(DecodedRawActive::new(image), &level_data)
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
        let output = Preprocessor.preprocess(DecodedRawActive::new(image), &levels(black))?;
        assert_eq!(output.image().dimensions(), (3, 2));
        Ok(())
    }
}
