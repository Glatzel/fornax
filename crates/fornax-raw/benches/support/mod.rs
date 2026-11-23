#![allow(dead_code)]

use std::path::PathBuf;

use fornax_core::MonoImage;
use fornax_devtool::root_dir;
use fornax_raw::{RawFloat, RawFormat, RawMosaic};
use generic_num::num;
use rand::rngs::StdRng;
use rand::{RngExt, SeedableRng};

pub fn synthetic_mono_image<T>((width, height): (u32, u32), upper: f64) -> MonoImage<T>
where
    T: RawFloat,
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

/// Returns the native in-memory byte count for a raw mosaic.
pub fn raw_mosaic_bytes(mosaic: &RawMosaic) -> u64 {
    let (width, height) = mosaic.dimensions();
    let bytes_per_sample = match mosaic {
        RawMosaic::U8(_) => size_of::<u8>(),
        RawMosaic::U16(_) => size_of::<u16>(),
        RawMosaic::U32(_) | RawMosaic::F32(_) => size_of::<u32>(),
    };
    u64::from(width) * u64::from(height) * u64::try_from(bytes_per_sample).unwrap()
}

pub fn raw_file(format: RawFormat) -> PathBuf {
    match format {
        RawFormat::Dng => root_dir().join("external/raw-images/images/colorchart-5D2-6000K.dng"),
        RawFormat::Cr3
        | RawFormat::Crm
        | RawFormat::Arw
        | RawFormat::Nrw
        | RawFormat::Nef
        | RawFormat::Raf
        | RawFormat::Hasselblad3fr
        | RawFormat::Rw2
        | RawFormat::Iiq => panic!("Format doesn't support: {format}"),
    }
}

/// An input selected for DNG writer benchmarks.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum DngBenchmarkInput {
    /// Deterministic high-entropy U16 samples.
    Noise,
    /// A mosaic decoded from a DNG file.
    Real(PathBuf),
}

/// Selects DNG writer benchmark inputs for CI and local runs.
pub fn dng_benchmark_inputs(
    ci: bool,
    explicit_real: Option<PathBuf>,
    default_real: PathBuf,
) -> Vec<DngBenchmarkInput> {
    if ci {
        vec![
            DngBenchmarkInput::Noise,
            DngBenchmarkInput::Real(default_real),
        ]
    } else {
        vec![explicit_real.map_or(DngBenchmarkInput::Noise, DngBenchmarkInput::Real)]
    }
}
