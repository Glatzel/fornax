mod linear;
mod ppg;
mod rcd;
use fornax_core::error::DemosaicError;
use fornax_core::protocol::{IDemosaic, LazyDemosaic};
use fornax_core::{BayerPattern, MonoImage, Primitive};
use linear::{DemosaicLinear, LinearLazy};
use ppg::{DemosaicPpg, PpgLazy};
use rcd::{DemosaicRcd, RcdLazy};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Demosaic {
    Linear,
    Ppg,
    Rcd,
}

pub enum AnyLazy<'a, T: Primitive> {
    Linear(LinearLazy<'a, T>),
    Ppg(PpgLazy<'a, T>),
    Rcd(RcdLazy<T>),
}

impl<T: Primitive + Send + Sync> LazyDemosaic<T> for AnyLazy<'_, T> {
    fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Linear(l) => l.dimensions(),
            Self::Ppg(p) => p.dimensions(),
            Self::Rcd(r) => r.dimensions(),
        }
    }

    fn fill_rows(&self, y0: usize, out: &mut [T]) {
        match self {
            Self::Linear(l) => l.fill_rows(y0, out),
            Self::Ppg(p) => p.fill_rows(y0, out),
            Self::Rcd(r) => r.fill_rows(y0, out),
        }
    }
}

impl<T: Primitive + Send + Sync> IDemosaic<T> for Demosaic {
    #[allow(
        refining_impl_trait,
        reason = "callers holding a concrete `Demosaic` should be able to name `AnyLazy`"
    )]
    fn demosaic<'a>(
        &self,
        bayer: &'a MonoImage<T>,
        pattern: BayerPattern,
    ) -> Result<AnyLazy<'a, T>, DemosaicError> {
        Ok(match self {
            Self::Linear => AnyLazy::Linear(DemosaicLinear.lazy(bayer, pattern)?),
            Self::Ppg => AnyLazy::Ppg(DemosaicPpg.lazy(bayer, pattern)?),
            Self::Rcd => AnyLazy::Rcd(DemosaicRcd.lazy(bayer, pattern)?),
        })
    }
}
