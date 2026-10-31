use fornax_core::BayerPattern;
use fornax_postprocessor::Demosaic;

use crate::case::{Case, ROBUST, ReferenceBackend, SampleType};

pub const AMAZE: Case = Case {
    name: "AMaZE",
    source_url: concat!(
        "https://github.com/darktable-org/darktable/blob/",
        env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT"),
        "/src/iop/demosaicing/amaze.cc#L129"
    ),
    reference_backend: ReferenceBackend::Darktable,
    algorithm: 3,
    tolerance: ROBUST,
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
    tolerance: ROBUST,
    run_fornax: run_lmmse,
};

fn run_amaze(
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Vec<u16>> {
    crate::cases::rust_runner::run(Demosaic::Amaze, codes, width, height, pattern, sample_type)
}

fn run_lmmse(
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Vec<u16>> {
    crate::cases::rust_runner::run(Demosaic::Lmmse, codes, width, height, pattern, sample_type)
}
