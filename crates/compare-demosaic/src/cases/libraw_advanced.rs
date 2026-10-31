use fornax_core::BayerPattern;
use fornax_postprocessor::Demosaic;

use crate::case::{Case, ROBUST, ReferenceBackend, SampleType};

pub const DCB: Case = Case {
    name: "DCB",
    source_url: concat!(
        "https://github.com/LibRaw/LibRaw/blob/",
        env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT"),
        "/src/demosaic/dcb_demosaic.cpp#L849"
    ),
    reference_backend: ReferenceBackend::LibRaw,
    algorithm: 4,
    border: 6,
    tolerance: ROBUST,
    run_fornax: run_dcb,
};

pub const DHT: Case = Case {
    name: "DHT",
    source_url: concat!(
        "https://github.com/LibRaw/LibRaw/blob/",
        env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT"),
        "/src/demosaic/dht_demosaic.cpp#L1012"
    ),
    reference_backend: ReferenceBackend::LibRaw,
    algorithm: 5,
    border: 32,
    tolerance: ROBUST,
    run_fornax: run_dht,
};

pub const AAHD: Case = Case {
    name: "AAHD",
    source_url: concat!(
        "https://github.com/LibRaw/LibRaw/blob/",
        env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT"),
        "/src/demosaic/aahd_demosaic.cpp#L771"
    ),
    reference_backend: ReferenceBackend::LibRaw,
    algorithm: 6,
    border: 4,
    tolerance: ROBUST,
    run_fornax: run_aahd,
};

fn run_dcb(
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Vec<u16>> {
    crate::cases::rust_runner::run(Demosaic::Dcb, codes, width, height, pattern, sample_type)
}

fn run_dht(
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Vec<u16>> {
    crate::cases::rust_runner::run(Demosaic::Dht, codes, width, height, pattern, sample_type)
}

fn run_aahd(
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Vec<u16>> {
    crate::cases::rust_runner::run(Demosaic::Aahd, codes, width, height, pattern, sample_type)
}
