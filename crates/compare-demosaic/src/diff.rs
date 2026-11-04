use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use serde::Serialize;

use crate::report::{Metrics, Report, Row, Verdict};

#[derive(Clone, Copy, Debug, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum ChangeKind {
    Regression,
    Removed,
    Added,
    Improvement,
    Statistics,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct RowKey<'a> {
    pub input: &'a str,
    pub algorithm: &'a str,
    pub reference: &'a str,
    pub pattern: &'a str,
}

impl std::fmt::Display for RowKey<'_> {
    fn fmt(&self, formatter: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(
            formatter,
            "{} {} {} {}",
            self.input, self.algorithm, self.reference, self.pattern
        )
    }
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct FieldChange {
    pub field: String,
    pub before: String,
    pub after: String,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct Change {
    pub kind: ChangeKind,
    pub row: String,
    pub before_verdict: Option<Verdict>,
    pub after_verdict: Option<Verdict>,
    pub all: Vec<FieldChange>,
    pub other_changed_fields: usize,
    pub before: Option<Row>,
    pub after: Option<Row>,
}

#[derive(Clone, Debug, PartialEq, Serialize)]
pub struct DiffReport {
    pub verdict_only: bool,
    pub differences: usize,
    pub regressions: usize,
    pub improvements: usize,
    pub changes: Vec<Change>,
}

impl DiffReport {
    pub fn exit_code(&self) -> u8 {
        if self.differences == 0 {
            0
        } else if self.verdict_only {
            let blocking = self.regressions
                + self.improvements
                + self
                    .changes
                    .iter()
                    .filter(|change| matches!(change.kind, ChangeKind::Removed | ChangeKind::Added))
                    .count();
            u8::from(blocking != 0)
        } else {
            1
        }
    }

    pub fn render_text(&self) -> String {
        let mut output = String::new();
        if self.changes.is_empty() {
            output.push_str("no differences\n");
            return output;
        }
        output.push_str(
            "| Change | Row | Before | After | Statistics | Other changed fields |\n\
| --- | --- | --- | --- | --- | ---: |\n",
        );
        for change in &self.changes {
            let statistics = if change.all.is_empty() {
                String::new()
            } else {
                change
                    .all
                    .iter()
                    .map(|field| format!("{} {} -> {}", field.field, field.before, field.after))
                    .collect::<Vec<_>>()
                    .join("<br>")
            };
            output.push_str(&format!(
                "| {} | `{}` | {} | {} | {} | {} |\n",
                kind_name(change.kind),
                change.row,
                change.before_verdict.map_or("missing", Verdict::symbol),
                change.after_verdict.map_or("missing", Verdict::symbol),
                statistics,
                change.other_changed_fields,
            ));
        }
        output.push_str(&format!(
            "\n{} differences: {} regressions, {} improvements\n",
            self.differences, self.regressions, self.improvements
        ));
        output
    }

    pub fn append_github_summary(&self) -> mischief::Result<()> {
        let Some(path) = std::env::var_os("GITHUB_STEP_SUMMARY") else {
            return Ok(());
        };
        let path = Path::new(&path);
        let mut file = std::fs::OpenOptions::new()
            .create(true)
            .append(true)
            .open(path)?;
        use std::io::Write;
        file.write_all(b"\n## compare-demosaic\n\n")?;
        file.write_all(self.render_text().as_bytes())?;
        Ok(())
    }
}

pub fn compare(
    baseline: &Report,
    new: &Report,
    verdict_only: bool,
) -> mischief::Result<DiffReport> {
    if baseline.meta.selection.is_partial() {
        return Err(mischief::mischief!(
            "the baseline is partial and cannot be used for comparison"
        ));
    }
    check_comparable(baseline, new)?;

    let before = baseline
        .rows
        .iter()
        .map(|row| (key(row), row))
        .collect::<BTreeMap<_, _>>();
    let after = new
        .rows
        .iter()
        .map(|row| (key(row), row))
        .collect::<BTreeMap<_, _>>();
    let keys = if new.meta.selection.is_partial() {
        after.keys().copied().collect::<BTreeSet<_>>()
    } else {
        before
            .keys()
            .chain(after.keys())
            .copied()
            .collect::<BTreeSet<_>>()
    };

    let mut changes = Vec::new();
    for key in keys {
        match (before.get(&key), after.get(&key)) {
            (Some(old), Some(current)) if *old == *current => {}
            (Some(old), Some(current)) => {
                let kind = match (old.verdict, current.verdict) {
                    (Verdict::Pass, Verdict::Fail) => ChangeKind::Regression,
                    (Verdict::Fail, Verdict::Pass) => ChangeKind::Improvement,
                    _ => ChangeKind::Statistics,
                };
                changes.push(Change {
                    kind,
                    row: key.to_string(),
                    before_verdict: Some(old.verdict),
                    after_verdict: Some(current.verdict),
                    all: metric_changes(&old.all, &current.all),
                    other_changed_fields: other_changed_fields(old, current),
                    before: Some((*old).clone()),
                    after: Some((*current).clone()),
                });
            }
            (Some(old), None) if !new.meta.selection.is_partial() => changes.push(Change {
                kind: ChangeKind::Removed,
                row: key.to_string(),
                before_verdict: Some(old.verdict),
                after_verdict: None,
                all: Vec::new(),
                other_changed_fields: 0,
                before: Some((*old).clone()),
                after: None,
            }),
            (Some(_), None) => {}
            (None, Some(current)) => changes.push(Change {
                kind: ChangeKind::Added,
                row: key.to_string(),
                before_verdict: None,
                after_verdict: Some(current.verdict),
                all: Vec::new(),
                other_changed_fields: 0,
                before: None,
                after: Some((*current).clone()),
            }),
            (None, None) => unreachable!("key came from one side"),
        }
    }
    changes.sort_by(|left, right| {
        severity(left.kind)
            .cmp(&severity(right.kind))
            .then_with(|| left.row.cmp(&right.row))
    });
    let regressions = changes
        .iter()
        .filter(|change| change.kind == ChangeKind::Regression)
        .count();
    let improvements = changes
        .iter()
        .filter(|change| change.kind == ChangeKind::Improvement)
        .count();
    Ok(DiffReport {
        verdict_only,
        differences: changes.len(),
        regressions,
        improvements,
        changes,
    })
}

fn check_comparable(baseline: &Report, new: &Report) -> mischief::Result<()> {
    let mut differences = Vec::new();
    if baseline.meta.references != new.meta.references {
        differences.push(format!(
            "references {:?} != {:?}",
            baseline.meta.references, new.meta.references
        ));
    }
    if baseline.meta.limits != new.meta.limits {
        differences.push(format!(
            "limits {:?} != {:?}",
            baseline.meta.limits, new.meta.limits
        ));
    }
    if baseline.meta.border_px != new.meta.border_px {
        differences.push(format!(
            "border_px {} != {}",
            baseline.meta.border_px, new.meta.border_px
        ));
    }
    if baseline.meta.pattern_source != new.meta.pattern_source {
        differences.push(format!(
            "pattern_source {:?} != {:?}",
            baseline.meta.pattern_source, new.meta.pattern_source
        ));
    }
    let baseline_inputs = baseline
        .meta
        .inputs
        .iter()
        .map(|input| (input.name.as_str(), input))
        .collect::<BTreeMap<_, _>>();
    if new.meta.selection.is_partial() {
        for input in &new.meta.inputs {
            match baseline_inputs.get(input.name.as_str()) {
                Some(before) if *before == input => {}
                Some(_) => differences.push(format!("inputs.{} differ", input.name)),
                None => differences.push(format!("inputs.{} is missing from baseline", input.name)),
            }
        }
    } else if baseline.meta.inputs != new.meta.inputs {
        differences.push("inputs differ".to_owned());
    }
    if differences.is_empty() {
        Ok(())
    } else {
        Err(mischief::mischief!(
            "cannot compare: {}",
            differences.join("; ")
        ))
    }
}

fn key(row: &Row) -> RowKey<'_> {
    RowKey {
        input: &row.input,
        algorithm: &row.algorithm,
        reference: &row.reference,
        pattern: &row.pattern,
    }
}

