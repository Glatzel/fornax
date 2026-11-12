pub mod preprocessor;
pub mod raw_pipeline;
pub use fornax_raw_core::{
    BayerChannel, BayerPattern, CroppedMosaic, MonoImage, NormalizedMosaic, Primitive, RawFormat,
    RawMosaic, RgbImage, error, metadata, protocol,
};

pub mod postprocessor;
pub use fornax_raw_decoder::Decoder;
pub use fornax_raw_pipeline::{
    Demosaic, IDemosaic, LazyDemosaic, OutputColorSpace, PostProcessor, Preprocessor,
};
