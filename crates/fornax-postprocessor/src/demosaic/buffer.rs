use fornax_core::protocol::LazyDemosaic;
use fornax_core::{BayerChannel, BayerPattern, MonoImage, Primitive, RgbImage};
use generic_num::num;

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

pub(super) fn normalized<T: Primitive>(image: &MonoImage<T>) -> Vec<f32> {
    image
        .as_raw()
        .iter()
        .map(|sample| sample.to_f32().unwrap_or_default().clamp(0.0, 1.0))
        .collect()
}

pub(super) fn codes<T: Primitive>(image: &MonoImage<T>) -> Vec<u16> {
    image
        .as_raw()
        .iter()
        .map(|sample| {
            (sample.to_f64().unwrap_or_default().clamp(0.0, 1.0) * f64::from(u16::MAX)).round()
                as u16
        })
        .collect()
}

pub(super) fn dense_from_codes<T: Primitive>(
    width: u32,
    height: u32,
    values: Vec<u16>,
) -> RgbImage<T> {
    RgbImage::from_raw(
        width,
        height,
        values
            .into_iter()
            .map(|value| num!(f64::from(value) / f64::from(u16::MAX)))
            .collect(),
    )
    .expect("demosaic output dimensions match the source image")
}

pub(super) fn dense_from_f32<T: Primitive>(
    width: u32,
    height: u32,
    values: Vec<f32>,
) -> RgbImage<T> {
    RgbImage::from_raw(
        width,
        height,
        values
            .into_iter()
            .map(|value| num!(value.clamp(0.0, 1.0)))
            .collect(),
    )
    .expect("demosaic output dimensions match the source image")
}

pub(super) const fn channel(pattern: BayerPattern, x: u32, y: u32) -> usize {
    match pattern.channel_at(x, y) {
        BayerChannel::R => 0,
        BayerChannel::GRedRow | BayerChannel::GBlueRow => 1,
        BayerChannel::B => 2,
    }
}
