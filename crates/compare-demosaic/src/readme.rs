use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use crate::report::{InputMeta, Report, Row};
use crate::status::{self, Status};
use crate::waivers::{WAIVERS, Waiver};

pub const README_FILE: &str = "crates/compare-demosaic/README.md";

const REQUIRED_REGIONS: &[&str] = &["usage", "legend", "summary"];
const NOTE_FIELDS: &[&str] = &["Why this reference", "Why we match", "Known differences"];

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
    write_with_waivers(report, usage, WAIVERS, path)
}

fn write_with_waivers(
    report: &Report,
    usage: &str,
    waivers: &[Waiver],
    path: Option<&Path>,
) -> mischief::Result<ReadmeReport> {
    let path = path.map_or_else(readme_path, Path::to_path_buf);
    let mut document = std::fs::read_to_string(&path)?;
    let regions = generated_regions_with_waivers(report, usage, waivers);
    for id in REQUIRED_REGIONS {
        document = replace_region(&document, id, &regions[*id])?;
    }
    for (id, body) in &regions {
        if id.starts_with("algo:") && has_marker(&document, id) {
            document = replace_region(&document, id, body)?;
        }
    }
    for (algorithm, rows) in algorithms(report) {
        let id = algorithm_id(algorithm);
        if !has_marker(&document, &format!("algo:{id}")) {
            document.push('\n');
            document.push_str(&new_section(
                algorithm,
                &regions[&format!("algo:{id}")],
                &rows,
                waivers,
            ));
        }
    }
    std::fs::write(path, document)?;
    Ok(ReadmeReport {
        issues: status::waiver_findings(report, waivers),
    })
}

pub fn check(report: &Report, usage: &str, path: Option<&Path>) -> mischief::Result<ReadmeReport> {
    check_with_waivers(report, usage, WAIVERS, path)
}

fn check_with_waivers(
    report: &Report,
    usage: &str,
    waivers: &[Waiver],
    path: Option<&Path>,
) -> mischief::Result<ReadmeReport> {
    let path = path.map_or_else(readme_path, Path::to_path_buf);
    let document = std::fs::read_to_string(&path)?;
    let regions = generated_regions_with_waivers(report, usage, waivers);
    let mut report_out = ReadmeReport {
        issues: status::waiver_findings(report, waivers),
    };
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
        check_note(
            &document,
            algorithm,
            &region_id,
            waivers,
            &mut report_out.issues,
        );
    }
    Ok(report_out)
}

pub fn generated_regions(report: &Report, usage: &str) -> BTreeMap<String, String> {
    generated_regions_with_waivers(report, usage, WAIVERS)
}

