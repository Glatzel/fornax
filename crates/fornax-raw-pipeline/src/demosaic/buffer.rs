use fornax_raw_core::error::DemosaicError;
use fornax_raw_core::{BayerChannel, BayerPattern, MonoImage, Primitive, RgbImage};

use crate::LazyDemosaic;

pub struct DenseLazy<T: Primitive> {
    image: RgbImage<T>,
}

impl<T: Primitive> DenseLazy<T> {
    pub(super) const fn new(image: RgbImage<T>) -> Self { Self { image } }
}

impl<T> LazyDemosaic<T> for DenseLazy<T>
where
    T: Primitive + Send + Sync,
{
    fn dimensions(&self) -> (u32, u32) { self.image.dimensions() }

    fn fill_rows(&self, y0: usize, out: &mut [T]) {
        let row_stride = self.image.width() as usize * 3;
        let start = y0 * row_stride;
        let end = start + out.len();
        out.copy_from_slice(&self.image.as_raw()[start..end]);
    }

    fn collect(&self) -> RgbImage<T> { self.image.clone() }
}

pub(super) fn reference_codes<T>(image: &MonoImage<T>) -> Result<Vec<u16>, DemosaicError>
where
    T: Primitive,
{
    let width = image.width() as usize;
    image
        .as_raw()
        .iter()
        .enumerate()
        .map(|(index, sample)| {
            let value = sample
                .to_f64()
                .ok_or_else(|| DemosaicError::InvalidInput("sample does not fit f64".into()))?;
            if !value.is_finite() || !(0.0..=1.0).contains(&value) {
                let x = index % width;
                let y = index / width;
                return Err(DemosaicError::InvalidInput(
                    format!(
                        "reference-precision demosaic requires finite [0,1] samples; \
                         ({x}, {y}) is {value}"
                    )
                    .into(),
                ));
            }
            Ok((value * f64::from(u16::MAX)).round() as u16)
        })
        .collect()
}

pub(super) fn dense_from_reference_codes<T>(
    width: u32,
    height: u32,
    values: Vec<u16>,
) -> RgbImage<T>
where
    T: Primitive,
{
    let scale = T::from(f64::from(u16::MAX)).expect("u16::MAX is representable");
    dense_from_vec(
        width,
        height,
        values
            .into_iter()
            .map(|value| T::from(value).expect("u16 is representable") / scale)
            .collect(),
    )
}

pub(super) fn dense_from_vec<T>(width: u32, height: u32, values: Vec<T>) -> RgbImage<T>
where
    T: Primitive,
{
    RgbImage::from_raw(width, height, values)
        .expect("demosaic output dimensions match the source image")
}

pub(super) const fn channel(pattern: BayerPattern, x: u32, y: u32) -> usize {
    match pattern.channel_at(x, y) {
        BayerChannel::R => 0,
        BayerChannel::GRedRow | BayerChannel::GBlueRow => 1,
        BayerChannel::B => 2,
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn reference_codes_reject_values_that_the_integer_reference_cannot_represent() {
        for value in [-1.0_f64, 1.01, f64::NAN, f64::INFINITY] {
            let image = MonoImage::from_raw(1, 1, vec![value]).unwrap();
            assert!(reference_codes(&image).is_err(), "{value:?} was accepted");
        }
    }

    #[test]
    fn generic_dense_output_preserves_signed_hdr_samples() {
        let values = vec![-3.0_f64, 1.0 + 2.0_f64.powi(-40), 1.0e6];
        let image = dense_from_vec(1, 1, values.clone());
        assert_eq!(image.into_raw(), values);
    }
}
