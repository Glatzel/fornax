//! Rust-native RAW decoding, preprocessing, and demosaicing.
//!
//! [`DecodedRaw`] combines decoder samples with immutable validated metadata
//! and provenance. Once its TODO substeps are implemented, standard
//! preprocessing will consume it and return the distinct, f32-only [`RawImage`]
//! type, so corrections cannot silently run twice.
//!
//! # Examples
//!
//! ```
//! use fornax::geometry::{ActiveArea, PixelCoord, PixelSize};
//! use fornax::{BayerLayout, BayerPattern, CfaColor};
//!
//! let area = ActiveArea::try_new(PixelSize::new(4, 4).unwrap(), 1, 1, 4, 4).unwrap();
//! let layout = BayerLayout::new(BayerPattern::Rggb, area);
//! assert_eq!(layout.color_at(PixelCoord::new(1, 1)), Ok(CfaColor::Red));
//! ```

#![warn(missing_docs)]

mod cfa;
mod decoded_raw;
pub mod decoder;
pub mod demosaic;
pub mod error;
pub mod geometry;
pub mod metadata;
mod plane;
mod raw_image;

pub use cfa::{BayerLayout, BayerPattern, BayerSlot, CfaColor, CfaShape};
pub use decoded_raw::DecodedRaw;
pub use demosaic::{DemosaicAlgorithm, RgbImageF32};
use error::{MetadataError, RawValidationError};
use geometry::{ActiveArea, PixelSize, StoredGeometry};
pub use plane::{F32Plane, SampleFormat, SampleSpec, StoredPlane, U8Plane, U16Plane, U32Plane};
pub use raw_image::{RawImage, into_preprocessed};
