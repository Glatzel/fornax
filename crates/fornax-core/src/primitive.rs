use std::fmt::Debug;

use image::{ImageBuffer, Luma, Rgb};
use num_traits::Float;

use crate::error::{PostProcessError, PreprocessError};
use crate::metadata::Levels;
use crate::protocol::{IPostProcessMetadata, IPostProcessor, IPreprocessor};

/// The type of each channel in a pixel.
/// Including `u8`, `u16`, `f32`, `f64`.
pub trait Primitive: image::Primitive + image::Enlargeable + Float + Send + Sync + Debug {}

// impl Primitive for f16 {}
impl Primitive for f32 {}
impl Primitive for f64 {}
// impl Primitive for f128 {}
//
pub type MonoImage<T> = ImageBuffer<Luma<T>, Vec<T>>;

pub type RgbImage<T> = ImageBuffer<Rgb<T>, Vec<T>>;

#[derive(Clone, Debug, PartialEq)]
pub enum RawMosaic {
    /// 8-bit unsigned samples.
    U8(MonoImage<u8>),
    /// 16-bit unsigned samples.
    U16(MonoImage<u16>),
    /// 32-bit unsigned samples.
    U32(MonoImage<u32>),
    /// 32-bit floating-point samples.
    F32(MonoImage<f32>),
}
impl RawMosaic {
    pub fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::U8(image_buffer) => image_buffer.dimensions(),
            Self::U16(image_buffer) => image_buffer.dimensions(),
            Self::U32(image_buffer) => image_buffer.dimensions(),
            Self::F32(image_buffer) => image_buffer.dimensions(),
        }
    }
}

#[derive(Clone, Debug)]
pub struct CroppedMosaic<T: Primitive>(MonoImage<T>);

impl<T: Primitive> CroppedMosaic<T> {
    pub const fn new(image: MonoImage<T>) -> Self { Self(image) }

    pub const fn image(&self) -> &MonoImage<T> { &self.0 }

    pub fn into_image(self) -> MonoImage<T> { self.0 }

    pub fn process<P>(
        self,
        preprocessor: &P,
        levels: &Levels<T>,
    ) -> Result<NormalizedMosaic<T>, PreprocessError>
    where
        P: IPreprocessor<T>,
    {
        preprocessor.process(self, levels)
    }
}

#[derive(Clone, Debug, PartialEq)]
pub struct NormalizedMosaic<T>(MonoImage<T>)
where
    T: Primitive;

impl<T> NormalizedMosaic<T>
where
    T: Primitive,
{
    pub const fn new(plane: MonoImage<T>) -> Self { Self(plane) }
    pub const fn image(&self) -> &MonoImage<T> { &self.0 }
    pub fn into_image(self) -> MonoImage<T> { self.0 }
    pub fn process<P, M>(self, processor: &P, metadata: &M) -> Result<RgbImage<T>, PostProcessError>
    where
        P: IPostProcessor<T, M>,
        M: IPostProcessMetadata<T>,
    {
        processor.process(self.image(), metadata)
    }
}
