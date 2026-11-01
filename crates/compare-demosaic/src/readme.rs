use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::report::{InputMeta, Kind, Report, Row, Verdict};

pub const README_FILE: &str = "crates/compare-demosaic/README.md";

const REQUIRED_REGIONS: &[&str] = &["usage", "legend", "summary"];
const NOTE_FIELDS: &[&str] = &[
    "Reference",
    "Why this reference",
    "Why we match",
    "Known differences",
];

#[derive(Clone, Debug, Default)]
pub struct ReadmeReport {
    pub issues: Vec<String>,
}

impl ReadmeReport {
    pub const fn exit_code(&self) -> u8 { if self.issues.is_empty() { 0 } else { 1 } }

    pub fn render(&self) -> String {
        if self.issues.is_empty() {
            "README is up to date\n".to_owned()
        } else {
            let mut output = String::new();
            for issue in &self.issues {
                output.push_str("- ");
                output.push_str(issue);
                output.push('\n');
            }
            output
        }
    }
}

pub fn load(path: &Path) -> mischief::Result<Report> {
    let report = Report::read(path)?;
    if report.meta.selection.is_partial() {
        return Err(mischief::mischief!(
            "{} is a partial run and cannot generate the README",
            path.display()
        ));
    }
    Ok(report)
}

pub fn write(report: &Report, usage: &str, path: Option<&Path>) -> mischief::Result<ReadmeReport> {
    let path = path.map_or_else(readme_path, Path::to_path_buf);
    let mut document = std::fs::read_to_string(&path)?;
    let regions = generated_regions(report, usage);
    for id in REQUIRED_REGIONS {
        document = replace_region(&document, id, &regions[*id])?;
    }
    for (id, body) in &regions {
        if id.starts_with("algo:") && has_marker(&document, id) {
            document = replace_region(&document, id, body)?;
        }
    }
    for (algorithm, _rows) in algorithms(report) {
        let id = algorithm_id(algorithm);
        if !has_marker(&document, &format!("algo:{id}")) {
            document.push('\n');
            document.push_str(&new_section(algorithm, &regions[&format!("algo:{id}")]));
        }
    }
    std::fs::write(path, document)?;
    Ok(ReadmeReport::default())
}

pub fn check(report: &Report, usage: &str, path: Option<&Path>) -> mischief::Result<ReadmeReport> {
    let path = path.map_or_else(readme_path, Path::to_path_buf);
    let document = std::fs::read_to_string(&path)?;
    let regions = generated_regions(report, usage);
    let mut report_out = ReadmeReport::default();
    for id in REQUIRED_REGIONS {
        check_region(&document, id, &regions[*id], &mut report_out.issues);
    }
    for (algorithm, _rows) in algorithms(report) {
        let id = algorithm_id(algorithm);
        let region_id = format!("algo:{id}");
        check_region(
            &document,
            &region_id,
            &regions[&region_id],
            &mut report_out.issues,
        );
        check_note(&document, algorithm, &region_id, &mut report_out.issues);
    }
    Ok(report_out)
}

pub fn generated_regions(report: &Report, usage: &str) -> BTreeMap<String, String> {
    let mut regions = BTreeMap::new();
    regions.insert("usage".to_owned(), render_usage(usage));
    regions.insert("legend".to_owned(), render_legend(report));
    regions.insert("summary".to_owned(), render_summary(report));
    for (algorithm, rows) in algorithms(report) {
        regions.insert(
            format!("algo:{}", algorithm_id(algorithm)),
            render_algorithm(&rows),
        );
    }
    regions
}

pub fn render_clap_usage(command: &clap::Command) -> String {
    let mut output = String::new();
    for subcommand in command.get_subcommands() {
        let mut subcommand = subcommand.clone();
        let name = subcommand.get_name().to_owned();
        let help = subcommand.render_long_help();
        output.push_str(&format!("$ compare-demosaic {name} --help\n{help}"));
        if !output.ends_with('\n') {
            output.push('\n');
        }
        output.push('\n');
    }
    output.trim_end().to_owned()
}

fn render_usage(usage: &str) -> String {
    let normalized = usage
        .lines()
        .map(str::trim_end)
        .collect::<Vec<_>>()
        .join("\n");
    format!("```text\n{}\n```", normalized.trim())
}

