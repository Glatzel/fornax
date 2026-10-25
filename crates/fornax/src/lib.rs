mod bayer;
mod decoded_raw;
pub mod decoder;
pub mod demosaic;
pub mod error;
pub mod metadata;
pub mod preprocess;
mod raw_image;

pub use bayer::BayerPattern;
pub use decoded_raw::DecodedRaw;
pub use demosaic::{RgbImageF32, demosaic};
pub use error::{DecodeError, PreprocessError, RawValidationError};
use image::{ImageBuffer, Luma};
pub use preprocess::preprocess;
pub use raw_image::RawImage;

pub type F32Plane = ImageBuffer<Luma<f32>, Vec<f32>>;
