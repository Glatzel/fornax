use fornax_raw_core::{BayerPattern, MonoImage};
use fornax_raw_pipeline::{Demosaic, IDemosaic, LazyDemosaic, LmmseMode};

const SIZE: u32 = 64;
const SIZE_SWEEP_LIMIT: u32 = 40;
const TILE_BOUNDARY_SIZE: u32 = 257;
// The largest acceptable accumulated roundoff for this f32 pipeline.
const REL_TOLERANCE: f32 = 2.0e-4;
const ABS_TOLERANCE: f32 = 1.0e-7;

#[derive(Clone, Copy)]
struct Algorithm {
    name: &'static str,
    demosaic: Demosaic,
}

const ALGORITHMS: [Algorithm; 11] = [
    Algorithm {
        name: "AAHD",
        demosaic: Demosaic::Aahd,
    },
    Algorithm {
        name: "AHD",
        demosaic: Demosaic::Ahd,
    },
    Algorithm {
        name: "AMaZE",
        demosaic: Demosaic::Amaze,
    },
    Algorithm {
        name: "DCB",
        demosaic: Demosaic::Dcb {
            iterations: 0,
            enhance: true,
        },
    },
    Algorithm {
        name: "DHT",
        demosaic: Demosaic::Dht,
    },
    Algorithm {
        name: "Linear",
        demosaic: Demosaic::Linear,
    },
    Algorithm {
        name: "LMMSE",
        demosaic: Demosaic::Lmmse {
            mode: LmmseMode::Median,
        },
    },
    Algorithm {
        name: "PPG",
        demosaic: Demosaic::Ppg,
    },
    Algorithm {
        name: "RCD",
        demosaic: Demosaic::Rcd,
    },
    Algorithm {
        name: "VNG",
        demosaic: Demosaic::Vng,
    },
    Algorithm {
        name: "VNG4",
        demosaic: Demosaic::Vng4,
    },
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Invariant {
    Scale,
    Shift,
    PatternFlip,
    Constant,
    Ramp,
    Tiling,
    Finite,
    NegativePreservation,
    HdrPreservation,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum Expected {
    Pass,
    Fail(&'static str),
    NotApplicable(&'static str),
}

fn expectation(algorithm: &str, invariant: Invariant) -> Expected {
    match invariant {
        Invariant::Shift if algorithm != "Linear" => {
            Expected::NotApplicable("Phase 0 classifies only Linear as shift-invariant.")
        }
        Invariant::Shift => Expected::Pass,
        Invariant::Scale if !matches!(algorithm, "Linear" | "PPG") => {
            Expected::Fail("fixed-range conversion, epsilon, or quantization breaks scaling")
        }
        Invariant::PatternFlip if matches!(algorithm, "AAHD" | "AMaZE" | "DCB" | "DHT" | "RCD") => {
            Expected::Fail("direction or border decisions differ after a horizontal flip")
        }
        Invariant::Constant if algorithm == "LMMSE" => {
            Expected::Fail("tile halo correction is blocked by the reference and evidence gates")
        }
        Invariant::Ramp if !matches!(algorithm, "Linear" | "VNG") => {
            Expected::NotApplicable("the algorithm does not claim exact linear-ramp recovery")
        }
        Invariant::Finite
            if matches!(algorithm, "AAHD" | "DCB" | "DHT" | "LMMSE" | "VNG" | "VNG4") =>
        {
            Expected::Fail("the explicit reference-precision adapter rejects out-of-range samples")
        }
        Invariant::NegativePreservation
            if matches!(
                algorithm,
                "AAHD" | "AHD" | "AMaZE" | "DCB" | "DHT" | "LMMSE" | "PPG" | "RCD" | "VNG" | "VNG4"
            ) =>
        {
            Expected::Fail("fixed-range conversion or a zero floor removes negative values")
        }
        Invariant::HdrPreservation
            if matches!(
                algorithm,
                "AAHD" | "AHD" | "AMaZE" | "DCB" | "DHT" | "LMMSE" | "VNG" | "VNG4"
            ) =>
        {
            Expected::Fail("the shared or algorithm-local conversion clamps fixed range")
        }
        _ => Expected::Pass,
    }
}

struct Lcg(u64);

impl Lcg {
    const fn new(seed: u64) -> Self { Self(seed) }

    fn next_f32(&mut self, low: f32, high: f32) -> f32 {
        self.0 = self
            .0
            .wrapping_mul(6_364_136_223_846_793_005)
            .wrapping_add(1_442_695_040_888_963_407);
        let unit = ((self.0 >> 40) as u32) as f32 / ((1_u32 << 24) as f32);
        low + (high - low) * unit
    }
}

fn seeded_mosaic(seed: u64, low: f32, high: f32) -> Vec<f32> {
    let mut rng = Lcg::new(seed);
    (0..SIZE * SIZE).map(|_| rng.next_f32(low, high)).collect()
}

fn attempt_run(
    algorithm: Demosaic,
    samples: &[f32],
    width: u32,
    height: u32,
    pattern: BayerPattern,
) -> Result<Vec<f32>, String> {
    let image = MonoImage::from_raw(width, height, samples.to_vec())
        .ok_or_else(|| "failed to construct the synthetic mosaic".to_owned())?;
    let lazy = algorithm
        .demosaic(&image, pattern)
        .map_err(|error| format!("{error:?}"))?;
    let output = lazy.collect().into_raw();
    let expected = width as usize * height as usize * 3;
    if output.len() != expected {
        return Err(format!(
            "output length {} != {expected} for {width}x{height}",
            output.len()
        ));
    }
    Ok(output)
}

fn panic_message(payload: Box<dyn std::any::Any + Send>) -> String {
    payload
        .downcast_ref::<&str>()
        .copied()
        .or_else(|| payload.downcast_ref::<String>().map(String::as_str))
        .unwrap_or("unknown panic")
        .to_owned()
}

fn run_sized(
    algorithm: Demosaic,
    samples: &[f32],
    width: u32,
    height: u32,
    pattern: BayerPattern,
) -> Result<Vec<f32>, String> {
    std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
        attempt_run(algorithm, samples, width, height, pattern)
    }))
    .unwrap_or_else(|payload| Err(format!("panicked: {}", panic_message(payload))))
}

fn run(algorithm: Demosaic, samples: &[f32], pattern: BayerPattern) -> Result<Vec<f32>, String> {
    run_sized(algorithm, samples, SIZE, SIZE, pattern)
}

fn assert_close_sized(
    actual: &[f32],
    expected: &[f32],
    width: u32,
    context: &str,
) -> Result<(), String> {
    if actual.len() != expected.len() {
        return Err(format!(
            "{context}: output length {} != {}",
            actual.len(),
            expected.len()
        ));
    }
    for (index, (&actual, &expected)) in actual.iter().zip(expected).enumerate() {
        let difference = (actual - expected).abs();
        let allowed = ABS_TOLERANCE + REL_TOLERANCE * actual.abs().max(expected.abs());
        if !actual.is_finite() || difference > allowed {
            let pixel = index / 3;
            let x = pixel as u32 % width;
            let y = pixel as u32 / width;
            return Err(format!(
                "{context}: ({x}, {y}) channel {} actual {actual:?}, expected {expected:?}, \
                 difference {difference:?}, allowed {allowed:?}",
                index % 3
            ));
        }
    }
    Ok(())
}

fn assert_close(actual: &[f32], expected: &[f32], context: &str) -> Result<(), String> {
    assert_close_sized(actual, expected, SIZE, context)
}

fn check_expectation(
    algorithm: &str,
    invariant: Invariant,
    actual: Result<(), String>,
    failures: &mut Vec<String>,
) {
    let expected = expectation(algorithm, invariant);
    let actual_status = if actual.is_ok() { "pass" } else { "fail" };
    let (expected_status, expected_reason) = match expected {
        Expected::Pass => ("pass", "expected invariant to hold"),
        Expected::Fail(reason) => ("fail", reason),
        Expected::NotApplicable(reason) => ("not-applicable", reason),
    };
    if actual_status != expected_status {
        let detail = actual.err().unwrap_or_else(|| "unexpected pass".to_owned());
        failures.push(format!(
            "{algorithm} {invariant:?} {SIZE}x{SIZE}: expected {expected_status}, got \
             {actual_status} ({expected_reason}): {detail}"
        ));
    }
}

fn scale_equivariance(algorithm: Demosaic) -> Result<(), String> {
    let mosaic = seeded_mosaic(0x5ca1_e001, 0.05, 0.95);
    let baseline = run(algorithm, &mosaic, BayerPattern::GBRG)?;
    for factor in [1.0e-3_f32, 1.0e-1, 1.0, 1.0e1, 1.0e3] {
        let scaled = mosaic
            .iter()
            .map(|sample| sample * factor)
            .collect::<Vec<_>>();
        let actual = run(algorithm, &scaled, BayerPattern::GBRG)?;
        let expected = baseline
            .iter()
            .map(|sample| sample * factor)
            .collect::<Vec<_>>();
        assert_close(&actual, &expected, &format!("scale factor {factor:e}"))?;
    }
    Ok(())
}

fn shift_equivariance(algorithm: Demosaic, pattern: BayerPattern) -> Result<(), String> {
    let mosaic = seeded_mosaic(0x5ca1_e002, 0.1, 0.8);
    let baseline = run(algorithm, &mosaic, pattern)?;
    let offset = 0.25_f32;
    let shifted = mosaic
        .iter()
        .map(|sample| sample + offset)
        .collect::<Vec<_>>();
    let actual = run(algorithm, &shifted, pattern)?;
    let expected = baseline
        .iter()
        .map(|sample| sample + offset)
        .collect::<Vec<_>>();
    assert_close(&actual, &expected, &format!("{pattern} shift"))
}

fn pattern_flip(algorithm: Demosaic, pattern: BayerPattern) -> Result<(), String> {
    let mosaic = seeded_mosaic(0x5ca1_e003, 0.05, 0.95);
    let baseline = run(algorithm, &mosaic, pattern)?;
    let mut flipped = vec![0.0; mosaic.len()];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let source = (y * SIZE + (SIZE - 1 - x)) as usize;
            flipped[(y * SIZE + x) as usize] = mosaic[source];
        }
    }
    let flipped_pattern = pattern.shifted(1, 0);
    let actual = run(algorithm, &flipped, flipped_pattern)?;
    let mut expected = vec![0.0; baseline.len()];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let source = ((y * SIZE + (SIZE - 1 - x)) * 3) as usize;
            let destination = ((y * SIZE + x) * 3) as usize;
            expected[destination..destination + 3].copy_from_slice(&baseline[source..source + 3]);
        }
    }
    assert_close(
        &actual,
        &expected,
        &format!("{pattern} -> {flipped_pattern} horizontal flip"),
    )
}

