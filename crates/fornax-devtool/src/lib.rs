use std::ops::Deref;
use std::path::{Path, PathBuf};

use generic_num::num;
use image::{ImageBuffer, Pixel, PixelWithColorType, Primitive};
use num_traits::Float;

pub fn setup() { clerk::init_log_with_level(clerk::LevelFilter::TRACE); }

pub fn root_dir() -> PathBuf {
    let root = PathBuf::from(std::env::var("CARGO_WORKSPACE_DIR").unwrap());
    assert!(root.is_dir());
    root
}

pub fn raw_file() -> PathBuf { root_dir().join("external/raw-images/images/colorchart-eos-7d.cr2") }
pub fn temp_dir() -> PathBuf { root_dir().join("temp") }
pub fn save_image<T, P, Q, C>(
    image: &ImageBuffer<P, C>,
    path: &Path,
    scale: T,
) -> mischief::Result<()>
where
    T: Primitive + Float,
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
