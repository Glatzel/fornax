use fornax_core::BayerPattern;
use fornax_postprocessor::Demosaic;

use crate::case::{Case, ROBUST, ReferenceBackend, SampleType};

const DARKTABLE_VNG_URL: &str = concat!(
    "https://github.com/darktable-org/darktable/blob/",
    env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT"),
    "/src/iop/demosaicing/vng.c#L171"
);
const LIBRAW_VNG_URL: &str = concat!(
    "https://github.com/LibRaw/LibRaw/blob/",
    env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT"),
    "/src/demosaic/misc_demosaic.cpp#L199"
);

pub const DARKTABLE_CASE: Case = Case {
    name: "VNG",
    source_url: DARKTABLE_VNG_URL,
    reference_backend: ReferenceBackend::Darktable,
    algorithm: 2,
    border: 2,
    tolerance: ROBUST,
    run_fornax: run,
};

pub const LIBRAW_CASE: Case = Case {
    name: "VNG",
    source_url: LIBRAW_VNG_URL,
    reference_backend: ReferenceBackend::LibRaw,
    algorithm: 3,
    border: 2,
    tolerance: ROBUST,
    run_fornax: run,
};

fn run(
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Vec<u16>> {
    crate::cases::rust_runner::run(Demosaic::Vng, codes, width, height, pattern, sample_type)
}
