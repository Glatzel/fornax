use fornax_core::BayerPattern;

use crate::case::{Case, SampleType};
use crate::{artifact, cases, data, ffi, metrics};

pub const LIBRAW_COMMIT: &str = env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT");
pub const DARKTABLE_RELEASE: &str = env!("COMPARE_DEMOSAIC_DARKTABLE_RELEASE");
pub const DARKTABLE_COMMIT: &str = env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT");
pub const LIBRAW_VERSION: &str = env!("COMPARE_DEMOSAIC_LIBRAW_VERSION");
#[cfg(all(test, target_os = "linux"))]
pub const TABLE_BEGIN: &str = "<!-- BEGIN:compare-table -->";
#[cfg(all(test, target_os = "linux"))]
pub const TABLE_END: &str = "<!-- END:compare-table -->";

#[derive(Clone, Debug)]
pub struct Row {
    pub fixture: String,
    pub source: String,
    pub case: &'static str,
    pub reference: &'static str,
    pub source_url: &'static str,
    pub pattern: BayerPattern,
    pub border: u32,
    pub tolerance: crate::case::Tolerance,
    pub metrics: metrics::Metrics,
    pub verdict: metrics::Verdict,
}

pub fn compare(
    case: &'static Case,
    fixture: &str,
    source: &str,
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
    let verdict = metrics.verdict(case.tolerance);
    if verdict == metrics::Verdict::Differs {
        artifact::write_failure(case, pattern, &reference, &actual, width, height)?;
    }
    Ok(Row {
        fixture: fixture.to_owned(),
        source: source.to_owned(),
        case: case.name,
        reference: case.reference_backend.library(),
        source_url: case.source_url,
        pattern,
        border: case.border,
        tolerance: case.tolerance,
        metrics,
        verdict,
    })
}

pub fn canonical_rows() -> mischief::Result<Vec<Row>> {
    let inputs = data::real_inputs();
    let mut rows = Vec::with_capacity(cases::CASES.len() * inputs.len());
    for input in inputs {
        let pattern = input
            .pattern
            .expect("real comparison input must declare its Bayer pattern");
        for case in cases::CASES {
            rows.push(compare(
                case,
                &input.label,
                &input.source,
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
