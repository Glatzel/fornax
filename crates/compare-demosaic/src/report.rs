use std::path::Path;
use std::time::Instant;

use fornax_core::BayerPattern;
use rayon::prelude::*;

use crate::artifacts::{ArtifactInput, ArtifactPaths, write as write_artifacts};
use crate::case::{Case, SampleType, Tolerance};
use crate::metrics::{self, Evaluation, FailedLimit, Verdict};
use crate::timings::{self, RowTiming};
use crate::{data, registry};

pub const CANONICAL_BORDER_PX: u32 = 16;
pub const LIBRAW_COMMIT: &str = env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT");
pub const DARKTABLE_RELEASE: &str = env!("COMPARE_DEMOSAIC_DARKTABLE_RELEASE");
pub const DARKTABLE_COMMIT: &str = env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT");
pub const LIBRAW_VERSION: &str = env!("COMPARE_DEMOSAIC_LIBRAW_VERSION");

#[derive(Clone, Debug)]
pub struct Row {
    pub case: &'static str,
    pub reference: &'static str,
    pub source_url: &'static str,
    pub pattern: BayerPattern,
    pub tolerance: Tolerance,
    pub evaluation: Evaluation,
    pub verdict: Verdict,
    pub failed_limits: Vec<FailedLimit>,
    pub artifacts: Option<ArtifactPaths>,
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn compare(
    case: &'static Case,
    input: &data::ActiveArea,
    pattern: BayerPattern,
    sample_type: SampleType,
    _artifacts: bool,
) -> mischief::Result<Row> {
    compare_timed(case, input, pattern, sample_type, None, CANONICAL_BORDER_PX).map(|(row, _)| row)
}

pub fn compare_timed(
    case: &'static Case,
    input: &data::ActiveArea,
    pattern: BayerPattern,
    sample_type: SampleType,
    artifact_dir: Option<&Path>,
    border_px: u32,
) -> mischief::Result<(Row, RowTiming)> {
    let reference_started = Instant::now();
    let reference = crate::ffi::run(
        case.reference_backend,
        case.algorithm,
        &input.codes,
        input.width,
        input.height,
        crate::ffi::libraw_filters(pattern),
    )?;
    let reference_time = reference_started.elapsed();
    let fornax_started = Instant::now();
    let actual = (case.run_fornax)(
        &input.codes,
        input.width,
        input.height,
        pattern,
        sample_type,
    )?;
    let fornax_time = fornax_started.elapsed();
    let metrics_started = Instant::now();
    let evaluation = metrics::calculate(
        &actual,
        &reference,
        input.width,
        input.height,
        border_px,
        pattern,
    )?;
    let (verdict, failed_limits) = metrics::verdict(&evaluation.metrics, case.tolerance);
    let metrics_time = metrics_started.elapsed();
    let artifacts_started = Instant::now();
    let artifacts = if let Some(output_dir) = artifact_dir {
        Some(write_artifacts(ArtifactInput {
            actual: &actual,
            reference: &reference,
            width: input.width,
            height: input.height,
            pattern,
            algorithm: case.name,
            reference_name: case.reference_backend.library(),
            evaluation: &evaluation,
            output_dir,
        })?)
    } else {
        None
    };
    let artifacts_time = artifacts_started.elapsed();
    let row = Row {
        case: case.name,
        reference: case.reference_backend.library(),
        source_url: case.source_url,
        pattern,
        tolerance: case.tolerance,
        evaluation,
        verdict,
        failed_limits,
        artifacts,
    };
    let timing = RowTiming {
        algorithm: case.name.to_owned(),
        pattern: pattern.to_string(),
        reference: case.reference_backend.library().to_ascii_lowercase(),
        fornax_ms: timings::millis(fornax_time),
        reference_ms: timings::millis(reference_time),
        metrics_ms: timings::millis(metrics_time),
        artifacts_ms: timings::millis(artifacts_time),
        peak_rss_mb: timings::peak_rss_mb(),
    };
    Ok((row, timing))
}

#[cfg(test)]
pub fn comparison_rows_filtered(
    input: &data::ActiveArea,
    pattern: Option<BayerPattern>,
    algorithm: Option<&str>,
) -> mischief::Result<Vec<Row>> {
    comparison_rows_filtered_with_artifacts(input, pattern, algorithm, false)
}

#[cfg_attr(not(test), allow(dead_code))]
pub fn comparison_rows_filtered_with_artifacts(
    input: &data::ActiveArea,
    pattern: Option<BayerPattern>,
    algorithm: Option<&str>,
    artifacts: bool,
) -> mischief::Result<Vec<Row>> {
    let algorithms = algorithm.map_or_else(Vec::new, |value| vec![value.to_owned()]);
    let patterns = pattern.map_or_else(Vec::new, |value| vec![value]);
    let output_dir = artifacts.then(|| crate::results::repo_path("target/compare-demosaic"));
    comparison_rows(
        input,
        &ComparisonOptions {
            algorithms: &algorithms,
            references: &[],
            patterns: &patterns,
            artifacts,
            output_dir: output_dir.as_deref(),
            jobs: rayon::current_num_threads().min(4),
            border_px: CANONICAL_BORDER_PX,
        },
    )
    .map(|(rows, _)| rows)
}

pub struct ComparisonOptions<'a> {
    pub algorithms: &'a [String],
    pub references: &'a [crate::case::ReferenceBackend],
    pub patterns: &'a [BayerPattern],
    pub artifacts: bool,
    pub output_dir: Option<&'a Path>,
    pub jobs: usize,
    pub border_px: u32,
}

pub fn comparison_rows(
    input: &data::ActiveArea,
    options: &ComparisonOptions<'_>,
) -> mischief::Result<(Vec<Row>, Vec<RowTiming>)> {
    let work = registry::selected_cases(options.algorithms, options.references, options.patterns);
    if options.jobs == 0 {
        return Err(mischief::mischief!("--jobs must be at least 1"));
    }
    let compare = || {
        work.par_iter()
            .map(|selection| {
                compare_timed(
                    selection.case,
                    input,
                    selection.pattern,
                    SampleType::F32,
                    if options.artifacts {
                        options.output_dir
                    } else {
                        None
                    },
                    options.border_px,
                )
            })
            .collect::<mischief::Result<Vec<_>>>()
    };
    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(options.jobs)
        .build()
        .map_err(|error| mischief::mischief!("cannot create worker pool: {error}"))?;
    let comparisons = pool.install(compare)?;
    Ok(comparisons.into_iter().unzip())
}
