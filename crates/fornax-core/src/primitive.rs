use image::{ImageBuffer, Luma, Rgb};
use num_traits::Float;

/// The type of each channel in a pixel.
/// Including `u8`, `u16`, `f32`, `f64`.
pub trait Primitive: image::Primitive + image::Enlargeable + Float + Send + Sync {}

// impl Primitive for f16 {}
impl Primitive for f32 {}
impl Primitive for f64 {}
// impl Primitive for f128 {}

#[derive(Clone, Debug, PartialEq)]
pub enum MosaicData {
    /// 8-bit unsigned samples.
    U8(MonoImage<u8>),
    /// 16-bit unsigned samples.
    U16(MonoImage<u16>),
    /// 32-bit unsigned samples.
    U32(MonoImage<u32>),
    /// 32-bit floating-point samples.
    F32(MonoImage<f32>),
}
impl MosaicData {
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            MosaicData::U8(image_buffer) => image_buffer.dimensions(),
            MosaicData::U16(image_buffer) => image_buffer.dimensions(),
            MosaicData::U32(image_buffer) => image_buffer.dimensions(),
            MosaicData::F32(image_buffer) => image_buffer.dimensions(),
        }
    }
}

pub type MonoImage<T> = ImageBuffer<Luma<T>, Vec<T>>;

pub type RgbImage<T> = ImageBuffer<Rgb<T>, Vec<T>>;
