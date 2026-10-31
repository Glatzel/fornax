use fornax_core::error::DemosaicError;
use fornax_core::{BayerChannel, BayerPattern, MonoImage, Primitive};
use fornax_demosaic_reference::{Algorithm, Error, demosaic};
use generic_num::num;

use super::buffer::DenseLazy;

pub(super) fn lazy<T>(
    algorithm: Algorithm,
    bayer: &MonoImage<T>,
    pattern: BayerPattern,
) -> Result<DenseLazy<T>, DemosaicError>
where
    T: Primitive + Send + Sync,
{
    if bayer.width() < 17 || bayer.height() < 17 {
        return Err(DemosaicError::InvalidInput(
            format!("{algorithm:?} requires an image at least 17x17").into(),
        ));
    }
    let input = bayer
        .as_raw()
        .iter()
        .map(|sample| {
            (sample.to_f64().unwrap_or_default().clamp(0.0, 1.0) * f64::from(u16::MAX)).round()
                as u16
        })
        .collect::<Vec<_>>();
    let output = demosaic(
        algorithm,
        &input,
        bayer.width(),
        bayer.height(),
        filters(pattern),
    )
    .map_err(|error| {
        DemosaicError::InvalidInput(
            format!(
                "{algorithm:?} reference backend failed: {}",
                error_message(error)
            )
            .into(),
        )
    })?;
    let image = fornax_core::RgbImage::from_raw(
        bayer.width(),
        bayer.height(),
        output
            .into_iter()
            .map(|sample| num!(f64::from(sample) / f64::from(u16::MAX)))
            .collect(),
    )
    .expect("reference demosaic output dimensions match the source image");
    Ok(DenseLazy::new(image))
}

fn filters(pattern: BayerPattern) -> u32 {
    let mut filters = 0_u32;
    for row in 0..8_u32 {
        for col in 0..2_u32 {
            let code = match pattern.channel_at(col, row) {
                BayerChannel::R => 0,
                BayerChannel::GRedRow => 1,
                BayerChannel::B => 2,
                BayerChannel::GBlueRow => 3,
            };
            filters |= code << (((row << 1 & 14) + (col & 1)) << 1);
        }
    }
    filters
}

const fn error_message(error: Error) -> &'static str {
    match error {
        Error::InvalidDimensions => "invalid dimensions",
        Error::DimensionOverflow => "dimension overflow",
        Error::AllocationFailed => "allocation failed",
        Error::NullPointer => "null pointer",
        Error::OutOfMemory => "out of memory",
        Error::Exception => "reference exception",
        Error::UnknownAlgorithm => "unknown algorithm",
    }
}
