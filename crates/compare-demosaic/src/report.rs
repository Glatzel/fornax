use fornax_core::BayerPattern;

use crate::case::{Case, SampleType};
use crate::{cases, data, ffi, metrics};

#[cfg(test)]
pub const LIBRAW_COMMIT: &str = env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT");
pub const DARKTABLE_COMMIT: &str = env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT");
pub const LIBRAW_VERSION: &str = env!("COMPARE_DEMOSAIC_LIBRAW_VERSION");
pub const TABLE_BEGIN: &str = "<!-- BEGIN:compare-table -->";
pub const TABLE_END: &str = "<!-- END:compare-table -->";

#[derive(Clone, Debug)]
pub struct Row {
    pub case: &'static str,
    pub reference: &'static str,
    pub source_name: &'static str,
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
        reference: case.reference,
        source_name: case.source_name,
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

pub fn render_table(rows: &[Row]) -> String {
    let mut output = String::from(
        "| Fornax algorithm | Reference algorithm | Source | Bayer pattern | Verdict | MAE (codes) | RMSE (codes) | Max difference (codes) | PSNR (dB) | Changed |\n\
         | --- | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: |\n",
    );
    for row in rows {
        let psnr = if row.metrics.peak_signal_to_noise_ratio.is_infinite() {
            "∞".to_owned()
        } else {
            format!("{:.2}", row.metrics.peak_signal_to_noise_ratio)
        };
        output.push_str(&format!(
            "| {} | `{}` | [`{}`]({}) | {} | {} | {:.3} | {:.3} | {} | {} | {:.3}% |\n",
            row.case,
            row.reference,
            row.source_name,
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
    let table = render_table(rows).replace('\n', line_ending);
    let content_start = begin + TABLE_BEGIN.len();
    let replacement = format!(
        "{line_ending}{table}{line_ending}Reference builds: LibRaw {LIBRAW_VERSION}; \
         darktable {DARKTABLE_COMMIT}.{line_ending}"
    );
    Ok(format!(
        "{}{replacement}{}",
        &readme[..content_start],
        &readme[end..],
    ))
}

const fn verdict(verdict: metrics::Verdict) -> &'static str {
    match verdict {
        metrics::Verdict::Exact => "✅ exact",
        metrics::Verdict::RoundingOnly => "✅ rounding only",
        metrics::Verdict::WithinTolerance(_) => "✅ within tolerance",
        metrics::Verdict::Differs => "❌ differs",
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
}