fn pattern_flip_top_left(algorithm: Demosaic, pattern: BayerPattern) -> Result<(), String> {
    let mosaic = seeded_mosaic(0x5ca1_e003, 0.05, 0.95);
    let baseline = run(algorithm, &mosaic, pattern)?;
    let mut flipped = vec![0.0; mosaic.len()];
    for y in 0..SIZE {
        for x in 0..SIZE {
            let source = (y * SIZE + (SIZE - 1 - x)) as usize;
            flipped[(y * SIZE + x) as usize] = mosaic[source];
        }
    }
    let actual = run(algorithm, &flipped, pattern.shifted(1, 0))?;
    let expected = &baseline[((SIZE - 1) * 3) as usize..((SIZE - 1) * 3 + 3) as usize];
    assert_close(
        &actual[..3],
        expected,
        &format!("{pattern} top-left horizontal flip"),
    )
}

fn constant_image(algorithm: Demosaic, pattern: BayerPattern) -> Result<(), String> {
    let value = 0.25;
    let actual = run(algorithm, &vec![value; (SIZE * SIZE) as usize], pattern)?;
    let expected = vec![value; actual.len()];
    assert_close(&actual, &expected, &format!("{pattern} constant"))
}

fn linear_ramp(algorithm: Demosaic, pattern: BayerPattern) -> Result<(), String> {
    let mosaic = (0..SIZE)
        .flat_map(|y| (0..SIZE).map(move |x| 0.1 + 0.005 * x as f32 + 0.0025 * y as f32))
        .collect::<Vec<_>>();
    let actual = run(algorithm, &mosaic, pattern)?;
    for y in 8..SIZE - 8 {
        for x in 8..SIZE - 8 {
            let expected = 0.1 + 0.005 * x as f32 + 0.0025 * y as f32;
            let start = ((y * SIZE + x) * 3) as usize;
            let expected = [expected; 3];
            assert_close(
                &actual[start..start + 3],
                &expected,
                &format!("{pattern} ramp at ({x}, {y})"),
            )?;
        }
    }
    Ok(())
}

