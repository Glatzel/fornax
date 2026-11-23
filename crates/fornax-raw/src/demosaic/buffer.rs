use super::LazyDemosaic;
use crate::{BayerChannel, BayerPattern, RawFloat, RgbImage};
pub struct DenseLazy<T: RawFloat> {
    image: RgbImage<T>,
}

impl<T: RawFloat> DenseLazy<T> {
    pub(super) const fn new(image: RgbImage<T>) -> Self { Self { image } }
}

impl<T> LazyDemosaic<T> for DenseLazy<T>
where
    T: RawFloat + Send + Sync,
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

pub(super) fn dense_from_vec<T>(width: u32, height: u32, values: Vec<T>) -> RgbImage<T>
where
    T: RawFloat,
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
    fn generic_dense_output_preserves_signed_hdr_samples() {
        let values = vec![-3.0_f64, 1.0 + 2.0_f64.powi(-40), 1.0e6];
        let image = dense_from_vec(1, 1, values.clone());
        assert_eq!(image.into_raw(), values);
    }
}
