use fornax_core::BayerPattern;

use crate::case::{Case, ReferenceBackend, SampleType, Tolerance};

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
    tolerance: Tolerance::Exact,
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
    border: 4,
    tolerance: Tolerance::Exact,
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
    tolerance: Tolerance::Exact,
    run_fornax: run_aahd,
};

fn run_dcb(
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Vec<u16>> {
    let _ = sample_type;
    crate::cases::reference_runner::run(
        fornax_demosaic_reference::Algorithm::Dcb,
        codes,
        width,
        height,
        pattern,
    )
}

fn run_dht(
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Vec<u16>> {
    let _ = sample_type;
    crate::cases::reference_runner::run(
        fornax_demosaic_reference::Algorithm::Dht,
        codes,
        width,
        height,
        pattern,
    )
}

fn run_aahd(
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Vec<u16>> {
    let _ = sample_type;
    crate::cases::reference_runner::run(
        fornax_demosaic_reference::Algorithm::Aahd,
        codes,
        width,
        height,
        pattern,
    )
}