fn tiled_output(algorithm: Demosaic, pattern: BayerPattern) -> Result<(), String> {
    let mosaic = seeded_mosaic(0x5ca1_e004, 0.05, 0.95);
    let image = MonoImage::from_raw(SIZE, SIZE, mosaic)
        .ok_or_else(|| "failed to construct the tiled synthetic mosaic".to_owned())?;
    let lazy = algorithm
        .demosaic(&image, pattern)
        .map_err(|error| format!("{error:?}"))?;
    let (width, height) = lazy.dimensions();
    let stride = width as usize * 3;
    let mut tiled = vec![0.0; stride * height as usize];
    for y in (0..height as usize).step_by(7) {
        let end = (y + 7).min(height as usize);
        lazy.fill_rows(y, &mut tiled[y * stride..end * stride]);
    }
    let whole = lazy.collect().into_raw();
    assert_close(
        &whole,
        &tiled,
        &format!("{pattern} whole versus tiled rows"),
    )
}

fn finite_output(algorithm: Demosaic, pattern: BayerPattern) -> Result<(), String> {
    for (label, samples) in [
        ("negative", seeded_mosaic(0x5ca1_e005, -0.9, -0.1)),
        ("zero", vec![0.0; (SIZE * SIZE) as usize]),
        ("1e4", seeded_mosaic(0x5ca1_e006, 1.0e4, 2.0e4)),
        ("1e6", seeded_mosaic(0x5ca1_e007, 1.0e6, 2.0e6)),
    ] {
        let output = run(algorithm, &samples, pattern)?;
        if let Some((index, &value)) = output
            .iter()
            .enumerate()
            .find(|(_, value)| !value.is_finite())
        {
            let pixel = index / 3;
            return Err(format!(
                "{pattern} {label} input produced non-finite {value} at ({}, {}) channel {}",
                pixel as u32 % SIZE,
                pixel as u32 / SIZE,
                index % 3
            ));
        }
    }
    Ok(())
}

