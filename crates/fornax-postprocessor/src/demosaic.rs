mod aahd;
mod ahd;
mod amaze;
mod buffer;
mod dcb;
mod dht;
mod linear;
mod lmmse;
mod ppg;
mod rcd;
mod vng;

use std::num::NonZeroU8;

use aahd::DemosaicAahd;
use ahd::{AhdLazy, DemosaicAhd};
use amaze::DemosaicAmaze;
use dcb::DemosaicDcb;
use dht::DemosaicDht;
use fornax_core::error::DemosaicError;
use fornax_core::{BayerPattern, MonoImage, Primitive};
use linear::{DemosaicLinear, LinearLazy};
use lmmse::DemosaicLmmse;
use ppg::{DemosaicPpg, PpgLazy};
use rcd::{DemosaicRcd, RcdLazy};
use vng::DemosaicVng;

use crate::protocol::{IDemosaic, LazyDemosaic};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Demosaic {
    Aahd,
    Ahd,
    Amaze,
    Dcb {
        iterations: u8,
        enhance: bool,
    },
    Dht,
    Linear,
    Lmmse {
        mode: LmmseMode,
    },
    Ppg,
    #[default]
    Rcd,
    Vng,
    Vng4,
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum LmmseMode {
    Basic,
    Median,
    ThreeMedians,
    Refine(NonZeroU8),
}

impl Default for LmmseMode {
    fn default() -> Self { Self::Median }
}

impl Demosaic {
    pub const fn dcb() -> Self {
        Self::Dcb {
            iterations: 0,
            enhance: true,
        }
    }

    pub const fn lmmse() -> Self {
        Self::Lmmse {
            mode: LmmseMode::Median,
        }
    }
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
    Vng4(buffer::DenseLazy<T>),
}

impl<T: Primitive + Send + Sync> LazyDemosaic<T> for AnyLazy<'_, T> {
    fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Ahd(a) => a.dimensions(),
            Self::Linear(l) => l.dimensions(),
            Self::Ppg(p) => p.dimensions(),
            Self::Rcd(r) => r.dimensions(),
            Self::Vng(d)
            | Self::Aahd(d)
            | Self::Amaze(d)
            | Self::Dcb(d)
            | Self::Dht(d)
            | Self::Lmmse(d)
            | Self::Vng4(d) => d.dimensions(),
        }
    }

    fn fill_rows(&self, y0: usize, out: &mut [T]) {
        match self {
            Self::Ahd(a) => a.fill_rows(y0, out),
            Self::Linear(l) => l.fill_rows(y0, out),
            Self::Ppg(p) => p.fill_rows(y0, out),
            Self::Rcd(r) => r.fill_rows(y0, out),

            Self::Aahd(d)
            | Self::Amaze(d)
            | Self::Dcb(d)
            | Self::Dht(d)
            | Self::Lmmse(d)
            | Self::Vng(d)
            | Self::Vng4(d) => d.fill_rows(y0, out),
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
            Self::Aahd => AnyLazy::Aahd(DemosaicAahd.lazy(bayer, pattern)?),
            Self::Ahd => AnyLazy::Ahd(DemosaicAhd.lazy(bayer, pattern)?),
            Self::Amaze => AnyLazy::Amaze(DemosaicAmaze.lazy(bayer, pattern)?),
            Self::Dcb { .. } => AnyLazy::Dcb(DemosaicDcb.lazy(bayer, pattern)?),
            Self::Dht => AnyLazy::Dht(DemosaicDht.lazy(bayer, pattern)?),
            Self::Linear => AnyLazy::Linear(DemosaicLinear.lazy(bayer, pattern)?),
            Self::Lmmse { .. } => AnyLazy::Lmmse(DemosaicLmmse.lazy(bayer, pattern)?),
            Self::Ppg => AnyLazy::Ppg(DemosaicPpg.lazy(bayer, pattern)?),
            Self::Rcd => AnyLazy::Rcd(DemosaicRcd.lazy(bayer, pattern)?),
            Self::Vng => AnyLazy::Vng(DemosaicVng.lazy(bayer, pattern)?),
            Self::Vng4 => AnyLazy::Vng4(DemosaicVng.lazy(bayer, pattern)?),
        })
    }
}

#[cfg(test)]
mod rust_algorithm_tests {
    use std::num::NonZeroU8;

    use fornax_core::MonoImage;
    use generic_num::num;

    use super::*;

    const RUST_ALGORITHMS: [Demosaic; 7] = [
        Demosaic::Vng,
        Demosaic::Vng4,
        Demosaic::Dcb {
            iterations: 0,
            enhance: true,
        },
        Demosaic::Dht,
        Demosaic::Aahd,
        Demosaic::Amaze,
        Demosaic::Lmmse {
            mode: LmmseMode::Median,
        },
    ];

    #[test]
    fn all_algorithms_preserve_linear_boundary() -> mischief::Result<()> {
        fn check<T: Primitive + Send + Sync + std::fmt::Debug>(value: T) -> mischief::Result<()> {
            let image = MonoImage::from_raw(64, 64, vec![value; 64 * 64]).unwrap();
            let algorithms = [
                Demosaic::Aahd,
                Demosaic::Ahd,
                Demosaic::Amaze,
                Demosaic::Dcb {
                    iterations: 0,
                    enhance: true,
                },
                Demosaic::Dht,
                Demosaic::Linear,
                Demosaic::Lmmse {
                    mode: LmmseMode::Median,
                },
                Demosaic::Ppg,
                Demosaic::Rcd,
                Demosaic::Vng,
                Demosaic::Vng4,
            ];
            for algorithm in algorithms {
                let output = algorithm.demosaic(&image, BayerPattern::RGGB)?.collect();
                for y in 16..48 {
                    for x in 16..48 {
                        let pixel = output.get_pixel(x, y);
                        for (channel, sample) in pixel.0.iter().enumerate() {
                            assert!(
                                (*sample - value).abs() < num!(8.0e-5),
                                "{algorithm:?} channel {channel} at ({x}, {y}): {sample:?}"
                            );
                        }
                    }
                }
            }
            Ok(())
        }

        check(0.25_f32)?;
        check(0.25_f64)?;
        assert_ne!(
            LmmseMode::Basic,
            LmmseMode::Refine(NonZeroU8::new(1).unwrap())
        );
        assert_eq!(
            Demosaic::dcb(),
            Demosaic::Dcb {
                iterations: 0,
                enhance: true
            }
        );
        assert_eq!(
            Demosaic::lmmse(),
            Demosaic::Lmmse {
                mode: LmmseMode::Median
            }
        );
        Ok(())
    }

    #[test]
    fn vng4_selects_split_green_vng_behavior() -> mischief::Result<()> {
        let samples = (0..64 * 64)
            .map(|sample| sample as f64 / f64::from(64 * 64 - 1))
            .collect::<Vec<_>>();
        let image = MonoImage::from_raw(64, 64, samples).unwrap();
        for pattern in BayerPattern::ALL {
            let vng = Demosaic::Vng.demosaic(&image, pattern)?.collect();
            let vng4 = Demosaic::Vng4.demosaic(&image, pattern)?.collect();
            assert_eq!(vng, vng4, "{pattern}");
        }
        Ok(())
    }

    #[test]
    fn reference_algorithms_preserve_uniform_mosaics() -> mischief::Result<()> {
        let image = MonoImage::from_raw(64, 64, vec![0.25_f32; 64 * 64]).unwrap();
        for algorithm in RUST_ALGORITHMS {
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
        for algorithm in RUST_ALGORITHMS {
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
