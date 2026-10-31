use fornax_core::BayerPattern;
use rayon::prelude::*;

use crate::case::{Case, SampleType};
use crate::{cases, data, ffi, metrics};

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
    pub border: u32,
    pub tolerance: crate::case::Tolerance,
    pub metrics: metrics::Metrics,
    pub verdict: metrics::Verdict,
}

pub fn compare(
    case: &'static Case,
    input: &data::ActiveArea,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Row> {
    let reference = ffi::run(
        case.reference_backend,
        case.algorithm,
        &input.codes,
        input.width,
        input.height,
        ffi::libraw_filters(pattern),
    )?;
    let actual = (case.run_fornax)(
        &input.codes,
        input.width,
        input.height,
        pattern,
        sample_type,
    )?;
    let metrics = metrics::calculate(&actual, &reference, input.width, input.height, case.border)?;
    let verdict = metrics.verdict(case.tolerance);
    Ok(Row {
        case: case.name,
        reference: case.reference_backend.library(),
        source_url: case.source_url,
        pattern,
        border: case.border,
        tolerance: case.tolerance,
        metrics,
        verdict,
    })
}

pub fn comparison_rows(input: &data::ActiveArea) -> mischief::Result<Vec<Row>> {
    let jobs = BayerPattern::ALL
        .iter()
        .copied()
        .flat_map(|pattern| cases::CASES.iter().map(move |case| (pattern, case)))
        .collect::<Vec<_>>();
    jobs.par_iter()
        .map(|(pattern, case)| compare(case, input, *pattern, SampleType::F32))
        .collect()
}
