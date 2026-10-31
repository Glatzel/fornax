use std::io::Write;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

pub const CANONICAL_FILE: &str = "target/compare-demosaic/canonical.json";
pub const BASELINE_FILE: &str = "crates/compare-demosaic/baseline/canonical.json";
pub const BORDER_PX: u32 = 16;
pub const HIGH_ERROR_THRESHOLD: u16 = 1024;
pub const LIMITS: Limits = Limits {
    mae: 2.0,
    p99: 64,
    high_err: HIGH_ERROR_THRESHOLD,
    high_err_frac: 0.001,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub meta: Meta,
    pub rows: Vec<Row>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// Invariant comparison inputs. Volatile run details belong in git or CI logs.
pub struct Meta {
    pub input: String,
    pub width: u32,
    pub height: u32,
    pub native_pattern: String,
    pub references: References,
    pub limits: Limits,
    pub border_px: u32,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct References {
    pub libraw: String,
    pub darktable: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub mae: f64,
    pub p99: u16,
    pub high_err: u16,
    pub high_err_frac: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Exact,
    Rounding,
    Robust,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub algorithm: String,
    pub reference: String,
    pub pattern: String,
    pub kind: Kind,
    pub verdict: Verdict,
    pub samples: u64,
    pub mae: f64,
    pub rmse: f64,
    pub max: u16,
    pub mean_signed: f64,
    pub p99: u16,
    pub p999: u16,
    pub count_gt1024: u64,
    /// Subset of `count_gt1024` inside the excluded border strip.
    pub count_gt1024_border: u64,
    pub changed_frac: f64,
}

#[derive(Clone, Copy, Debug)]
pub struct VerdictInput {
    pub samples: u64,
    pub mae: f64,
    pub p99: u16,
    pub max: u16,
    pub count_gt1024: u64,
    pub count_below_reference: u64,
}

/// Each kind keeps the rule it inherited from the previous comparison contract.
pub const fn verdict(kind: Kind, input: VerdictInput, limits: Limits) -> Verdict {
    let pass = match kind {
        Kind::Exact => input.max == 0,
        Kind::Rounding => input.max <= 1 && input.count_below_reference == 0,
        Kind::Robust => {
            let high_error_frac = if input.samples == 0 {
                0.0
            } else {
                input.count_gt1024 as f64 / input.samples as f64
            };
            input.mae <= limits.mae
                && input.p99 <= limits.p99
                && high_error_frac <= limits.high_err_frac
        }
    };
    if pass { Verdict::Pass } else { Verdict::Fail }
}

impl Report {
    pub fn new(meta: Meta, mut rows: Vec<Row>) -> Self {
        rows.sort_unstable_by(row_order);
        Self { meta, rows }
    }

    pub fn read(path: &Path) -> mischief::Result<Self> {
        let bytes = std::fs::read(path)?;
        let report = serde_json::from_slice::<Self>(&bytes)
            .map_err(|error| mischief::mischief!("cannot parse {}: {error}", path.display()))?;
        report.validate()?;
        Ok(report)
    }

    pub fn write(&self, path: &Path) -> mischief::Result<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut file = std::fs::File::create(path)?;
        let meta = serde_json::to_string(&self.meta)
            .map_err(|error| mischief::mischief!("cannot serialize metadata: {error}"))?;
        writeln!(file, "{{")?;
        writeln!(file, "  \"meta\": {meta},")?;
        writeln!(file, "  \"rows\": [")?;
        for (index, row) in self.rows.iter().enumerate() {
            write!(
                file,
                "    {{\"algorithm\":\"{}\",\"reference\":\"{}\",\"pattern\":\"{}\",\"kind\":\"{}\",\"verdict\":\"{}\",\"samples\":{},\"mae\":{:.6},\"rmse\":{:.6},\"max\":{},\"mean_signed\":{:.6},\"p99\":{},\"p999\":{},\"count_gt1024\":{},\"count_gt1024_border\":{},\"changed_frac\":{:.6}}}",
                row.algorithm,
                row.reference,
                row.pattern,
                kind_name(row.kind),
                verdict_name(row.verdict),
                row.samples,
                row.mae,
                row.rmse,
                row.max,
                row.mean_signed,
                row.p99,
                row.p999,
                row.count_gt1024,
                row.count_gt1024_border,
                row.changed_frac,
            )?;
            writeln!(
                file,
                "{}",
                if index + 1 == self.rows.len() {
                    ""
                } else {
                    ","
                }
            )?;
        }
        writeln!(file, "  ]")?;
        writeln!(file, "}}")?;
        Ok(())
    }

    pub fn write_canonical(&self) -> mischief::Result<PathBuf> {
        let path = crate::run::repo_path(CANONICAL_FILE);
        self.write(&path)?;
        Ok(path)
    }

    pub fn write_baseline(&self) -> mischief::Result<PathBuf> {
        let path = crate::run::repo_path(BASELINE_FILE);
        self.write(&path)?;
        Ok(path)
    }

    fn validate(&self) -> mischief::Result<()> {
        if self.rows.is_empty() {
            return Err(mischief::mischief!("report has no rows"));
        }
        for pair in self.rows.windows(2) {
            if row_order(&pair[0], &pair[1]) != std::cmp::Ordering::Less {
                return Err(mischief::mischief!(
                    "report rows are not strictly sorted at {} {} {}",
                    pair[1].algorithm,
                    pair[1].reference,
                    pair[1].pattern
                ));
            }
        }
        Ok(())
    }
}

const fn kind_name(kind: Kind) -> &'static str {
    match kind {
        Kind::Exact => "exact",
        Kind::Rounding => "rounding",
        Kind::Robust => "robust",
    }
}

const fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Pass => "pass",
        Verdict::Fail => "fail",
    }
}

fn row_order(left: &Row, right: &Row) -> std::cmp::Ordering {
    (&left.algorithm, &left.reference, &left.pattern).cmp(&(
        &right.algorithm,
        &right.reference,
        &right.pattern,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn input() -> VerdictInput {
        VerdictInput {
            samples: 1_000,
            mae: 1.0,
            p99: 32,
            max: 2_000,
            count_gt1024: 1,
            count_below_reference: 0,
        }
    }

    fn row(algorithm: &str) -> Row {
        Row {
            algorithm: algorithm.to_owned(),
            reference: "libraw".to_owned(),
            pattern: "RGGB".to_owned(),
            kind: Kind::Robust,
            verdict: Verdict::Pass,
            samples: 1_000,
            mae: 1.0,
            rmse: 2.0,
            max: 2_000,
            mean_signed: 0.5,
            p99: 32,
            p999: 512,
            count_gt1024: 1,
            count_gt1024_border: 0,
            changed_frac: 0.25,
        }
    }

    #[test]
    fn verdict_boundaries_are_inclusive() {
        assert_eq!(verdict(Kind::Exact, input(), LIMITS), Verdict::Fail);
        assert_eq!(
            verdict(Kind::Rounding, VerdictInput { max: 1, ..input() }, LIMITS),
            Verdict::Pass
        );
        assert_eq!(verdict(Kind::Robust, input(), LIMITS), Verdict::Pass);
        assert_eq!(
            verdict(
                Kind::Robust,
                VerdictInput {
                    count_gt1024: 2,
                    ..input()
                },
                LIMITS
            ),
            Verdict::Fail
        );
    }

    #[test]
    fn write_then_read_is_strict_and_lossless() -> mischief::Result<()> {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("canonical.json");
        let report = Report::new(
            Meta {
                input: "input.raw".to_owned(),
                width: 2,
                height: 2,
                native_pattern: "RGGB".to_owned(),
                references: References {
                    libraw: "0.22.3".to_owned(),
                    darktable: "5.6.2".to_owned(),
                },
                limits: LIMITS,
                border_px: BORDER_PX,
            },
            vec![row("rcd"), row("ahd")],
        );
        report.write(&path)?;
        assert_eq!(Report::read(&path)?, report);

        let mut tampered = std::fs::read_to_string(&path)?;
        tampered = tampered.replacen(
            "\"border_px\":16",
            "\"border_px\":16,\"unexpected\":true",
            1,
        );
        std::fs::write(&path, tampered)?;
        assert!(Report::read(&path).is_err());
        Ok(())
    }
}
