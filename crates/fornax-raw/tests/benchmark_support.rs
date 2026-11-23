#[path = "../benches/support/mod.rs"]
mod support;

use fornax_raw::{MonoImage, RawMosaic};
use support::{DngBenchmarkInput, dng_benchmark_inputs, raw_mosaic_bytes, synthetic_mono_image};

#[test]
fn synthetic_mono_image_is_deterministic_and_in_range() {
    let size = (150, 100);
    let first = synthetic_mono_image::<f32>(size, 1.0);
    let second = synthetic_mono_image::<f32>(size, 1.0);

    assert_eq!(first.dimensions(), size);
    assert_eq!(first.as_raw(), second.as_raw());
    assert!(
        first
            .as_raw()
            .iter()
            .all(|sample| (0.0..1.0).contains(sample))
    );
}

#[test]
fn raw_mosaic_bytes_uses_native_sample_width() {
    assert_eq!(
        raw_mosaic_bytes(&RawMosaic::U8(
            MonoImage::from_raw(2, 3, vec![0; 6]).unwrap()
        )),
        6
    );
    assert_eq!(
        raw_mosaic_bytes(&RawMosaic::U16(
            MonoImage::from_raw(2, 3, vec![0; 6]).unwrap()
        )),
        12
    );
    assert_eq!(
        raw_mosaic_bytes(&RawMosaic::U32(
            MonoImage::from_raw(2, 3, vec![0; 6]).unwrap()
        )),
        24
    );
    assert_eq!(
        raw_mosaic_bytes(&RawMosaic::F32(
            MonoImage::from_raw(2, 3, vec![0.0; 6]).unwrap()
        )),
        24
    );
}

#[test]
fn dng_benchmark_input_selection_preserves_ci_and_local_behavior() {
    let default_real = std::path::PathBuf::from("fixtures/default.dng");
    assert_eq!(
        dng_benchmark_inputs(
            true,
            Some(std::path::PathBuf::from("ignored.dng")),
            default_real.clone(),
        ),
        vec![
            DngBenchmarkInput::Noise,
            DngBenchmarkInput::Real(default_real)
        ]
    );
    assert_eq!(
        dng_benchmark_inputs(false, None, std::path::PathBuf::from("unused.dng")),
        vec![DngBenchmarkInput::Noise]
    );

    let real = std::path::PathBuf::from("samples/input.dng");
    assert_eq!(
        dng_benchmark_inputs(
            false,
            Some(real.clone()),
            std::path::PathBuf::from("unused.dng"),
        ),
        vec![DngBenchmarkInput::Real(real)]
    );
}
