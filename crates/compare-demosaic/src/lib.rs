mod case;
mod cases;
pub mod check;
mod data;
pub mod error;
mod ffi;
mod metrics;
pub mod registry;
pub mod report;
pub mod results_md;
pub mod run;

#[cfg(test)]
mod tests {
    use fornax_core::BayerPattern;
    use fornax_devtool::synthetic_mono_image;
    use rstest::{fixture, rstest};
    use serde::Serialize;

    use crate::case::ReferenceBackend;
    use crate::metrics::{FailedLimit, Metrics, Verdict, WorstPixel};
    use crate::run::{CaseEvaluation, LIBRAW_COMMIT, LIBRAW_VERSION};

    #[fixture]
    #[once]
    fn synthetic_128() -> crate::data::ActiveArea {
        let image = synthetic_mono_image::<f32>((128, 128), 65_535.0);
        crate::data::ActiveArea {
            source: "fornax-devtool::synthetic_mono_image".to_owned(),
            width: 128,
            height: 128,
            codes: image
                .as_raw()
                .iter()
                .map(|sample| sample.round().clamp(0.0, 65_535.0) as u16)
                .collect(),
            native_pattern: BayerPattern::GBRG,
        }
    }

    #[fixture]
    #[once]
    fn cropped_real_128() -> crate::data::ActiveArea {
        crop_128(&crate::data::active_area().expect("pinned CR2 fixture decodes"))
    }

    fn crop_128(input: &crate::data::ActiveArea) -> crate::data::ActiveArea {
        let width = 128;
        let height = 128;
        let x = ((input.width - width) / 2) & !1;
        let y = ((input.height - height) / 2) & !1;
        let codes = (y..y + height)
            .flat_map(|row| {
                let start = row as usize * input.width as usize + x as usize;
                input.codes[start..start + width as usize].iter().copied()
            })
            .collect();
        crate::data::ActiveArea {
            source: input.source.clone(),
            width,
            height,
            codes,
            native_pattern: input.native_pattern,
        }
    }

    #[rstest]
    fn every_algorithm_accepts_synthetic_128(
        synthetic_128: &crate::data::ActiveArea,
        #[values(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)] case_index: usize,
    ) -> mischief::Result<()> {
        crate::run::evaluate_case(
            &crate::cases::CASES[case_index],
            synthetic_128,
            BayerPattern::GBRG,
            16,
        )?;
        Ok(())
    }

    #[rstest]
    fn algorithm_filter_selects_the_registered_algorithm(
        synthetic_128: &crate::data::ActiveArea,
    ) -> mischief::Result<()> {
        let algorithm = "VNG".to_owned();
        let rows = crate::run::selected_evaluations(synthetic_128, &[algorithm], 16)?;
        assert_eq!(rows.len(), 8);
        assert!(rows.iter().all(|row| row.case.name == "VNG"));
        Ok(())
    }

    #[test]
    fn algorithm_ids_are_unique_and_unknown_ids_are_rejected() -> mischief::Result<()> {
        let mut ids = crate::cases::CASES
            .iter()
            .map(|case| (case.reference_backend, case.algorithm))
            .collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), crate::cases::CASES.len());

        let error = crate::ffi::run(
            ReferenceBackend::LibRaw,
            u32::MAX,
            &[0_u16; 9],
            3,
            3,
            crate::ffi::libraw_filters(BayerPattern::RGGB),
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
        assert_eq!(LIBRAW_VERSION, "0.22.3");
        assert_eq!(crate::run::DARKTABLE_RELEASE, "5.6.2");
        for case in crate::cases::CASES {
            let commit = match case.reference_backend {
                ReferenceBackend::LibRaw => LIBRAW_COMMIT,
                ReferenceBackend::Darktable => crate::run::DARKTABLE_COMMIT,
            };
            assert!(case.source_url.contains(commit));
            assert!(case.source_url.contains("#L"));
        }
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[rstest]
    fn algorithm_checksum(
        cropped_real_128: &crate::data::ActiveArea,
        #[values(
            BayerPattern::RGGB,
            BayerPattern::BGGR,
            BayerPattern::GRBG,
            BayerPattern::GBRG
        )]
        pattern: BayerPattern,
        #[values(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)] case_index: usize,
    ) -> mischief::Result<()> {
        let evaluation = crate::run::evaluate_case(
            &crate::cases::CASES[case_index],
            cropped_real_128,
            pattern,
            16,
        )?;
        let comparison = LegacyComparison::from_evaluation(cropped_real_128, &evaluation);
        let snapshot_name = format!(
            "{}-{}-{}",
            comparison.pattern, comparison.algorithm, comparison.reference
        );
        insta::assert_snapshot!(snapshot_name, comparison_checksum(&comparison));
        Ok(())
    }

    #[derive(Serialize)]
    struct LegacyComparison {
        algorithm: &'static str,
        source_url: &'static str,
        pattern: String,
        native: bool,
        reference: String,
        kind: &'static str,
        verdict: Verdict,
        failed_limits: Vec<FailedLimit>,
        total_samples: u64,
        metrics: Metrics,
        metrics_interior: Metrics,
        per_channel: LegacyPerChannel,
        per_site: LegacyPerSite,
        worst_pixels: Vec<WorstPixel>,
    }

    #[derive(Serialize)]
    struct LegacyPerChannel {
        r: Metrics,
        g: Metrics,
        b: Metrics,
    }

    #[derive(Serialize)]
    struct LegacyPerSite {
        sampled: Metrics,
        interpolated: Metrics,
    }

    impl LegacyComparison {
        fn from_evaluation(input: &crate::data::ActiveArea, evaluation: &CaseEvaluation) -> Self {
            Self {
                algorithm: evaluation.case.name,
                source_url: evaluation.case.source_url,
                pattern: evaluation.pattern.to_string(),
                native: evaluation.pattern == input.native_pattern,
                reference: evaluation
                    .case
                    .reference_backend
                    .library()
                    .to_ascii_lowercase(),
                kind: match evaluation.case.tolerance {
                    crate::case::Tolerance::RoundingUp => "rounding",
                    crate::case::Tolerance::Robust { .. } => "robust",
                    crate::case::Tolerance::Exact => "exact",
                    crate::case::Tolerance::MaxError(_) => "max_error",
                },
                verdict: evaluation.verdict,
                failed_limits: evaluation.failed_limits.clone(),
                total_samples: evaluation.evaluation.metrics.total_samples,
                metrics: evaluation.evaluation.metrics.clone(),
                metrics_interior: evaluation.evaluation.metrics_interior.clone(),
                per_channel: LegacyPerChannel {
                    r: evaluation.evaluation.per_channel[0].clone(),
                    g: evaluation.evaluation.per_channel[1].clone(),
                    b: evaluation.evaluation.per_channel[2].clone(),
                },
                per_site: LegacyPerSite {
                    sampled: evaluation.evaluation.per_site[0].clone(),
                    interpolated: evaluation.evaluation.per_site[1].clone(),
                },
                worst_pixels: evaluation.evaluation.worst_pixels.clone(),
            }
        }
    }

    fn comparison_checksum(comparison: &LegacyComparison) -> String {
        let bytes =
            serde_json::to_vec(comparison).expect("comparison result must serialize to JSON");
        let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
        format!("{hash:016x}")
    }
}
