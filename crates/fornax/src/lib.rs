pub mod preprocessor;
pub use fornax_core::{
    BayerChannel, BayerPattern, DecodedRaw, MonoImage, Primitive, RawImage, RgbImage, error,
    metadata, protocol,
};
pub mod postprocessor;
pub use fornax_core::Decoder;
