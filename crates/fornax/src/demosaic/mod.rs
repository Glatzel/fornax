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

use image::{ImageBuffer, Rgb};

use crate::RawImage;
use crate::error::DemosaicError;

/// Linear interleaved RGB `f32` output backed by image-rs.
///
/// # Examples
///
/// ```
/// use fornax::demosaic::RgbImageF32;
///
/// let image = RgbImageF32::from_raw(1, 1, vec![0.1, 0.2, 0.3]).unwrap();
/// assert_eq!(image.as_raw(), &[0.1, 0.2, 0.3]);
/// ```
pub type RgbImageF32 = ImageBuffer<Rgb<f32>, Vec<f32>>;

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

impl DemosaicAlgorithm {
    /// Reconstructs linear interleaved RGB from one preprocessed Bayer image.
    ///
    /// # Errors
    ///
    /// Returns [`DemosaicError`] when the image cannot be reconstructed or the
    /// selected algorithm fails.
    pub(crate) fn demosaic(&self, input: &RawImage) -> Result<RgbImageF32, DemosaicError> {
        match self {
            Self::Linear => linear::demosaic(input),
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
