use std::fmt::Debug;

use fornax_core::{MonoImage, RgbImage};
use num_traits::Float;

use crate::error::{PostProcessError, PreprocessError};
use crate::metadata::Levels;
use crate::protocol::{IPostProcessMetadata, IPostProcessor, IPreprocessor};

/// A floating-point image sample type.
///
/// The trait intentionally includes [`num_traits::Float`], so integer image
/// types cannot enter processing stages that require arithmetic, comparisons,
/// and mathematical functions.
pub trait RawFloat:
    image::Primitive + image::Enlargeable + Float + bytemuck::Zeroable + Send + Sync + Debug + 'static
{
}

// impl Primitive for f16 {}
impl RawFloat for f32 {}
impl RawFloat for f64 {}
// impl Primitive for f128 {}

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
pub struct CroppedMosaic<T: RawFloat>(MonoImage<T>);

impl<T: RawFloat> CroppedMosaic<T> {
    pub const fn new(image: MonoImage<T>) -> Self { Self(image) }

    pub const fn image(&self) -> &MonoImage<T> { &self.0 }

    pub fn into_image(self) -> MonoImage<T> { self.0 }

    pub fn process<P>(
        self,
        preprocessor: &P,
        levels: &Levels,
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
    T: RawFloat;

impl<T> NormalizedMosaic<T>
where
    T: RawFloat,
{
    pub const fn new(plane: MonoImage<T>) -> Self { Self(plane) }
    pub const fn image(&self) -> &MonoImage<T> { &self.0 }
    pub fn into_image(self) -> MonoImage<T> { self.0 }
    pub fn process<P, M>(self, processor: &P, metadata: &M) -> Result<RgbImage<T>, PostProcessError>
    where
        P: IPostProcessor<T, M>,
        M: IPostProcessMetadata,
    {
        processor.process(self.0, metadata)
    }
}
