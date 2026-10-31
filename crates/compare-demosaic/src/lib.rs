mod case;
mod cases;
pub mod data;
mod ffi;
mod metrics;
#[cfg(feature = "render-readme")]
pub mod readme;
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
    use crate::results::{ComparisonResults, DEFAULT_RESULTS_FILE};

    #[test]
    fn algorithm_ids_are_unique_and_unknown_ids_are_rejected() -> mischief::Result<()> {
        let mut ids = cases::CASES
            .iter()
            .map(|case| (case.reference_backend, case.algorithm))
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), cases::CASES.len());

        let input = data::test_inputs().remove(0);
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
    fn every_case_accepts_a_small_synthetic_mosaic() -> mischief::Result<()> {
        let input = data::seeded("small", 65, 65);
        for case in cases::CASES {
            let row = report::compare(
                case,
                &input.codes,
                input.width,
                input.height,
                BayerPattern::GBRG,
                SampleType::F32,
            )?;
            assert!(
                row.metrics.mean_absolute_error.is_finite(),
                "{} produced non-finite metrics",
                case.name
            );
        }
        Ok(())
    }

    #[fixture]
    #[once]
    fn cropped_real_input() -> data::Input {
        data::canonical_crop().expect("the pinned CR2 fixture must decode")
    }

    #[rstest]
    fn every_case_accepts_the_cropped_real_image(
        cropped_real_input: &data::Input,
        #[values(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)] case_index: usize,
    ) -> mischief::Result<()> {
        let case = &cases::CASES[case_index];
        let row = report::compare(
            case,
            &cropped_real_input.codes,
            cropped_real_input.width,
            cropped_real_input.height,
            cropped_real_input.pattern,
            SampleType::F32,
        )?;
        assert!(
            row.metrics.mean_absolute_error.is_finite(),
            "{} produced non-finite metrics",
            case.name
        );
        Ok(())
    }

    #[test]
    fn saturated_input_is_exact_for_linear_interpolation() -> mischief::Result<()> {
        let input = data::saturated("saturated", 65, 65);
        for pattern in BayerPattern::ALL {
            let row = report::compare(
                &cases::LINEAR,
                &input.codes,
                input.width,
                input.height,
                pattern,
                SampleType::F32,
            )?;
            assert_eq!(row.verdict, metrics::Verdict::Exact);
        }
        Ok(())
    }

    #[test]
    fn repeated_calls_leave_libraw_owned_by_its_context() -> mischief::Result<()> {
        let input = data::seeded("repeat", 65, 65);
        for _ in 0..16 {
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

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[test]
    fn canonical_artifact_describes_the_full_fixture_and_all_algorithms() -> mischief::Result<()> {
        let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join(DEFAULT_RESULTS_FILE);
        let stored = std::fs::read_to_string(path)?;
        let stored = serde_json::from_str::<ComparisonResults>(&stored)?;
        assert_eq!(stored.schema_version, crate::results::SCHEMA_VERSION);
        assert_eq!(stored.input.fixture, "colorchart-eos-7d");
        assert_eq!(stored.input.source, data::CANONICAL_SOURCE);
        assert_eq!((stored.input.width, stored.input.height), (5_202, 3_464));
        assert_eq!(stored.input.pattern, "GBRG");
        assert_eq!(stored.comparisons.len(), ALGORITHM_COUNT);
        Ok(())
    }

    #[test]
    fn progress_events_wrap_each_algorithm() -> mischief::Result<()> {
        let mut events = Vec::new();
        let input = data::seeded("progress", 65, 65);
        let rows = report::canonical_rows(&input, |event| events.push(event))?;
        assert_eq!(rows.len(), ALGORITHM_COUNT);
        assert_eq!(events.len(), ALGORITHM_COUNT * 2);
        let reported_cases = events
            .iter()
            .filter_map(|event| match event {
                ProgressEvent::Started {
                    algorithm,
                    reference,
                } => Some((*algorithm, *reference)),
                ProgressEvent::Finished => None,
            })
            .collect::<Vec<_>>();
        let expected_cases = cases::CASES
            .iter()
            .map(|case| (case.name, case.reference_backend.library()))
            .collect::<Vec<_>>();
        assert_eq!(reported_cases, expected_cases);
        let (pairs, remainder) = events.as_chunks::<2>();
        assert!(remainder.is_empty());
        for [started, finished] in pairs {
            assert!(matches!(started, ProgressEvent::Started { .. }));
            assert_eq!(*finished, ProgressEvent::Finished);
        }
        Ok(())
    }
}
