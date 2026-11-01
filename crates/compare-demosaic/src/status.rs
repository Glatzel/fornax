use crate::report::{Kind, Report, Row, Verdict};
use crate::waivers::Waiver;

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum Status {
    Exact,
    Pass,
    Rounding,
    AcceptedOutliers,
    UnreviewedOutliers,
    Fail,
}

impl Status {
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Exact | Self::Pass => "✅",
            Self::Rounding => "🟢",
            Self::AcceptedOutliers => "⚖️",
            Self::UnreviewedOutliers => "⚠️",
            Self::Fail => "❌",
        }
    }
}

pub fn evaluate(row: &Row, waivers: &[Waiver]) -> Status {
    if row.verdict == Verdict::Fail {
        Status::Fail
    } else if row.all.changed_frac == 0.0 {
        Status::Exact
    } else if row.kind == Kind::Rounding {
        Status::Rounding
    } else if row.all.count_gt1024 == 0 {
        Status::Pass
    } else if waivers
        .iter()
        .any(|waiver| waiver.matches(row) && waiver.accepts(row))
    {
        Status::AcceptedOutliers
    } else {
        Status::UnreviewedOutliers
    }
}

pub fn waiver_findings(report: &Report, waivers: &[Waiver]) -> Vec<String> {
    let mut findings = Vec::new();
    for waiver in waivers {
        let subject = waiver_subject(waiver);
        if waiver.reason.trim().is_empty() {
            findings.push(format!("{subject} has an empty reason"));
        }
        if waiver.reason.contains("TODO") {
            findings.push(format!("{subject} reason still contains TODO"));
        }
        if waiver.evidence.trim().is_empty() {
            findings.push(format!("{subject} has empty evidence"));
        }
        if waiver.evidence.contains("TODO") {
            findings.push(format!("{subject} evidence still contains TODO"));
        }

        let matching = report
            .rows
            .iter()
            .filter(|row| waiver.matches(row))
            .collect::<Vec<_>>();
        if matching.is_empty() {
            findings.push(format!("{subject} matches no row"));
            continue;
        }
        if let Some(row) = matching.iter().find(|row| row.verdict == Verdict::Fail) {
            findings.push(format!(
                "{subject} applies to failing row {}/{}; failures cannot be waived",
                row.pattern, row.input
            ));
            continue;
        }

        let outliers = matching
            .iter()
            .copied()
            .filter(|row| row.all.count_gt1024 > 0)
            .collect::<Vec<_>>();
        if outliers.is_empty() {
            findings.push(format!("{subject} is unused"));
            continue;
        }
        if let Some(row) = outliers.iter().find(|row| !waiver.accepts(row)) {
            findings.push(format!(
                "{subject} is exceeded by {}/{}: count_gt1024 {}, max {}; accepted at most {}, max {}",
                row.pattern,
                row.input,
                row.all.count_gt1024,
                row.all.max,
                waiver.max_count_gt1024,
                waiver.max_error
            ));
        }
    }
    findings
}

impl Waiver {
    pub(crate) fn matches(self, row: &Row) -> bool {
        self.algorithm.eq_ignore_ascii_case(&row.algorithm)
            && self
                .reference
                .library()
                .eq_ignore_ascii_case(&row.reference)
            && self
                .pattern
                .is_none_or(|pattern| pattern.eq_ignore_ascii_case(&row.pattern))
            && self.input.is_none_or(|input| input == row.input)
    }

    fn accepts(self, row: &Row) -> bool {
        row.all.count_gt1024 <= self.max_count_gt1024 && row.all.max <= self.max_error
    }
}