fn metric_changes(before: &Metrics, after: &Metrics) -> Vec<FieldChange> {
    let mut changes = Vec::new();
    push_float(&mut changes, "mae", before.mae, after.mae);
    push_float(&mut changes, "rmse", before.rmse, after.rmse);
    push_integer(&mut changes, "max", before.max, after.max);
    push_float(
        &mut changes,
        "mean_signed",
        before.mean_signed,
        after.mean_signed,
    );
    push_integer(&mut changes, "p99", before.p99, after.p99);
    push_integer(&mut changes, "p999", before.p999, after.p999);
    push_integer(&mut changes, "p9999", before.p9999, after.p9999);
    push_float(&mut changes, "frac_gt1", before.frac_gt1, after.frac_gt1);
    push_float(&mut changes, "frac_gt16", before.frac_gt16, after.frac_gt16);
    push_integer(
        &mut changes,
        "count_gt1024",
        before.count_gt1024,
        after.count_gt1024,
    );
    push_float(
        &mut changes,
        "changed_frac",
        before.changed_frac,
        after.changed_frac,
    );
    changes
}

fn push_float(changes: &mut Vec<FieldChange>, field: &str, before: f64, after: f64) {
    if before.to_bits() != after.to_bits() {
        changes.push(FieldChange {
            field: field.to_owned(),
            before: format!("{before:.6}"),
            after: format!("{after:.6}"),
        });
    }
}

