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
pub fn crop_and_cast<S, T>(
    mut image: MonoImage<S>,
    area: Rect,
) -> Result<CroppedMosaic<T>, DecodeError>
where
    S: image::Primitive + Send + Sync,
    T: Primitive,
{
    image.crop_in_place(area);
    let (width, height) = image.dimensions();
    let mut source = image.into_raw();
    source.truncate(width as usize * height as usize);
    let mut samples = fornax_large_alloc::zeroed_samples(source.len()).map_err(|error| {
        DecodeError::InvalidInput(format!("cropped mosaic allocation failed: {error}").into())
    })?;
    samples
        .par_iter_mut()
        .zip(source.into_par_iter())
        .for_each(|(output, value)| *output = num!(value));
    MonoImage::from_raw(width, height, samples)
        .map(CroppedMosaic::new)
        .ok_or_else(|| DecodeError::InvalidInput("cropped mosaic dimensions are invalid".into()))
}