fn render_legend(report: &Report) -> String {
    let mut output = format!(
        "- `✅`: exact or passing without high-error outliers.\n\
- `🟢`: rounding-only differences.\n\
- `⚠️`: every limit passes, but high-error outliers remain.\n\
- `❌`: at least one limit failed.\n\
- Limits: MAE <= {:.6}, P99 <= {}, high-error fraction <= {:.6} above {} codes.\n\
- Kinds: `exact` requires equality; `rounding` permits one-code upward rounding differences; `robust` applies the numeric limits.\n\
- A row is `native` only when its pattern equals the input's original CFA layout. Relabelled patterns are consistency checks, not independent tests.\n\
- Interior statistics exclude a {} px border.\n",
        report.meta.limits.mae,
        report.meta.limits.p99,
        report.meta.limits.high_err_frac,
        report.meta.limits.high_err,
        report.meta.border_px,
    );
    output.push_str(&format!(
        "- References: LibRaw {}; darktable {}.\n",
        report.meta.references.libraw, report.meta.references.darktable
    ));
    output.push_str(&render_inputs(&report.meta.inputs));
    output.trim_end().to_owned()
}

fn render_inputs(inputs: &[InputMeta]) -> String {
    let mut output = String::from("- Inputs:\n");
    for input in inputs {
        output.push_str(&format!(
            "  - `{}`: {}x{}, native `{}`, sha256 `{}`.\n",
            input.name, input.width, input.height, input.native_pattern, input.sha256
        ));
    }
    output
}

fn render_summary(report: &Report) -> String {
    let mut groups = BTreeMap::<(&str, &str), Vec<&Row>>::new();
    for row in &report.rows {
        groups
            .entry((&row.algorithm, &row.reference))
            .or_default()
            .push(row);
    }
    let mut output = String::from(
        "| Algorithm | Reference | Worst verdict | Native-pattern result |\n\
| --- | --- | --- | --- |\n",
    );
    for ((algorithm, reference), rows) in groups {
        let worst = if rows.iter().any(|row| row.verdict == Verdict::Fail) {
            Verdict::Fail
        } else {
            Verdict::Pass
        };
        let native = rows
            .iter()
            .filter(|row| row.native)
            .map(|row| format!("{} {}", row.pattern, status(row)))
            .collect::<Vec<_>>()
            .join(", ");
        output.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            crate::cases::display_name(algorithm),
            reference,
            status_verdict(worst),
            if native.is_empty() {
                "not selected".to_owned()
            } else {
                native
            },
        ));
    }
    output.trim_end().to_owned()
}

fn render_algorithm(rows: &[&Row]) -> String {
    let mut output = String::from(
        "| Reference | Pattern | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |\n\
| --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
    );
    for row in rows {
        let reference = crate::cases::source_url(&row.algorithm, &row.reference).map_or_else(
            || row.reference.clone(),
            |url| format!("[{}]({url})", row.reference),
        );
        let border = row
            .all
            .count_gt1024
            .saturating_sub(row.interior.count_gt1024);
        output.push_str(&format!(
            "| {} | {} | {} | {} | {:.6} | {:.6} | {} | {:.6} | {} | {} | {} ({}) | {:.6} |\n",
            reference,
            row.pattern,
            if row.native { "yes" } else { "no" },
            status(row),
            row.all.mae,
            row.all.rmse,
            row.all.max,
            row.all.mean_signed,
            row.all.p99,
            row.all.p999,
            row.all.count_gt1024,
            border,
            row.all.changed_frac,
        ));
    }
    output.trim_end().to_owned()
}

fn status(row: &Row) -> String {
    match (row.kind, row.verdict) {
        (_, Verdict::Fail) => "❌ fail".to_owned(),
        (_, Verdict::Pass) if row.all.changed_frac == 0.0 => "✅ exact".to_owned(),
        (Kind::Rounding, Verdict::Pass) => "🟢 rounding".to_owned(),
        (_, Verdict::Pass) if row.all.count_gt1024 > 0 => "⚠️ pass with outliers".to_owned(),
        (_, Verdict::Pass) => "✅ pass".to_owned(),
    }
}

fn status_verdict(verdict: Verdict) -> String {
    format!("{} {}", verdict.symbol(), verdict_name(verdict))
}

