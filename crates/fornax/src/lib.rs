pub mod decoder;
mod preprocess;
mod demosaic;
mod raw_image;

pub use preprocess::Preprocess;
pub use demosaic::{Demosaic, IDemosaic};
pub use fornax_core::{
    BayerChannel, BayerPattern, MonoImage, Primitive, RawValidationError, RgbImage,
};
pub use fornax_decoder_protocol::{DecodeError, DecodedRawActive, DecodedRawFull};
pub use fornax_metadata::{Levels, Metadata};
pub use raw_image::RawImage;

pub mod metadata {
    pub use fornax_metadata::*;
}
