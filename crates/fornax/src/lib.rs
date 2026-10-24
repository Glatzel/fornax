//! Rust-native RAW decoding, preprocessing, and demosaicing.
//!
//! [`DecodedRaw`] combines decoder samples with immutable validated metadata
//! and provenance. Standard preprocessing consumes it and returns the distinct,
//! f32-only [`RawImage`] type, so corrections cannot silently run twice.
//!
//! # Examples
//!
//! ```
//! use fornax::{ActiveArea, BayerLayout, BayerPattern, CfaColor, PixelCoord, PixelSize};
//!
//! let area = ActiveArea::try_new(PixelSize::new(4, 4).unwrap(), 1, 1, 4, 4).unwrap();
//! let layout = BayerLayout::new(BayerPattern::Rggb, area);
//! assert_eq!(layout.color_at(PixelCoord::new(1, 1)), Ok(CfaColor::Red));
//! ```

#![warn(missing_docs)]

pub mod model;
pub mod protocol;

pub use model::*;
pub use protocol::{
    DecodeConfig, DecodeError, DemosaicError, DemosaicOptions, IDemosaic, IRawDecoder,
    IRawPreprocessor, PreprocessError, PreprocessOptions, RawInput, RgbImageF32,
};