fn size_sweep(algorithm: Algorithm, pattern: BayerPattern, failures: &mut Vec<String>) {
    let dimensions = (1..=SIZE_SWEEP_LIMIT)
        .flat_map(|size| [(size, size), (size, SIZE), (SIZE, size)])
        .chain([(41, 63), (63, 41), (127, 43)]);
    for (width, height) in dimensions {
        let samples = (0..width as usize * height as usize)
            .map(|index| 0.05 + ((index * 37) % 97) as f32 / 100.0)
            .collect::<Vec<_>>();
        let outcome = std::panic::catch_unwind(std::panic::AssertUnwindSafe(|| {
            attempt_run(algorithm.demosaic, &samples, width, height, pattern)
        }));
        match outcome {
            Err(payload) => failures.push(format!(
                "{} SizeSweep {pattern} {width}x{height}: panicked: {}",
                algorithm.name,
                panic_message(payload)
            )),
            Ok(Ok(output)) => {
                if let Some((index, value)) = output
                    .iter()
                    .enumerate()
                    .find(|(_, value)| !value.is_finite())
                {
                    failures.push(format!(
                        "{} SizeSweep {pattern} {width}x{height}: non-finite {value} at output \
                         channel {index}",
                        algorithm.name
                    ));
                }
            }
            Ok(Err(_)) => {}
        }
    }
}

