use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::case::Tolerance;
use crate::{data, metrics, report};

pub const CANONICAL_SCHEMA_VERSION: u32 = 3;
pub const SUMMARY_SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_RESULTS_DIR: &str = "results";
pub const CANONICAL_RESULTS_FILE: &str = "canonical.json";
pub const SUMMARY_RESULTS_FILE: &str = "summary.json";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ComparisonResults {
    pub schema_version: u32,
    pub input: ComparisonInput,
    pub reference_builds: ReferenceBuilds,
    pub comparisons: Vec<ComparisonResult>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ComparisonInput {
    pub fixture: String,
    pub source: String,
    pub pattern: String,
    pub width: u32,
    pub height: u32,
    pub sample_type: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ReferenceBuilds {
    pub libraw_version: String,
    pub libraw_commit: String,
    pub darktable_release: String,
    pub darktable_commit: String,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ComparisonResult {
    pub algorithm: String,
    pub reference_library: String,
    pub source_url: String,
    pub bayer_pattern: String,
    pub border: u32,
    pub tolerance: ToleranceResult,
    pub verdict: VerdictResult,
    pub metrics: MetricsResult,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SummaryResults {
    pub schema_version: u32,
    pub input: ComparisonInput,
    pub reference_builds: ReferenceBuilds,
    pub comparisons: Vec<SummaryComparison>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SummaryComparison {
    pub algorithm: String,
    pub reference_library: String,
    pub source_url: String,
    pub bayer_pattern: String,
    pub tolerance: ToleranceResult,
    pub verdict: VerdictResult,
    pub metrics: SummaryMetrics,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct SummaryMetrics {
    pub mean_absolute_error: f64,
    pub root_mean_square_error: f64,
    pub maximum_error: u16,
    pub changed_percent: f64,
    pub p99_error: u16,
    pub over_1024_percent: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ToleranceResult {
    Exact,
    RoundingUp,
    MaxError {
        max_error: u16,
    },
    Robust {
        max_mae: f64,
        max_p99: u16,
        max_over_1024_percent: f64,
    },
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Eq, Serialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum VerdictResult {
    Exact,
    RoundingOnly,
    WithinTolerance { max_error: u16 },
    WithinRobustTolerance,
    Differs,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
pub struct MetricsResult {
    pub mean_absolute_error: f64,
    pub root_mean_square_error: f64,
    pub maximum_error: u16,
    pub changed_percent: f64,
    pub psnr_db: Option<f64>,
    pub below_reference: u64,
    pub p95_error: u16,
    pub p99_error: u16,
    pub p99_9_error: u16,
    pub over_64_percent: f64,
    pub over_1024_percent: f64,
    pub channel_mae: [f64; 3],
    pub luminance_rmse: f64,
    pub chroma_rmse: f64,
}

impl ComparisonResults {
    pub fn build(
        input: data::Input,
        progress: impl FnMut(report::ProgressEvent),
    ) -> mischief::Result<Self> {
        let mut comparisons = report::canonical_rows(&input, progress)?
            .into_iter()
            .map(ComparisonResult::from)
            .collect::<Vec<_>>();
        comparisons.sort_unstable_by(comparison_order);

        Ok(Self {
            schema_version: CANONICAL_SCHEMA_VERSION,
            input: ComparisonInput::from(&input),
            reference_builds: reference_builds(),
            comparisons,
        })
    }

    pub fn summary(&self) -> SummaryResults {
        SummaryResults {
            schema_version: SUMMARY_SCHEMA_VERSION,
            input: self.input.clone(),
            reference_builds: self.reference_builds.clone(),
            comparisons: self
                .comparisons
                .iter()
                .map(SummaryComparison::from)
                .collect(),
        }
    }

    pub fn write(&self, path: &Path) -> mischief::Result<()> { write_json(self, path) }
}

impl SummaryResults {
    pub fn write(&self, path: &Path) -> mischief::Result<()> { write_json(self, path) }
}

impl From<&data::Input> for ComparisonInput {
    fn from(input: &data::Input) -> Self {
        Self {
            fixture: "colorchart-eos-7d".to_owned(),
            source: input.source.clone(),
            pattern: input.pattern.to_string(),
            width: input.width,
            height: input.height,
            sample_type: "f32".to_owned(),
        }
    }
}

impl From<report::Row> for ComparisonResult {
    fn from(row: report::Row) -> Self {
        Self {
            algorithm: row.case.to_owned(),
            reference_library: row.reference.to_owned(),
            source_url: row.source_url.to_owned(),
            bayer_pattern: row.pattern.to_string(),
            border: row.border,
            tolerance: row.tolerance.into(),
            verdict: row.verdict.into(),
            metrics: row.metrics.into(),
        }
    }
}

impl From<&ComparisonResult> for SummaryComparison {
    fn from(result: &ComparisonResult) -> Self {
        Self {
            algorithm: result.algorithm.clone(),
            reference_library: result.reference_library.clone(),
            source_url: result.source_url.clone(),
            bayer_pattern: result.bayer_pattern.clone(),
            tolerance: result.tolerance,
            verdict: result.verdict,
            metrics: result.metrics.into(),
        }
    }
}

fn reference_builds() -> ReferenceBuilds {
    ReferenceBuilds {
        libraw_version: report::LIBRAW_VERSION.to_owned(),
        libraw_commit: report::LIBRAW_COMMIT.to_owned(),
        darktable_release: report::DARKTABLE_RELEASE.to_owned(),
        darktable_commit: report::DARKTABLE_COMMIT.to_owned(),
    }
}

fn comparison_order(left: &ComparisonResult, right: &ComparisonResult) -> std::cmp::Ordering {
    (
        left.algorithm.to_ascii_lowercase(),
        left.reference_library.to_ascii_lowercase(),
        left.bayer_pattern.as_str(),
    )
        .cmp(&(
            right.algorithm.to_ascii_lowercase(),
            right.reference_library.to_ascii_lowercase(),
            right.bayer_pattern.as_str(),
        ))
}

fn write_json(value: &impl Serialize, path: &Path) -> mischief::Result<()> {
    if let Some(parent) = path.parent() {
        std::fs::create_dir_all(parent)?;
    }
    let mut json = serde_json::to_string_pretty(value)
        .map_err(|error| mischief::mischief!("cannot serialize results: {error}"))?;
    json.push('\n');
    std::fs::write(path, json)?;
    Ok(())
}

impl From<Tolerance> for ToleranceResult {
    fn from(tolerance: Tolerance) -> Self {
        match tolerance {
            Tolerance::Exact => Self::Exact,
            Tolerance::RoundingUp => Self::RoundingUp,
            Tolerance::MaxError(max_error) => Self::MaxError { max_error },
            Tolerance::Robust {
                max_mae,
                max_p99,
                max_over_1024_percent,
            } => Self::Robust {
                max_mae,
                max_p99,
                max_over_1024_percent,
            },
        }
    }
}

impl From<metrics::Verdict> for VerdictResult {
    fn from(verdict: metrics::Verdict) -> Self {
        match verdict {
            metrics::Verdict::Exact => Self::Exact,
            metrics::Verdict::RoundingOnly => Self::RoundingOnly,
            metrics::Verdict::WithinTolerance(max_error) => Self::WithinTolerance { max_error },
            metrics::Verdict::WithinRobustTolerance => Self::WithinRobustTolerance,
            metrics::Verdict::Differs => Self::Differs,
        }
    }
}

impl From<metrics::Metrics> for MetricsResult {
    fn from(metrics: metrics::Metrics) -> Self {
        Self {
            mean_absolute_error: metrics.mean_absolute_error,
            root_mean_square_error: metrics.root_mean_square_error,
            maximum_error: metrics.maximum_error,
            changed_percent: metrics.changed_percent,
            psnr_db: metrics
                .peak_signal_to_noise_ratio
                .is_finite()
                .then_some(metrics.peak_signal_to_noise_ratio),
            below_reference: metrics.below_reference,
            p95_error: metrics.p95_error,
            p99_error: metrics.p99_error,
            p99_9_error: metrics.p99_9_error,
            over_64_percent: metrics.over_64_percent,
            over_1024_percent: metrics.over_1024_percent,
            channel_mae: metrics.channel_mae,
            luminance_rmse: metrics.luminance_rmse,
            chroma_rmse: metrics.chroma_rmse,
        }
    }
}

impl From<MetricsResult> for SummaryMetrics {
    fn from(metrics: MetricsResult) -> Self {
        Self {
            mean_absolute_error: metrics.mean_absolute_error,
            root_mean_square_error: metrics.root_mean_square_error,
            maximum_error: metrics.maximum_error,
            changed_percent: metrics.changed_percent,
            p99_error: metrics.p99_error,
            over_1024_percent: metrics.over_1024_percent,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn results() -> ComparisonResults {
        ComparisonResults {
            schema_version: CANONICAL_SCHEMA_VERSION,
            input: ComparisonInput {
                fixture: "colorchart-eos-7d".to_owned(),
                source: data::CANONICAL_SOURCE.to_owned(),
                pattern: "GBRG".to_owned(),
                width: 5_202,
                height: 3_464,
                sample_type: "f32".to_owned(),
            },
            reference_builds: ReferenceBuilds {
                libraw_version: "1".to_owned(),
                libraw_commit: "2".to_owned(),
                darktable_release: "3".to_owned(),
                darktable_commit: "4".to_owned(),
            },
            comparisons: vec![ComparisonResult {
                algorithm: "AHD".to_owned(),
                reference_library: "LibRaw".to_owned(),
                source_url: "https://example.com/source".to_owned(),
                bayer_pattern: "GBRG".to_owned(),
                border: 5,
                tolerance: ToleranceResult::Robust {
                    max_mae: 2.0,
                    max_p99: 64,
                    max_over_1024_percent: 0.1,
                },
                verdict: VerdictResult::Differs,
                metrics: MetricsResult {
                    mean_absolute_error: 3.0,
                    root_mean_square_error: 4.0,
                    maximum_error: 100,
                    changed_percent: 25.0,
                    psnr_db: Some(80.0),
                    below_reference: 10,
                    p95_error: 8,
                    p99_error: 20,
                    p99_9_error: 50,
                    over_64_percent: 1.0,
                    over_1024_percent: 0.0,
                    channel_mae: [1.0, 2.0, 3.0],
                    luminance_rmse: 5.0,
                    chroma_rmse: 6.0,
                },
            }],
        }
    }

    #[test]
    fn summary_keeps_readme_fields_and_omits_detailed_metrics() -> mischief::Result<()> {
        let summary = results().summary();
        let value = serde_json::to_value(summary)?;
        assert_eq!(value["schema_version"], SUMMARY_SCHEMA_VERSION);
        assert_eq!(value["comparisons"][0]["metrics"]["p99_error"], 20);
        assert!(
            value["comparisons"][0]["metrics"]
                .get("p95_error")
                .is_none()
        );
        assert!(
            value["comparisons"][0]["metrics"]
                .get("channel_mae")
                .is_none()
        );
        assert!(value["comparisons"][0].get("border").is_none());
        Ok(())
    }

    #[test]
    fn canonical_retains_detailed_metrics_and_border() -> mischief::Result<()> {
        let value = serde_json::to_value(results())?;
        assert_eq!(value["schema_version"], CANONICAL_SCHEMA_VERSION);
        assert_eq!(value["comparisons"][0]["border"], 5);
        assert_eq!(value["comparisons"][0]["metrics"]["p95_error"], 8);
        assert_eq!(value["comparisons"][0]["metrics"]["channel_mae"][1], 2.0);
        Ok(())
    }
}