fn push_integer<T>(changes: &mut Vec<FieldChange>, field: &str, before: T, after: T)
where
    T: PartialEq + ToString,
{
    if before != after {
        changes.push(FieldChange {
            field: field.to_owned(),
            before: before.to_string(),
            after: after.to_string(),
        });
    }
}

fn other_changed_fields(before: &Row, after: &Row) -> usize {
    let mut changed = 0;
    changed += usize::from(before.samples != after.samples);
    changed += usize::from(before.crop != after.crop);
    changed += usize::from(before.failed_limits != after.failed_limits);
    changed += usize::from(before.kind != after.kind);
    changed += usize::from(before.native != after.native);
    changed += metrics_difference_count(&before.interior, &after.interior);
    changed += metrics_difference_count(&before.channels.r, &after.channels.r);
    changed += metrics_difference_count(&before.channels.g, &after.channels.g);
    changed += metrics_difference_count(&before.channels.b, &after.channels.b);
    changed += metrics_difference_count(&before.sites.sampled, &after.sites.sampled);
    changed += metrics_difference_count(&before.sites.interpolated, &after.sites.interpolated);
    changed += usize::from(before.worst != after.worst);
    changed
}

fn metrics_difference_count(before: &Metrics, after: &Metrics) -> usize {
    metric_changes(before, after).len()
}

const fn severity(kind: ChangeKind) -> u8 {
    match kind {
        ChangeKind::Regression => 0,
        ChangeKind::Removed => 1,
        ChangeKind::Added => 2,
        ChangeKind::Improvement => 3,
        ChangeKind::Statistics => 4,
    }
}

