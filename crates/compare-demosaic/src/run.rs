use std::path::PathBuf;

use fornax_core::BayerPattern;

use crate::case::{Case, SampleType, Tolerance};
use crate::metrics::{self, Evaluation, FailedLimit, Verdict as DetailedVerdict};
use crate::registry;
use crate::report::{self, Kind, Limits, Meta, References, Report, Row, Verdict, VerdictInput};

pub const LIBRAW_COMMIT: &str = env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT");
pub const DARKTABLE_RELEASE: &str = env!("COMPARE_DEMOSAIC_DARKTABLE_RELEASE");
pub const DARKTABLE_COMMIT: &str = env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT");
pub const LIBRAW_VERSION: &str = env!("COMPARE_DEMOSAIC_LIBRAW_VERSION");

#[derive(Clone, Debug)]
pub struct RunRequest {
    pub algorithms: Vec<String>,
    pub input: Option<PathBuf>,
    pub border_px: u32,
}

#[derive(Debug)]
pub struct RunOutput {
    pub path: PathBuf,
    pub report: Report,
}

#[derive(Clone, Debug)]
pub(crate) struct CaseEvaluation {
    pub case: &'static Case,
    pub pattern: BayerPattern,
    pub evaluation: Evaluation,
    pub verdict: DetailedVerdict,
    #[allow(dead_code, reason = "used by the unchanged snapshot regression tests")]
    pub failed_limits: Vec<FailedLimit>,
}

pub fn execute(request: &RunRequest) -> mischief::Result<RunOutput> {
    let input_path = request
        .input
        .clone()
        .unwrap_or_else(fornax_devtool::raw_file);
    let input = crate::data::active_area_from(&input_path)?;
    let rows = selected_evaluations(&input, &request.algorithms, request.border_px)?
        .into_iter()
        .map(|evaluation| row_from_evaluation(&evaluation))
        .collect::<mischief::Result<Vec<_>>>()?;
    let report = Report::new(
        Meta {
            input: input.source.clone(),
            width: input.width,
            height: input.height,
            native_pattern: input.native_pattern.to_string(),
            references: References {
                libraw: LIBRAW_VERSION.to_owned(),
                darktable: DARKTABLE_RELEASE.to_owned(),
            },
            limits: Limits {
                mae: report::LIMITS.mae,
                p99: report::LIMITS.p99,
                high_err: report::LIMITS.high_err,
                high_err_frac: report::LIMITS.high_err_frac,
            },
            border_px: request.border_px,
        },
        rows,
    );
    let path = report.write_canonical()?;
    Ok(RunOutput { path, report })
}

pub(crate) fn selected_evaluations(
    input: &crate::data::ActiveArea,
    algorithms: &[String],
    border_px: u32,
) -> mischief::Result<Vec<CaseEvaluation>> {
    registry::selected_cases(algorithms)
        .into_iter()
        .map(|selection| evaluate_case(selection.case, input, selection.pattern, border_px))
        .collect()
}

#[cfg_attr(
    not(test),
    allow(dead_code, reason = "used by comparison crate integration tests")
)]
pub(crate) fn evaluate_case(
    case: &'static Case,
    input: &crate::data::ActiveArea,
    pattern: BayerPattern,
    border_px: u32,
) -> mischief::Result<CaseEvaluation> {
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
        SampleType::F32,
    )?;
    let evaluation = metrics::calculate(
        &actual,
        &reference,
        input.width,
        input.height,
        border_px,
        pattern,
    )?;
    let (verdict, failed_limits) = metrics::verdict(&evaluation.metrics, case.tolerance);
    Ok(CaseEvaluation {
        case,
        pattern,
        evaluation,
        verdict,
        failed_limits,
    })
}

fn row_from_evaluation(evaluation: &CaseEvaluation) -> mischief::Result<Row> {
    let metrics = &evaluation.evaluation.metrics;
    let kind = kind(evaluation.case.tolerance)?;
    let verdict = report::verdict(
        kind,
        VerdictInput {
            samples: metrics.total_samples,
            mae: metrics.mae,
            p99: metrics.p99,
            max: metrics.max,
            count_gt1024: metrics.count_gt1024,
            count_below_reference: metrics.count_below_reference,
        },
        report::LIMITS,
    );
    debug_assert_eq!(
        verdict == Verdict::Pass,
        evaluation.verdict != DetailedVerdict::Fail
    );
    Ok(Row {
        algorithm: evaluation.case.name.to_ascii_lowercase(),
        reference: evaluation
            .case
            .reference_backend
            .library()
            .to_ascii_lowercase(),
        pattern: evaluation.pattern.to_string(),
        kind,
        verdict,
        samples: metrics.total_samples,
        mae: metrics.mae,
        rmse: metrics.rmse,
        max: metrics.max,
        mean_signed: metrics.mean_signed,
        p99: metrics.p99,
        p999: metrics.p999,
        count_gt1024: metrics.count_gt1024,
        count_gt1024_border: metrics
            .count_gt1024
            .saturating_sub(evaluation.evaluation.metrics_interior.count_gt1024),
        changed_frac: metrics.changed_frac,
    })
}

fn kind(tolerance: Tolerance) -> mischief::Result<Kind> {
    match tolerance {
        Tolerance::Exact => Ok(Kind::Exact),
        Tolerance::RoundingUp => Ok(Kind::Rounding),
        Tolerance::Robust { .. } => Ok(Kind::Robust),
        Tolerance::MaxError(_) => Err(mischief::mischief!(
            "max-error cases are not supported by the simplified report"
        )),
    }
}

pub fn repo_path(relative_path: &str) -> PathBuf { fornax_devtool::root_dir().join(relative_path) }
