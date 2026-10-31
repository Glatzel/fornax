use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use crate::report::{Report, Row, Verdict};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum ChangeKind {
    Meta,
    Missing,
    New,
    Regression,
    Improvement,
    Metrics,
}

#[derive(Clone, Debug)]
struct Change {
    kind: ChangeKind,
    key: String,
    before: Option<Row>,
    after: Option<Row>,
}

#[derive(Clone, Debug)]
pub struct CheckReport {
    changes: Vec<Change>,
    regressions: usize,
    improvements: usize,
}

impl CheckReport {
    pub const fn exit_code(&self) -> u8 { if self.regressions == 0 { 0 } else { 1 } }

    pub fn render_text(&self) -> String {
        if self.changes.is_empty() {
            return "no changes\n".to_owned();
        }
        let mut output = String::from(
            "| Change | Row | Before | After | Metrics |\n\
| --- | --- | --- | --- | --- |\n",
        );
        for change in &self.changes {
            let before = change.before.as_ref();
            let after = change.after.as_ref();
            output.push_str(&format!(
                "| {} | `{}` | {} | {} | {} |\n",
                kind_name(change.kind),
                change.key,
                before.map_or("missing", |row| verdict_name(row.verdict)),
                after.map_or("missing", |row| verdict_name(row.verdict)),
                metric_delta(before, after),
            ));
        }
        output.push_str(&format!(
            "\n{} regressions, {} improvements\n",
            self.regressions, self.improvements
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
        std::io::Write::write_all(&mut file, b"\n## compare-demosaic\n\n")?;
        std::io::Write::write_all(&mut file, self.render_text().as_bytes())?;
        Ok(())
    }
}

pub fn check_file(
    baseline_path: &Path,
    current: &Report,
    algorithms: &[String],
) -> mischief::Result<CheckReport> {
    let baseline = Report::read(baseline_path)?;
    Ok(compare(&baseline, current, algorithms))
}

pub fn compare(baseline: &Report, current: &Report, algorithms: &[String]) -> CheckReport {
    let mut changes = Vec::new();
    if baseline.meta != current.meta {
        changes.push(Change {
            kind: ChangeKind::Meta,
            key: "meta".to_owned(),
            before: None,
            after: None,
        });
    }

    let algorithms = algorithms
        .iter()
        .map(|algorithm| algorithm.to_ascii_lowercase())
        .collect::<BTreeSet<_>>();
    let before = baseline
        .rows
        .iter()
        .filter(|row| algorithms.is_empty() || algorithms.contains(&row.algorithm))
        .map(|row| (key(row), row))
        .collect::<BTreeMap<_, _>>();
    let after = current
        .rows
        .iter()
        .map(|row| (key(row), row))
        .collect::<BTreeMap<_, _>>();
    let keys = before
        .keys()
        .chain(after.keys())
        .cloned()
        .collect::<BTreeSet<_>>();
    for key in keys {
        match (before.get(&key).copied(), after.get(&key).copied()) {
            (Some(old), Some(new)) if old == new => {}
            (Some(old), Some(new)) => {
                let kind = match (old.verdict, new.verdict) {
                    (Verdict::Pass, Verdict::Fail) => ChangeKind::Regression,
                    (Verdict::Fail, Verdict::Pass) => ChangeKind::Improvement,
                    _ => ChangeKind::Metrics,
                };
                changes.push(Change {
                    kind,
                    key,
                    before: Some(old.clone()),
                    after: Some(new.clone()),
                });
            }
            (Some(old), None) => changes.push(Change {
                kind: ChangeKind::Missing,
                key,
                before: Some(old.clone()),
                after: None,
            }),
            (None, Some(new)) => changes.push(Change {
                kind: ChangeKind::New,
                key,
                before: None,
                after: Some(new.clone()),
            }),
            (None, None) => unreachable!("key came from one side"),
        }
    }

    changes.sort_by(|left, right| {
        severity(left.kind)
            .cmp(&severity(right.kind))
            .then_with(|| left.key.cmp(&right.key))
    });
    let regressions = changes
        .iter()
        .filter(|change| {
            matches!(
                change.kind,
                ChangeKind::Meta | ChangeKind::Missing | ChangeKind::New | ChangeKind::Regression
            )
        })
        .count();
    let improvements = changes
        .iter()
        .filter(|change| change.kind == ChangeKind::Improvement)
        .count();
    CheckReport {
        changes,
        regressions,
        improvements,
    }
}

fn key(row: &Row) -> String { format!("{} {} {}", row.algorithm, row.pattern, row.reference) }

fn metric_delta(before: Option<&Row>, after: Option<&Row>) -> String {
    let (Some(before), Some(after)) = (before, after) else {
        return String::new();
    };
    [
        ("mae", before.mae, after.mae),
        ("rmse", before.rmse, after.rmse),
        ("max", f64::from(before.max), f64::from(after.max)),
        ("p99", f64::from(before.p99), f64::from(after.p99)),
        (
            "gt1024",
            before.count_gt1024 as f64,
            after.count_gt1024 as f64,
        ),
        (
            "border",
            before.count_gt1024_border as f64,
            after.count_gt1024_border as f64,
        ),
        ("changed", before.changed_frac, after.changed_frac),
    ]
    .into_iter()
    .filter(|(_, before, after)| before.to_bits() != after.to_bits())
    .map(|(name, before, after)| format!("{name} {before:.6} -> {after:.6}"))
    .collect::<Vec<_>>()
    .join(", ")
}

const fn severity(kind: ChangeKind) -> u8 {
    match kind {
        ChangeKind::Meta => 0,
        ChangeKind::Missing => 1,
        ChangeKind::New => 2,
        ChangeKind::Regression => 3,
        ChangeKind::Improvement => 4,
        ChangeKind::Metrics => 5,
    }
}

const fn kind_name(kind: ChangeKind) -> &'static str {
    match kind {
        ChangeKind::Meta => "META",
        ChangeKind::Missing => "MISSING",
        ChangeKind::New => "NEW",
        ChangeKind::Regression => "REGRESSION",
        ChangeKind::Improvement => "IMPROVEMENT",
        ChangeKind::Metrics => "CHANGED",
    }
}

const fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Pass => "pass",
        Verdict::Fail => "fail",
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{BORDER_PX, Kind, LIMITS, Limits, Meta, References};

