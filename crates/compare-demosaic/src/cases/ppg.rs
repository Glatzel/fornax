use fornax_core::protocol::{IDemosaic, LazyDemosaic};
use fornax_core::{BayerPattern, MonoImage, Primitive};
use fornax_postprocessor::Demosaic;

use crate::case::{Case, ReferenceBackend, SampleType, Tolerance};

const LIBRAW_PPG_URL: &str = concat!(
    "https://github.com/LibRaw/LibRaw/blob/",
    env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT"),
    "/src/demosaic/misc_demosaic.cpp#L363"
);
const DARKTABLE_PPG_URL: &str = concat!(
    "https://github.com/darktable-org/darktable/blob/",
    env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT"),
    "/src/iop/demosaicing/ppg.c#L20"
);

pub const LIBRAW_CASE: Case = Case {
    name: "PPG",
    reference: "LibRaw::ppg_interpolate",
    source_name: "misc_demosaic.cpp",
    source_url: LIBRAW_PPG_URL,
    reference_backend: ReferenceBackend::LibRaw,
    algorithm: 1,
    border: 0,
    tolerance: Tolerance::MaxError(12_000),
    run_fornax: run,
};

pub const DARKTABLE_CASE: Case = Case {
    name: "PPG",
    reference: "darktable demosaic_ppg",
    source_name: "ppg.c",
    source_url: DARKTABLE_PPG_URL,
    reference_backend: ReferenceBackend::Darktable,
    algorithm: 0,
    border: 0,
    tolerance: Tolerance::MaxError(12_000),
    run_fornax: run,
};

fn run(
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Vec<u16>> {
    match sample_type {
        SampleType::F32 => run_typed::<f32>(codes, width, height, pattern),
        SampleType::F64 => run_typed::<f64>(codes, width, height, pattern),
    }
}

fn run_typed<T>(
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
) -> mischief::Result<Vec<u16>>
where
    T: Primitive,
{
    let expected = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| mischief::mischief!("synthetic image dimensions overflow"))?;
    if codes.len() != expected {
        return Err(mischief::mischief!(
            "synthetic image contains {} samples, expected {expected}",
            codes.len()
        ));
    }
    let samples = codes
        .iter()
        .map(|code| {
            let normalized = f64::from(*code) / f64::from(u16::MAX);
            num_traits::cast(normalized).ok_or_else(|| {
                mischief::mischief!(
                    "sample cannot be represented as {}",
                    std::any::type_name::<T>()
                )
            })
        })
        .collect::<mischief::Result<Vec<T>>>()?;
    let image = MonoImage::from_raw(width, height, samples)
        .ok_or_else(|| mischief::mischief!("synthetic image dimensions are inconsistent"))?;
    let output = Demosaic::Ppg.demosaic(&image, pattern)?;
    Ok(output
        .collect()
        .as_raw()
        .iter()
        .map(|sample| {
            let maximum =
                num_traits::cast(f64::from(u16::MAX)).expect("sample supports 16-bit range");
            let normalized = sample.clamp(T::zero(), T::one());
            num_traits::cast((normalized * maximum).round()).expect("rounded sample fits u16")
        })
        .collect())
}
