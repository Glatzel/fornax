//! Pure-Rust RAW decoding, preprocessing, demosaicing, and post-processing.
//!
//! The crate accepts conventional 2x2 RGB Bayer data at the processing
//! boundary and exposes native DNG and vendor decoders through one facade.
//!
//! # Examples
//!
//! ```
//! use fornax_raw::RawFormat;
//!
//! assert_eq!("cr3".parse(), Ok(RawFormat::Cr3));
//! ```

mod bayer;
mod crop_cast;
pub mod decoder;
pub mod demosaic;
pub mod dng;
pub mod error;
mod format;
pub mod metadata;
pub mod postprocessor;
pub mod preprocessor;
mod primitive;
pub mod protocol;
mod utils;

pub use bayer::{BayerChannel, BayerPattern};
pub use crop_cast::crop_and_cast;
pub use error::{
    DecodeError, DemosaicError, MetadataValidationError, PostProcessError, PreprocessError,
    RawError, RawValidationError,
};
pub use format::RawFormat;
pub use fornax_core::{MonoImage, RgbImage};
pub use metadata::{EncodingMetadata, EncodingSampleFormat, ProcessMetadata};
pub use primitive::{CroppedMosaic, NormalizedMosaic, RawFloat, RawMosaic};
