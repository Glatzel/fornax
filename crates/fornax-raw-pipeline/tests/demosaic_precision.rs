use fornax_raw_core::{BayerPattern, MonoImage, Primitive};
use fornax_raw_pipeline::{Demosaic, IDemosaic, LazyDemosaic};
use num_traits::cast;

const WIDTH: u32 = 64;
const HEIGHT: u32 = 64;
const F64_RELATIVE_TOLERANCE: f64 = 2.0e-10;
const F64_ABSOLUTE_TOLERANCE: f64 = 1.0e-12;

fn sample<T: Primitive>(value: f64) -> T { cast(value).expect("the test sample fits T") }

fn seeded_mosaic<T: Primitive>(low: f64, high: f64) -> Vec<T> {
    let mut state = 0x5ca1_ab1e_u64;
    (0..WIDTH * HEIGHT)
        .map(|_| {
            state = state
                .wrapping_mul(6_364_136_223_846_793_005)
                .wrapping_add(1_442_695_040_888_963_407);
            let unit = (state >> 40) as f64 / (1_u64 << 24) as f64;
            sample(low + (high - low) * unit)
        })
        .collect()
}

fn run<T: Primitive + Send + Sync>(algorithm: Demosaic, samples: Vec<T>) -> Result<Vec<T>, String> {
    let image = MonoImage::from_raw(WIDTH, HEIGHT, samples)
        .ok_or_else(|| "failed to construct the test mosaic".to_owned())?;
    algorithm
        .demosaic(&image, BayerPattern::GBRG)
        .map_err(|error| format!("{error:?}"))
        .map(|lazy| lazy.collect().into_raw())
}

fn assert_close<T: Primitive>(actual: &[T], expected: &[T], context: &str) {
    assert_eq!(actual.len(), expected.len(), "{context}: length");
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        let actual = actual.to_f64().expect("test output fits f64");
        let expected = expected.to_f64().expect("test expectation fits f64");
        let difference = (actual - expected).abs();
        let allowed =
            F64_ABSOLUTE_TOLERANCE + F64_RELATIVE_TOLERANCE * actual.abs().max(expected.abs());
        assert!(
            actual.is_finite() && difference <= allowed,
            "{context}: channel {index} actual {actual:?}, expected {expected:?}, \
             difference {difference:?}, allowed {allowed:?}"
        );
    }
}

#[test]
fn linear_f64_is_scale_homogeneous_across_sensor_dynamic_ranges() {
    let mosaic = seeded_mosaic::<f64>(0.05, 2.0);
    let baseline = run(Demosaic::Linear, mosaic.clone()).unwrap();
    for factor in [1.0e-6, 1.0e-3, 1.0, 1.0e3, 1.0e6] {
        let scaled = mosaic.iter().map(|value| *value * factor).collect();
        let actual = run(Demosaic::Linear, scaled).unwrap();
        let expected = baseline
            .iter()
            .map(|value| *value * factor)
            .collect::<Vec<_>>();
        assert_close(&actual, &expected, &format!("Linear f64 scale {factor:e}"));
    }
}

#[test]
fn linear_f64_preserves_negative_hdr_and_sub_f32_values() {
    let sub_f32 = 1.0 + 2.0_f64.powi(-40);
    for value in [-4.0, sub_f32, 2.0e6] {
        let output = run(
            Demosaic::Linear,
            vec![sample::<f64>(value); (WIDTH * HEIGHT) as usize],
        )
        .unwrap();
        let expected = vec![value; output.len()];
        assert_close(
            &output,
            &expected,
            &format!("Linear f64 constant {value:?}"),
        );
    }
    let sub_f32_output = run(
        Demosaic::Linear,
        vec![sample::<f64>(sub_f32); (WIDTH * HEIGHT) as usize],
    )
    .unwrap();
    assert!(sub_f32_output[0] > 1.0);
}

#[test]
fn ppg_f64_retains_scale_equivariance_above_one() {
    let mosaic = seeded_mosaic::<f64>(0.05, 1.5);
    let baseline = run(Demosaic::Ppg, mosaic.clone()).unwrap();
    for factor in [1.0e-3, 1.0, 1.0e3, 1.0e6] {
        let scaled = mosaic.iter().map(|value| *value * factor).collect();
        let actual = run(Demosaic::Ppg, scaled).unwrap();
        let expected = baseline
            .iter()
            .map(|value| *value * factor)
            .collect::<Vec<_>>();
        assert_close(&actual, &expected, &format!("PPG f64 scale {factor:e}"));
    }
}

#[test]
fn ppg_f64_preserves_negative_constant_fields() {
    let expected = -0.25_f64;
    let output = run(
        Demosaic::Ppg,
        vec![sample::<f64>(expected); (WIDTH * HEIGHT) as usize],
    )
    .unwrap();
    assert_close(
        &output,
        &vec![expected; output.len()],
        "PPG f64 negative constant",
    );
}

