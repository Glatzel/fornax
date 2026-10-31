use std::path::{Path, PathBuf};

use fornax_core::BayerPattern;
use serde::{Deserialize, Serialize};

use crate::case::Tolerance;
use crate::{data, metrics, report};

pub const SCHEMA_VERSION: u32 = 1;
pub const DEFAULT_SUMMARY_FILE: &str = "crates/compare-demosaic/summary.json";
pub const FILTERED_SUMMARY_FILE: &str = "target/compare-demosaic/summary.json";

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct SummaryResults {
    pub schema_version: u32,
    pub source: String,
    pub width: u32,
    pub height: u32,
    pub sample_type: String,
    pub reference_builds: ReferenceBuilds,
    pub comparisons: Vec<SummaryComparison>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct ReferenceBuilds {
    pub libraw_version: String,
    pub libraw_commit: String,
    pub darktable_release: String,
    pub darktable_commit: String,
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

pub fn generate() -> mischief::Result<SummaryResults> { generate_filtered(None, None) }

pub fn generate_filtered(
    pattern: Option<BayerPattern>,
    algorithm: Option<&str>,
) -> mischief::Result<SummaryResults> {
    let input = data::active_area()?;
    let mut comparisons = report::comparison_rows_filtered(&input, pattern, algorithm)?
        .into_iter()
        .map(SummaryComparison::from)
        .collect::<Vec<_>>();
    comparisons.sort_unstable_by(comparison_order);
    Ok(SummaryResults {
        schema_version: SCHEMA_VERSION,
        source: input.source,
        width: input.width,
        height: input.height,
        sample_type: "f32".to_owned(),
        reference_builds: reference_builds(),
        comparisons,
    })
}

impl SummaryResults {
    pub fn write(&self, path: &Path) -> mischief::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut json = serde_json::to_string_pretty(self)
            .map_err(|error| mischief::mischief!("cannot serialize summary: {error}"))?;
        json.push('\n');
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn write_default(&self) -> mischief::Result<PathBuf> {
        self.write_repo_file(DEFAULT_SUMMARY_FILE)
    }

    pub fn write_filtered(&self) -> mischief::Result<PathBuf> {
        self.write_repo_file(FILTERED_SUMMARY_FILE)
    }

    fn write_repo_file(&self, relative_path: &str) -> mischief::Result<PathBuf> {
        let path = fornax_devtool::root_dir().join(relative_path);
        self.write(&path)?;
        Ok(path)
    }
}

impl From<report::Row> for SummaryComparison {
    fn from(row: report::Row) -> Self {
        Self {
            algorithm: row.case.to_owned(),
            reference_library: row.reference.to_owned(),
            source_url: row.source_url.to_owned(),
            bayer_pattern: row.pattern.to_string(),
            tolerance: row.tolerance.into(),
            verdict: row.verdict.into(),
            metrics: row.metrics.into(),
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

fn comparison_order(left: &SummaryComparison, right: &SummaryComparison) -> std::cmp::Ordering {
    (
        left.algorithm.to_ascii_lowercase(),
        left.bayer_pattern.as_str(),
        left.reference_library.to_ascii_lowercase(),
    )
        .cmp(&(
            right.algorithm.to_ascii_lowercase(),
            right.bayer_pattern.as_str(),
            right.reference_library.to_ascii_lowercase(),
        ))
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

impl From<metrics::Metrics> for SummaryMetrics {
    fn from(metrics: metrics::Metrics) -> Self {
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
