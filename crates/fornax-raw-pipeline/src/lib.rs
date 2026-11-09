//! RAW pipeline stages after decoding and before output encoding.
//!
//! The crate owns black/white normalization, Bayer demosaicing, and optional
//! RGB color processing.

mod demosaic;
mod postprocessor;
mod preprocessor;

pub use demosaic::{AnyLazy, Demosaic, IDemosaic, LazyDemosaic, LmmseMode};
pub use postprocessor::{OutputColorSpace, PostProcessor};
pub use preprocessor::Preprocessor;