const fn kind_name(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Regression => "REGRESSION",
        ChangeKind::Removed => "REMOVED",
        ChangeKind::Added => "ADDED",
        ChangeKind::Improvement => "IMPROVEMENT",
        ChangeKind::Statistics => "STATISTICS",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{
        Channels, InputMeta, Kind, Limits, Meta, PatternSource, References, Selection, Sites,
        WorstSample,
    };

    fn metrics(mae: f64) -> Metrics {
        Metrics {
            mae,
            rmse: mae,
            max: 2,
            mean_signed: 0.5,
            p99: 1,
            p999: 1,
            p9999: 2,
            frac_gt1: 0.5,
            frac_gt16: 0.0,
            count_gt1024: 0,
            changed_frac: 0.5,
        }
    }

    fn row(algorithm: &str, verdict: Verdict, mae: f64) -> Row {
        let values = metrics(mae);
        Row {
            input: "input.raw".to_owned(),
            algorithm: algorithm.to_owned(),
            reference: "libraw".to_owned(),
            pattern: "RGGB".to_owned(),
            crop: [0, 0],
            native: true,
            kind: Kind::Robust,
            samples: 12,
            verdict,
            failed_limits: if verdict == Verdict::Fail {
                vec!["mae".to_owned()]
            } else {
                Vec::new()
            },
            all: values.clone(),
            interior: values.clone(),
            channels: Channels {
                r: values.clone(),
                g: values.clone(),
                b: values,
            },
            sites: Sites {
                sampled: metrics(mae),
                interpolated: metrics(mae),
            },
            worst: vec![WorstSample {
                x: 0,
                y: 0,
                c: 0,
                fornax: 2,
                reference: 0,
            }],
        }
    }

    fn report(rows: Vec<Row>) -> Report {
        Report::new(
            Meta {
                inputs: vec![InputMeta {
                    name: "input.raw".to_owned(),
                    sha256: "00".repeat(32),
                    width: 2,
                    height: 2,
                    black: vec![0.0; 4],
                    black_width: 2,
                    black_height: 2,
                    white: 65_535.0,
                    native_pattern: "RGGB".to_owned(),
                }],
                references: References {
                    libraw: "0.22.3".to_owned(),
                    darktable: "5.6.2".to_owned(),
                    rawtherapee: "5.13".to_owned(),
                },
                limits: Limits {
                    mae: 2.0,
                    p99: 64,
                    high_err: 1024,
                    high_err_frac: 0.001,
                },
                border_px: 0,
                pattern_source: PatternSource::ShiftedCrops,
                selection: Selection::default(),
            },
            rows,
        )
    }

    #[test]
    fn identical_reports_have_no_differences() {
        let report = report(vec![row("ahd", Verdict::Pass, 1.0)]);
        let diff = compare(&report, &report, false).expect("comparable");
        assert_eq!(diff.exit_code(), 0);
        assert!(diff.changes.is_empty());
    }

    #[test]
    fn verdicts_removals_additions_and_statistics_are_reported() {
        let baseline = report(vec![
            row("ahd", Verdict::Pass, 1.0),
            row("rcd", Verdict::Fail, 3.0),
            row("vng", Verdict::Pass, 1.0),
        ]);
        let current = report(vec![
            row("ahd", Verdict::Fail, 3.0),
            row("rcd", Verdict::Pass, 1.0),
            row("ppg", Verdict::Pass, 1.0),
        ]);
        let diff = compare(&baseline, &current, false).expect("comparable");
        assert_eq!(diff.changes[0].kind, ChangeKind::Regression);
        assert_eq!(diff.regressions, 1);
        assert_eq!(diff.improvements, 1);
        assert!(
            diff.changes
                .iter()
                .any(|change| change.kind == ChangeKind::Removed)
        );
        assert!(
            diff.changes
                .iter()
                .any(|change| change.kind == ChangeKind::Added)
        );
    }

    #[test]
    fn partial_new_ignores_missing_baseline_rows_and_partial_baseline_is_rejected() {
        let baseline = report(vec![
            row("ahd", Verdict::Pass, 1.0),
            row("rcd", Verdict::Pass, 1.0),
        ]);
        let mut partial = report(vec![row("ahd", Verdict::Pass, 1.0)]);
        partial.meta.selection.algorithm = vec!["ahd".to_owned()];
        assert!(
            compare(&baseline, &partial, false)
                .expect("partial new")
                .changes
                .is_empty()
        );

        let mut partial_baseline = baseline.clone();
        partial_baseline.meta.selection.algorithm = vec!["ahd".to_owned()];
        assert!(compare(&partial_baseline, &partial, false).is_err());
    }

    #[test]
    fn meta_mismatch_is_an_error_and_verdict_only_ignores_statistics() {
        let baseline = report(vec![row("ahd", Verdict::Pass, 1.0)]);
        let mut changed = report(vec![row("ahd", Verdict::Pass, 1.1)]);
        let full = compare(&baseline, &changed, false).expect("comparable");
        assert_eq!(full.changes[0].kind, ChangeKind::Statistics);
        assert_eq!(full.exit_code(), 1);
        let verdict_only = compare(&baseline, &changed, true).expect("comparable");
        assert_eq!(verdict_only.exit_code(), 0);
        changed.meta.border_px = 8;
        assert!(compare(&baseline, &changed, false).is_err());
    }
}
