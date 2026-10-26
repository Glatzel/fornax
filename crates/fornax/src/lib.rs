pub mod decoder;
mod demosaic;
mod preprocess;
mod raw_image;

pub use demosaic::{Demosaic, IDemosaic};
pub use fornax_core::{
    BayerChannel, BayerPattern, DecodeError, DecodedRawActive, DecodedRawFull, MonoImage,
    Primitive, RawValidationError, RgbImage, metadata,
};
pub use preprocess::Preprocess;
pub use raw_image::RawImage;