const fn verdict_name(verdict: Verdict) -> &'static str {
    match verdict {
        Verdict::Pass => "pass",
        Verdict::Fail => "fail",
    }
}

fn algorithms(report: &Report) -> BTreeMap<&str, Vec<&Row>> {
    let mut algorithms = BTreeMap::<&str, Vec<&Row>>::new();
    for row in &report.rows {
        algorithms.entry(&row.algorithm).or_default().push(row);
    }
    algorithms
}

fn algorithm_id(algorithm: &str) -> String {
    algorithm
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}

fn new_section(algorithm: &str, table: &str) -> String {
    format!(
        "## {}\n\n<!-- compare-demosaic:begin algo:{} -->\n{}\n<!-- compare-demosaic:end algo:{} -->\n\n{}",
        crate::cases::display_name(algorithm),
        algorithm_id(algorithm),
        table,
        algorithm_id(algorithm),
        stub_note(),
    )
}

const fn stub_note() -> &'static str {
    "> [!NOTE]\n\
> **Reference:** TODO\n\
> **Why this reference:** TODO\n\
> **Why we match:** TODO\n\
> **Known differences:** TODO\n"
}

fn readme_path() -> PathBuf { crate::run::repo_path(README_FILE) }

fn begin_marker(id: &str) -> String { format!("<!-- compare-demosaic:begin {id} -->") }

fn end_marker(id: &str) -> String { format!("<!-- compare-demosaic:end {id} -->") }

fn has_marker(document: &str, id: &str) -> bool {
    document.contains(&begin_marker(id)) && document.contains(&end_marker(id))
}

fn replace_region(document: &str, id: &str, body: &str) -> mischief::Result<String> {
    let begin = begin_marker(id);
    let end = end_marker(id);
    let Some(begin_at) = document.find(&begin) else {
        return Err(mischief::mischief!("README is missing marker `{id}`"));
    };
    let content_start = begin_at + begin.len();
    let Some(relative_end) = document[content_start..].find(&end) else {
        return Err(mischief::mischief!("README has an unmatched marker `{id}`"));
    };
    let end_at = content_start + relative_end;
    let mut output = String::with_capacity(document.len() + body.len());
    output.push_str(&document[..content_start]);
    output.push('\n');
    output.push_str(body);
    output.push('\n');
    output.push_str(&document[end_at..]);
    Ok(output)
}

fn region_body<'a>(document: &'a str, id: &str) -> Option<&'a str> {
    let begin = begin_marker(id);
    let end = end_marker(id);
    let begin_at = document.find(&begin)?;
    let content_start = begin_at + begin.len();
    let relative_end = document[content_start..].find(&end)?;
    Some(document[content_start..content_start + relative_end].trim())
}

fn check_region(document: &str, id: &str, expected: &str, issues: &mut Vec<String>) {
    match region_body(document, id) {
        Some(actual) if actual == expected.trim() => {}
        Some(_) => issues.push(format!("generated region `{id}` is out of date")),
        None => issues.push(format!("generated region `{id}` is missing")),
    }
}

fn check_note(document: &str, algorithm: &str, region_id: &str, issues: &mut Vec<String>) {
    let Some(body) = region_body(document, region_id) else {
        return;
    };
    let after_marker = end_marker(region_id);
    let Some(marker_at) = document.find(&after_marker) else {
        return;
    };
    let after = &document[marker_at + after_marker.len()..];
    let note_end = after.find("\n## ").unwrap_or(after.len());
    let note = &after[..note_end];
    if !note.contains("> [!NOTE]") {
        issues.push(format!("{algorithm} is missing its note block"));
        return;
    }
    if note.contains("TODO") {
        issues.push(format!("{algorithm} note still contains TODO"));
    }
    for field in NOTE_FIELDS {
        let prefix = format!("> **{field}:**");
        let value = note
            .lines()
            .find_map(|line| line.trim().strip_prefix(&prefix).map(str::trim));
        match value {
            Some(value) if !value.is_empty() => {}
            _ => issues.push(format!("{algorithm} note is missing `{field}`")),
        }
    }
    if let Some(open) = note
        .lines()
        .find_map(|line| line.trim().strip_prefix("> **Open issues:**"))
        && open.trim().is_empty()
    {
        issues.push(format!("{algorithm} has an empty `Open issues` field"));
    }
    let _ = body;
}

