use std::path::{Path, PathBuf};
use std::process::Command;

use fornax_core::BayerPattern;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::artifacts::ArtifactPaths;
use crate::case::Tolerance;
use crate::metrics::{FailedLimit, Metrics, Verdict, WorstPixel};
use crate::report::{self, CANONICAL_BORDER_PX};

pub const SCHEMA_VERSION: u32 = 2;
pub const CANONICAL_FILE: &str = "target/compare-demosaic/canonical.json";
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
    pub stable: StableMeta,
    pub run: RunMetadata,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct StableMeta {
    pub fornax_commit: String,
    pub dirty: bool,
    pub references: References,
    pub input: Input,
    pub border_px: u32,
    pub limits: Limits,
    pub conventions: Conventions,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RunMetadata {
    pub coverage: Coverage,
    pub shards: Vec<ShardMetadata>,
    pub generated_by: String,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Coverage {
    Full,
    Partial,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ShardMetadata {
    pub name: String,
    pub selection: Selection,
    pub started: String,
    pub finished: String,
    pub runner: String,
}

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub algorithms: Vec<String>,
    pub references: Vec<String>,
    pub patterns: Vec<String>,
}

impl Selection {
    pub fn new(algorithms: &[String], references: &[String], patterns: &[String]) -> Self {
        let mut algorithms = algorithms
            .iter()
            .map(|value| value.to_ascii_lowercase())
            .collect::<Vec<_>>();
        let mut references = references
            .iter()
            .map(|value| value.to_ascii_lowercase())
            .collect::<Vec<_>>();
        let mut patterns = patterns
            .iter()
            .map(|value| value.to_ascii_uppercase())
            .collect::<Vec<_>>();
        algorithms.sort();
        algorithms.dedup();
        references.sort();
        references.dedup();
        patterns.sort();
        patterns.dedup();
        Self {
            algorithms,
            references,
            patterns,
        }
    }

    pub const fn is_full(&self) -> bool {
        self.algorithms.is_empty() && self.references.is_empty() && self.patterns.is_empty()
    }
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
    pub sha256: String,
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

#[derive(Clone, Debug, Deserialize, JsonSchema, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Conventions {
    pub fornax: String,
    pub reference: String,
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

pub fn stable_metadata(input: &crate::data::ActiveArea, border_px: u32) -> StableMeta {
    let (fornax_commit, dirty) = fornax_revision();
    StableMeta {
        fornax_commit,
        dirty,
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
            sha256: input.source_sha256.clone(),
            width: input.width,
            height: input.height,
            black: input.black.iter().copied().map(round6).collect(),
            black_width: input.black_width,
            black_height: input.black_height,
            white: round6(input.white),
            native_pattern: input.native_pattern.to_string(),
        },
        border_px,
        limits: robust_limits(),
        conventions: Conventions {
            fornax: "clamp:[0,1];rounding:nearest".to_owned(),
            reference: "rounding:truncate".to_owned(),
        },
    }
}

pub fn from_rows(
    input: &crate::data::ActiveArea,
    rows: Vec<report::Row>,
    stable: StableMeta,
    run: RunMetadata,
) -> CanonicalResults {
    let mut results = rows
        .into_iter()
        .map(|row| ComparisonResult::from_row(&input.native_pattern, row))
        .collect::<Vec<_>>();
    results.sort_unstable_by(comparison_order);
    CanonicalResults {
        schema_version: SCHEMA_VERSION,
        meta: RunMeta { stable, run },
        results,
    }
}

pub fn generated_by() -> String { format!("compare-demosaic {}", env!("CARGO_PKG_VERSION")) }

pub fn full_run_metadata() -> RunMetadata {
    RunMetadata {
        coverage: Coverage::Full,
        shards: Vec::new(),
        generated_by: generated_by(),
    }
}

pub const fn default_border_px() -> u32 { CANONICAL_BORDER_PX }

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
    pub(crate) const fn kind(self) -> ToleranceKind {
        match self {
            Self::Exact => ToleranceKind::Exact,
            Self::RoundingUp => ToleranceKind::Rounding,
            Self::MaxError(_) => ToleranceKind::MaxError,
            Self::Robust { .. } => ToleranceKind::Robust,
        }
    }
}

pub fn robust_limits() -> Limits {
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

pub fn comparison_order(left: &ComparisonResult, right: &ComparisonResult) -> std::cmp::Ordering {
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

fn fornax_revision() -> (String, bool) {
    let root = fornax_devtool::root_dir();
    let commit = Command::new("git")
        .args(["rev-parse", "HEAD"])
        .current_dir(&root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .map_or_else(
            || "unknown".to_owned(),
            |output| String::from_utf8_lossy(&output.stdout).trim().to_owned(),
        );
    let dirty = Command::new("git")
        .args(["status", "--porcelain"])
        .current_dir(root)
        .output()
        .ok()
        .filter(|output| output.status.success())
        .is_some_and(|output| !output.stdout.is_empty());
    (commit, dirty)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn schema_contains_the_canonical_contract() -> mischief::Result<()> {
        let schema = schema_json()?;
        assert!(schema.contains("\"CanonicalResults\""));
        assert!(schema.contains("\"stable\""));
        assert!(schema.contains("\"coverage\""));
        assert!(schema.contains("\"sha256\""));
        Ok(())
    }
}
