mod case;
mod cases;
mod data;
mod ffi;
mod metrics;
mod report;
pub mod results;

#[cfg(test)]
mod tests {
    use fornax_core::BayerPattern;
    use fornax_devtool::synthetic_mono_image;
    use rstest::{fixture, rstest};

    use crate::case::{ReferenceBackend, SampleType};
    use crate::report::compare;
    use crate::results::SummaryComparison;

    #[fixture]
    #[once]
    fn synthetic_512() -> crate::data::ActiveArea {
        let image = synthetic_mono_image::<f32>((512, 512), 65_535.0);
        crate::data::ActiveArea {
            source: "fornax-devtool::synthetic_mono_image".to_owned(),
            width: 512,
            height: 512,
            codes: image
                .as_raw()
                .iter()
                .map(|sample| sample.round().clamp(0.0, 65_535.0) as u16)
                .collect(),
        }
    }

    fn crop_512(input: &crate::data::ActiveArea) -> crate::data::ActiveArea {
        let width = 512;
        let height = 512;
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
        }
    }

    #[rstest]
    fn every_algorithm_accepts_synthetic_512(
        synthetic_512: &crate::data::ActiveArea,
        #[values(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)] case_index: usize,
    ) -> mischief::Result<()> {
        let case = &crate::cases::CASES[case_index];
        let row = compare(case, synthetic_512, BayerPattern::GBRG, SampleType::F32)?;
        assert!(
            row.metrics.mean_absolute_error.is_finite(),
            "{} produced non-finite metrics",
            case.name
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
        assert_eq!(crate::report::LIBRAW_VERSION, "0.22.3");
        assert_eq!(crate::report::DARKTABLE_RELEASE, "5.6.2");
        for case in crate::cases::CASES {
            let commit = match case.reference_backend {
                ReferenceBackend::LibRaw => crate::report::LIBRAW_COMMIT,
                ReferenceBackend::Darktable => crate::report::DARKTABLE_COMMIT,
            };
            assert!(case.source_url.contains(commit));
            assert!(case.source_url.contains("#L"));
        }
    }

    #[cfg(all(target_os = "linux", target_arch = "x86_64"))]
    #[rstest]
    #[case::rggb(BayerPattern::RGGB, "rggb")]
    #[case::bggr(BayerPattern::BGGR, "bggr")]
    #[case::grbg(BayerPattern::GRBG, "grbg")]
    #[case::gbrg(BayerPattern::GBRG, "gbrg")]
    fn pattern_snapshot(
        #[case] pattern: BayerPattern,
        #[case] snapshot_name: &str,
    ) -> mischief::Result<()> {
        let input = crop_512(&crate::data::active_area()?);
        let comparisons = crate::report::comparison_rows_for_pattern(&input, pattern)?
            .into_iter()
            .map(SummaryComparison::from)
            .collect::<Vec<_>>();
        insta::assert_json_snapshot!(snapshot_name, comparisons);
        Ok(())
    }
}
