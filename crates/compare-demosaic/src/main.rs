mod case;
mod cases;
mod data;
mod ffi;
mod metrics;
mod report;
mod results;

use std::path::{Path, PathBuf};

use results::{ComparisonResults, DEFAULT_RESULTS_FILE};

fn main() -> mischief::Result<()> {
    match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => write_results(None),
        [argument] if argument == "--stdout" => {
            print!("{}", ComparisonResults::canonical()?.to_pretty_json()?);
            Ok(())
        }
        [argument, path] if argument == "--output" => write_results(Some(Path::new(path))),
        _ => Err(mischief::mischief!(
            "usage: compare-demosaic [--stdout | --output PATH]"
        )),
    }
}

fn write_results(path: Option<&Path>) -> mischief::Result<()> {
    let default = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_RESULTS_FILE);
    let path = path.unwrap_or(&default);
    ComparisonResults::canonical()?.write(path)?;
    println!("wrote {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use std::sync::LazyLock;

    use fornax_core::BayerPattern;
    use rstest::rstest;

    use super::*;
    use crate::case::{ReferenceBackend, SampleType};
    use crate::cases::CASES;

    static TEST_INPUTS: LazyLock<Vec<data::Input>> = LazyLock::new(data::test_inputs);

    #[rstest]
    fn every_case_matches_for_all_patterns_precisions_and_inputs(
        #[values(0, 1, 2, 3)] case_index: usize,
        #[values(
            BayerPattern::RGGB,
            BayerPattern::BGGR,
            BayerPattern::GRBG,
            BayerPattern::GBRG
        )]
        pattern: BayerPattern,
        #[values(SampleType::F32, SampleType::F64)] sample_type: SampleType,
        #[values(
            "canonical",
            "odd",
            "partial-row-chunk",
            "tiny",
            "saturated",
            "saturated-tiny"
        )]
        input_label: &str,
    ) -> mischief::Result<()> {
        let case = &CASES[case_index];
        let input = TEST_INPUTS
            .iter()
            .find(|input| input.label == input_label)
            .expect("test input");
        if case.border.saturating_mul(2) >= input.width.min(input.height) {
            return Ok(());
        }
        let row = report::compare(
            case,
            &input.codes,
            input.width,
            input.height,
            pattern,
            sample_type,
        )?;
        assert_ne!(
            row.verdict,
            metrics::Verdict::Differs,
            "{} / {pattern} / {sample_type:?} / {} ({})",
            case.name,
            input.label,
            row.metrics.summary()
        );
        Ok(())
    }

    #[test]
    fn algorithm_ids_are_unique_and_unknown_ids_are_rejected() -> mischief::Result<()> {
        let mut ids = CASES
            .iter()
            .map(|case| (case.reference_backend, case.algorithm))
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), CASES.len());

        let input = data::canonical();
        let error = ffi::run(
            ReferenceBackend::LibRaw,
            u32::MAX,
            &input.codes,
            input.width,
            input.height,
            ffi::libraw_filters(BayerPattern::RGGB),
        )
        .expect_err("unknown algorithm must fail");
        assert!(
            error
                .to_string()
                .contains("reference returned unknown algorithm")
        );
        Ok(())
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn canonical_results_artifact_is_current() -> mischief::Result<()> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_RESULTS_FILE);
        let stored = std::fs::read_to_string(path)?;
        let expected = ComparisonResults::canonical()?;
        assert_eq!(
            normalize_line_endings(&stored),
            normalize_line_endings(&expected.to_pretty_json()?),
            "comparison results are stale; run `pixi run -e dev compare-demosaic`"
        );
        let parsed = serde_json::from_str::<ComparisonResults>(&stored).unwrap();
        assert_eq!(parsed.schema_version, expected.schema_version);
        assert_eq!(parsed.comparisons.len(), expected.comparisons.len());
        assert_eq!(
            parsed.comparisons[0].algorithm,
            expected.comparisons[0].algorithm
        );
        Ok(())
    }

    #[test]
    fn result_freshness_ignores_platform_line_endings() {
        let lf = "{\n  \"schema_version\": 1\n}\n";
        let crlf = lf.replace('\n', "\r\n");
        assert_eq!(normalize_line_endings(lf), normalize_line_endings(&crlf));
    }

    #[cfg(target_os = "linux")]
    #[test]
    fn readme_tables_match_results_and_warning_notes() -> mischief::Result<()> {
        let readme = include_str!("../README.md");
        let results = ComparisonResults::canonical()?;
        assert_eq!(readme.matches(report::TABLE_BEGIN).count(), 1);
        assert_eq!(readme.matches(report::TABLE_END).count(), 1);
        let algorithms = results
            .comparisons
            .iter()
            .map(|result| result.algorithm.as_str())
            .collect::<std::collections::BTreeSet<_>>();
        let mut positions = Vec::new();
        for algorithm in &algorithms {
            let heading = format!("### {algorithm}");
            let position = readme
                .find(&heading)
                .unwrap_or_else(|| panic!("missing README table for {algorithm}"));
            positions.push(position);
            let next = readme[position + heading.len()..]
                .find("\n### ")
                .map_or(readme.len(), |offset| position + heading.len() + offset);
            let section = &readme[position..next];

            for result in results
                .comparisons
                .iter()
                .filter(|result| result.algorithm == *algorithm)
            {
                assert!(
                    section.contains(&expected_markdown_row(result)),
                    "README row is stale for {algorithm}: {result:?}"
                );
            }

            let has_warning = results.comparisons.iter().any(|result| {
                result.algorithm == *algorithm
                    && matches!(
                        result.verdict,
                        results::VerdictResult::WithinTolerance { .. }
                    )
            });
            assert_eq!(
                section.contains("> [!NOTE]"),
                has_warning,
                "warning note presence is wrong for {algorithm}"
            );
        }
        assert!(positions.windows(2).all(|pair| pair[0] < pair[1]));
        assert!(
            readme.contains("- `⚠️`: every sample is within the case's declared maximum error.")
        );
        assert!(readme.contains("Fornax clamps"));
        assert!(readme.contains("reconstructed RGB to `[0, 1]`"));
        Ok(())
    }

    #[cfg(target_os = "linux")]
    fn expected_markdown_row(result: &results::ComparisonResult) -> String {
        let verdict = match result.verdict {
            results::VerdictResult::Exact => "✅",
            results::VerdictResult::RoundingOnly => "🟢",
            results::VerdictResult::WithinTolerance { .. } => "⚠️",
            results::VerdictResult::Differs => "❌",
        };
        let psnr = result
            .metrics
            .psnr_db
            .map_or_else(|| "∞".to_owned(), |psnr| format!("{psnr:.2}"));
        format!(
            "| [{}]({}) | {} | {} | {:.3} | {:.3} | {} | {} | {:.3}% |",
            result.reference_library,
            result.source_url,
            result.bayer_pattern,
            verdict,
            result.metrics.mean_absolute_error,
            result.metrics.root_mean_square_error,
            result.metrics.maximum_error,
            psnr,
            result.metrics.changed_percent,
        )
    }

    fn normalize_line_endings(text: &str) -> String { text.replace("\r\n", "\n") }

    #[test]
    fn source_link_matches_packaged_version() {
        assert_eq!(report::LIBRAW_VERSION, "0.22.3");
        assert_eq!(report::DARKTABLE_RELEASE, "5.6.2");
        for case in CASES {
            let commit = match case.reference_backend {
                ReferenceBackend::LibRaw => report::LIBRAW_COMMIT,
                ReferenceBackend::Darktable => report::DARKTABLE_COMMIT,
            };
            assert!(case.source_url.contains(commit));
            assert!(case.source_url.contains("#L"));
        }
    }

    #[test]
    fn repeated_calls_leave_libraw_owned_by_its_context() -> mischief::Result<()> {
        let input = data::canonical();
        for _ in 0..16 {
            ffi::run(
                ReferenceBackend::LibRaw,
                cases::LINEAR.algorithm,
                &input.codes,
                input.width,
                input.height,
                ffi::libraw_filters(BayerPattern::RGGB),
            )?;
        }
        Ok(())
    }

    #[test]
    fn mismatched_mosaic_length_is_rejected() {
        let codes = [0_u16; 10];
        let error = ffi::run(
            ReferenceBackend::LibRaw,
            cases::LINEAR.algorithm,
            &codes,
            8,
            8,
            ffi::libraw_filters(BayerPattern::RGGB),
        )
        .expect_err("mismatched mosaic length must fail");
        assert!(error.to_string().contains("expected 64"));
    }

    #[rstest]
    fn saturated_input_is_exact(
        #[values(0, 1, 2, 3)] case_index: usize,
        #[values(
            BayerPattern::RGGB,
            BayerPattern::BGGR,
            BayerPattern::GRBG,
            BayerPattern::GBRG
        )]
        pattern: BayerPattern,
        #[values(SampleType::F32, SampleType::F64)] sample_type: SampleType,
    ) -> mischief::Result<()> {
        let case = &CASES[case_index];
        let input = TEST_INPUTS
            .iter()
            .find(|input| input.label == "saturated")
            .expect("saturated test input");
        if case.border.saturating_mul(2) >= input.width.min(input.height) {
            return Ok(());
        }
        let row = report::compare(
            case,
            &input.codes,
            input.width,
            input.height,
            pattern,
            sample_type,
        )?;
        assert_eq!(row.verdict, metrics::Verdict::Exact);
        Ok(())
    }
}
