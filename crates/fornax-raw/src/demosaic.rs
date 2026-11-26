#[cfg(test)]
mod test_support;

mod amaze;
mod buffer;
mod linear;
mod lmmse;
mod numerics;
mod ppg;
mod protocol;
mod rcd;
mod vng;

use std::num::NonZeroU8;

use amaze::DemosaicAmaze;
use linear::{DemosaicLinear, LinearLazy};
use lmmse::DemosaicLmmse;
use ppg::{DemosaicPpg, PpgLazy};
pub use protocol::{IDemosaic, LazyDemosaic};
use rcd::{DemosaicRcd, RcdLazy};
use vng::DemosaicVng;

use crate::error::DemosaicError;
use crate::{BayerPattern, MonoImage, RawFloat};

/// Selects a Bayer demosaic implementation and its behavioral parameters.
///
/// Every implementation accepts normalized scene-linear samples and returns
/// normalized linear RGB without clamping to `[0, 1]`.
///
/// # Examples
///
/// ```
/// use fornax_raw::demosaic::Demosaic;
///
/// assert_eq!(Demosaic::default(), Demosaic::rcd());
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Demosaic {
    /// Aliasing minimization and zipper elimination.
    Amaze(AmazeOptions),
    /// Bilinear interpolation.
    Linear,
    /// Linear minimum mean-square-error demosaicing.
    Lmmse(LmmseOptions),
    /// Patterned pixel grouping.
    Ppg,
    /// Ratio-corrected demosaicing.
    Rcd(RcdOptions),
    /// Variable-number-of-gradients demosaicing.
    Vng,
    /// Split-green VNG demosaicing.
    Vng4,
}

impl Default for Demosaic {
    fn default() -> Self { Self::rcd() }
}

/// Configuration for [`Demosaic::Amaze`].
///
/// # Examples
///
/// ```
/// use fornax_raw::demosaic::AmazeOptions;
///
/// assert_eq!(
///     AmazeOptions::default().tile_size,
///     AmazeOptions::DEFAULT_TILE_SIZE
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct AmazeOptions {
    /// Square tile size (including the 16-pixel border halo on each side).
    /// Must be even and at least 36.
    pub tile_size: usize,
}

impl AmazeOptions {
    /// Default square tile size (`160`).
    pub const DEFAULT_TILE_SIZE: usize = 160;

    /// Default AMaZE options (`tile_size: 160`).
    pub const DEFAULT: Self = Self {
        tile_size: Self::DEFAULT_TILE_SIZE,
    };
}

impl Default for AmazeOptions {
    fn default() -> Self { Self::DEFAULT }
}

/// Configuration for [`Demosaic::Rcd`].
///
/// # Examples
///
/// ```
/// use fornax_raw::demosaic::RcdOptions;
///
/// assert_eq!(
///     RcdOptions::default().tile_size,
///     RcdOptions::DEFAULT_TILE_SIZE
/// );
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct RcdOptions {
    /// Square tile size (including the 10-pixel border halo on each side).
    /// Must be even and at least 22.
    pub tile_size: usize,
}

impl RcdOptions {
    /// Default square tile size (`112`).
    pub const DEFAULT_TILE_SIZE: usize = 112;

    /// Default RCD options (`tile_size: 112`).
    pub const DEFAULT: Self = Self {
        tile_size: Self::DEFAULT_TILE_SIZE,
    };
}

impl Default for RcdOptions {
    fn default() -> Self { Self::DEFAULT }
}

/// Configuration for [`Demosaic::Lmmse`].
///
/// # Examples
///
/// ```
/// use fornax_raw::demosaic::{LmmseMode, LmmseOptions};
///
/// let opts = LmmseOptions::default();
/// assert_eq!(opts.mode, LmmseMode::Median);
/// assert_eq!(opts.tile_size, LmmseOptions::DEFAULT_TILE_SIZE);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub struct LmmseOptions {
    /// Post-interpolation median and refinement pass selection.
    pub mode: LmmseMode,
    /// Square tile size (including the reflected border halo on each side).
    /// Must be even and large enough for the selected `mode` halo.
    pub tile_size: usize,
}

impl LmmseOptions {
    /// Default square tile size (`136`).
    pub const DEFAULT_TILE_SIZE: usize = 136;

    /// Default LMMSE options (`mode: LmmseMode::Median`, `tile_size: 136`).
    pub const DEFAULT: Self = Self {
        mode: LmmseMode::Median,
        tile_size: Self::DEFAULT_TILE_SIZE,
    };
}

impl Default for LmmseOptions {
    fn default() -> Self { Self::DEFAULT }
}

/// Selects LMMSE median and edge-directed refinement passes.
///
/// # Examples
///
/// ```
/// use fornax_raw::demosaic::LmmseMode;
///
/// assert_eq!(LmmseMode::default(), LmmseMode::Median);
/// ```
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum LmmseMode {
    /// Directional LMMSE interpolation without median or refinement passes.
    Basic,
    /// Applies one 3x3 color-difference median pass.
    #[default]
    Median,
    /// Applies three 3x3 color-difference median passes without refinement.
    ThreeMedians,
    /// Applies three median passes followed by the given number of refinement
    /// passes.
    Refine(NonZeroU8),
}

