use generic_num::num;
use image::math::Rect;
use rayon::prelude::*;

use crate::error::DecodeError;
use crate::{CroppedMosaic, MonoImage, Primitive};

/// Crops an image to `area` and converts every retained sample to `T`.
///
/// The output allocation goes through the shared large-allocation policy.
///
/// # Errors
///
/// Returns [`DecodeError::InvalidInput`] if the requested crop cannot form an
/// image or if the output layout overflows.
#[doc(hidden)]
pub fn crop_and_cast<S, T>(image: MonoImage<S>, area: Rect) -> Result<CroppedMosaic<T>, DecodeError>
where
    S: image::Primitive + Send + Sync,
    T: Primitive,
{
    let (src_w, src_h) = image.dimensions();
    let (x, y) = (area.x as usize, area.y as usize);
    let (w, h) = (area.width as usize, area.height as usize);

    if w == 0 || h == 0 || x + w > src_w as usize || y + h > src_h as usize {
        return Err(DecodeError::InvalidInput(
            "crop area is out of bounds".into(),
        ));
    }

    let stride = src_w as usize;
    let source: &[S] = image.as_raw(); // borrow, no copy

    let mut samples = fornax_large_alloc::zeroed_samples(w * h).map_err(|error| {
        DecodeError::InvalidInput(format!("cropped mosaic allocation failed: {error}").into())
    })?;

    samples
        .par_chunks_exact_mut(w)
        .enumerate()
        .for_each(|(row, out)| {
            let start = (y + row) * stride + x;
            let src = &source[start..start + w];
            for (o, &v) in out.iter_mut().zip(src) {
                *o = num!(v);
            }
        });

    drop(image);
    MonoImage::from_raw(w as u32, h as u32, samples)
        .map(CroppedMosaic::new)
        .ok_or_else(|| DecodeError::InvalidInput("cropped mosaic dimensions are invalid".into()))
}
