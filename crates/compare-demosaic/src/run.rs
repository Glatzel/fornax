use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use fornax_core::BayerPattern;
use rayon::prelude::*;

use crate::cases::{self, Case, SampleType, Tolerance};
use crate::metrics::{self, Evaluation, FailedLimit, Verdict as DetailedVerdict};
use crate::report::{
    self, Channels, InputMeta, Kind, Meta, PatternSource, References, Report, Row, Selection,
    Sites, Verdict, WorstSample,
};
use crate::status;
use crate::waivers::WAIVERS;

pub const LIBRAW_COMMIT: &str = env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT");
pub const DARKTABLE_RELEASE: &str = env!("COMPARE_DEMOSAIC_DARKTABLE_RELEASE");
pub const DARKTABLE_COMMIT: &str = env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT");
pub const RAWTHERAPEE_RELEASE: &str = env!("COMPARE_DEMOSAIC_RAWTHERAPEE_RELEASE");
pub const RAWTHERAPEE_COMMIT: &str = env!("COMPARE_DEMOSAIC_RAWTHERAPEE_COMMIT");
pub const LIBRAW_VERSION: &str = env!("COMPARE_DEMOSAIC_LIBRAW_VERSION");

#[derive(Clone, Debug)]
pub struct RunRequest {
    pub algorithms: Vec<String>,
    pub references: Vec<String>,
    pub patterns: Vec<BayerPattern>,
    pub inputs: Vec<PathBuf>,
    pub out: PathBuf,
    pub shard: Option<String>,
    pub artifacts: bool,
    pub border_px: u32,
    pub jobs: usize,
}

#[derive(Debug)]
pub struct RunOutput {
    pub path: PathBuf,
    pub report: Report,
    pub artifacts: bool,
}
#[cfg(test)]
#[derive(Clone, Debug)]
pub(crate) struct CaseEvaluation {
    pub case: &'static Case,
    pub pattern: BayerPattern,
    pub evaluation: Evaluation,
    pub verdict: DetailedVerdict,
    #[allow(dead_code, reason = "used by the unchanged snapshot regression tests")]
    pub failed_limits: Vec<FailedLimit>,
}

struct CaseComputation {
    case: &'static Case,
    pattern: BayerPattern,
    actual: Vec<u16>,
    reference: Vec<u16>,
    evaluation: Evaluation,
    verdict: DetailedVerdict,
    failed_limits: Vec<FailedLimit>,
}

#[derive(Clone, Copy)]
struct Job<'a> {
    input_index: usize,
    case: &'a Case,
    pattern: BayerPattern,
}

pub fn execute(request: &RunRequest) -> mischief::Result<RunOutput> {
    let input_paths = if request.inputs.is_empty() {
        vec![fornax_devtool::raw_file()]
    } else {
        request.inputs.clone()
    };
    let inputs = input_paths
        .iter()
        .map(|path| crate::reference::active_area_from(path))
        .collect::<mischief::Result<Vec<_>>>()?;
    execute_with_inputs(request, inputs)
}

