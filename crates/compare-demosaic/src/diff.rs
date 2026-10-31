use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Serialize;

use crate::error::Result;
use crate::metrics::Metrics;
use crate::results::{CanonicalResults, ComparisonResult};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FailOn {
    Regression,
    Any,
    Never,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Removed,
    Regression,
    Added,
    Improvement,
    Changed,
}

#[derive(Clone, Debug, Serialize)]
pub struct DiffChange {
    pub kind: ChangeKind,
    pub key: String,
    pub before_verdict: Option<&'static str>,
    pub after_verdict: Option<&'static str>,
    pub details: Vec<String>,
}

#[derive(Clone, Debug, Serialize)]
pub struct DiffReport {
    pub stable_metadata_matches: bool,
    pub stable_differences: Vec<String>,
    pub changes: Vec<DiffChange>,
    pub regressions: usize,
    pub improvements: usize,
    pub exit_code: u8,
}

pub fn compare(
    old: &CanonicalResults,
    new: &CanonicalResults,
    fail_on: FailOn,
    only_native: bool,
) -> Result<DiffReport> {
    let stable_differences = stable_differences(old, new)?;
    let old_rows = keyed(&old.results);
    let new_rows = keyed(&new.results);
    let keys = old_rows
        .keys()
        .chain(new_rows.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut changes = Vec::new();
    let mut regressions = 0;
    let mut improvements = 0;

    for key in keys {
        let before = old_rows.get(&key).copied();
        let after = new_rows.get(&key).copied();
        if only_native
            && !before.is_some_and(|row| row.native)
            && !after.is_some_and(|row| row.native)
        {
            continue;
        }
        match (before, after) {
            (Some(before), Some(after)) => {
                let details = metric_changes(&before.metrics, &after.metrics);
                let mut before_without_paths = (*before).clone();
                let mut after_without_paths = (*after).clone();
                before_without_paths.artifacts = None;
                after_without_paths.artifacts = None;
                if before_without_paths == after_without_paths {
                    continue;
                }
                let change = match after.verdict.rank().cmp(&before.verdict.rank()) {
                    std::cmp::Ordering::Less => {
                        regressions += 1;
                        DiffChange {
                            kind: ChangeKind::Regression,
                            key,
                            before_verdict: Some(before.verdict.name()),
                            after_verdict: Some(after.verdict.name()),
                            details,
                        }
                    }
                    std::cmp::Ordering::Greater => {
                        improvements += 1;
                        DiffChange {
                            kind: ChangeKind::Improvement,
                            key,
                            before_verdict: Some(before.verdict.name()),
                            after_verdict: Some(after.verdict.name()),
                            details,
                        }
                    }
                    std::cmp::Ordering::Equal => DiffChange {
                        kind: ChangeKind::Changed,
                        key,
                        before_verdict: Some(before.verdict.name()),
                        after_verdict: Some(after.verdict.name()),
                        details,
                    },
                };
                changes.push(change);
            }
            (Some(before), None) => {
                regressions += 1;
                changes.push(DiffChange {
                    kind: ChangeKind::Removed,
                    key,
                    before_verdict: Some(before.verdict.name()),
                    after_verdict: None,
                    details: vec!["row is missing from the new evidence".to_owned()],
                });
            }
            (None, Some(after)) => {
                improvements += 1;
                changes.push(DiffChange {
                    kind: ChangeKind::Added,
                    key,
                    before_verdict: None,
                    after_verdict: Some(after.verdict.name()),
                    details: vec!["row was added to the new evidence".to_owned()],
                });
            }
            (None, None) => unreachable!("key came from one side"),
        }
    }

    changes.sort_by(|left, right| {
        change_order(left.kind)
            .cmp(&change_order(right.kind))
            .then_with(|| left.key.cmp(&right.key))
    });
    let exit_code = if stable_differences.is_empty() {
        match fail_on {
            FailOn::Never => 0,
            FailOn::Regression if regressions > 0 => 1,
            FailOn::Any if !changes.is_empty() => 1,
            FailOn::Regression | FailOn::Any => 0,
        }
    } else {
        2
    };
    Ok(DiffReport {
        stable_metadata_matches: stable_differences.is_empty(),
        stable_differences,
        changes,
        regressions,
        improvements,
        exit_code,
    })
}

pub fn compare_files(
    old_path: &Path,
    new_path: &Path,
    fail_on: FailOn,
    only_native: bool,
) -> Result<DiffReport> {
    let old = CanonicalResults::read(old_path)
        .map_err(|error| crate::error::CompareError::invalid(error.to_string()))?;
    let new = CanonicalResults::read(new_path)
        .map_err(|error| crate::error::CompareError::invalid(error.to_string()))?;
    compare(&old, &new, fail_on, only_native)
}

impl DiffReport {
    pub fn render_text(&self) -> String {
        let mut output = String::new();
        for difference in &self.stable_differences {
            output.push_str(&format!("META MISMATCH {difference}\n"));
        }
        for change in &self.changes {
            let before = change.before_verdict.unwrap_or("missing");
            let after = change.after_verdict.unwrap_or("missing");
            let details = if change.details.is_empty() {
                String::new()
            } else {
                format!("; {}", change.details.join(", "))
            };
            output.push_str(&format!(
                "{:?} {}: {before} -> {after}{details}\n",
                change.kind, change.key
            ));
        }
        output.push_str(&format!(
            "{} regressions, {} improvements",
            self.regressions, self.improvements
        ));
        output
    }

    pub fn render_markdown(&self) -> String {
        let mut output = String::from("## compare-demosaic diff\n\n");
        if !self.stable_metadata_matches {
            output.push_str(
                "> [!CAUTION]\n> Stable metadata differs. This comparison is not meaningful as a baseline diff.\n\n",
            );
            for difference in &self.stable_differences {
                output.push_str(&format!("- `{difference}`\n"));
            }
            output.push('\n');
        }
        output.push_str(&format!(
            "{} regressions, {} improvements, {} total changes.\n\n",
            self.regressions,
            self.improvements,
            self.changes.len()
        ));
        if !self.changes.is_empty() {
            output.push_str("| Change | Row | Before | After | Details |\n");
            output.push_str("| --- | --- | --- | --- | --- |\n");
            for change in &self.changes {
                output.push_str(&format!(
                    "| {:?} | `{}` | {} | {} | {} |\n",
                    change.kind,
                    change.key,
                    change.before_verdict.unwrap_or("missing"),
                    change.after_verdict.unwrap_or("missing"),
                    change.details.join(", ").replace('|', "\\|")
                ));
            }
        }
        output
    }
}

fn keyed(results: &[ComparisonResult]) -> BTreeMap<String, &ComparisonResult> {
    results
        .iter()
        .map(|result| {
            (
                format!(
                    "{} {} {}",
                    result.algorithm, result.pattern, result.reference
                ),
                result,
            )
        })
        .collect()
}

fn metric_changes(before: &Metrics, after: &Metrics) -> Vec<String> {
    let mut changes = Vec::new();
    push_float(&mut changes, "mae", before.mae, after.mae);
    push_float(&mut changes, "rmse", before.rmse, after.rmse);
    push_integer(&mut changes, "max", before.max.into(), after.max.into());
    push_float(
        &mut changes,
        "mean_signed",
        before.mean_signed,
        after.mean_signed,
    );
    push_integer(&mut changes, "p99", before.p99.into(), after.p99.into());
    push_integer(&mut changes, "p999", before.p999.into(), after.p999.into());
    push_integer(
        &mut changes,
        "count_gt1024",
        before.count_gt1024,
        after.count_gt1024,
    );
    push_float(
        &mut changes,
        "frac_gt1024",
        before.frac_gt1024,
        after.frac_gt1024,
    );
    changes
}

fn push_float(changes: &mut Vec<String>, name: &str, before: f64, after: f64) {
    if before.to_bits() != after.to_bits() {
        changes.push(format!("{name} {before:.6} -> {after:.6}"));
    }
}

fn push_integer(changes: &mut Vec<String>, name: &str, before: u64, after: u64) {
    if before != after {
        changes.push(format!("{name} {before} -> {after}"));
    }
}

fn stable_differences(old: &CanonicalResults, new: &CanonicalResults) -> Result<Vec<String>> {
    let old = serde_json::to_value(&old.meta.stable)
        .map_err(|error| crate::error::CompareError::invalid(error.to_string()))?;
    let new = serde_json::to_value(&new.meta.stable)
        .map_err(|error| crate::error::CompareError::invalid(error.to_string()))?;
    let mut differences = Vec::new();
    flatten_differences("meta.stable", &old, &new, &mut differences);
    Ok(differences)
}

fn flatten_differences(
    path: &str,
    old: &serde_json::Value,
    new: &serde_json::Value,
    output: &mut Vec<String>,
) {
    if old == new {
        return;
    }
    match (old, new) {
        (serde_json::Value::Object(old), serde_json::Value::Object(new)) => {
            let keys = old.keys().chain(new.keys()).collect::<BTreeSet<_>>();
            for key in keys {
                flatten_differences(
                    &format!("{path}.{key}"),
                    old.get(key).unwrap_or(&serde_json::Value::Null),
                    new.get(key).unwrap_or(&serde_json::Value::Null),
                    output,
                );
            }
        }
        _ => output.push(format!("{path}: {old} -> {new}")),
    }
}

const fn change_order(kind: ChangeKind) -> u8 {
    match kind {
        ChangeKind::Removed => 0,
        ChangeKind::Regression => 1,
        ChangeKind::Added => 2,
        ChangeKind::Improvement => 3,
        ChangeKind::Changed => 4,
    }
}

trait VerdictName {
    fn name(self) -> &'static str;
}

impl VerdictName for crate::metrics::Verdict {
    fn name(self) -> &'static str {
        match self {
            Self::Exact => "exact",
            Self::RoundingOnly => "rounding_only",
            Self::Pass => "pass",
            Self::PassWithOutliers => "pass_with_outliers",
            Self::Fail => "fail",
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{FailedLimit, Verdict, WorstPixel};
    use crate::results::{
        Conventions, Input, PerChannel, PerSite, ReferenceBuild, References, RunMeta, RunMetadata,
        StableMeta, ToleranceKind,
    };

    fn metrics(mae: f64) -> Metrics {
        Metrics {
            total_samples: 1,
            mae,
            rmse: mae,
            max: 1,
            mean_signed: 0.0,
            p99: 1,
            p999: 1,
            p9999: 1,
            count_gt1: 0,
            frac_gt1: 0.0,
            count_gt16: 0,
            frac_gt16: 0.0,
            count_gt1024: 0,
            frac_gt1024: 0.0,
            count_below_reference: 0,
            changed_count: 1,
            changed_frac: 1.0,
        }
    }

    fn results(verdict: Verdict, mae: f64) -> CanonicalResults {
        let metric_values = metrics(mae);
        CanonicalResults {
            schema_version: crate::results::SCHEMA_VERSION,
            meta: RunMeta {
                stable: StableMeta {
                    fornax_commit: "commit".to_owned(),
                    dirty: false,
                    references: References {
                        libraw: ReferenceBuild {
                            version: "0.22.3".to_owned(),
                            commit: "libraw".to_owned(),
                        },
                        darktable: ReferenceBuild {
                            version: "5.6.2".to_owned(),
                            commit: "darktable".to_owned(),
                        },
                    },
                    input: Input {
                        file: "input.raw".to_owned(),
                        sha256: "0".repeat(64),
                        width: 1,
                        height: 1,
                        black: vec![0.0],
                        black_width: 1,
                        black_height: 1,
                        white: 1.0,
                        native_pattern: "RGGB".to_owned(),
                    },
                    border_px: 16,
                    limits: crate::results::robust_limits(),
                    conventions: Conventions {
                        fornax: "fornax".to_owned(),
                        reference: "reference".to_owned(),
                    },
                },
                run: RunMetadata {
                    coverage: crate::results::Coverage::Full,
                    shards: Vec::new(),
                    generated_by: "test".to_owned(),
                },
            },
            results: vec![ComparisonResult {
                algorithm: "AHD".to_owned(),
                source_url: String::new(),
                pattern: "RGGB".to_owned(),
                native: true,
                reference: "libraw".to_owned(),
                kind: ToleranceKind::Robust,
                verdict,
                failed_limits: if verdict == Verdict::Fail {
                    vec![FailedLimit::Mae]
                } else {
                    Vec::new()
                },
                total_samples: 1,
                metrics: metric_values.clone(),
                metrics_interior: metric_values.clone(),
                per_channel: PerChannel {
                    r: metric_values.clone(),
                    g: metric_values.clone(),
                    b: metric_values,
                },
                per_site: PerSite {
                    sampled: metrics(mae),
                    interpolated: metrics(mae),
                },
                worst_pixels: Vec::<WorstPixel>::new(),
                artifacts: None,
            }],
        }
    }

    #[test]
    fn verdict_regression_fails_by_default() {
        let report = compare(
            &results(Verdict::Pass, 1.0),
            &results(Verdict::Fail, 3.0),
            FailOn::Regression,
            false,
        )
        .unwrap();
        assert_eq!(report.regressions, 1);
        assert_eq!(report.exit_code, 1);
        assert!(
            report.changes[0]
                .details
                .iter()
                .any(|line| line.contains("mae"))
        );
    }

    #[test]
    fn metric_only_change_only_fails_with_fail_on_any() {
        let old = results(Verdict::Pass, 1.0);
        let mut new = results(Verdict::Pass, 1.5);
        new.results[0].failed_limits = Vec::new();
        let report = compare(&old, &new, FailOn::Regression, false).unwrap();
        assert_eq!(report.exit_code, 0);
        let report = compare(&old, &new, FailOn::Any, false).unwrap();
        assert_eq!(report.exit_code, 1);
    }

    #[test]
    fn stable_metadata_mismatch_exits_two() {
        let old = results(Verdict::Pass, 1.0);
        let mut new = old.clone();
        new.meta.stable.input.sha256 = "1".repeat(64);
        let report = compare(&old, &new, FailOn::Never, false).unwrap();
        assert_eq!(report.exit_code, 2);
        assert!(report.stable_differences[0].contains("sha256"));
    }

    #[test]
    fn change_order_puts_regressions_before_improvements() {
        assert!(change_order(ChangeKind::Removed) < change_order(ChangeKind::Improvement));
    }
}