fn tiled_constant_image(algorithm: Demosaic, pattern: BayerPattern) -> Result<(), String> {
    let value = 0.25;
    let actual = run_sized(
        algorithm,
        &vec![value; (TILE_BOUNDARY_SIZE * TILE_BOUNDARY_SIZE) as usize],
        TILE_BOUNDARY_SIZE,
        TILE_BOUNDARY_SIZE,
        pattern,
    )?;
    assert_close_sized(
        &actual,
        &vec![value; (TILE_BOUNDARY_SIZE * TILE_BOUNDARY_SIZE * 3) as usize],
        TILE_BOUNDARY_SIZE,
        &format!("{pattern} constant across tile boundaries"),
    )
}

fn outermost_constant_image(algorithm: Demosaic, pattern: BayerPattern) -> Result<(), String> {
    let value = 0.25;
    let actual = run(algorithm, &vec![value; (SIZE * SIZE) as usize], pattern)?;
    for (x, y) in [(0, 0), (SIZE - 1, 0), (0, SIZE - 1), (SIZE - 1, SIZE - 1)] {
        let start = ((y * SIZE + x) * 3) as usize;
        assert_close(
            &actual[start..start + 3],
            &[value; 3],
            &format!("{pattern} outermost constant pixel at ({x}, {y})"),
        )?;
    }
    Ok(())
}

fn constant_range_preservation(
    algorithm: Demosaic,
    pattern: BayerPattern,
    value: f32,
    expect_negative: bool,
) -> Result<(), String> {
    let output = run(algorithm, &vec![value; (SIZE * SIZE) as usize], pattern)?;
    let preserved = if expect_negative {
        output.iter().all(|sample| *sample < 0.0)
    } else {
        output.iter().all(|sample| *sample > 1.0)
    };
    if !preserved {
        let minimum = output.iter().copied().fold(f32::INFINITY, f32::min);
        let maximum = output.iter().copied().fold(f32::NEG_INFINITY, f32::max);
        let expected = if expect_negative {
            "a negative output"
        } else {
            "an output above 1.0"
        };
        return Err(format!(
            "{pattern} constant input {value} produced [{minimum}, {maximum}], expected \
             {expected}"
        ));
    }
    Ok(())
}

fn run_scale_checks(failures: &mut Vec<String>) {
    for algorithm in ALGORITHMS {
        check_expectation(
            algorithm.name,
            Invariant::Scale,
            scale_equivariance(algorithm.demosaic),
            failures,
        );
    }
}

fn run_shift_checks(failures: &mut Vec<String>) {
    for algorithm in ALGORITHMS {
        if expectation(algorithm.name, Invariant::Shift) != Expected::Pass {
            continue;
        }
        let actual = BayerPattern::ALL
            .into_iter()
            .try_for_each(|pattern| shift_equivariance(algorithm.demosaic, pattern));
        check_expectation(algorithm.name, Invariant::Shift, actual, failures);
    }
}

fn run_pattern_checks(failures: &mut Vec<String>) {
    for algorithm in ALGORITHMS {
        let actual = BayerPattern::ALL
            .into_iter()
            .try_for_each(|pattern| pattern_flip(algorithm.demosaic, pattern));
        check_expectation(algorithm.name, Invariant::PatternFlip, actual, failures);
    }
}

fn run_structure_checks(failures: &mut Vec<String>) {
    for algorithm in ALGORITHMS {
        let actual = BayerPattern::ALL
            .into_iter()
            .try_for_each(|pattern| constant_image(algorithm.demosaic, pattern));
        check_expectation(algorithm.name, Invariant::Constant, actual, failures);
    }
    for algorithm in ALGORITHMS {
        if expectation(algorithm.name, Invariant::Ramp) != Expected::Pass {
            continue;
        }
        let actual = BayerPattern::ALL
            .into_iter()
            .try_for_each(|pattern| linear_ramp(algorithm.demosaic, pattern));
        check_expectation(algorithm.name, Invariant::Ramp, actual, failures);
    }
    for algorithm in ALGORITHMS {
        let actual = tiled_output(algorithm.demosaic, BayerPattern::GBRG);
        check_expectation(algorithm.name, Invariant::Tiling, actual, failures);
    }
}