pub(crate) fn execute_with_inputs(
    request: &RunRequest,
    mut inputs: Vec<crate::reference::ActiveArea>,
) -> mischief::Result<RunOutput> {
    let patterns = if request.patterns.is_empty() {
        BayerPattern::ALL.to_vec()
    } else {
        normalize_patterns(&request.patterns)
    };
    let algorithms = normalize_names(&request.algorithms);
    let references = normalize_names(&request.references);
    validate_algorithms(&algorithms)?;
    validate_references(&references)?;
    validate_jobs(request.jobs)?;
    inputs.sort_unstable_by(|left, right| left.source.cmp(&right.source));
    if inputs
        .windows(2)
        .any(|pair| pair[0].source == pair[1].source)
    {
        return Err(mischief::mischief!(
            "selected inputs must have distinct file names"
        ));
    }

    let jobs = inputs
        .iter()
        .enumerate()
        .flat_map(|(input_index, _)| {
            cases::selected_cases_filtered(&algorithms, &references, &patterns)
                .into_iter()
                .map(move |selection| Job {
                    input_index,
                    case: selection.case,
                    pattern: selection.pattern,
                })
        })
        .collect::<Vec<_>>();
    if jobs.is_empty() {
        return Err(mischief::mischief!(
            "the selected algorithm/reference/pattern combination has no rows"
        ));
    }

    let pool = rayon::ThreadPoolBuilder::new()
        .num_threads(request.jobs)
        .build()
        .map_err(|error| mischief::mischief!("cannot create worker pool: {error}"))?;
    let computed = pool.install(|| {
        jobs.par_iter()
            .map(|job| {
                let native_input = &inputs[job.input_index];
                let input = crate::reference::shifted_to_pattern(native_input, job.pattern)?;
                let computation = compute_case(job.case, &input, job.pattern, request.border_px)?;
                let row = row_from_computation(native_input, &computation)?;
                if request.artifacts {
                    crate::artifacts::write(
                        &request.out,
                        &native_input.source,
                        job.case.name,
                        job.case.reference_backend.library(),
                        job.pattern,
                        &computation.actual,
                        &computation.reference,
                        input.width,
                        input.height,
                        &computation.evaluation,
                    )?;
                }
                Ok(row)
            })
            .collect::<Vec<mischief::Result<Row>>>()
    });
    let rows = computed.into_iter().collect::<mischief::Result<Vec<_>>>()?;
    let selection = Selection {
        algorithm: algorithms,
        reference: references,
        pattern: if request.patterns.is_empty() {
            Vec::new()
        } else {
            patterns.iter().map(ToString::to_string).collect()
        },
        input: if request.inputs.is_empty() {
            Vec::new()
        } else {
            inputs.iter().map(|input| input.source.clone()).collect()
        },
    };
    let report = Report::new(
        Meta {
            inputs: inputs.iter().map(input_meta).collect(),
            references: References {
                libraw: LIBRAW_VERSION.to_owned(),
                darktable: DARKTABLE_RELEASE.to_owned(),
                rawtherapee: RAWTHERAPEE_RELEASE.to_owned(),
            },
            limits: report::LIMITS,
            border_px: request.border_px,
            pattern_source: PatternSource::ShiftedCrops,
            selection,
        },
        rows,
    );
    let path = output_path(request)?;
    report.write(&path)?;
    Ok(RunOutput {
        path,
        report,
        artifacts: request.artifacts,
    })
}

#[cfg(test)]
pub(crate) fn selected_evaluations(
    input: &crate::reference::ActiveArea,
    algorithms: &[String],
    border_px: u32,
) -> mischief::Result<Vec<CaseEvaluation>> {
    cases::selected_cases(algorithms)
        .into_iter()
        .map(|selection| evaluate_case(selection.case, input, selection.pattern, border_px))
        .collect()
}
#[cfg(test)]
pub(crate) fn evaluate_case(
    case: &'static Case,
    input: &crate::reference::ActiveArea,
    pattern: BayerPattern,
    border_px: u32,
) -> mischief::Result<CaseEvaluation> {
    let input = crate::reference::shifted_to_pattern(input, pattern)?;
    let computation = compute_case(case, &input, pattern, border_px)?;
    Ok(CaseEvaluation {
        case,
        pattern,
        evaluation: computation.evaluation,
        verdict: computation.verdict,
        failed_limits: computation.failed_limits,
    })
}

