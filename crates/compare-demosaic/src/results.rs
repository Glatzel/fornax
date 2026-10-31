use std::path::Path;

use serde::{Deserialize, Serialize};

use crate::case::Tolerance;
use crate::{data, metrics, report};

pub const SCHEMA_VERSION: u32 = 2;
pub const DEFAULT_RESULTS_FILE: &str = "results/canonical.json";

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
    pub kind: String,
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
    pub fn canonical() -> mischief::Result<Self> {
        let input = data::canonical();
        let mut comparisons = report::canonical_rows()?
            .into_iter()
            .map(ComparisonResult::from)
            .collect::<Vec<_>>();
        comparisons.sort_unstable_by(|left, right| {
            (
                left.algorithm.to_ascii_lowercase(),
                left.reference_library.to_ascii_lowercase(),
                left.bayer_pattern.clone(),
            )
                .cmp(&(
                    right.algorithm.to_ascii_lowercase(),
                    right.reference_library.to_ascii_lowercase(),
                    right.bayer_pattern.clone(),
                ))
        });

        Ok(Self {
            schema_version: SCHEMA_VERSION,
            input: ComparisonInput {
                fixture: input.label,
                source: input.source,
                kind: input.kind,
                pattern: input.pattern.expect("canonical pattern").to_string(),
                width: input.width,
                height: input.height,
                sample_type: "f32".to_owned(),
            },
            reference_builds: ReferenceBuilds {
                libraw_version: report::LIBRAW_VERSION.to_owned(),
                libraw_commit: report::LIBRAW_COMMIT.to_owned(),
                darktable_release: report::DARKTABLE_RELEASE.to_owned(),
                darktable_commit: report::DARKTABLE_COMMIT.to_owned(),
            },
            comparisons,
        })
    }

    pub fn to_pretty_json(&self) -> mischief::Result<String> {
        let mut json = serde_json::to_string_pretty(self)
            .map_err(|error| mischief::mischief!("cannot serialize comparison results: {error}"))?;
        json.push('\n');
        Ok(json)
    }

    pub fn write(&self, path: &Path) -> mischief::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        std::fs::write(path, self.to_pretty_json()?)?;
        Ok(())
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
