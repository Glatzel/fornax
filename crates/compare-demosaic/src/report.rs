use fornax_core::BayerPattern;

use crate::case::{Case, SampleType};
use crate::{cases, data, ffi, metrics};

#[cfg(test)]
pub const LIBRAW_COMMIT: &str = env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT");
pub const DARKTABLE_RELEASE: &str = env!("COMPARE_DEMOSAIC_DARKTABLE_RELEASE");
pub const DARKTABLE_COMMIT: &str = env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT");
pub const LIBRAW_VERSION: &str = env!("COMPARE_DEMOSAIC_LIBRAW_VERSION");
pub const TABLE_BEGIN: &str = "<!-- BEGIN:compare-table -->";
pub const TABLE_END: &str = "<!-- END:compare-table -->";

#[derive(Clone, Debug)]
pub struct Row {
    pub case: &'static str,
    pub reference: &'static str,
    pub source_url: &'static str,
    pub pattern: BayerPattern,
    pub metrics: metrics::Metrics,
    pub verdict: metrics::Verdict,
}

pub fn compare(
    case: &'static Case,
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Row> {
    let reference = ffi::run(
        case.reference_backend,
        case.algorithm,
        codes,
        width,
        height,
        ffi::libraw_filters(pattern),
    )?;
    let actual = (case.run_fornax)(codes, width, height, pattern, sample_type)?;
    let metrics = metrics::calculate(&actual, &reference, width, height, case.border)?;
    Ok(Row {
        case: case.name,
        reference: case.reference_backend.library(),
        source_url: case.source_url,
        pattern,
        metrics,
        verdict: metrics.verdict(case.tolerance),
    })
}

pub fn canonical_rows() -> mischief::Result<Vec<Row>> {
    let input = data::canonical();
    let mut rows = Vec::with_capacity(cases::CASES.len() * BayerPattern::ALL.len());
    for case in cases::CASES {
        for pattern in BayerPattern::ALL {
            rows.push(compare(
                case,
                &input.codes,
                input.width,
                input.height,
                pattern,
                SampleType::F32,
            )?);
        }
    }
    Ok(rows)
}

pub fn render_tables(rows: &[Row]) -> String {
    let mut algorithms = rows.iter().map(|row| row.case).collect::<Vec<_>>();
    algorithms.sort_unstable_by_key(|name| name.to_ascii_lowercase());
    algorithms.dedup();

    let mut output = String::from(
        "> [!NOTE]\n\
         > `⚠️`: every sample is within the case's declared maximum error.\n",
    );
    for algorithm in algorithms {
        output.push('\n');
        output.push_str(&format!("### {algorithm}\n\n"));
        output.push_str(
            "| Reference Library | Bayer | Verdict | MAE | RMSE | Max | PSNR | Changed |\n\
             | --- | --- | --- | ---: | ---: | ---: | ---: | ---: |\n",
        );

        let mut algorithm_rows = rows
            .iter()
            .filter(|row| row.case == algorithm)
            .collect::<Vec<_>>();
        algorithm_rows.sort_unstable_by_key(|row| {
            (row.reference.to_ascii_lowercase(), row.pattern.to_string())
        });
        for row in algorithm_rows {
            let psnr = if row.metrics.peak_signal_to_noise_ratio.is_infinite() {
                "∞".to_owned()
            } else {
                format!("{:.2}", row.metrics.peak_signal_to_noise_ratio)
            };
            output.push_str(&format!(
                "| [{}]({}) | {} | {} | {:.3} | {:.3} | {} | {} | {:.3}% |\n",
                row.reference,
                row.source_url,
                row.pattern,
                verdict(row.verdict),
                row.metrics.mean_absolute_error,
                row.metrics.root_mean_square_error,
                row.metrics.maximum_error,
                psnr,
                row.metrics.changed_percent,
            ));
        }
    }
    output
}

pub fn update_table(readme: &str, rows: &[Row]) -> mischief::Result<String> {
    if readme.matches(TABLE_BEGIN).count() != 1 || readme.matches(TABLE_END).count() != 1 {
        return Err(mischief::mischief!(
            "README must contain exactly one comparison table marker pair"
        ));
    }
    let begin = readme
        .find(TABLE_BEGIN)
        .ok_or_else(|| mischief::mischief!("README table begin marker is missing"))?;
    let end = readme
        .find(TABLE_END)
        .ok_or_else(|| mischief::mischief!("README table end marker is missing"))?;
    if end < begin {
        return Err(mischief::mischief!(
            "README comparison table markers are out of order"
        ));
    }
    let line_ending = if readme.contains("\r\n") {
        "\r\n"
    } else {
        "\n"
    };
    let table = render_tables(rows)
        .trim_end_matches('\n')
        .replace('\n', line_ending);
    let content_start = begin + TABLE_BEGIN.len();
    let replacement = format!(
        "{line_ending}{table}{line_ending}{line_ending}Reference builds: LibRaw {LIBRAW_VERSION}; \
         darktable {DARKTABLE_RELEASE} ({DARKTABLE_COMMIT}).{line_ending}"
    );
    Ok(format!(
        "{}{replacement}{}",
        &readme[..content_start],
        &readme[end..],
    ))
}

const fn verdict(verdict: metrics::Verdict) -> &'static str {
    match verdict {
        metrics::Verdict::Exact => "✅",
        metrics::Verdict::RoundingOnly => "🟢",
        metrics::Verdict::WithinTolerance(_) => "⚠️",
        metrics::Verdict::Differs => "❌",
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn table_generation_preserves_crlf_line_endings() -> mischief::Result<()> {
        let readme = concat!(
            "# Test\r\n",
            "<!-- BEGIN:compare-table -->\r\n",
            "stale\r\n",
            "<!-- END:compare-table -->\r\n",
            "footer\r\n",
        );
        let updated = update_table(readme, &[])?;
        assert!(!updated.replace("\r\n", "").contains('\n'));
        assert!(updated.contains("Reference builds: LibRaw"));
        assert!(updated.ends_with("footer\r\n"));
        Ok(())
    }

    #[test]
    fn table_uses_compact_headers_and_emoji_only_verdicts() {
        let rows = canonical_rows().expect("canonical comparison rows");
        let table = render_tables(&rows);
        let algorithms = rows
            .iter()
            .map(|row| row.case)
            .collect::<std::collections::BTreeSet<_>>();
        assert!(table.contains("| Reference Library |"));
        assert!(table.contains("| Verdict |"));
        assert_eq!(
            table.matches("| Reference Library |").count(),
            algorithms.len()
        );
        assert!(!table.contains("| Fornax |"));
        assert!(!table.contains("Reference algorithm"));
        assert!(!table.contains("Source |"));
        assert_eq!(verdict(metrics::Verdict::Exact), "✅");
        assert_eq!(verdict(metrics::Verdict::RoundingOnly), "🟢");
        assert_eq!(verdict(metrics::Verdict::WithinTolerance(1)), "⚠️");
        assert_eq!(verdict(metrics::Verdict::Differs), "❌");
    }

    #[test]
    fn reference_cells_only_show_library_names() {
        let rows = canonical_rows().expect("canonical comparison rows");
        let table = render_tables(&rows);
        assert!(table.contains("| [LibRaw]("));
        assert!(table.contains("| [darktable]("));
        assert!(!table.contains("`LibRaw::"));
        assert!(!table.contains("demosaic_ppg"));
    }

    #[test]
    fn algorithm_tables_are_alphabetical_and_use_github_note_syntax() {
        let rows = canonical_rows().expect("canonical comparison rows");
        let table = render_tables(&rows);
        let linear = table.find("### Linear").expect("Linear table");
        let ppg = table.find("### PPG").expect("PPG table");
        let rcd = table.find("### RCD").expect("RCD table");
        assert!(linear < ppg && ppg < rcd);
        assert!(table.starts_with("> [!NOTE]\n"));
        assert!(
            table.contains("> `⚠️`: every sample is within the case's declared maximum error.")
        );
    }
}
