use std::collections::BTreeSet;
use std::path::{Path, PathBuf};

use comfy_table::presets::ASCII_MARKDOWN;
use comfy_table::{Cell, CellAlignment, Table};

use crate::results::{ComparisonResult, ComparisonResults, DEFAULT_RESULTS_FILE, VerdictResult};

pub const TABLE_BEGIN: &str = "<!-- BEGIN:compare-table -->";
pub const TABLE_END: &str = "<!-- END:compare-table -->";

pub fn render_comparison_block(results: &ComparisonResults) -> String {
    let algorithms = results
        .comparisons
        .iter()
        .map(|comparison| {
            (
                comparison.algorithm.to_ascii_lowercase(),
                comparison.algorithm.as_str(),
            )
        })
        .collect::<BTreeSet<_>>();

    let mut block = String::from(TABLE_BEGIN);
    block.push('\n');
    for (_, algorithm) in algorithms {
        let mut comparisons = results
            .comparisons
            .iter()
            .filter(|comparison| comparison.algorithm == algorithm)
            .collect::<Vec<_>>();
        comparisons.sort_unstable_by_key(|comparison| {
            (
                comparison.reference_library.to_ascii_lowercase(),
                comparison.reference_library.as_str(),
            )
        });

        let mut table = Table::new();
        table.load_style(ASCII_MARKDOWN);
        table.set_header([
            "Reference Library",
            "Bayer",
            "Verdict",
            "MAE",
            "RMSE",
            "P99",
            ">1024",
        ]);
        for comparison in comparisons {
            table.add_row(markdown_row(comparison));
        }

        block.push_str(&format!("\n### {algorithm}\n\n{table}\n"));
    }
    block.push_str(TABLE_END);
    block
}

fn markdown_row(comparison: &ComparisonResult) -> Vec<Cell> {
    let metrics = &comparison.metrics;
    vec![
        Cell::new(format!(
            "[{}]({})",
            comparison.reference_library, comparison.source_url
        )),
        Cell::new(&comparison.bayer_pattern),
        Cell::new(verdict_mark(comparison.verdict)),
        right_cell(format!("{:.3}", metrics.mean_absolute_error)),
        right_cell(format!("{:.3}", metrics.root_mean_square_error)),
        right_cell(metrics.p99_error.to_string()),
        right_cell(format!("{:.3}%", metrics.over_1024_percent)),
    ]
}

fn right_cell(content: String) -> Cell { Cell::new(content).set_alignment(CellAlignment::Right) }

const fn verdict_mark(verdict: VerdictResult) -> &'static str {
    match verdict {
        VerdictResult::Exact => "✅",
        VerdictResult::RoundingOnly => "🟢",
        VerdictResult::WithinTolerance { .. } | VerdictResult::WithinRobustTolerance => "⚠️",
        VerdictResult::Differs => "❌",
    }
}

pub fn replace_comparison_block(readme: &str, block: &str) -> mischief::Result<String> {
    let begin_count = readme.matches(TABLE_BEGIN).count();
    let end_count = readme.matches(TABLE_END).count();
    if begin_count != 1 || end_count != 1 {
        return Err(mischief::mischief!(
            "README must contain exactly one comparison block, found {begin_count} begin and {end_count} end markers"
        ));
    }
    let start = readme.find(TABLE_BEGIN).expect("begin marker was counted");
    let end = readme.find(TABLE_END).expect("end marker was counted") + TABLE_END.len();
    if end <= start {
        return Err(mischief::mischief!(
            "README comparison end marker precedes its begin marker"
        ));
    }

    let mut updated = String::with_capacity(readme.len() + block.len());
    updated.push_str(&readme[..start]);
    updated.push_str(block);
    updated.push_str(&readme[end..]);
    Ok(updated)
}

pub fn update_readme(results_path: &Path, readme_path: &Path) -> mischief::Result<String> {
    let json = std::fs::read_to_string(results_path)?;
    let results = serde_json::from_str::<ComparisonResults>(&json)
        .map_err(|error| mischief::mischief!("cannot read {}: {error}", results_path.display()))?;
    let readme = std::fs::read_to_string(readme_path)?;
    let updated = replace_comparison_block(&readme, &render_comparison_block(&results))?;
    std::fs::write(readme_path, &updated)?;
    Ok(updated)
}