impl Demosaic {
    /// Returns AMaZE with its default tile size (`160`).
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_raw::demosaic::{AmazeOptions, Demosaic};
    ///
    /// assert_eq!(Demosaic::amaze(), Demosaic::Amaze(AmazeOptions::DEFAULT));
    /// ```
    pub const fn amaze() -> Self { Self::Amaze(AmazeOptions::DEFAULT) }

    /// Returns RCD with its default tile size (`112`).
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_raw::demosaic::{Demosaic, RcdOptions};
    ///
    /// assert_eq!(Demosaic::rcd(), Demosaic::Rcd(RcdOptions::DEFAULT));
    /// ```
    pub const fn rcd() -> Self { Self::Rcd(RcdOptions::DEFAULT) }

    /// Returns LMMSE with its default options (`Median`, `tile_size: 136`).
    ///
    /// # Examples
    ///
    /// ```
    /// use fornax_raw::demosaic::{Demosaic, LmmseOptions};
    ///
    /// assert_eq!(Demosaic::lmmse(), Demosaic::Lmmse(LmmseOptions::DEFAULT));
    /// ```
    pub const fn lmmse() -> Self { Self::Lmmse(LmmseOptions::DEFAULT) }
}

pub enum AnyLazy<'a, T: RawFloat> {
    Amaze(buffer::DenseLazy<T>),
    Linear(LinearLazy<'a, T>),
    Lmmse(buffer::DenseLazy<T>),
    Ppg(PpgLazy<'a, T>),
    Rcd(RcdLazy<'a, T>),
    Vng(buffer::DenseLazy<T>),
    Vng4(buffer::DenseLazy<T>),
}

impl<T: RawFloat + Send + Sync> LazyDemosaic<T> for AnyLazy<'_, T> {
    fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::Linear(l) => l.dimensions(),
            Self::Ppg(p) => p.dimensions(),
            Self::Rcd(r) => r.dimensions(),
            Self::Vng(d) | Self::Amaze(d) | Self::Lmmse(d) | Self::Vng4(d) => d.dimensions(),
        }
    }

    fn fill_rows(&self, y0: usize, out: &mut [T]) {
        match self {
            Self::Linear(l) => l.fill_rows(y0, out),
            Self::Ppg(p) => p.fill_rows(y0, out),
            Self::Rcd(r) => r.fill_rows(y0, out),

            Self::Amaze(d) | Self::Lmmse(d) | Self::Vng(d) | Self::Vng4(d) => d.fill_rows(y0, out),
        }
    }

    fn collect_with(&self, post: impl Fn(usize, &mut [T]) + Sync) -> crate::RgbImage<T> {
        match self {
            Self::Linear(l) => l.collect_with(post),
            Self::Ppg(p) => p.collect_with(post),
            Self::Rcd(r) => r.collect_with(post),

            Self::Amaze(d) | Self::Lmmse(d) | Self::Vng(d) | Self::Vng4(d) => d.collect_with(post),
        }
    }
}

impl<T: RawFloat + Send + Sync> IDemosaic<T> for Demosaic {
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
            Self::Amaze(options) => {
                AnyLazy::Amaze(DemosaicAmaze.lazy_with_options(bayer, pattern, *options)?)
            }
            Self::Linear => AnyLazy::Linear(DemosaicLinear.lazy(bayer, pattern)?),
            Self::Lmmse(options) => {
                AnyLazy::Lmmse(DemosaicLmmse.lazy_with_options(bayer, pattern, *options)?)
            }
            Self::Ppg => AnyLazy::Ppg(DemosaicPpg.lazy(bayer, pattern)?),
            Self::Rcd(options) => {
                AnyLazy::Rcd(DemosaicRcd.lazy_with_options(bayer, pattern, *options)?)
            }
            Self::Vng => AnyLazy::Vng(DemosaicVng.lazy(bayer, pattern)?),
            Self::Vng4 => AnyLazy::Vng4(DemosaicVng.lazy(bayer, pattern)?),
        })
    }
}

#[cfg(test)]
mod rust_algorithm_tests {
    use std::num::NonZeroU8;

    use generic_num::num;

    use super::*;
    use crate::MonoImage;

    const RUST_ALGORITHMS: [Demosaic; 4] = [
        Demosaic::Vng,
        Demosaic::Vng4,
        Demosaic::amaze(),
        Demosaic::lmmse(),
    ];

    #[test]
    fn all_algorithms_preserve_linear_boundary() -> mischief::Result<()> {
        fn check<T>(value: T) -> mischief::Result<()>
        where
            T: RawFloat + Send + Sync + std::fmt::Debug,
        {
            let image = MonoImage::from_raw(64, 64, vec![value; 64 * 64]).unwrap();
            let algorithms = [
                Demosaic::amaze(),
                Demosaic::Linear,
                Demosaic::lmmse(),
                Demosaic::Ppg,
                Demosaic::rcd(),
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
            Demosaic::lmmse(),
            Demosaic::Lmmse(LmmseOptions {
                mode: LmmseMode::Median,
                tile_size: LmmseOptions::DEFAULT_TILE_SIZE,
            })
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
                        crate::BayerChannel::R => 0,
                        crate::BayerChannel::GRedRow | crate::BayerChannel::GBlueRow => 1,
                        crate::BayerChannel::B => 2,
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

    fn image_channel(pattern: BayerPattern, x: u32, y: u32) -> crate::BayerChannel {
        pattern.channel_at(x, y)
    }
}