#[test]
fn rcd_f64_preserves_constant_precision_and_hdr_magnitude() {
    let sub_f32 = 1.0 + 2.0_f64.powi(-40);
    for value in [0.5, sub_f32, 4.0] {
        let output = run(
            Demosaic::Rcd,
            vec![sample::<f64>(value); (WIDTH * HEIGHT) as usize],
        )
        .unwrap();
        let expected = vec![value; output.len()];
        assert_close(&output, &expected, &format!("RCD f64 constant {value:?}"));
    }
}

#[test]
fn rcd_f64_is_scale_homogeneous() {
    let mosaic = seeded_mosaic::<f64>(0.05, 1.5);
    let baseline = run(Demosaic::Rcd, mosaic.clone()).unwrap();
    for factor in [1.0e-3, 1.0, 1.0e3, 1.0e6] {
        let scaled = mosaic.iter().map(|value| *value * factor).collect();
        let actual = run(Demosaic::Rcd, scaled).unwrap();
        let expected = baseline
            .iter()
            .map(|value| *value * factor)
            .collect::<Vec<_>>();
        assert_close(&actual, &expected, &format!("RCD f64 scale {factor:e}"));
    }
}

#[test]
fn amaze_f64_is_scale_homogeneous_and_preserves_range() {
    let mosaic = seeded_mosaic::<f64>(0.05, 1.5);
    let baseline = run(Demosaic::Amaze, mosaic.clone()).unwrap();
    for factor in [1.0e-3, 1.0, 1.0e3, 1.0e6] {
        let scaled = mosaic.iter().map(|value| *value * factor).collect();
        let actual = run(Demosaic::Amaze, scaled).unwrap();
        let expected = baseline
            .iter()
            .map(|value| *value * factor)
            .collect::<Vec<_>>();
        assert_close(&actual, &expected, &format!("AMaZE f64 scale {factor:e}"));
    }
    for value in [-0.25, 2.0, 1.0e4] {
        let output = run(
            Demosaic::Amaze,
            vec![sample::<f64>(value); (WIDTH * HEIGHT) as usize],
        )
        .unwrap();
        assert_close(
            &output,
            &vec![value; output.len()],
            &format!("AMaZE f64 constant {value}"),
        );
    }
}

#[test]
fn vng_and_vng4_f64_are_scale_homogeneous_and_preserve_range() {
    let mosaic = seeded_mosaic::<f64>(0.05, 1.5);
    for algorithm in [Demosaic::Vng, Demosaic::Vng4] {
        let baseline = run(algorithm, mosaic.clone()).unwrap();
        for factor in [1.0e-3, 1.0, 1.0e3, 1.0e6] {
            let scaled = mosaic.iter().map(|value| *value * factor).collect();
            let actual = run(algorithm, scaled).unwrap();
            let expected = baseline
                .iter()
                .map(|value| *value * factor)
                .collect::<Vec<_>>();
            assert_close(
                &actual,
                &expected,
                &format!("{algorithm:?} f64 scale {factor:e}"),
            );
        }
        for value in [-0.25, 2.0, 1.0e4] {
            let output = run(
                algorithm,
                vec![sample::<f64>(value); (WIDTH * HEIGHT) as usize],
            )
            .unwrap();
            assert_close(
                &output,
                &vec![value; output.len()],
                &format!("{algorithm:?} f64 constant {value}"),
            );
        }
    }
}

#[test]
fn lmmse_f64_is_scale_homogeneous_and_preserves_range() {
    let mosaic = seeded_mosaic::<f64>(0.05, 1.5);
    let baseline = run(Demosaic::lmmse(), mosaic.clone()).unwrap();
    for factor in [1.0e-3, 1.0, 1.0e3, 1.0e6] {
        let scaled = mosaic.iter().map(|value| *value * factor).collect();
        let actual = run(Demosaic::lmmse(), scaled).unwrap();
        let expected = baseline
            .iter()
            .map(|value| *value * factor)
            .collect::<Vec<_>>();
        assert_close(&actual, &expected, &format!("LMMSE f64 scale {factor:e}"));
    }
    for value in [-0.25, 2.0, 1.0e4] {
        let output = run(
            Demosaic::lmmse(),
            vec![sample::<f64>(value); (WIDTH * HEIGHT) as usize],
        )
        .unwrap();
        assert_close(
            &output,
            &vec![value; output.len()],
            &format!("LMMSE f64 constant {value}"),
        );
    }
}

#[test]
fn f32_and_f64_generic_kernels_are_consistent_at_f32_precision() {
    let mosaic = seeded_mosaic::<f64>(0.1, 0.9);
    let f64 = run(Demosaic::Linear, mosaic.clone()).unwrap();
    let f32 = run(
        Demosaic::Linear,
        mosaic.iter().map(|value| sample::<f32>(*value)).collect(),
    )
    .unwrap();
    assert_eq!(f32.len(), f64.len());
    for (index, (actual, expected)) in f32.iter().zip(&f64).enumerate() {
        let actual = f64::from(*actual);
        let expected = *expected;
        let allowed = 1.0e-7 + 2.0e-4 * actual.abs().max(expected.abs());
        assert!(
            (actual - expected).abs() <= allowed,
            "Linear f32/f64 channel {index}: {actual:?} vs {expected:?}"
        );
    }
}
