use fornax_core::error::DemosaicError;
use fornax_core::{BayerPattern, MonoImage, Primitive};
use fornax_postprocessor::{IDemosaic, LazyDemosaic};

struct IdentityLazy;

impl<T: Primitive> LazyDemosaic<T> for IdentityLazy {
    fn dimensions(&self) -> (u32, u32) { (1, 1) }

    fn fill_rows(&self, _y0: usize, out: &mut [T]) { out.fill(T::one()); }
}

struct IdentityDemosaic;

impl<T: Primitive> IDemosaic<T> for IdentityDemosaic {
    fn demosaic<'a>(
        &self,
        _bayer: &'a MonoImage<T>,
        _pattern: BayerPattern,
    ) -> Result<impl LazyDemosaic<T> + 'a, DemosaicError> {
        Ok(IdentityLazy)
    }
}

#[test]
fn protocol_paths_expose_lazy_and_demosaic_traits() -> mischief::Result<()> {
    let mosaic = MonoImage::from_raw(1, 1, vec![0.5_f32]).expect("valid 1x1 mosaic");
    let rgb = IdentityDemosaic
        .demosaic(&mosaic, BayerPattern::RGGB)?
        .collect();

    assert_eq!(rgb.as_raw(), &[1.0, 1.0, 1.0]);
    Ok(())
}
