mod aahd;
mod ahd;
mod amaze;
mod buffer;
mod dcb;
mod dht;
mod linear;
mod lmmse;
mod ppg;
mod protocol;
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
pub use protocol::{IDemosaic, LazyDemosaic};
use rcd::{DemosaicRcd, RcdLazy};
use vng::DemosaicVng;

/// Selects a Bayer demosaic implementation and its behavioral parameters.
///
/// Every implementation accepts normalized scene-linear samples and returns
/// normalized linear RGB. AAHD gamma-encodes samples only for scoring; LMMSE
/// interpolates in a gamma-encoded workspace and decodes its output.
///
/// # Examples
///
/// ```
/// use fornax_raw_pipeline::Demosaic;
///
/// let demosaic = Demosaic::Dcb {
///     iterations: 1,
///     enhance: false,
/// };
/// assert!(matches!(demosaic, Demosaic::Dcb { iterations: 1, .. }));
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum Demosaic {
    /// Adaptive homogeneity-directed demosaicing using LibRaw's integer path.
    Aahd,
    /// Adaptive homogeneity-directed demosaicing.
    Ahd,
    /// Aliasing minimization and zipper elimination.
    Amaze,
    /// Directional color balance.
    Dcb {
        /// Number of Nyquist correction rounds; zero disables them.
        iterations: u8,
        /// Whether to run final refinement and full-color reconstruction.
        enhance: bool,
    },
    /// Directional homogeneity transform.
    Dht,
    /// Bilinear interpolation.
    Linear,
    /// Linear minimum mean-square-error demosaicing.
    Lmmse {
        /// Median and refinement passes from darktable's mode parameter.
        mode: LmmseMode,
    },
    /// Patterned pixel grouping.
    Ppg,
    /// Ratio-corrected demosaicing.
    #[default]
    Rcd,
    /// Variable-number-of-gradients demosaicing.
    Vng,
    /// Split-green VNG demosaicing.
    Vng4,
}

/// Selects darktable-compatible LMMSE median and refinement passes.
///
/// # Examples
///
/// ```
/// use fornax_raw_pipeline::LmmseMode;
///
/// assert_eq!(LmmseMode::default(), LmmseMode::Median);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LmmseMode {
    /// Uses the PPG seed and interpolation without a median or refinement pass.
    Basic,
    /// Applies one median pass, matching darktable mode 1.
    #[default]
    Median,
    /// Applies three median passes without refinement.
    ThreeMedians,
    /// Applies three median passes followed by the given refinement passes.
    Refine(NonZeroU8),
}

impl Demosaic {
    /// Returns DCB with the default `(iterations, enhance)` values `(0, true)`.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_raw_pipeline::Demosaic;
    ///
    /// assert_eq!(
    ///     Demosaic::dcb(),
    ///     Demosaic::Dcb {
    ///         iterations: 0,
    ///         enhance: true,
    ///     }
    /// );
    /// ```
    pub const fn dcb() -> Self {
        Self::Dcb {
            iterations: 0,
            enhance: true,
        }
    }

    /// Returns LMMSE with its default one-median-pass mode.
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_raw_pipeline::{Demosaic, LmmseMode};
    ///
    /// assert_eq!(
    ///     Demosaic::lmmse(),
    ///     Demosaic::Lmmse {
    ///         mode: LmmseMode::Median,
    ///     }
    /// );
    /// ```
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
            Self::Dcb {
                iterations,
                enhance,
            } => AnyLazy::Dcb(DemosaicDcb.lazy_with_options(
                bayer,
                pattern,
                *iterations,
                *enhance,
            )?),
            Self::Dht => AnyLazy::Dht(DemosaicDht.lazy(bayer, pattern)?),
            Self::Linear => AnyLazy::Linear(DemosaicLinear.lazy(bayer, pattern)?),
            Self::Lmmse { mode } => {
                AnyLazy::Lmmse(DemosaicLmmse.lazy_with_mode(bayer, pattern, *mode)?)
            }
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
        fn check<T>(value: T) -> mischief::Result<()>
        where
            T: Primitive + Send + Sync + std::fmt::Debug,
        {
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
