mod linear;
use fornax_core::error::DemosaicError;
use fornax_core::protocol::{IDemosaic, LazyDemosaic};
use fornax_core::{BayerPattern, MonoImage, Primitive};
use linear::{DemosaicLinear, LinearLazy};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Demosaic {
    Linear,
    // Ahd, Rcd, ...
}

pub enum AnyLazy<'a, T> {
    Linear(LinearLazy<'a, T>),
    // Ahd(AhdLazy<'a, T>),
}

impl<T: Primitive + Send + Sync> LazyDemosaic<T> for AnyLazy<'_, T> {
    fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Linear(l) => l.dimensions(),
        }
    }

    fn fill_rows(&self, y0: usize, out: &mut [T]) {
        match self {
            Self::Linear(l) => l.fill_rows(y0, out),
        }
    }
}

impl<T: Primitive + Send + Sync> IDemosaic<T> for Demosaic {
    #[allow(refining_impl_trait)]
    fn demosaic<'a>(
        &self,
        bayer: &'a MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<AnyLazy<'a, T>, DemosaicError> {
        Ok(match self {
            Self::Linear => AnyLazy::Linear(DemosaicLinear.lazy(bayer, pattern)?),
        })
    }
}