#[cfg(test)]
fn note_algorithm_names(document: &str) -> std::collections::BTreeSet<String> {
    document
        .match_indices("<!-- compare-demosaic:begin algo:")
        .filter_map(|(index, marker)| {
            let rest = &document[index + marker.len()..];
            rest.split_once(" -->").map(|(id, _)| id.to_owned())
        })
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::report::{Channels, Limits, Meta, References, Selection, Sites, WorstSample};

    fn report(algorithm: &str) -> Report {
        let metrics = crate::report::Metrics {
            mae: 1.0,
            rmse: 1.0,
            max: 2,
            mean_signed: 0.5,
            p99: 1,
            p999: 1,
            p9999: 2,
            frac_gt1: 0.5,
            frac_gt16: 0.0,
            count_gt1024: 0,
            changed_frac: 0.5,
        };
        let row = Row {
            input: "input.raw".to_owned(),
            algorithm: algorithm.to_owned(),
            reference: "libraw".to_owned(),
            pattern: "RGGB".to_owned(),
            native: true,
            kind: Kind::Robust,
            samples: 12,
            verdict: Verdict::Pass,
            failed_limits: Vec::new(),
            all: metrics.clone(),
            interior: metrics.clone(),
            channels: Channels {
                r: metrics.clone(),
                g: metrics.clone(),
                b: metrics.clone(),
            },
            sites: Sites {
                sampled: metrics.clone(),
                interpolated: metrics,
            },
            worst: vec![WorstSample {
                x: 0,
                y: 0,
                c: 0,
                fornax: 2,
                reference: 0,
            }],
        };
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
                },
                limits: Limits {
                    mae: 2.0,
                    p99: 64,
                    high_err: 1024,
                    high_err_frac: 0.001,
                },
                border_px: 0,
                selection: Selection::default(),
            },
            vec![row],
        )
    }

    fn shell() -> String {
        format!(
            "# test\n\n{}rust\nusage\n{}\n{}legend\n{}\n{}summary\n{}\n",
            begin_marker("usage"),
            end_marker("usage"),
            begin_marker("legend"),
            end_marker("legend"),
            begin_marker("summary"),
            end_marker("summary"),
        )
    }

    #[test]
    fn write_is_idempotent_and_creates_stub_notes() -> mischief::Result<()> {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("README.md");
        std::fs::write(&path, shell())?;
        let report = report("ahd");
        write(&report, "usage", Some(&path))?;
        let first = std::fs::read_to_string(&path)?;
        write(&report, "usage", Some(&path))?;
        assert_eq!(std::fs::read_to_string(&path)?, first);
        let checked = check(&report, "usage", Some(&path))?;
        assert!(checked.issues.iter().any(|issue| issue.contains("TODO")));
        Ok(())
    }

    #[test]
    fn check_detects_table_edits_and_missing_note_fields() -> mischief::Result<()> {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("README.md");
        std::fs::write(&path, shell())?;
        let report = report("ahd");
        write(&report, "usage", Some(&path))?;
        let mut text = std::fs::read_to_string(&path)?;
        text = text.replace("| yes |", "| no |");
        text = text.replace(
            "> **Known differences:** TODO",
            "> **Known differences:** expected",
        );
        text = text.replace("> **Why this reference:** TODO\n", "");
        std::fs::write(&path, text)?;
        let checked = check(&report, "usage", Some(&path))?;
        assert!(
            checked
                .issues
                .iter()
                .any(|issue| issue.contains("out of date"))
        );
        assert!(checked.issues.iter().any(|issue| issue.contains("TODO")));
        assert!(
            checked
                .issues
                .iter()
                .any(|issue| issue.contains("Why this reference"))
        );
        Ok(())
    }

    #[test]
    fn partial_reports_are_refused() -> mischief::Result<()> {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("partial.json");
        let mut report = report("ahd");
        report.meta.selection.algorithm = vec!["ahd".to_owned()];
        report.write(&path)?;
        assert!(load(&path).is_err());
        Ok(())
    }

    #[test]
    fn marker_ids_are_discoverable() {
        let text = begin_marker("algo:ahd");
        assert_eq!(
            note_algorithm_names(&text).into_iter().collect::<Vec<_>>(),
            vec!["ahd"]
        );
    }
}
