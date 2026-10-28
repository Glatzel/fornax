mod case;
mod cases;
mod data;
mod ffi;
mod metrics;
mod report;

use std::path::PathBuf;

fn main() -> mischief::Result<()> {
    match std::env::args().skip(1).collect::<Vec<_>>().as_slice() {
        [] => {
            print!("{}", report::render_table(&report::canonical_rows()?));
            Ok(())
        }
        [argument] if argument == "--update-readme" => update_readme(),
        _ => Err(mischief::mischief!(
            "usage: compare-demosaic [--update-readme]"
        )),
    }
}

fn update_readme() -> mischief::Result<()> {
    let path = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("README.md");
    let current = std::fs::read_to_string(&path)?;
    let updated = report::update_table(&current, &report::canonical_rows()?)?;
    std::fs::write(&path, updated)?;
    println!("updated {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use fornax_core::BayerPattern;

    use super::*;
    use crate::case::SampleType;
    use crate::cases::CASES;

    #[test]
    fn every_case_matches_for_all_patterns_precisions_and_inputs() -> mischief::Result<()> {
        for case in CASES {
            for pattern in BayerPattern::ALL {
                for sample_type in [SampleType::F32, SampleType::F64] {
                    for input in data::test_inputs() {
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
                    }
                }
            }
        }
        Ok(())
    }

    #[test]
    fn algorithm_ids_are_unique_and_unknown_ids_are_rejected() -> mischief::Result<()> {
        let mut ids = CASES.iter().map(|case| case.algorithm).collect::<Vec<_>>();
        ids.sort_unstable();
        ids.dedup();
        assert_eq!(ids.len(), CASES.len());

        let input = data::canonical();
        let error = ffi::run(
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
                .contains("LibRaw reference returned unknown algorithm")
        );
        Ok(())
    }

    #[test]
    fn readme_table_is_generated_from_cases() -> mischief::Result<()> {
        let readme = include_str!("../README.md");
        let rows = report::canonical_rows()?;
        let expected = report::update_table(readme, &rows)?;
        assert_eq!(
            readme, expected,
            "README results are stale; run `pixi run -e dev compare-demosaic`"
        );
        assert_eq!(
            report::render_table(&rows).lines().count(),
            CASES.len() * BayerPattern::ALL.len() + 2
        );
        Ok(())
    }

    #[test]
    fn source_link_matches_packaged_version() {
        assert_eq!(report::LIBRAW_VERSION, "0.22.3");
        for case in CASES {
            assert!(case.source_url.contains(report::LIBRAW_COMMIT));
            assert!(case.source_url.contains("#L146"));
        }
    }

    #[test]
    fn repeated_calls_leave_libraw_owned_by_its_context() -> mischief::Result<()> {
        let input = data::canonical();
        for _ in 0..16 {
            ffi::run(
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
    fn saturated_input_is_exact() -> mischief::Result<()> {
        let input = data::test_inputs()
            .into_iter()
            .find(|input| input.label == "saturated")
            .expect("saturated test input");
        for pattern in BayerPattern::ALL {
            for sample_type in [SampleType::F32, SampleType::F64] {
                let row = report::compare(
                    &cases::LINEAR,
                    &input.codes,
                    input.width,
                    input.height,
                    pattern,
                    sample_type,
                )?;
                assert_eq!(row.verdict, metrics::Verdict::Exact);
            }
        }
        Ok(())
    }
}
