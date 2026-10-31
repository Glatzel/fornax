use std::collections::{BTreeMap, BTreeSet};

use crate::metrics::Metrics;
use crate::results::{CanonicalResults, ComparisonResult};

#[derive(Debug)]
pub struct DiffReport {
    pub lines: Vec<String>,
    pub regressions: usize,
    pub improvements: usize,
}

pub fn compare(old: &CanonicalResults, new: &CanonicalResults) -> DiffReport {
    let old_rows = keyed(&old.results);
    let new_rows = keyed(&new.results);
    let keys = old_rows
        .keys()
        .chain(new_rows.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    let mut lines = Vec::new();
    let mut regressions = 0;
    let mut improvements = 0;

    for key in keys {
        match (old_rows.get(&key), new_rows.get(&key)) {
            (Some(before), Some(after)) => {
                let metric_changes = metric_changes(&before.metrics, &after.metrics);
                if before.verdict == after.verdict && metric_changes.is_empty() {
                    continue;
                }
                let details = if metric_changes.is_empty() {
                    String::new()
                } else {
                    format!("; {}", metric_changes.join(", "))
                };
                if after.verdict.rank() < before.verdict.rank() {
                    regressions += 1;
                    lines.push(format!(
                        "REGRESSION {}: {} -> {}{}",
                        key,
                        before.verdict.symbol(),
                        after.verdict.symbol(),
                        details
                    ));
                } else if after.verdict.rank() > before.verdict.rank() {
                    improvements += 1;
                    lines.push(format!(
                        "IMPROVEMENT {}: {} -> {}{}",
                        key,
                        before.verdict.symbol(),
                        after.verdict.symbol(),
                        details
                    ));
                } else {
                    lines.push(format!(
                        "CHANGED {}: {}{}",
                        key,
                        after.verdict.symbol(),
                        details
                    ));
                }
            }
            (Some(before), None) => {
                regressions += 1;
                lines.push(format!(
                    "REGRESSION {}: {} -> missing",
                    key,
                    before.verdict.symbol()
                ));
            }
            (None, Some(after)) => {
                improvements += 1;
                lines.push(format!(
                    "IMPROVEMENT {}: missing -> {}",
                    key,
                    after.verdict.symbol()
                ));
            }
            (None, None) => unreachable!("key came from one side"),
        }
    }

    DiffReport {
        lines,
        regressions,
        improvements,
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
    if (after - before).abs() > 0.000_000_5 {
        changes.push(format!("{name} {before:.6} -> {after:.6}"));
    }
}

fn push_integer(changes: &mut Vec<String>, name: &str, before: u64, after: u64) {
    if before != after {
        changes.push(format!("{name} {before} -> {after}"));
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::metrics::{FailedLimit, Verdict, WorstPixel};
    use crate::results::{
        Input, Limits, Machine, PerChannel, PerSite, ReferenceBuild, References, RunMeta,
        SCHEMA_VERSION, ToleranceKind,
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
            schema_version: SCHEMA_VERSION,
            meta: RunMeta {
                generated_at: String::new(),
                machine: Machine {
                    os: String::new(),
                    architecture: String::new(),
                },
                fornax_commit: String::new(),
                references: References {
                    libraw: ReferenceBuild {
                        version: String::new(),
                        commit: String::new(),
                    },
                    darktable: ReferenceBuild {
                        version: String::new(),
                        commit: String::new(),
                    },
                },
                input: Input {
                    file: String::new(),
                    path: String::new(),
                    width: 0,
                    height: 0,
                    black: Vec::new(),
                    black_width: 0,
                    black_height: 0,
                    white: 0.0,
                    native_pattern: "RGGB".to_owned(),
                },
                border_px: 16,
                limits: Limits {
                    mae: 2.0,
                    p99: 64,
                    high_err_threshold: 1024,
                    high_err_frac: 0.001,
                },
                fornax_rounding: String::new(),
                fornax_clamping: String::new(),
                reference_rounding: String::new(),
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
    fn verdict_regression_fails_and_reports_metric_deltas() {
        let report = compare(&results(Verdict::Pass, 1.0), &results(Verdict::Fail, 3.0));
        assert_eq!(report.regressions, 1);
        assert!(report.lines[0].contains("mae 1.000000 -> 3.000000"));
    }
}