fn run_finite_checks(failures: &mut Vec<String>) {
    for algorithm in ALGORITHMS {
        check_expectation(
            algorithm.name,
            Invariant::Finite,
            finite_output(algorithm.demosaic, BayerPattern::GBRG),
            failures,
        );
    }
}

fn run_negative_range_checks(failures: &mut Vec<String>) {
    for algorithm in ALGORITHMS {
        check_expectation(
            algorithm.name,
            Invariant::NegativePreservation,
            constant_range_preservation(algorithm.demosaic, BayerPattern::GBRG, -0.25, true),
            failures,
        );
    }
}

fn run_hdr_range_checks(failures: &mut Vec<String>) {
    for algorithm in ALGORITHMS {
        check_expectation(
            algorithm.name,
            Invariant::HdrPreservation,
            constant_range_preservation(algorithm.demosaic, BayerPattern::GBRG, 2.0, false),
            failures,
        );
    }
}

fn finish(failures: Vec<String>) {
    assert!(
        failures.is_empty(),
        "{} invariant expectation mismatch(es) at {SIZE}x{SIZE}:\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
fn scale_equivariance_matches_expectations() {
    let mut failures = Vec::new();
    run_scale_checks(&mut failures);
    finish(failures);
}

#[test]
fn shift_invariance_matches_expectations() {
    let mut failures = Vec::new();
    run_shift_checks(&mut failures);
    finish(failures);
}

#[test]
fn pattern_flip_symmetry_matches_expectations() {
    let mut failures = Vec::new();
    run_pattern_checks(&mut failures);
    finish(failures);
}

#[test]
fn structural_invariants_match_expectations() {
    let mut failures = Vec::new();
    run_structure_checks(&mut failures);
    finish(failures);
}

#[test]
fn finite_inputs_match_expectations() {
    let mut failures = Vec::new();
    run_finite_checks(&mut failures);
    finish(failures);
}

#[test]
fn negative_range_preservation_matches_expectations() {
    let mut failures = Vec::new();
    run_negative_range_checks(&mut failures);
    finish(failures);
}

#[test]
fn hdr_range_preservation_matches_expectations() {
    let mut failures = Vec::new();
    run_hdr_range_checks(&mut failures);
    finish(failures);
}

#[test]
fn size_sweep_never_panics_or_returns_invalid_images() {
    let mut failures = Vec::new();
    for algorithm in ALGORITHMS {
        for pattern in BayerPattern::ALL {
            size_sweep(algorithm, pattern, &mut failures);
        }
    }
    assert!(
        failures.is_empty(),
        "{} size-sweep failure(s):\n{}",
        failures.len(),
        failures.join("\n")
    );
}

#[test]
#[ignore = "tracked Phase A blocker: the halo fix regresses darktable beyond ground-truth evidence"]
fn lmmse_constant_field_is_preserved_across_tile_boundaries() {
    let failures = BayerPattern::ALL
        .into_iter()
        .filter_map(|pattern| {
            tiled_constant_image(
                Demosaic::Lmmse {
                    mode: LmmseMode::Median,
                },
                pattern,
            )
            .err()
        })
        .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "LMMSE failed the {TILE_BOUNDARY_SIZE}x{TILE_BOUNDARY_SIZE} tile-boundary constant test:\n{}",
        failures.join("\n")
    );
}

#[test]
fn aahd_constant_field_is_preserved_at_outermost_pixels() {
    let failures = BayerPattern::ALL
        .into_iter()
        .filter_map(|pattern| outermost_constant_image(Demosaic::Aahd, pattern).err())
        .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "AAHD failed the outermost-pixel constant test:\n{}",
        failures.join("\n")
    );
}

#[test]
fn amaze_corner_reflection_is_horizontally_symmetric() {
    let failures = BayerPattern::ALL
        .into_iter()
        .filter_map(|pattern| pattern_flip_top_left(Demosaic::Amaze, pattern).err())
        .collect::<Vec<_>>();
    assert!(
        failures.is_empty(),
        "AMaZE failed the corner-only horizontal-flip test:\n{}",
        failures.join("\n")
    );
}
