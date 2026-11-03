pub mod artifacts;
pub mod cases;
pub mod diff;
pub mod metrics;
pub mod readme;
pub mod reference;
pub mod report;
pub mod run;
pub mod status;
pub mod waivers;

#[cfg(test)]
mod tests {
    use fornax_core::BayerPattern;
    use fornax_devtool::synthetic_mono_image;
    use rstest::{fixture, rstest};
    use serde::Serialize;

    use crate::cases::ReferenceBackend;
    use crate::metrics::{FailedLimit, Metrics, Verdict, WorstPixel};
    use crate::run::{CaseEvaluation, LIBRAW_COMMIT, LIBRAW_VERSION};

    #[fixture]
    #[once]
    fn synthetic_128() -> crate::reference::ActiveArea {
        let image = synthetic_mono_image::<f32>((128, 128), 65_535.0);
        crate::reference::ActiveArea {
            source: "fornax-devtool::synthetic_mono_image".to_owned(),
            sha256: "00".repeat(32),
            width: 128,
            height: 128,
            codes: image
                .as_raw()
                .iter()
                .map(|sample| sample.round().clamp(0.0, 65_535.0) as u16)
                .collect(),
            black: vec![0.0; 4],
            black_width: 2,
            black_height: 2,
            white: 65_535.0,
            native_pattern: BayerPattern::GBRG,
        }
    }

    #[fixture]
    #[once]
    fn cropped_real_128() -> crate::reference::ActiveArea {
        crop_128(
            &crate::reference::active_area_from(&fornax_devtool::raw_file())
                .expect("pinned CR2 fixture decodes"),
        )
    }

    fn crop_128(input: &crate::reference::ActiveArea) -> crate::reference::ActiveArea {
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
        crate::reference::ActiveArea {
            source: input.source.clone(),
            sha256: input.sha256.clone(),
            width,
            height,
            codes,
            black: input.black.clone(),
            black_width: input.black_width,
            black_height: input.black_height,
            white: input.white,
            native_pattern: input.native_pattern,
        }
    }

    #[rstest]
    fn every_algorithm_accepts_synthetic_128(
        synthetic_128: &crate::reference::ActiveArea,
        #[values(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)] case_index: usize,
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
        synthetic_128: &crate::reference::ActiveArea,
    ) -> mischief::Result<()> {
        let algorithm = "VNG".to_owned();
        let rows = crate::run::selected_evaluations(synthetic_128, &[algorithm], 16)?;
        assert_eq!(rows.len(), 8);
        assert!(rows.iter().all(|row| row.case.name == "VNG"));
        Ok(())
    }

    #[rstest]
    fn native_vng_meets_reference_limits(
        cropped_real_128: &crate::reference::ActiveArea,
        #[values("libraw", "darktable")] reference: &str,
    ) -> mischief::Result<()> {
        let case = crate::cases::case_for("vng", reference)
            .expect("the VNG reference case must be registered");
        let evaluation = crate::run::evaluate_case(case, cropped_real_128, BayerPattern::GBRG, 16)?;
        match case.reference_backend {
            ReferenceBackend::Darktable => assert!(
                matches!(
                    evaluation.verdict,
                    crate::metrics::Verdict::Exact | crate::metrics::Verdict::Pass
                ),
                "VNG versus {reference} failed limits: {:?}",
                evaluation.failed_limits
            ),
            // LibRaw uses a different green-plane convention. Record its
            // distance for information without making it a VNG acceptance gate.
            ReferenceBackend::LibRaw => {
                println!(
                    "VNG versus LibRaw: MAE {}, max {}",
                    evaluation.evaluation.metrics.mae, evaluation.evaluation.metrics.max
                );
            }
        }
        Ok(())
    }

    #[rstest]
    fn native_vng4_meets_reference_limits(
        cropped_real_128: &crate::reference::ActiveArea,
    ) -> mischief::Result<()> {
        let case = crate::cases::case_for("vng4", "libraw")
            .expect("the LibRaw VNG4 reference case must be registered");
        let evaluation = crate::run::evaluate_case(case, cropped_real_128, BayerPattern::GBRG, 16)?;
        assert!(
            matches!(
                evaluation.verdict,
                crate::metrics::Verdict::Exact | crate::metrics::Verdict::Pass
            ),
            "VNG4 versus LibRaw failed limits: {:?}",
            evaluation.failed_limits
        );
        Ok(())
    }

    #[rstest]
    fn native_aahd_meets_reference_limits(
        cropped_real_128: &crate::reference::ActiveArea,
    ) -> mischief::Result<()> {
        let case = crate::cases::case_for("aahd", "libraw")
            .expect("the LibRaw AAHD reference case must be registered");
        let evaluation = crate::run::evaluate_case(case, cropped_real_128, BayerPattern::GBRG, 16)?;
        assert!(
            matches!(
                evaluation.verdict,
                crate::metrics::Verdict::Exact | crate::metrics::Verdict::Pass
            ),
            "AAHD versus LibRaw failed limits: {:?}",
            evaluation.failed_limits
        );
        Ok(())
    }

    #[rstest]
    fn native_lmmse_meets_reference_limits(
        cropped_real_128: &crate::reference::ActiveArea,
    ) -> mischief::Result<()> {
        let case = crate::cases::case_for("lmmse", "darktable")
            .expect("the darktable LMMSE reference case must be registered");
        let evaluation = crate::run::evaluate_case(case, cropped_real_128, BayerPattern::GBRG, 16)?;
        assert!(
            matches!(
                evaluation.verdict,
                crate::metrics::Verdict::Exact | crate::metrics::Verdict::Pass
            ),
            "LMMSE versus darktable failed limits: {:?}",
            evaluation.failed_limits
        );
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

        let error = crate::reference::run(
            ReferenceBackend::LibRaw,
            u32::MAX,
            &[0_u16; 9],
            3,
            3,
            crate::reference::libraw_filters(BayerPattern::RGGB),
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

    #[test]
    fn snapshot_filenames_are_unique_on_case_insensitive_filesystems() -> mischief::Result<()> {
        let snapshot_dir = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("src/snapshots");
        let mut names = std::collections::BTreeMap::<String, Vec<String>>::new();
        for entry in std::fs::read_dir(snapshot_dir)? {
            let path = entry?.path();
            if path
                .extension()
                .is_some_and(|extension| extension == "snap")
            {
                let filename = path
                    .file_name()
                    .expect("snapshot path has a filename")
                    .to_string_lossy()
                    .into_owned();
                names
                    .entry(filename.to_ascii_lowercase())
                    .or_default()
                    .push(filename);
            }
        }
        let collisions = names
            .into_values()
            .filter(|filenames| filenames.len() > 1)
            .collect::<Vec<_>>();
        assert!(
            collisions.is_empty(),
            "snapshot filenames must be unique ignoring ASCII case: {collisions:#?}"
        );
        Ok(())
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[rstest]
    fn algorithm_checksum(
        cropped_real_128: &crate::reference::ActiveArea,
        #[values(
            BayerPattern::RGGB,
            BayerPattern::BGGR,
            BayerPattern::GRBG,
            BayerPattern::GBRG
        )]
        pattern: BayerPattern,
        #[values(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12)] case_index: usize,
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
        fn from_evaluation(
            input: &crate::reference::ActiveArea,
            evaluation: &CaseEvaluation,
        ) -> Self {
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
                    crate::cases::Tolerance::RoundingUp => "rounding",
                    crate::cases::Tolerance::Robust { .. } => "robust",
                    crate::cases::Tolerance::Exact => "exact",
                    crate::cases::Tolerance::MaxError(_) => "max_error",
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
