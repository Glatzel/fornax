mod bench;
mod dirs;

use std::ops::Deref;
use std::path::Path;

pub use bench::*;
pub use dirs::*;
use fornax_core::{MonoImage, Primitive};
use generic_num::num;
use image::{ImageBuffer, Pixel, PixelWithColorType, Primitive as ImagePrimitive};
use num_traits::Float;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

pub fn setup() { clerk::init_log_with_level(clerk::LevelFilter::TRACE); }

pub fn synthetic_mono_image<T>((width, height): (u32, u32), upper: f64) -> MonoImage<T>
where
    T: Primitive,
{
    assert!(upper.is_finite() && upper > 0.0);
    let mut rng = StdRng::seed_from_u64(12_345);
    let pixels = usize::try_from(u64::from(width) * u64::from(height))
        .expect("benchmark input exceeds platform memory");
    let samples = (0..pixels)
        .map(|_| num!(rng.random_range(0.0..upper)))
        .collect();
    MonoImage::from_raw(width, height, samples).expect("benchmark dimensions match sample count")
}

pub fn save_image<T, P, Q, C>(
    image: &ImageBuffer<P, C>,
    path: &Path,
    scale: T,
) -> mischief::Result<()>
where
    T: ImagePrimitive + Float,
    P: Pixel<Subpixel = T>,
    Q: Pixel<Subpixel = u16> + PixelWithColorType,
    C: Deref<Target = [P::Subpixel]>,
    ImageBuffer<Q, Vec<u16>>: image::GenericImageView,
{
    std::fs::create_dir_all(path.parent().unwrap())?;
    let (width, height) = image.dimensions();
    ImageBuffer::<Q, Vec<u16>>::from_raw(
        width,
        height,
        image
            .as_raw()
            .iter()
            .map(|p| {
                (*p * scale)
                    .round()
                    .min(num!(u16::MAX))
                    .max(num!(0))
                    .to_u16()
                    .unwrap()
            })
            .collect(),
    )
    .ok_or_else(|| std::io::Error::other("f32 plane dimensions are inconsistent"))?
    .save(path)?;
    Ok(())
}
