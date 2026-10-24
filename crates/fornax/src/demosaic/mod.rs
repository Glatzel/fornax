//! Demosaicing algorithms and runtime algorithm selection.
//!
//! # Examples
//!
//! ```
//! use fornax::demosaic::DemosaicAlgorithm;
//!
//! let algorithm = DemosaicAlgorithm::default();
//! assert_eq!(algorithm, DemosaicAlgorithm::Linear);
//! ```

pub mod linear;

use crate::{DemosaicError, DemosaicOptions, IDemosaic, RawImage, RgbImageF32};

/// A selectable demosaicing algorithm.
///
/// # Examples
///
/// ```
/// use fornax::demosaic::DemosaicAlgorithm;
///
/// assert_eq!(DemosaicAlgorithm::default(), DemosaicAlgorithm::Linear);
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, Hash, PartialEq, PartialOrd, Ord)]
#[cfg_attr(feature = "serde", derive(serde::Serialize))]
pub enum DemosaicAlgorithm {
    /// Bilinear interpolation across the Bayer mosaic.
    #[default]
    Linear,
}

impl IDemosaic for DemosaicAlgorithm {
    fn demosaic(
        &self,
        input: &RawImage,
        options: &DemosaicOptions,
    ) -> Result<RgbImageF32, DemosaicError> {
        match self {
            Self::Linear => linear::demosaic(input, options),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_is_the_default_algorithm() {
        assert_eq!(DemosaicAlgorithm::default(), DemosaicAlgorithm::Linear);
    }
}
