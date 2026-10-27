use std::ops::Deref;
use std::path::{Path, PathBuf};

use fornax_core::{MonoImage, Primitive};
use generic_num::num;
use image::{ImageBuffer, Pixel, PixelWithColorType, Primitive as ImagePrimitive};
use num_traits::Float;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub struct BenchInput {
    pub name: &'static str,
    pub width: u32,
    pub height: u32,
}

impl BenchInput {
    pub fn pixels(self) -> u64 { u64::from(self.width) * u64::from(self.height) }
}

pub const BENCH_INPUTS: &[BenchInput] = &[
    BenchInput {
        name: "1080p",
        width: 1_920,
        height: 1_080,
    },
    BenchInput {
        name: "4k",
        width: 3_840,
        height: 2_160,
    },
    BenchInput {
        name: "3000mp",
        width: 75_000,
        height: 40_000,
    },
    BenchInput {
        name: "6000mp",
        width: 100_000,
        height: 60_000,
    },
    BenchInput {
        name: "10000mp",
        width: 125_000,
        height: 80_000,
    },
];

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum BenchSampleBits {
    B32,
    B64,
}

impl BenchSampleBits {
    pub const fn bits(self) -> u8 {
        match self {
            Self::B32 => 32,
            Self::B64 => 64,
        }
    }

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

pub fn bench_inputs() -> Vec<BenchInput> {
    parse_bench_inputs(std::env::var("FORNAX_BENCH_SIZES").ok().as_deref())
}

pub fn bench_sample_bits() -> Vec<BenchSampleBits> {
    parse_bench_sample_bits(std::env::var("FORNAX_BENCH_BITS").ok().as_deref())
}

pub fn bench_mono_image<T>(input: BenchInput, upper: f64) -> MonoImage<T>
where
    T: Primitive,
{
    assert!(upper.is_finite() && upper > 0.0);
    let mut rng = StdRng::seed_from_u64(12_345);
    let pixels = usize::try_from(input.pixels()).expect("benchmark preset exceeds platform memory");
    let samples = (0..pixels)
        .map(|_| num!(rng.random_range(0.0..upper)))
        .collect();
    MonoImage::from_raw(input.width, input.height, samples)
        .expect("benchmark preset dimensions match sample count")
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

fn parse_bench_inputs(value: Option<&str>) -> Vec<BenchInput> {
    let Some(value) = value else {
        return BENCH_INPUTS[..2].to_vec();
    };
    if value.trim().eq_ignore_ascii_case("all") {
        return BENCH_INPUTS.to_vec();
    }

    value
        .split(',')
        .map(str::trim)
        .filter(|name| !name.is_empty())
        .map(|name| {
            BENCH_INPUTS
                .iter()
                .copied()
                .find(|input| input.name.eq_ignore_ascii_case(name))
                .unwrap_or_else(|| {
                    panic!(
                        "unknown FORNAX_BENCH_SIZES value {name:?}; expected 1080p, 4k, 3000mp, 6000mp, 10000mp, or all"
                    )
                })
        })
        .collect()
}

fn parse_bench_sample_bits(value: Option<&str>) -> Vec<BenchSampleBits> {
    let Some(value) = value else {
        return vec![BenchSampleBits::B32, BenchSampleBits::B64];
    };
    if value.trim().eq_ignore_ascii_case("all") {
        return vec![BenchSampleBits::B32, BenchSampleBits::B64];
    }

    value
        .split(',')
        .map(str::trim)
        .filter(|bits| !bits.is_empty())
        .map(|bits| match bits {
            "32" => BenchSampleBits::B32,
            "64" => BenchSampleBits::B64,
            _ => panic!("unknown FORNAX_BENCH_BITS value {bits:?}; expected 32, 64, or all"),
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn benchmark_inputs_default_to_video_sizes() {
        let inputs = parse_bench_inputs(None);
        assert_eq!(inputs, BENCH_INPUTS[..2]);
    }

    #[test]
    fn benchmark_inputs_support_all_and_explicit_mp_presets() {
        assert_eq!(parse_bench_inputs(Some("all")), BENCH_INPUTS);
        assert_eq!(
            parse_bench_inputs(Some("3000mp, 4k")),
            vec![BENCH_INPUTS[2], BENCH_INPUTS[1]]
        );
    }

    #[test]
    fn benchmark_sample_bits_default_to_both() {
        assert_eq!(
            parse_bench_sample_bits(None),
            vec![BenchSampleBits::B32, BenchSampleBits::B64]
        );
        assert_eq!(
            parse_bench_sample_bits(Some("64")),
            vec![BenchSampleBits::B64]
        );
    }

    #[test]
    fn benchmark_data_has_requested_shape_and_range() {
        let input = BENCH_INPUTS[0];
        let image = bench_mono_image::<f32>(input, 1.0);
        assert_eq!(image.dimensions(), (input.width, input.height));
        assert!(
            image
                .as_raw()
                .iter()
                .all(|value| (0.0..1.0).contains(value))
        );
    }
}
