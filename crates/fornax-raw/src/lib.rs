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
mod decoder;
mod demosaic;
pub mod dng;
pub mod error;
mod format;
pub mod metadata;
pub mod postprocessor;
pub mod preprocessor;
mod primitive;
pub mod protocol;
pub mod raw_pipeline;
pub mod utils;

pub use bayer::{BayerChannel, BayerPattern};
pub use crop_cast::crop_and_cast;
pub use decoder::Decoder;
pub use demosaic::{AnyLazy, Demosaic, IDemosaic, LazyDemosaic, LmmseMode};
pub use error::{
    DecodeError, DemosaicError, MetadataValidationError, PostProcessError, PreprocessError,
    RawError, RawValidationError,
};
pub use format::RawFormat;
pub use fornax_core::{MonoImage, RgbImage};
pub use metadata::{EncodingMetadata, EncodingSampleFormat, ProcessMetadata};
pub use postprocessor::{OutputColorSpace, PostProcessor};
pub use preprocessor::Preprocessor;
pub use primitive::{CroppedMosaic, NormalizedMosaic, RawFloat, RawMosaic};

pub mod writer {
    //! DNG writing and compression policy.
    //!
    //! # Examples
    //!
    //! ```
    //! use fornax_raw::writer::{DngWriter, DngWriterCompression};
    //!
    //! let writer = DngWriter::new(DngWriterCompression::Lossless);
    //! assert_eq!(writer.compression(), DngWriterCompression::Lossless);
    //! ```
    //!
    //! Writer types are not re-exported from the crate root:
    //!
    //! ```compile_fail
    //! use fornax_raw::DngWriter;
    //! ```

    pub use crate::dng::writer::*;
}
