mod case;
mod cases;
pub mod data;
mod ffi;
mod metrics;
mod report;
pub mod results;

pub use report::ProgressEvent;

pub const ALGORITHM_COUNT: usize = cases::CASES.len();

#[cfg(test)]
mod tests {
    use std::path::PathBuf;

    use fornax_core::BayerPattern;
    use rstest::{fixture, rstest};

    use super::*;
    use crate::case::{ReferenceBackend, SampleType};
    use crate::results::{
        CANONICAL_RESULTS_FILE, CANONICAL_SCHEMA_VERSION, ComparisonResults, DEFAULT_RESULTS_DIR,
        SUMMARY_RESULTS_FILE, SUMMARY_SCHEMA_VERSION, SummaryResults,
    };

    #[fixture]
    #[once]
    fn synthetic_512() -> data::Input {
        let input = data::synthetic_input();
        assert_eq!((input.width, input.height), (512, 512));
        input
    }

    #[rstest]
    fn every_algorithm_accepts_synthetic_512(
        synthetic_512: &data::Input,
        #[values(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)] case_index: usize,
    ) -> mischief::Result<()> {
        let case = &cases::CASES[case_index];
        let row = report::compare(case, synthetic_512, SampleType::F32)?;
        assert!(
            row.metrics.mean_absolute_error.is_finite(),
            "{} produced non-finite metrics",
            case.name
        );
        Ok(())
    }

    #[test]
    fn algorithm_ids_are_unique_and_unknown_ids_are_rejected() -> mischief::Result<()> {
        let mut ids = cases::CASES
            .iter()
            .map(|case| (case.reference_backend, case.algorithm))
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), cases::CASES.len());

        let input = data::synthetic_input();
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

    #[test]
    fn source_links_match_packaged_versions() {
        assert_eq!(report::LIBRAW_VERSION, "0.22.3");
        assert_eq!(report::DARKTABLE_RELEASE, "5.6.2");
        for case in cases::CASES {
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
        let input = data::synthetic_input();
        for _ in 0..4 {
            ffi::run(
                ReferenceBackend::LibRaw,
                cases::LINEAR.algorithm,
                &input.codes,
                input.width,
                input.height,
                ffi::libraw_filters(BayerPattern::GBRG),
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

    #[test]
    fn progress_events_wrap_each_comparison() -> mischief::Result<()> {
        let mut started = Vec::new();
        let mut finished = 0;
        let input = data::synthetic_input();
        let rows = report::canonical_rows(&input, |event| match event {
            ProgressEvent::Started {
                algorithm,
                reference,
            } => started.push((algorithm.to_owned(), reference.to_owned())),
            ProgressEvent::Finished => finished += 1,
        })?;
        assert_eq!(rows.len(), ALGORITHM_COUNT);
        assert_eq!(finished, rows.len());
        let expected = cases::CASES
            .iter()
            .map(|case| {
                (
                    case.name.to_owned(),
                    case.reference_backend.library().to_owned(),
                )
            })
            .collect::<Vec<_>>();
        assert_eq!(started, expected);
        Ok(())
    }

    #[test]
    fn canonical_and_summary_artifacts_are_consistent() -> mischief::Result<()> {
        let results_dir = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_RESULTS_DIR);
        let canonical = std::fs::read_to_string(results_dir.join(CANONICAL_RESULTS_FILE))?;
        let summary = std::fs::read_to_string(results_dir.join(SUMMARY_RESULTS_FILE))?;
        let canonical = serde_json::from_str::<ComparisonResults>(&canonical)?;
        let summary = serde_json::from_str::<SummaryResults>(&summary)?;

        assert_eq!(canonical.schema_version, CANONICAL_SCHEMA_VERSION);
        assert_eq!(summary.schema_version, SUMMARY_SCHEMA_VERSION);
        assert_eq!(canonical.input, summary.input);
        assert_eq!(canonical.reference_builds, summary.reference_builds);
        assert_eq!(canonical.comparisons.len(), ALGORITHM_COUNT);
        assert_eq!(canonical.comparisons.len(), summary.comparisons.len());
        Ok(())
    }
}
