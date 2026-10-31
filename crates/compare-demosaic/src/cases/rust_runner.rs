use fornax_core::protocol::{IDemosaic, LazyDemosaic};
use fornax_core::{BayerPattern, MonoImage, Primitive};
use fornax_postprocessor::Demosaic;

use crate::case::SampleType;

pub fn run(
    demosaic: Demosaic,
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Vec<u16>> {
    match sample_type {
        SampleType::F32 => run_typed::<f32>(demosaic, codes, width, height, pattern),
        SampleType::F64 => run_typed::<f64>(demosaic, codes, width, height, pattern),
    }
}

fn run_typed<T>(
    demosaic: Demosaic,
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
    let output = demosaic.demosaic(&image, pattern)?;
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