fn generated_regions_with_waivers(
    report: &Report,
    usage: &str,
    waivers: &[Waiver],
) -> BTreeMap<String, String> {
    let mut regions = BTreeMap::new();
    regions.insert("usage".to_owned(), render_usage(usage));
    regions.insert("legend".to_owned(), render_legend(report));
    regions.insert("summary".to_owned(), render_summary(report, waivers));
    for (algorithm, rows) in algorithms(report) {
        regions.insert(
            format!("algo:{}", algorithm_id(algorithm)),
            render_algorithm(&rows, waivers),
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
        "| Verdict | Meaning |\n\
| --- | --- |\n\
| `✅` | Exact or passing without high-error outliers. |\n\
| `🟢` | Rounding-only differences. |\n\
| `⚖️` | Every limit passes; high-error outliers remain and were reviewed and accepted for the stated bounds and reason. |\n\
| `⚠️` | Every limit passes; high-error outliers remain and are not yet explained. |\n\
| `❌` | At least one limit failed. |\n\n\
- Limits: MAE <= {:.6}, P99 <= {}, high-error fraction <= {:.6} above {} codes.\n\
- Kinds: `exact` requires equality; `rounding` permits one-code upward rounding differences; `robust` applies the numeric limits.\n\
- A row is `native` only when it uses the input's original CFA layout and crop `[0, 0]`. Other patterns use the smallest valid one-pixel shift and are consistency checks, not independent tests.\n\
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

fn render_summary(report: &Report, waivers: &[Waiver]) -> String {
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
        let worst = rows
            .iter()
            .map(|row| status::evaluate(row, waivers))
            .max()
            .unwrap_or(Status::Fail);
        let native = rows
            .iter()
            .filter(|row| row.native)
            .map(|row| {
                format!(
                    "{} {}",
                    row.pattern,
                    status::evaluate(row, waivers).symbol()
                )
            })
            .collect::<Vec<_>>()
            .join(", ");
        let native = if native.is_empty() {
            "not selected".to_owned()
        } else {
            native
        };
        output.push_str(&format!(
            "| {} | {} | {} | {} |\n",
            crate::cases::display_name(algorithm),
            reference,
            worst.symbol(),
            native,
        ));
    }
    output.trim_end().to_owned()
}

fn render_algorithm(rows: &[&Row], waivers: &[Waiver]) -> String {
    let mut output = String::from(
        "| Reference | Pattern | Crop | Native | Verdict | MAE | RMSE | Max | Mean signed | P99 | P99.9 | >1024 (border) | Changed share |\n\
| --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | ---: | ---: | ---: |\n",
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
            "| {} | {} | [{}, {}] | {} | {} | {:.6} | {:.6} | {} | {:.6} | {} | {} | {} ({}) | {:.6} |\n",
            reference,
            row.pattern,
            row.crop[0],
            row.crop[1],
            if row.native { "yes" } else { "no" },
            status::evaluate(row, waivers).symbol(),
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

fn render_waiver(waiver: &Waiver) -> String {
    let pattern = waiver.pattern.map_or_else(
        || "all patterns".to_owned(),
        |pattern| format!("pattern `{pattern}`"),
    );
    let mut scope = pattern;
    if let Some(input) = waiver.input {
        scope.push_str(&format!(", input `{input}`"));
    }
    format!(
        "**Accepted outliers:** {} vs {}, {scope}: at most {} samples above 1024, maximum error at most {}. Reason: {} Evidence: {}",
        waiver.algorithm,
        waiver.reference.library(),
        waiver.max_count_gt1024,
        waiver.max_error,
        waiver.reason,
        waiver.evidence,
    )
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

fn new_section(algorithm: &str, table: &str, rows: &[&Row], waivers: &[Waiver]) -> String {
    let references = rows
        .iter()
        .map(|row| row.reference.as_str())
        .collect::<std::collections::BTreeSet<_>>();
    let notes = references
        .iter()
        .map(|reference| {
            let waiver = waivers.iter().find(|waiver| {
                waiver.algorithm.eq_ignore_ascii_case(algorithm)
                    && waiver.reference.library().eq_ignore_ascii_case(reference)
            });
            stub_note(reference_display_name(reference), waiver)
        })
        .collect::<Vec<_>>()
        .join("\n\n");
    format!(
        "## {}\n\n<!-- compare-demosaic:begin algo:{} -->\n{}\n<!-- compare-demosaic:end algo:{} -->\n\n{}",
        crate::cases::display_name(algorithm),
        algorithm_id(algorithm),
        table,
        algorithm_id(algorithm),
        notes,
    )
}

const fn reference_display_name(reference: &str) -> &str {
    if reference.eq_ignore_ascii_case("libraw") {
        "LibRaw"
    } else if reference.eq_ignore_ascii_case("darktable") {
        "darktable"
    } else if reference.eq_ignore_ascii_case("rawtherapee") {
        "RawTherapee"
    } else {
        reference
    }
}

fn stub_note(reference: &str, waiver: Option<&Waiver>) -> String {
    let waiver = waiver.map_or_else(String::new, |waiver| {
        format!("\n\n{}", render_waiver(waiver))
    });
    format!(
        "### {reference} `TODO`\n\
         \n\
         **Why this reference:** TODO\n\
         \n\
         **Why we match:** TODO\n\
         \n\
         **Known differences:** TODO{waiver}\n"
    )
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

fn check_note(
    document: &str,
    algorithm: &str,
    region_id: &str,
    waivers: &[Waiver],
    issues: &mut Vec<String>,
) {
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
    let sections = note_sections(note);
    if sections.is_empty() {
        issues.push(format!("{algorithm} is missing its note block"));
        return;
    }

    let mut documented = std::collections::BTreeMap::new();
    for (index, (heading, block)) in sections.iter().enumerate() {
        let section_number = index + 1;
        let Some(reference) = note_heading_reference(heading) else {
            issues.push(format!(
                "{algorithm} note section {section_number} has an invalid reference heading"
            ));
            continue;
        };
        if documented.insert(reference.clone(), *block).is_some() {
            issues.push(format!(
                "{algorithm} has multiple note sections for reference {reference}"
            ));
        }
        if block.contains("TODO") {
            issues.push(format!(
                "{algorithm} note section for {reference} still contains TODO"
            ));
        }
        for field in NOTE_FIELDS {
            let prefix = format!("**{field}:**");
            let value = block
                .lines()
                .find_map(|line| line.strip_prefix(&prefix).map(str::trim));
            match value {
                Some(value) if !value.is_empty() => {}
                _ => issues.push(format!(
                    "{algorithm} note section for {reference} is missing `{field}`"
                )),
            }
        }
        if let Some(open) = block
            .lines()
            .find_map(|line| line.strip_prefix("**Open issues:**"))
            && open.trim().is_empty()
        {
            issues.push(format!(
                "{algorithm} note section for {reference} has an empty `Open issues` field"
            ));
        }
    }

    let expected = reference_names_in_region(body);
    for reference in expected
        .iter()
        .filter(|reference| !documented.contains_key(*reference))
    {
        issues.push(format!(
            "{algorithm} is missing a separate reference section for {reference}"
        ));
    }
    for reference in documented
        .keys()
        .filter(|reference| !expected.contains(*reference))
    {
        issues.push(format!(
            "{algorithm} has a reference section for unexpected reference {reference}"
        ));
    }

    for waiver in waivers
        .iter()
        .filter(|waiver| waiver.algorithm.eq_ignore_ascii_case(algorithm))
    {
        let reference = waiver.reference.library().to_ascii_lowercase();
        let expected_waiver = render_waiver(waiver);
        let matching = documented
            .get(&reference)
            .map_or(0, |section| section.matches(&expected_waiver).count());
        if matching != 1 {
            issues.push(format!(
                "{algorithm} {reference} Accepted outliers must appear exactly once under its reference heading"
            ));
        }
    }
}

fn note_sections(note: &str) -> Vec<(&str, &str)> {
    let mut sections = Vec::new();
    let mut heading = None;
    let mut start = 0;
    let mut cursor = 0;
    for line in note.split_inclusive('\n') {
        if let Some(value) = line.strip_prefix("### ") {
            if let Some(previous) = heading {
                sections.push((previous, note[start..cursor].trim()));
            }
            heading = Some(value.trim_end());
            start = cursor + line.len();
        }
        cursor += line.len();
    }
    if let Some(heading) = heading {
        sections.push((heading, note[start..].trim()));
    }
    sections
}

fn reference_names_in_region(body: &str) -> std::collections::BTreeSet<String> {
    body.lines()
        .filter_map(|line| {
            let row = line.trim_start().strip_prefix("| [")?;
            let end = row.find(']')?;
            Some(row[..end].to_ascii_lowercase())
        })
        .collect()
}

fn note_heading_reference(heading: &str) -> Option<String> {
    let mut parts = heading.split_whitespace();
    let reference = parts.next()?;
    let function = parts.next()?;
    if !function.starts_with('`') || !function.ends_with('`') {
        return None;
    }
    Some(reference.trim_matches('`').to_ascii_lowercase())
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
    use crate::cases::ReferenceBackend;
    use crate::report::{
        Channels, Kind, Limits, Meta, References, Selection, Sites, Verdict, WorstSample,
    };

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
            crop: [0, 0],
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
                    rawtherapee: "5.13".to_owned(),
                },
                limits: Limits {
                    mae: 2.0,
                    p99: 64,
                    high_err: 1024,
                    high_err_frac: 0.001,
                },
                border_px: 0,
                pattern_source: crate::report::PatternSource::ShiftedCrops,
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

    fn waiver() -> Waiver {
        Waiver {
            algorithm: "ahd",
            reference: ReferenceBackend::LibRaw,
            pattern: None,
            input: None,
            max_count_gt1024: 1,
            max_error: 1_500,
            reason: "Near-tie direction decisions differ at one high-contrast edge.",
            evidence: "The stage diff first diverges in the direction map.",
        }
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
    fn write_creates_one_stub_note_per_reference() -> mischief::Result<()> {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("README.md");
        std::fs::write(&path, shell())?;
        let mut report = report("ahd");
        let mut rawtherapee = report.rows[0].clone();
        rawtherapee.reference = "rawtherapee".to_owned();
        rawtherapee.pattern = "GBRG".to_owned();
        rawtherapee.native = false;
        report.rows.push(rawtherapee);
        report = Report::new(report.meta.clone(), report.rows);

        write(&report, "usage", Some(&path))?;
        let document = std::fs::read_to_string(&path)?;
        assert!(!document.contains("> [!NOTE]"));
        assert_eq!(document.matches("### LibRaw `TODO`").count(), 1);
        assert_eq!(document.matches("### RawTherapee `TODO`").count(), 1);
        Ok(())
    }

    #[test]
    fn write_is_idempotent_with_waivers_and_renders_their_bounds() -> mischief::Result<()> {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("README.md");
        std::fs::write(&path, shell())?;
        let mut report = report("ahd");
        report.rows[0].all.count_gt1024 = 1;
        report.rows[0].all.max = 1_500;
        let waivers = [waiver()];

        let first_result = write_with_waivers(&report, "usage", &waivers, Some(&path))?;
        assert!(first_result.issues.is_empty(), "{}", first_result.render());
        let first = std::fs::read_to_string(&path)?;
        let table = region_body(&first, "algo:ahd").expect("generated algorithm table");
        write_with_waivers(&report, "usage", &waivers, Some(&path))?;
        assert_eq!(std::fs::read_to_string(&path)?, first);
        assert!(first.contains("| ⚖️ |"));
        assert!(!table.contains("Accepted outliers"));
        assert!(first.contains(
            "**Accepted outliers:** ahd vs LibRaw, all patterns: at most 1 samples above 1024, maximum error at most 1500."
        ));
        assert!(first.contains("Reason: Near-tie direction decisions differ"));
        assert!(first.contains("Evidence: The stage diff first diverges"));
        assert!(
            first
                .find("### LibRaw `TODO`")
                .expect("LibRaw note section")
                < first.find("Accepted outliers").expect("LibRaw waiver")
        );
        for (line_number, line) in first.lines().enumerate() {
            if ["✅", "🟢", "⚖️", "⚠️", "❌"]
                .iter()
                .any(|symbol| line.contains(symbol))
            {
                assert!(
                    line.trim_start().starts_with('|'),
                    "README symbol outside a table on line {}: {line}",
                    line_number + 1
                );
            }
        }

        let checked = check_with_waivers(&report, "usage", &waivers, Some(&path))?;
        assert!(
            checked
                .issues
                .iter()
                .all(|issue| !issue.contains("waiver for ahd"))
        );

        report.rows[0].all.max = 1_501;
        let exceeded = write_with_waivers(&report, "usage", &waivers, Some(&path))?;
        assert_eq!(exceeded.exit_code(), 1);
        assert!(
            exceeded
                .issues
                .iter()
                .any(|issue| issue.contains("is exceeded"))
        );
        assert!(std::fs::read_to_string(&path)?.contains("| ⚠️ |"));
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
            "**Known differences:** TODO",
            "**Known differences:** expected",
        );
        text = text.replace("**Why this reference:** TODO\n", "");
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
    fn check_requires_one_section_per_reference() {
        let id = "algo:ahd";
        let document = format!(
            "{}\n\
             | [libraw](https://example.com/libraw) | row |\n\
             | [rawtherapee](https://example.com/rawtherapee) | row |\n\
             {}\n\
             \n\
             ### LibRaw `ahd_interpolate`\n\
             \n\
             **Why this reference:** explanatory.\n\
             \n\
             **Why we match:** explanatory.\n\
             \n\
             **Known differences:** explanatory.\n",
            begin_marker(id),
            end_marker(id),
        );
        let mut issues = Vec::new();
        check_note(&document, "ahd", id, &[], &mut issues);
        assert!(
            issues.iter().any(|issue| issue.contains("rawtherapee")),
            "a combined note must not satisfy multiple references: {issues:?}"
        );
    }

    #[test]
    fn check_requires_waiver_under_matching_reference() {
        let id = "algo:ahd";
        let fields = "**Why this reference:** explanatory.\n\
                      \n\
                      **Why we match:** explanatory.\n\
                      \n\
                      **Known differences:** explanatory.\n";
        let document = format!(
            "{}\n\
             | [libraw](https://example.com/libraw) | row |\n\
             {}\n\
             \n\
             ### LibRaw `ahd_interpolate`\n\
             \n\
             {fields}",
            begin_marker(id),
            end_marker(id),
        );
        let waivers = [waiver()];
        let mut issues = Vec::new();
        check_note(&document, "ahd", id, &waivers, &mut issues);
        assert!(
            issues
                .iter()
                .any(|issue| issue.contains("Accepted outliers")),
            "a waiver must be documented under its reference section: {issues:?}"
        );

        let document = format!("{document}\n{}", render_waiver(&waiver()));
        issues.clear();
        check_note(&document, "ahd", id, &waivers, &mut issues);
        assert!(issues.is_empty(), "{issues:?}");
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
