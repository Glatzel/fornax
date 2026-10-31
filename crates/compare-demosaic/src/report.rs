use fornax_core::BayerPattern;
use rayon::prelude::*;

use crate::artifacts::{ArtifactPaths, write as write_artifacts};
use crate::case::{Case, SampleType, Tolerance};
use crate::metrics::{self, Evaluation, FailedLimit, Verdict};
use crate::{cases, data};

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

pub fn compare(
    case: &'static Case,
    input: &data::ActiveArea,
    pattern: BayerPattern,
    sample_type: SampleType,
    artifacts: bool,
) -> mischief::Result<Row> {
    let reference = crate::ffi::run(
        case.reference_backend,
        case.algorithm,
        &input.codes,
        input.width,
        input.height,
        crate::ffi::libraw_filters(pattern),
    )?;
    let actual = (case.run_fornax)(
        &input.codes,
        input.width,
        input.height,
        pattern,
        sample_type,
    )?;
    let evaluation = metrics::calculate(
        &actual,
        &reference,
        input.width,
        input.height,
        CANONICAL_BORDER_PX,
        pattern,
    )?;
    let (verdict, failed_limits) = metrics::verdict(&evaluation.metrics, case.tolerance);
    let artifacts = if artifacts || verdict == Verdict::Fail {
        Some(write_artifacts(
            &actual,
            &reference,
            input.width,
            input.height,
            pattern,
            case.name,
            case.reference_backend.library(),
            &evaluation,
        )?)
    } else {
        None
    };
    Ok(Row {
        case: case.name,
        reference: case.reference_backend.library(),
        source_url: case.source_url,
        pattern,
        tolerance: case.tolerance,
        evaluation,
        verdict,
        failed_limits,
        artifacts,
    })
}

#[cfg(test)]
pub fn comparison_rows_filtered(
    input: &data::ActiveArea,
    pattern: Option<BayerPattern>,
    algorithm: Option<&str>,
) -> mischief::Result<Vec<Row>> {
    comparison_rows_filtered_with_artifacts(input, pattern, algorithm, false)
}

pub fn comparison_rows_filtered_with_artifacts(
    input: &data::ActiveArea,
    pattern: Option<BayerPattern>,
    algorithm: Option<&str>,
    artifacts: bool,
) -> mischief::Result<Vec<Row>> {
    let patterns = pattern.map_or_else(|| BayerPattern::ALL.to_vec(), |pattern| vec![pattern]);
    let jobs = patterns
        .into_iter()
        .flat_map(|pattern| cases::CASES.iter().map(move |case| (pattern, case)))
        .filter(|(_, case)| {
            algorithm.is_none_or(|algorithm| case.name.eq_ignore_ascii_case(algorithm))
        })
        .collect::<Vec<_>>();
    jobs.par_iter()
        .map(|(pattern, case)| compare(case, input, *pattern, SampleType::F32, artifacts))
        .collect()
}
