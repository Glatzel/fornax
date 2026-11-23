mod bench;
pub mod canon_crx_audit;
mod dirs;

use std::ops::Deref;
use std::path::Path;

pub use bench::*;
pub use dirs::*;
use generic_num::num;
use image::{ImageBuffer, Pixel, PixelWithColorType, Primitive as ImagePrimitive};
use num_traits::Float;

pub fn setup() { clerk::init_log_with_level(clerk::LevelFilter::TRACE); }

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
