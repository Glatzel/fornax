mod ahd;
mod buffer;
mod linear;
mod ppg;
mod rcd;
mod reference;

use ahd::{AhdLazy, DemosaicAhd};
use fornax_core::error::DemosaicError;
use fornax_core::protocol::{IDemosaic, LazyDemosaic};
use fornax_core::{BayerPattern, MonoImage, Primitive};
use fornax_demosaic_reference::Algorithm as ReferenceAlgorithm;
use linear::{DemosaicLinear, LinearLazy};
use ppg::{DemosaicPpg, PpgLazy};
use rcd::{DemosaicRcd, RcdLazy};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Demosaic {
    Aahd,
    Ahd,
    Amaze,
    Dcb,
    Dht,
    Linear,
    Lmmse,
    Ppg,
    #[default]
    Rcd,
    Vng,
}

pub enum AnyLazy<'a, T: Primitive> {
    Aahd(buffer::DenseLazy<T>),
    Ahd(AhdLazy<T>),
    Amaze(buffer::DenseLazy<T>),
    Dcb(buffer::DenseLazy<T>),
    Dht(buffer::DenseLazy<T>),
    Linear(LinearLazy<'a, T>),
    Lmmse(buffer::DenseLazy<T>),
    Ppg(PpgLazy<'a, T>),
    Rcd(RcdLazy<T>),
    Vng(buffer::DenseLazy<T>),
}

impl<T: Primitive + Send + Sync> LazyDemosaic<T> for AnyLazy<'_, T> {
    fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Aahd(a) => a.dimensions(),
            Self::Ahd(a) => a.dimensions(),
            Self::Amaze(a) => a.dimensions(),
            Self::Dcb(d) => d.dimensions(),
            Self::Dht(d) => d.dimensions(),
            Self::Linear(l) => l.dimensions(),
            Self::Lmmse(l) => l.dimensions(),
            Self::Ppg(p) => p.dimensions(),
            Self::Rcd(r) => r.dimensions(),
            Self::Vng(v) => v.dimensions(),
        }
    }

    fn fill_rows(&self, y0: usize, out: &mut [T]) {
        match self {
            Self::Aahd(a) => a.fill_rows(y0, out),
            Self::Ahd(a) => a.fill_rows(y0, out),
            Self::Amaze(a) => a.fill_rows(y0, out),
            Self::Dcb(d) => d.fill_rows(y0, out),
            Self::Dht(d) => d.fill_rows(y0, out),
            Self::Linear(l) => l.fill_rows(y0, out),
            Self::Lmmse(l) => l.fill_rows(y0, out),
            Self::Ppg(p) => p.fill_rows(y0, out),
            Self::Rcd(r) => r.fill_rows(y0, out),
            Self::Vng(v) => v.fill_rows(y0, out),
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
            Self::Aahd => AnyLazy::Aahd(reference::lazy(ReferenceAlgorithm::Aahd, bayer, pattern)?),
            Self::Ahd => AnyLazy::Ahd(DemosaicAhd.lazy(bayer, pattern)?),
            Self::Amaze => {
                AnyLazy::Amaze(reference::lazy(ReferenceAlgorithm::Amaze, bayer, pattern)?)
            }
            Self::Dcb => AnyLazy::Dcb(reference::lazy(ReferenceAlgorithm::Dcb, bayer, pattern)?),
            Self::Dht => AnyLazy::Dht(reference::lazy(ReferenceAlgorithm::Dht, bayer, pattern)?),
            Self::Linear => AnyLazy::Linear(DemosaicLinear.lazy(bayer, pattern)?),
            Self::Lmmse => {
                AnyLazy::Lmmse(reference::lazy(ReferenceAlgorithm::Lmmse, bayer, pattern)?)
            }
            Self::Ppg => AnyLazy::Ppg(DemosaicPpg.lazy(bayer, pattern)?),
            Self::Rcd => AnyLazy::Rcd(DemosaicRcd.lazy(bayer, pattern)?),
            Self::Vng => AnyLazy::Vng(reference::lazy(ReferenceAlgorithm::Vng, bayer, pattern)?),
        })
    }
}

#[cfg(test)]
mod reference_tests {
    use fornax_core::MonoImage;

    use super::*;

    const REFERENCE_ALGORITHMS: [Demosaic; 6] = [
        Demosaic::Vng,
        Demosaic::Dcb,
        Demosaic::Dht,
        Demosaic::Aahd,
        Demosaic::Amaze,
        Demosaic::Lmmse,
    ];

    #[test]
    fn reference_algorithms_preserve_uniform_mosaics() -> mischief::Result<()> {
        let image = MonoImage::from_raw(64, 64, vec![0.25_f32; 64 * 64]).unwrap();
        for algorithm in REFERENCE_ALGORITHMS {
            let output = algorithm.demosaic(&image, BayerPattern::RGGB)?.collect();
            for y in 16..48 {
                for x in 16..48 {
                    assert!(
                        output
                            .get_pixel(x, y)
                            .0
                            .iter()
                            .all(|sample| (*sample - 0.25).abs() < 8.0e-5),
                        "{algorithm:?} at ({x}, {y}): {:?}",
                        output.get_pixel(x, y)
                    );
                }
            }
        }
        Ok(())
    }

    #[test]
    fn reference_algorithms_preserve_measured_bayer_samples() -> mischief::Result<()> {
        let samples = (0..64 * 64)
            .map(|sample| sample as f64 / f64::from(64 * 64 - 1))
            .collect::<Vec<_>>();
        let image = MonoImage::from_raw(64, 64, samples).unwrap();
        for algorithm in REFERENCE_ALGORITHMS {
            let output = algorithm.demosaic(&image, BayerPattern::GRBG)?.collect();
            for y in 16..48 {
                for x in 16..48 {
                    let channel = match image_channel(BayerPattern::GRBG, x, y) {
                        fornax_core::BayerChannel::R => 0,
                        fornax_core::BayerChannel::GRedRow
                        | fornax_core::BayerChannel::GBlueRow => 1,
                        fornax_core::BayerChannel::B => 2,
                    };
                    assert!(
                        (output.get_pixel(x, y)[channel] - image.get_pixel(x, y)[0]).abs() < 2.0e-5,
                        "{algorithm:?} at ({x}, {y})"
                    );
                }
            }
        }
        Ok(())
    }

    fn image_channel(pattern: BayerPattern, x: u32, y: u32) -> fornax_core::BayerChannel {
        pattern.channel_at(x, y)
    }
}