pub fn update_default_readme() -> mischief::Result<PathBuf> {
    let manifest = PathBuf::from(env!("CARGO_MANIFEST_DIR"));
    let results = manifest.join(DEFAULT_RESULTS_FILE);
    let readme = manifest.join("README.md");
    update_readme(&results, &readme)?;
    Ok(readme)
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::results::{ComparisonInput, MetricsResult, ReferenceBuilds, ToleranceResult};

    fn comparison(algorithm: &str, reference: &str, verdict: VerdictResult) -> ComparisonResult {
        ComparisonResult {
            algorithm: algorithm.to_owned(),
            reference_library: reference.to_owned(),
            source_url: format!("https://example.com/{reference}"),
            bayer_pattern: "GBRG".to_owned(),
            border: 4,
            tolerance: ToleranceResult::Robust {
                max_mae: 2.0,
                max_p99: 64,
                max_over_1024_percent: 0.1,
            },
            verdict,
            metrics: MetricsResult {
                mean_absolute_error: 1.25,
                root_mean_square_error: 2.5,
                maximum_error: 8,
                changed_percent: 10.0,
                psnr_db: Some(80.0),
                below_reference: 4,
                p95_error: 2,
                p99_error: 4,
                p99_9_error: 6,
                over_64_percent: 0.0,
                over_1024_percent: 0.125,
                channel_mae: [1.0, 1.5, 1.25],
                luminance_rmse: 1.75,
                chroma_rmse: 3.5,
            },
        }
    }

    fn results() -> ComparisonResults {
        ComparisonResults {
            schema_version: 3,
            input: ComparisonInput {
                fixture: "fixture".to_owned(),
                source: "fixture.raw".to_owned(),
                pattern: "GBRG".to_owned(),
                width: 10,
                height: 10,
                sample_type: "f32".to_owned(),
            },
            reference_builds: ReferenceBuilds {
                libraw_version: "1".to_owned(),
                libraw_commit: "libraw".to_owned(),
                darktable_release: "2".to_owned(),
                darktable_commit: "darktable".to_owned(),
            },
            comparisons: vec![
                comparison("VNG", "LibRaw", VerdictResult::Differs),
                comparison("AHD", "LibRaw", VerdictResult::WithinRobustTolerance),
                comparison("AHD", "darktable", VerdictResult::Exact),
            ],
        }
    }

    #[test]
    fn rendering_is_deterministic_markdown() {
        let block = render_comparison_block(&results());
        assert_eq!(block, render_comparison_block(&results()));
        assert!(block.starts_with(TABLE_BEGIN));
        assert!(block.ends_with(TABLE_END));
        assert!(block.contains("| Reference Library"), "{block}");
        assert!(block.contains("| Bayer"), "{block}");
        assert!(block.contains("| Verdict"), "{block}");
        assert!(block.contains("| MAE"), "{block}");
        assert!(
            block.contains("[darktable](https://example.com/darktable)") && block.contains("✅"),
            "{block}"
        );
        assert!(
            block.contains("[LibRaw](https://example.com/LibRaw)") && block.contains("⚠️"),
            "{block}"
        );

        let ahd = block.find("### AHD").expect("AHD table");
        let vng = block.find("### VNG").expect("VNG table");
        assert!(ahd < vng);
        let darktable = block
            .find("https://example.com/darktable")
            .expect("darktable row");
        let libraw = block
            .find("https://example.com/LibRaw")
            .expect("LibRaw row");
        assert!(darktable < libraw);
    }

    #[test]
    fn replacement_changes_only_the_marked_block() {
        let readme = "before\n<!-- BEGIN:compare-table -->old<!-- END:compare-table -->\nafter\n";
        let updated = replace_comparison_block(
            readme,
            "<!-- BEGIN:compare-table -->new<!-- END:compare-table -->",
        )
        .expect("markers are valid");
        assert_eq!(
            updated,
            "before\n<!-- BEGIN:compare-table -->new<!-- END:compare-table -->\nafter\n"
        );
    }

    #[test]
    fn replacement_rejects_missing_or_duplicate_markers() {
        for readme in [
            "no markers",
            "<!-- BEGIN:compare-table --><!-- BEGIN:compare-table --><!-- END:compare-table -->",
            "<!-- BEGIN:compare-table -->",
        ] {
            assert!(replace_comparison_block(readme, "replacement").is_err());
        }
    }
}