fn compute_case(
    case: &'static Case,
    input: &crate::reference::ActiveArea,
    pattern: BayerPattern,
    border_px: u32,
) -> mischief::Result<CaseComputation> {
    let reference = crate::reference::run(
        case.reference_backend,
        case.algorithm,
        &input.codes,
        input.width,
        input.height,
        crate::reference::libraw_filters(pattern),
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
    Ok(CaseComputation {
        case,
        pattern,
        actual,
        reference,
        evaluation,
        verdict,
        failed_limits,
    })
}

fn row_from_computation(
    native_input: &crate::reference::ActiveArea,
    evaluation: &CaseComputation,
) -> mischief::Result<Row> {
    let metrics = &evaluation.evaluation.metrics;
    let kind = kind(evaluation.case.tolerance);
    let (verdict, failed_limits) = report::verdict(
        kind,
        &report_metrics(metrics),
        metrics.total_samples,
        report::LIMITS,
    );
    debug_assert_eq!(
        verdict == Verdict::Pass,
        evaluation.verdict != DetailedVerdict::Fail
    );
    let crop = crate::reference::pattern_offset(native_input.native_pattern, evaluation.pattern)
        .ok_or_else(|| mischief::mischief!("no Bayer crop converts to pattern"))?;
    Ok(Row {
        input: native_input.source.clone(),
        algorithm: evaluation.case.name.to_ascii_lowercase(),
        reference: evaluation
            .case
            .reference_backend
            .library()
            .to_ascii_lowercase(),
        pattern: evaluation.pattern.to_string(),
        crop: [crop.0, crop.1],
        native: evaluation.pattern == native_input.native_pattern,
        kind,
        samples: metrics.total_samples,
        verdict,
        failed_limits,
        all: report_metrics(metrics),
        interior: report_metrics(&evaluation.evaluation.metrics_interior),
        channels: Channels {
            r: report_metrics(&evaluation.evaluation.per_channel[0]),
            g: report_metrics(&evaluation.evaluation.per_channel[1]),
            b: report_metrics(&evaluation.evaluation.per_channel[2]),
        },
        sites: Sites {
            sampled: report_metrics(&evaluation.evaluation.per_site[0]),
            interpolated: report_metrics(&evaluation.evaluation.per_site[1]),
        },
        worst: evaluation
            .evaluation
            .worst_pixels
            .iter()
            .map(|sample| WorstSample {
                x: sample.x,
                y: sample.y,
                c: sample.c,
                fornax: sample.fornax,
                reference: sample.reference,
            })
            .collect(),
    })
}

pub(crate) const fn report_metrics(metrics: &metrics::Metrics) -> report::Metrics {
    report::Metrics {
        mae: metrics.mae,
        rmse: metrics.rmse,
        max: metrics.max,
        mean_signed: metrics.mean_signed,
        p99: metrics.p99,
        p999: metrics.p999,
        p9999: metrics.p9999,
        frac_gt1: metrics.frac_gt1,
        frac_gt16: metrics.frac_gt16,
        count_gt1024: metrics.count_gt1024,
        changed_frac: metrics.changed_frac,
    }
}

fn kind(tolerance: Tolerance) -> Kind {
    match tolerance {
        Tolerance::Exact => Kind::Exact,
        Tolerance::RoundingUp => Kind::Rounding,
        Tolerance::Robust { .. } => Kind::Robust,
        Tolerance::MaxError(_) => unreachable!("registered comparison cases use three kinds"),
    }
}

fn input_meta(input: &crate::reference::ActiveArea) -> InputMeta {
    InputMeta {
        name: input.source.clone(),
        sha256: input.sha256.clone(),
        width: input.width,
        height: input.height,
        black: input.black.clone(),
        black_width: input.black_width,
        black_height: input.black_height,
        white: input.white,
        native_pattern: input.native_pattern.to_string(),
    }
}

fn validate_algorithms(algorithms: &[String]) -> mischief::Result<()> {
    let valid = cases::algorithm_names()
        .into_iter()
        .map(str::to_ascii_lowercase)
        .collect::<BTreeSet<_>>();
    for algorithm in algorithms {
        if !valid.contains(algorithm) {
            return Err(mischief::mischief!(
                "unknown algorithm `{algorithm}`; valid algorithms: {}",
                valid.into_iter().collect::<Vec<_>>().join(", ")
            ));
        }
    }
    Ok(())
}

fn validate_references(references: &[String]) -> mischief::Result<()> {
    let valid = cases::reference_names();
    for reference in references {
        if !valid
            .iter()
            .any(|name| name.eq_ignore_ascii_case(reference))
        {
            return Err(mischief::mischief!(
                "unknown reference `{reference}`; valid references: {}",
                valid.join(", ")
            ));
        }
    }
    Ok(())
}

fn validate_jobs(jobs: usize) -> mischief::Result<()> {
    if jobs == 0 {
        Err(mischief::mischief!("--jobs must be at least 1"))
    } else {
        Ok(())
    }
}

fn normalize_names(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn normalize_patterns(patterns: &[BayerPattern]) -> Vec<BayerPattern> {
    patterns
        .iter()
        .copied()
        .collect::<BTreeSet<_>>()
        .into_iter()
        .collect()
}

fn output_path(request: &RunRequest) -> mischief::Result<PathBuf> {
    let file = match &request.shard {
        Some(name) => {
            if name.is_empty() || name == "." || name == ".." || name.contains(['/', '\\']) {
                return Err(mischief::mischief!("--shard must be a file-name component"));
            }
            format!("{name}.shard.json")
        }
        None => "canonical.json".to_owned(),
    };
    Ok(request.out.join(file))
}

pub fn render_text(output: &RunOutput) -> String {
    let mut text = String::from(
        "algorithm\treference\tpattern\tnative\tverdict\tMAE\tP99\tP99.9\tmax\t>1024\n",
    );
    for row in &output.report.rows {
        text.push_str(&format!(
            "{}\t{}\t{}\t{}\t{}\t{:.6}\t{}\t{}\t{}\t{}\n",
            cases::display_name(&row.algorithm),
            row.reference,
            row.pattern,
            if row.native { "yes" } else { "no" },
            status::evaluate(row, WAIVERS).symbol(),
            row.all.mae,
            row.all.p99,
            row.all.p999,
            row.all.max,
            row.all.count_gt1024,
        ));
    }
    text.push_str(&format!("\nwrote {}\n", output.path.display()));
    if output.artifacts {
        text.push_str(&format!(
            "artifacts {}\n",
            output
                .path
                .parent()
                .unwrap_or_else(|| Path::new("."))
                .join("artifacts")
                .display()
        ));
    }
    text
}

pub fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(4)
}

pub fn repo_path(relative_path: &str) -> PathBuf { fornax_devtool::root_dir().join(relative_path) }

#[cfg(test)]
mod tests {
    use fornax_devtool::synthetic_mono_image;

    use super::*;

    fn input() -> crate::reference::ActiveArea {
        let image = synthetic_mono_image::<f32>((128, 128), 65_535.0);
        crate::reference::ActiveArea {
            source: "synthetic.raw".to_owned(),
            sha256: "ab".repeat(32),
            width: 128,
            height: 128,
            codes: image
                .as_raw()
                .iter()
                .map(|sample| sample.round().clamp(0.0, 65_535.0) as u16)
                .collect(),
            black: vec![0.0; 4],
            black_width: 2,
            black_height: 2,
            white: 65_535.0,
            native_pattern: BayerPattern::RGGB,
        }
    }

    fn request(out: &Path, jobs: usize) -> RunRequest {
        RunRequest {
            algorithms: vec!["ahd".to_owned()],
            references: vec!["libraw".to_owned()],
            patterns: vec![BayerPattern::RGGB],
            inputs: Vec::new(),
            out: out.to_owned(),
            shard: None,
            artifacts: false,
            border_px: 16,
            jobs,
        }
    }

    #[test]
    fn filtered_runs_record_selection_and_reject_unknown_names() -> mischief::Result<()> {
        let directory = tempfile::tempdir().expect("temporary directory");
        let output = execute_with_inputs(&request(directory.path(), 1), vec![input()])?;
        assert_eq!(output.report.rows.len(), 1);
        assert_eq!(output.report.meta.selection.algorithm, ["ahd"]);
        assert_eq!(output.report.meta.selection.reference, ["libraw"]);
        assert_eq!(output.report.meta.selection.pattern, ["RGGB"]);
        assert!(output.report.meta.selection.input.is_empty());

        let mut invalid = request(directory.path(), 1);
        invalid.algorithms = vec!["not-an-algorithm".to_owned()];
        let error = execute_with_inputs(&invalid, vec![input()]).expect_err("unknown algorithm");
        assert!(error.to_string().contains("valid algorithms"));
        Ok(())
    }

    #[test]
    fn shifted_patterns_record_original_input_dimensions_and_crop() -> mischief::Result<()> {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut native = input();
        native.native_pattern = BayerPattern::GBRG;
        let mut request = request(directory.path(), 1);
        request.patterns = vec![BayerPattern::RGGB];

        let output = execute_with_inputs(&request, vec![native])?;
        let row = &output.report.rows[0];
        assert_eq!(row.crop, [0, 1]);
        assert!(!row.native);
        assert_eq!(row.samples, 128 * 127 * 3);
        assert_eq!(output.report.meta.inputs[0].height, 128);
        assert_eq!(output.report.meta.inputs[0].native_pattern, "GBRG");
        Ok(())
    }

    #[test]
    fn jobs_one_and_four_are_byte_identical() -> mischief::Result<()> {
        let first = tempfile::tempdir().expect("temporary directory");
        let second = tempfile::tempdir().expect("temporary directory");
        let one = execute_with_inputs(&request(first.path(), 1), vec![input()])?;
        let four = execute_with_inputs(&request(second.path(), 4), vec![input()])?;
        assert_eq!(std::fs::read(one.path)?, std::fs::read(four.path)?);
        Ok(())
    }

    #[test]
    fn artifacts_use_the_documented_layout_without_entering_json() -> mischief::Result<()> {
        let directory = tempfile::tempdir().expect("temporary directory");
        let mut request = request(directory.path(), 1);
        request.artifacts = true;
        let output = execute_with_inputs(&request, vec![input()])?;
        let artifact_root = directory.path().join("artifacts/synthetic-raw");
        assert!(artifact_root.join("ahd-libraw-RGGB/error.png").is_file());
        assert!(artifact_root.join("ahd-libraw-RGGB/tiles.json").is_file());
        let json = std::fs::read_to_string(output.path)?;
        assert!(!json.contains("error.png"));
        assert!(!json.contains("tiles.json"));
        Ok(())
    }
}
