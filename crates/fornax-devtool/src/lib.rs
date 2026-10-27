use std::ops::Deref;
use std::path::{Path, PathBuf};

use fornax_core::{MonoImage, Primitive};
use generic_num::num;
use image::{ImageBuffer, Pixel, PixelWithColorType, Primitive as ImagePrimitive};
use num_traits::Float;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchSampleBits {
    B32,
    B64,
}

impl BenchSampleBits {
    pub const fn name(self) -> &'static str {
        match self {
            Self::B32 => "f32",
            Self::B64 => "f64",
        }
    }
}

pub fn setup() { clerk::init_log_with_level(clerk::LevelFilter::TRACE); }

pub fn root_dir() -> PathBuf {
    let root = PathBuf::from(std::env::var("CARGO_WORKSPACE_DIR").unwrap());
    assert!(root.is_dir());
    root
}

pub fn raw_file() -> PathBuf { root_dir().join("external/raw-images/images/colorchart-eos-7d.cr2") }
pub fn temp_dir() -> PathBuf { root_dir().join("temp") }

/// Returns benchmark pixel-count labels and dimensions configured by
/// `FORNAX_BENCH_SIZE`.
///
/// The environment variable accepts one short edge or a comma-separated list.
/// The default is `1000,2000`; dimensions are landscape 3:2 images.
pub fn bench_size() -> Vec<(String, (u32, u32))> {
    std::env::var("FORNAX_BENCH_SIZE")
        .unwrap_or_else(|_| "1000,2000".to_owned())
        .split(',')
        .map(str::trim)
        .map(|short_edge| {
            let short_edge = short_edge.parse::<u32>().unwrap_or_else(|_| {
                panic!(
                    "unknown FORNAX_BENCH_SIZE value {short_edge:?}; expected a positive integer"
                )
            });
            assert!(
                short_edge > 0,
                "unknown FORNAX_BENCH_SIZE value {short_edge:?}; expected a positive integer"
            );
            let long_edge = (u64::from(short_edge) * 3).div_ceil(2);
            let width = u32::try_from(long_edge).unwrap_or_else(|_| {
                panic!("FORNAX_BENCH_SIZE value {short_edge:?} exceeds supported dimensions")
            });
            let pixels = u64::from(width) * u64::from(short_edge);
            (
                format!("{:.2}MP", pixels as f64 / 1_000_000.0),
                (width, short_edge),
            )
        })
        .collect()
}

/// Returns benchmark sample bit depths configured by `FORNAX_BENCH_BITS`.
///
/// The environment variable accepts `32`, `64`, or a comma-separated list.
/// The default benchmarks both sample types.
pub fn bench_bits() -> Vec<BenchSampleBits> {
    std::env::var("FORNAX_BENCH_BITS")
        .unwrap_or_else(|_| "32,64".to_owned())
        .split(',')
        .map(str::trim)
        .map(|bits| match bits {
            "32" => BenchSampleBits::B32,
            "64" => BenchSampleBits::B64,
            _ => panic!("unknown FORNAX_BENCH_BITS value {bits:?}; expected 32, 64, or a comma-separated list"),
        })
        .collect()
}

pub fn bench_mono_image<T>((width, height): (u32, u32), upper: f64) -> MonoImage<T>
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




#[cfg(test)]
mod tests {
    use super::*;

    static BENCH_ENV_LOCK: std::sync::Mutex<()> = std::sync::Mutex::new(());

    #[test]
    fn benchmark_configuration_reads_environment() {
        let _guard = BENCH_ENV_LOCK
            .lock()
            .unwrap_or_else(|poisoned| poisoned.into_inner());
        let previous_size = std::env::var_os("FORNAX_BENCH_SIZE");
        let previous_bits = std::env::var_os("FORNAX_BENCH_BITS");

        unsafe {
            std::env::remove_var("FORNAX_BENCH_SIZE");
            std::env::remove_var("FORNAX_BENCH_BITS");
        }
        assert_eq!(
            bench_size(),
            vec![
                ("1.50MP".to_owned(), (1_500, 1_000)),
                ("6.00MP".to_owned(), (3_000, 2_000)),
            ]
        );
        assert_eq!(
            bench_bits(),
            vec![BenchSampleBits::B32, BenchSampleBits::B64]
        );

        unsafe { std::env::set_var("FORNAX_BENCH_SIZE", "1500, 2001") };
        unsafe { std::env::set_var("FORNAX_BENCH_BITS", "64, 32") };
        assert_eq!(
            bench_size(),
            vec![
                ("3.38MP".to_owned(), (2_250, 1_500)),
                ("6.01MP".to_owned(), (3_002, 2_001)),
            ]
        );
        assert_eq!(
            bench_bits(),
            vec![BenchSampleBits::B64, BenchSampleBits::B32]
        );

        for value in ["", "1000,,2000", "0", "invalid", "4294967295"] {
            unsafe { std::env::set_var("FORNAX_BENCH_SIZE", value) };
            assert!(
                std::panic::catch_unwind(bench_size).is_err(),
                "{value:?} should be rejected"
            );
        }

        unsafe { std::env::set_var("FORNAX_BENCH_BITS", "invalid") };
        assert!(std::panic::catch_unwind(bench_bits).is_err());

        unsafe {
            match previous_size {
                Some(value) => std::env::set_var("FORNAX_BENCH_SIZE", value),
                None => std::env::remove_var("FORNAX_BENCH_SIZE"),
            }
            match previous_bits {
                Some(value) => std::env::set_var("FORNAX_BENCH_BITS", value),
                None => std::env::remove_var("FORNAX_BENCH_BITS"),
            }
        }
    }

    #[test]
    fn benchmark_data_has_requested_shape_and_range() {
        let size = (150, 100);
        let image = bench_mono_image::<f32>(size, 1.0);
        assert_eq!(image.dimensions(), size);
        assert!(
            image
                .as_raw()
                .iter()
                .all(|value| (0.0..1.0).contains(value))
        );
    }
}
