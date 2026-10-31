use std::path::{Path, PathBuf};
use std::process::Command;

use fornax_core::BayerPattern;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::artifacts::ArtifactPaths;
use crate::case::Tolerance;
use crate::metrics::{FailedLimit, Metrics, Verdict, WorstPixel};
use crate::report::{self, CANONICAL_BORDER_PX};

pub const SCHEMA_VERSION: u32 = 1;
pub const CANONICAL_FILE: &str = "target/compare-demosaic/canonical.json";
pub const FILTERED_FILE: &str = "target/compare-demosaic/canonical.filtered.json";
pub const BASELINE_FILE: &str = "crates/compare-demosaic/baseline/canonical.json";
pub const SCHEMA_FILE: &str = "crates/compare-demosaic/canonical.schema.json";

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CanonicalResults {
    pub schema_version: u32,
    pub meta: RunMeta,
    pub results: Vec<ComparisonResult>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunMeta {
    pub generated_at: String,
    pub machine: Machine,
    pub fornax_commit: String,
    pub references: References,
    pub input: Input,
    pub border_px: u32,
    pub limits: Limits,
    pub fornax_rounding: String,
    pub fornax_clamping: String,
    pub reference_rounding: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Machine {
    pub os: String,
    pub architecture: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct References {
    pub libraw: ReferenceBuild,
    pub darktable: ReferenceBuild,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceBuild {
    pub version: String,
    pub commit: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Input {
    pub file: String,
    pub path: String,
    pub width: u32,
    pub height: u32,
    pub black: Vec<f64>,
    pub black_width: u32,
    pub black_height: u32,
    pub white: f64,
    pub native_pattern: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub mae: f64,
    pub p99: u16,
    pub high_err_threshold: u16,
    pub high_err_frac: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ToleranceKind {
    Exact,
    Rounding,
    MaxError,
    Robust,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ComparisonResult {
    pub algorithm: String,
    pub source_url: String,
    pub pattern: String,
    pub native: bool,
    pub reference: String,
    pub kind: ToleranceKind,
    pub verdict: Verdict,
    pub failed_limits: Vec<FailedLimit>,
    pub total_samples: u64,
    pub metrics: Metrics,
    pub metrics_interior: Metrics,
    pub per_channel: PerChannel,
    pub per_site: PerSite,
    pub worst_pixels: Vec<WorstPixel>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub artifacts: Option<ArtifactPaths>,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PerChannel {
    pub r: Metrics,
    pub g: Metrics,
    pub b: Metrics,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct PerSite {
    pub sampled: Metrics,
    pub interpolated: Metrics,
}

pub fn generate_filtered(
    pattern: Option<BayerPattern>,
    algorithm: Option<&str>,
    artifacts: bool,
) -> mischief::Result<CanonicalResults> {
    let input = crate::data::active_area()?;
    let rows =
        report::comparison_rows_filtered_with_artifacts(&input, pattern, algorithm, artifacts)?;
    Ok(from_rows(&input, rows))
}

pub fn from_rows(input: &crate::data::ActiveArea, rows: Vec<report::Row>) -> CanonicalResults {
    let mut results = rows
        .into_iter()
        .map(|row| ComparisonResult::from_row(&input.native_pattern, row))
        .collect::<Vec<_>>();
    results.sort_unstable_by(comparison_order);
    CanonicalResults {
        schema_version: SCHEMA_VERSION,
        meta: RunMeta {
            generated_at: jiff::Timestamp::now().to_string(),
            machine: Machine {
                os: std::env::consts::OS.to_owned(),
                architecture: std::env::consts::ARCH.to_owned(),
            },
            fornax_commit: fornax_commit(),
            references: References {
                libraw: ReferenceBuild {
                    version: report::LIBRAW_VERSION.to_owned(),
                    commit: report::LIBRAW_COMMIT.to_owned(),
                },
                darktable: ReferenceBuild {
                    version: report::DARKTABLE_RELEASE.to_owned(),
                    commit: report::DARKTABLE_COMMIT.to_owned(),
                },
            },
            input: Input {
                file: input.source.clone(),
                path: input.source_path.clone(),
                width: input.width,
                height: input.height,
                black: input.black.iter().copied().map(round6).collect(),
                black_width: input.black_width,
                black_height: input.black_height,
                white: round6(input.white),
                native_pattern: input.native_pattern.to_string(),
            },
            border_px: CANONICAL_BORDER_PX,
            limits: robust_limits(),
            fornax_rounding: "round_to_nearest".to_owned(),
            fornax_clamping: "[0, 1]".to_owned(),
            reference_rounding: "truncate".to_owned(),
        },
        results,
    }
}

impl CanonicalResults {
    pub fn read(path: &Path) -> mischief::Result<Self> {
        let bytes = std::fs::read(path)?;
        let results = serde_json::from_slice::<Self>(&bytes)
            .map_err(|error| mischief::mischief!("cannot parse {}: {error}", path.display()))?;
        if results.schema_version != SCHEMA_VERSION {
            return Err(mischief::mischief!(
                "{} uses schema version {}, expected {SCHEMA_VERSION}",
                path.display(),
                results.schema_version
            ));
        }
        Ok(results)
    }

    pub fn write(&self, path: &Path) -> mischief::Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut json = serde_json::to_string_pretty(self)
            .map_err(|error| mischief::mischief!("cannot serialize canonical results: {error}"))?;
        json.push('\n');
        std::fs::write(path, json)?;
        Ok(())
    }

    pub fn write_default(&self) -> mischief::Result<PathBuf> {
        self.write_repo_file(CANONICAL_FILE)
    }

    pub fn write_filtered(&self) -> mischief::Result<PathBuf> {
        self.write_repo_file(FILTERED_FILE)
    }

    pub fn write_baseline(&self) -> mischief::Result<PathBuf> {
        self.write_repo_file(BASELINE_FILE)
    }

    fn write_repo_file(&self, relative_path: &str) -> mischief::Result<PathBuf> {
        let path = repo_path(relative_path);
        self.write(&path)?;
        Ok(path)
    }
}

pub fn repo_path(relative_path: &str) -> PathBuf { fornax_devtool::root_dir().join(relative_path) }

pub fn schema_json() -> mischief::Result<String> {
    let schema = schemars::schema_for!(CanonicalResults);
    let mut json = serde_json::to_string_pretty(&schema)
        .map_err(|error| mischief::mischief!("cannot serialize canonical schema: {error}"))?;
    json.push('\n');
    Ok(json)
}

impl ComparisonResult {
    pub(crate) fn from_row(native_pattern: &BayerPattern, row: report::Row) -> Self {
        let [red, green, blue] = row.evaluation.per_channel;
        let [sampled, interpolated] = row.evaluation.per_site;
        Self {
            algorithm: row.case.to_owned(),
            source_url: row.source_url.to_owned(),
            pattern: row.pattern.to_string(),
            native: row.pattern == *native_pattern,
            reference: row.reference.to_ascii_lowercase(),
            kind: row.tolerance.kind(),
            verdict: row.verdict,
            failed_limits: row.failed_limits,
            total_samples: row.evaluation.metrics.total_samples,
            metrics: row.evaluation.metrics,
            metrics_interior: row.evaluation.metrics_interior,
            per_channel: PerChannel {
                r: red,
                g: green,
                b: blue,
            },
            per_site: PerSite {
                sampled,
                interpolated,
            },
            worst_pixels: row.evaluation.worst_pixels,
            artifacts: row.artifacts,
        }
    }
}

impl Tolerance {
    const fn kind(self) -> ToleranceKind {
        match self {
            Self::Exact => ToleranceKind::Exact,
            Self::RoundingUp => ToleranceKind::Rounding,
            Self::MaxError(_) => ToleranceKind::MaxError,
            Self::Robust { .. } => ToleranceKind::Robust,
        }
    }
}

fn robust_limits() -> Limits {
    match crate::case::ROBUST {
        Tolerance::Robust {
            max_mae,
            max_p99,
            max_high_error_frac,
        } => Limits {
            mae: max_mae,
            p99: max_p99,
            high_err_threshold: 1024,
            high_err_frac: max_high_error_frac,
        },
        Tolerance::Exact | Tolerance::RoundingUp | Tolerance::MaxError(_) => {
            unreachable!("ROBUST must use the robust tolerance")
        }
    }
}

fn comparison_order(left: &ComparisonResult, right: &ComparisonResult) -> std::cmp::Ordering {
    (
        left.algorithm.to_ascii_lowercase(),
        left.reference.to_ascii_lowercase(),
        left.pattern.as_str(),
    )
        .cmp(&(
            right.algorithm.to_ascii_lowercase(),
            right.reference.to_ascii_lowercase(),
            right.pattern.as_str(),
        ))
}

fn round6(value: f64) -> f64 { (value * 1_000_000.0).round() / 1_000_000.0 }

fn fornax_commit() -> String {
    let root = fornax_devtool::root_dir();
    let commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root)
        .output()
        .ok()
        .filter(|output| output.status.success()).map_or_else(|| "unknown".to_owned(), |output| String::from_utf8_lossy(&output.stdout).trim().to_owned());
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| !output.stdout.is_empty());
    if dirty {
        format!("{commit}-dirty")
    } else {
        commit
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_contains_the_canonical_contract() -> mischief::Result<()> {
        let schema = schema_json()?;
        assert!(schema.contains("\"CanonicalResults\""));
        assert!(schema.contains("\"failed_limits\""));
        assert!(schema.contains("\"metrics_interior\""));
        assert!(schema.contains("\"native\""));
        Ok(())
    }
}
