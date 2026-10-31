use fornax_core::BayerPattern;

pub fn run(
    algorithm: fornax_demosaic_reference::Algorithm,
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
) -> mischief::Result<Vec<u16>> {
    fornax_demosaic_reference::demosaic(
        algorithm,
        codes,
        width,
        height,
        crate::ffi::libraw_filters(pattern),
    )
    .map_err(|error| mischief::mischief!("reference backend failed: {error:?}"))
}
