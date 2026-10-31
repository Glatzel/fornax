use fornax_core::protocol::LazyDemosaic;
use fornax_core::{Primitive, RgbImage};

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