fn waiver_subject(waiver: &Waiver) -> String {
    let mut scope = String::new();
    if let Some(pattern) = waiver.pattern {
        scope.push_str(&format!(", pattern {pattern}"));
    }
    if let Some(input) = waiver.input {
        scope.push_str(&format!(", input {input}"));
    }
    format!(
        "waiver for {} vs {}{scope}",
        waiver.algorithm,
        waiver.reference.library()
    )
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::cases::ReferenceBackend;
    use crate::report::{
        Channels, InputMeta, Limits, Meta, Metrics, PatternSource, References, Selection, Sites,
        WorstSample,
    };

    fn metrics(count_gt1024: u64, max: u16, changed_frac: f64) -> Metrics {
        Metrics {
            mae: 1.0,
            rmse: 1.0,
            max,
            mean_signed: 0.5,
            p99: 1,
            p999: 1,
            p9999: max,
            frac_gt1: changed_frac,
            frac_gt16: 0.0,
            count_gt1024,
            changed_frac,
        }
    }

    fn row(kind: Kind, verdict: Verdict, count_gt1024: u64, max: u16) -> Row {
        let changed_frac = if max == 0 { 0.0 } else { 0.5 };
        let all = metrics(count_gt1024, max, changed_frac);
        Row {
            input: "input.raw".to_owned(),
            algorithm: "rcd".to_owned(),
            reference: "darktable".to_owned(),
            pattern: "RGGB".to_owned(),
            crop: [0, 0],
            native: true,
            kind,
            samples: 12,
            verdict,
            failed_limits: Vec::new(),
            all: all.clone(),
            interior: all.clone(),
            channels: Channels {
                r: all.clone(),
                g: all.clone(),
                b: all,
            },
            sites: Sites {
                sampled: metrics(count_gt1024, max, changed_frac),
                interpolated: metrics(count_gt1024, max, changed_frac),
            },
            worst: vec![WorstSample {
                x: 0,
                y: 0,
                c: 0,
                fornax: max,
                reference: 0,
            }],
        }
    }

    fn waiver() -> Waiver {
        Waiver {
            algorithm: "rcd",
            reference: ReferenceBackend::Darktable,
            pattern: None,
            input: None,
            max_count_gt1024: 2,
            max_error: 2_000,
            reason: "The first divergence occurs in a near-tie direction decision.",
            evidence: "The stage diff and worst-sample classification agree.",
        }
    }

    fn report(rows: Vec<Row>) -> Report {
        Report {
            meta: Meta {
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
        }
    }

    #[test]
    fn status_rule_checks_each_branch_and_inclusive_bounds() {
        let waiver = waiver();
        let waivers = [waiver];
        assert_eq!(
            evaluate(&row(Kind::Robust, Verdict::Fail, 3, 3_000), &waivers),
            Status::Fail
        );
        assert_eq!(
            evaluate(&row(Kind::Exact, Verdict::Pass, 0, 0), &waivers),
            Status::Exact
        );
        assert_eq!(
            evaluate(&row(Kind::Rounding, Verdict::Pass, 0, 1), &waivers),
            Status::Rounding
        );
        assert_eq!(
            evaluate(&row(Kind::Robust, Verdict::Pass, 0, 1), &waivers),
            Status::Pass
        );
        assert_eq!(
            evaluate(&row(Kind::Robust, Verdict::Pass, 2, 2_000), &waivers),
            Status::AcceptedOutliers
        );
        assert_eq!(
            evaluate(&row(Kind::Robust, Verdict::Pass, 3, 2_000), &waivers),
            Status::UnreviewedOutliers
        );
        assert_eq!(
            evaluate(&row(Kind::Robust, Verdict::Pass, 2, 2_001), &waivers),
            Status::UnreviewedOutliers
        );
        assert_eq!(
            evaluate(&row(Kind::Robust, Verdict::Pass, 1, 2_000), &[]),
            Status::UnreviewedOutliers
        );
    }

    #[test]
    fn findings_cover_failure_unused_and_missing_matches() {
        let waiver = waiver();
        let failing = waiver_findings(
            &report(vec![row(Kind::Robust, Verdict::Fail, 3, 3_000)]),
            &[waiver],
        );
        assert!(
            failing
                .iter()
                .any(|issue| issue.contains("failures cannot be waived"))
        );

        let unused = waiver_findings(
            &report(vec![row(Kind::Robust, Verdict::Pass, 0, 1)]),
            &[waiver],
        );
        assert!(unused.iter().any(|issue| issue.contains("is unused")));

        let missing = waiver_findings(
            &report(vec![row(Kind::Robust, Verdict::Pass, 1, 2_000)]),
            &[Waiver {
                algorithm: "missing",
                ..waiver
            }],
        );
        assert!(missing.iter().any(|issue| issue.contains("matches no row")));
    }

    #[test]
    fn findings_report_exceeded_bounds_for_each_matching_row() {
        let mut second = row(Kind::Robust, Verdict::Pass, 3, 2_000);
        second.pattern = "BGGR".to_owned();
        second.native = false;
        let findings = waiver_findings(
            &report(vec![row(Kind::Robust, Verdict::Pass, 1, 2_000), second]),
            &[waiver()],
        );
        assert!(findings.iter().any(|issue| {
            issue.contains("is exceeded by BGGR/input.raw") && issue.contains("count_gt1024 3")
        }));
    }

    #[test]
    fn findings_require_documented_reason_and_evidence() {
        let documented_row = row(Kind::Robust, Verdict::Pass, 1, 2_000);
        for (reason, evidence, expected) in [
            ("", "verified", "empty reason"),
            ("mechanism TODO", "verified", "reason still contains TODO"),
            ("mechanism", "", "empty evidence"),
            ("mechanism", "verified TODO", "evidence still contains TODO"),
        ] {
            let findings = waiver_findings(
                &report(vec![documented_row.clone()]),
                &[Waiver {
                    reason,
                    evidence,
                    ..waiver()
                }],
            );
            assert!(
                findings.iter().any(|issue| issue.contains(expected)),
                "{findings:?}"
            );
        }
    }
}
