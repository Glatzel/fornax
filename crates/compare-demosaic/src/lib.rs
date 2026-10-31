mod artifacts;
mod case;
mod cases;
mod data;
pub mod diff;
mod ffi;
mod metrics;
pub mod render;
mod report;
pub mod results;

#[cfg(test)]
mod tests {
    use fornax_core::BayerPattern;
    use fornax_devtool::synthetic_mono_image;
    use rstest::{fixture, rstest};

    use crate::case::{ReferenceBackend, SampleType};
    use crate::report::compare;
    use crate::results::ComparisonResult;

    #[fixture]
    #[once]
    fn synthetic_128() -> crate::data::ActiveArea {
        let image = synthetic_mono_image::<f32>((128, 128), 65_535.0);
        crate::data::ActiveArea {
            source: "fornax-devtool::synthetic_mono_image".to_owned(),
            source_path: "fornax-devtool::synthetic_mono_image".to_owned(),
            width: 128,
            height: 128,
            codes: image
                .as_raw()
                .iter()
                .map(|sample| sample.round().clamp(0.0, 65_535.0) as u16)
                .collect(),
            black: vec![0.0],
            black_width: 1,
            black_height: 1,
            white: 65_535.0,
            native_pattern: BayerPattern::GBRG,
        }
    }

    #[fixture]
    #[once]
    fn cropped_real_128() -> crate::data::ActiveArea {
        let input = crate::data::active_area().expect("the pinned CR2 fixture must decode");
        crop_128(&input)
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
            source_path: input.source_path.clone(),
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
        synthetic_128: &crate::data::ActiveArea,
        #[values(0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11)] case_index: usize,
    ) -> mischief::Result<()> {
        let case = &crate::cases::CASES[case_index];
        compare(
            case,
            synthetic_128,
            BayerPattern::GBRG,
            SampleType::F32,
            false,
        )?;
        Ok(())
    }

    #[rstest]
    fn comparison_filter_selects_pattern_and_algorithm(
        synthetic_128: &crate::data::ActiveArea,
    ) -> mischief::Result<()> {
        let rows = crate::report::comparison_rows_filtered(
            synthetic_128,
            Some(BayerPattern::RGGB),
            Some("vng"),
        )?;
        assert_eq!(rows.len(), 2);
        assert!(rows.iter().all(|row| row.pattern == BayerPattern::RGGB));
        assert!(rows.iter().all(|row| row.case == "VNG"));
        Ok(())
    }

    fn comparison_checksum(comparison: &ComparisonResult) -> String {
        let bytes =
            serde_json::to_vec(comparison).expect("comparison result must serialize to JSON");
        let hash = bytes.iter().fold(0xcbf2_9ce4_8422_2325_u64, |hash, byte| {
            (hash ^ u64::from(*byte)).wrapping_mul(0x0000_0100_0000_01b3)
        });
        format!("{hash:016x}")
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
        let case = &crate::cases::CASES[case_index];
        let row = compare(case, cropped_real_128, pattern, SampleType::F32, false)?;
        let comparison = ComparisonResult::from_row(&cropped_real_128.native_pattern, row);
        let snapshot_name = format!(
            "{}-{}-{}",
            comparison.pattern, comparison.algorithm, comparison.reference
        );
        insta::assert_snapshot!(snapshot_name, comparison_checksum(&comparison));
        Ok(())
    }
}
