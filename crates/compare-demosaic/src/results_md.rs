use std::collections::BTreeMap;
use std::path::PathBuf;

use crate::report::{BASELINE_FILE, Kind, Report, Row, Verdict};

pub const RESULTS_FILE: &str = "crates/compare-demosaic/RESULTS.md";

pub fn render(report: &Report) -> String {
    let mut output = format!(
        "# Demosaic comparison results\n\n\
Input: `{}` ({}x{}, native `{}`). Reference builds: LibRaw {}, darktable {}.\n\
Limits: MAE <= {:.6}, P99 <= {}, high-error fraction <= {:.6} above {} codes; border width {}.\n\n",
        report.meta.input,
        report.meta.width,
        report.meta.height,
        report.meta.native_pattern,
        report.meta.references.libraw,
        report.meta.references.darktable,
        report.meta.limits.mae,
        report.meta.limits.p99,
        report.meta.limits.high_err_frac,
        report.meta.limits.high_err,
        report.meta.border_px,
    );

    let mut algorithms = BTreeMap::<&str, Vec<&Row>>::new();
    for row in &report.rows {
        algorithms.entry(&row.algorithm).or_default().push(row);
    }
    for (algorithm, rows) in algorithms {
        output.push_str(&format!("## {algorithm}\n\n"));
        output.push_str(
            "| Reference | Pattern | Status | MAE | RMSE | Max | P99 | P999 | >1024 | Border >1024 | Changed |\n\
| --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
        );
        for row in rows {
            output.push_str(&format!(
                "| {} | {} | {} | {:.6} | {:.6} | {} | {} | {} | {} | {} | {:.6} |\n",
                row.reference,
                row.pattern,
                status(row),
                row.mae,
                row.rmse,
                row.max,
                row.p99,
                row.p999,
                row.count_gt1024,
                row.count_gt1024_border,
                row.changed_frac,
            ));
        }
        output.push('\n');
    }
    output
}

pub fn baseline_path() -> PathBuf { crate::run::repo_path(BASELINE_FILE) }

pub fn results_path() -> PathBuf { crate::run::repo_path(RESULTS_FILE) }

pub fn write(report: &Report) -> mischief::Result<PathBuf> {
    let path = results_path();
    std::fs::write(&path, render(report))?;
    Ok(path)
}

pub fn write_baseline() -> mischief::Result<PathBuf> { write(&Report::read(&baseline_path())?) }

fn status(row: &Row) -> &'static str {
    match (row.kind, row.verdict, row.changed_frac) {
        (_, Verdict::Fail, _) => "❌ fail",
        (_, Verdict::Pass, 0.0) => "✅ exact",
        (Kind::Rounding, Verdict::Pass, _) => "🟢 rounding",
        (_, Verdict::Pass, _) if row.max > crate::report::HIGH_ERROR_THRESHOLD => {
            "⚠️ pass with outliers"
        }
        (_, Verdict::Pass, _) => "✅ pass",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn committed_results_match_the_baseline() -> mischief::Result<()> {
        let expected = render(&Report::read(&baseline_path())?);
        let actual = std::fs::read_to_string(results_path())?;
        assert_eq!(actual, expected);
        Ok(())
    }
}
