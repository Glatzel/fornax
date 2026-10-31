use fornax_core::BayerPattern;

use crate::case::{Case, ReferenceBackend, SampleType, Tolerance};

pub const AMAZE: Case = Case {
    name: "AMaZE",
    source_url: concat!(
        "https://github.com/darktable-org/darktable/blob/",
        env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT"),
        "/src/iop/demosaicing/amaze.cc#L129"
    ),
    reference_backend: ReferenceBackend::Darktable,
    algorithm: 3,
    border: 16,
    tolerance: Tolerance::Exact,
    run_fornax: run_amaze,
};

pub const LMMSE: Case = Case {
    name: "LMMSE",
    source_url: concat!(
        "https://github.com/darktable-org/darktable/blob/",
        env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT"),
        "/src/iop/demosaicing/lmmse.c#L120"
    ),
    reference_backend: ReferenceBackend::Darktable,
    algorithm: 4,
    border: 4,
    tolerance: Tolerance::Exact,
    run_fornax: run_lmmse,
};

fn run_amaze(
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Vec<u16>> {
    let _ = sample_type;
    crate::cases::reference_runner::run(
        fornax_demosaic_reference::Algorithm::Amaze,
        codes,
        width,
        height,
        pattern,
    )
}

fn run_lmmse(
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Vec<u16>> {
    let _ = sample_type;
    crate::cases::reference_runner::run(
        fornax_demosaic_reference::Algorithm::Lmmse,
        codes,
        width,
        height,
        pattern,
    )
}
