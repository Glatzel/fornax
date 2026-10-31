use fornax_core::BayerPattern;

use crate::case::{Case, ReferenceBackend, SampleType, Tolerance};

const DARKTABLE_VNG_URL: &str = concat!(
    "https://github.com/darktable-org/darktable/blob/",
    env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT"),
    "/src/iop/demosaicing/vng.c#L171"
);

pub const DARKTABLE_CASE: Case = Case {
    name: "VNG",
    source_url: DARKTABLE_VNG_URL,
    reference_backend: ReferenceBackend::Darktable,
    algorithm: 2,
    border: 2,
    tolerance: Tolerance::Exact,
    run_fornax: run,
};

fn run(
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Vec<u16>> {
    let _ = sample_type;
    crate::cases::reference_runner::run(
        fornax_demosaic_reference::Algorithm::Vng,
        codes,
        width,
        height,
        pattern,
    )
}