    fn meta() -> Meta {
        Meta {
            input: "input.raw".to_owned(),
            width: 1,
            height: 1,
            native_pattern: "RGGB".to_owned(),
            references: References {
                libraw: "0.22.3".to_owned(),
                darktable: "5.6.2".to_owned(),
            },
            limits: LIMITS,
            border_px: BORDER_PX,
        }
    }

    fn row(algorithm: &str, verdict: Verdict, mae: f64) -> Row {
        Row {
            algorithm: algorithm.to_owned(),
            reference: "libraw".to_owned(),
            pattern: "RGGB".to_owned(),
            kind: Kind::Robust,
            verdict,
            samples: 10,
            mae,
            rmse: mae,
            max: 1024,
            mean_signed: 0.0,
            p99: 1,
            p999: 1,
            count_gt1024: 0,
            count_gt1024_border: 0,
            changed_frac: 0.1,
        }
    }

    #[test]
    fn reports_regressions_improvements_and_metric_changes() {
        let baseline = Report {
            meta: meta(),
            rows: vec![
                row("ahd", Verdict::Pass, 1.0),
                row("dcb", Verdict::Fail, 3.0),
                row("rcd", Verdict::Pass, 1.0),
            ],
        };
        let current = Report {
            meta: meta(),
            rows: vec![
                row("ahd", Verdict::Fail, 3.0),
                row("dcb", Verdict::Pass, 1.0),
                row("rcd", Verdict::Pass, 1.1),
            ],
        };
        let report = compare(&baseline, &current, &[]);
        assert_eq!(report.regressions, 1);
        assert_eq!(report.improvements, 1);
        assert_eq!(report.exit_code(), 1);
        assert_eq!(report.changes[0].kind, ChangeKind::Regression);
    }

    #[test]
    fn filtered_check_ignores_unselected_algorithms() {
        let baseline = Report {
            meta: meta(),
            rows: vec![
                row("ahd", Verdict::Pass, 1.0),
                row("rcd", Verdict::Pass, 1.0),
            ],
        };
        let current = Report {
            meta: meta(),
            rows: vec![row("ahd", Verdict::Pass, 1.0)],
        };
        let report = compare(&baseline, &current, &["ahd".to_owned()]);
        assert_eq!(report.regressions, 0);
        assert!(report.changes.is_empty());
    }

    #[test]
    fn missing_new_and_meta_changes_are_regressions() {
        let baseline = Report {
            meta: meta(),
            rows: vec![row("ahd", Verdict::Pass, 1.0)],
        };
        let mut current = Report {
            meta: meta(),
            rows: vec![row("rcd", Verdict::Pass, 1.0)],
        };
        current.meta.border_px = 8;
        let report = compare(&baseline, &current, &[]);
        assert_eq!(report.regressions, 3);
    }

    #[test]
    fn limits_are_comparable() {
        assert_eq!(
            Limits {
                mae: 2.0,
                p99: 64,
                high_err: 1024,
                high_err_frac: 0.001,
            },
            LIMITS
        );
    }
}
