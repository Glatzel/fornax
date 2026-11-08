use fornax_core::error::PreprocessError;
use fornax_core::metadata::Levels;
use fornax_core::protocol::IPreprocessor;
use fornax_core::{CroppedMosaic, MonoImage, NormalizedMosaic, Primitive};
use generic_num::num;
use rayon::prelude::*;
pub struct Preprocessor;
impl<T> IPreprocessor<T> for Preprocessor
where
    T: Primitive,
{
    fn process(
        &self,
        decoded: CroppedMosaic<T>,
        levels: &Levels,
    ) -> Result<NormalizedMosaic<T>, PreprocessError> {
        let image = decoded.into_image();
        let (width, height) = image.dimensions();
        let (w, h) = (width as usize, height as usize);

        #[cfg(debug_assertions)]
        validate_levels(w, h, levels)?;

        let pw = levels.black.width() as usize;
        let ph = levels.black.height() as usize;
        let pattern: Vec<T> = levels.black.as_raw().iter().map(|v| num!(*v)).collect();
        let white: T = num!(levels.white);
        let zero = T::zero();

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

        let delta_h: Option<Vec<T>> = (!levels.black_delta_h.is_empty())
            .then(|| levels.black_delta_h.iter().map(|v| num!(*v)).collect());
        let delta_v: Option<Vec<T>> = (!levels.black_delta_v.is_empty())
            .then(|| levels.black_delta_v.iter().map(|v| num!(*v)).collect());
        let rows_per_band = band_rows(w, h, std::mem::size_of::<T>());
        // Full-width black row per pattern phase (ph rows, usually 2).
        // Folds in delta_h, so the inner loop has no modulo/cycle.
        let base: Vec<Vec<T>> = (0..ph)
            .map(|py| {
                (0..w)
                    .map(|x| {
                        let b = pattern[py * pw + x % pw];
                        match &delta_h {
                            Some(d) => b + d[x],
                            None => b,
                        }
                    })
                    .collect()
            })
            .collect();

        match delta_v {
            None => {
                // Fast path: black and 1/range are periodic -> precompute, no
                // division in loop. Invalid range => black=0, inv=0 => result
                // is 0 without a branch.
                let tables: Vec<(Vec<T>, Vec<T>)> = base
                    .iter()
                    .map(|row| {
                        let mut blk = Vec::with_capacity(w);
                        let mut inv = Vec::with_capacity(w);
                        for &b in row {
                            let range = white - b;
                            if range > zero {
                                blk.push(b);
                                inv.push(T::one() / range);
                            } else {
                                blk.push(zero);
                                inv.push(zero);
                            }
                        }
                        (blk, inv)
                    })
                    .collect();

                raw.par_chunks_mut(w * rows_per_band)
                    .enumerate()
                    .for_each(|(band, rows)| {
                        for (r, row) in rows.chunks_exact_mut(w).enumerate() {
                            let (blk, inv) = &tables[(band * rows_per_band + r) % ph];
                            for ((p, &b), &k) in row.iter_mut().zip(blk).zip(inv) {
                                let v = (*p - b) * k;
                                *p = if v > zero { v } else { zero };
                            }
                        }
                    });
            }
            Some(dv_all) => {
                raw.par_chunks_mut(w * rows_per_band)
                    .enumerate()
                    .for_each(|(band, rows)| {
                        for (r, row) in rows.chunks_exact_mut(w).enumerate() {
                            let y = band * rows_per_band + r;
                            let dv = dv_all[y];
                            for (p, &b0) in row.iter_mut().zip(&base[y % ph]) {
                                let b = b0 + dv;
                                let range = white - b;
                                let v = (*p - b) / range;
                                *p = if range > zero && v > zero { v } else { zero };
                            }
                        }
                    });
            }
        }
        let normalized =
            MonoImage::from_raw(width, height, raw).expect("pixel count matches image dimensions");
        Ok(NormalizedMosaic::new(normalized))
    }
}

fn band_rows(w: usize, h: usize, elem_size: usize) -> usize {
    let target_bytes = fornax_core::utils::l2_cache_bytes() / 4; // headroom for tables, shared L2, hybrid cores
    let by_bytes = (target_bytes / (w * elem_size).max(1)).max(1);

    // Aim for several bands per thread so rayon can load-balance.
    let threads = rayon::current_num_threads();
    let by_threads = h.div_ceil(threads * 4).max(1);

    by_bytes.min(by_threads)
}
#[cfg(debug_assertions)]
fn validate_levels(width: usize, height: usize, levels: &Levels) -> Result<(), PreprocessError> {
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

    fn levels(black: MonoImage<f64>) -> Levels {
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
}
