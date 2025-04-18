mod bayer;
mod decoder;
mod post_processor;
use std::fmt::{Debug, Display};

pub use bayer::{BayerChannel, BayerImage, BayerPattern, IBayerImage};
pub use decoder::IDecoder;
pub use post_processor::{IPostProcessor, NullPostProcessor};

pub trait FornaxPrimitive:
    image::PrimitiveExt + std::marker::Send + std::marker::Sync + Debug + Display + 'static + Sized
{
}
// impl FornaxPrimitive for usize {}
impl FornaxPrimitive for u8 {}
impl FornaxPrimitive for u16 {}
// impl FornaxPrimitive for u32 {}
// impl FornaxPrimitive for u64 {}

// impl FornaxPrimitive for isize {}
// impl FornaxPrimitive for i8 {}
// impl FornaxPrimitive for i16 {}
// impl FornaxPrimitive for i32 {}
// impl FornaxPrimitive for i64 {}
impl FornaxPrimitive for f32 {}
impl FornaxPrimitive for f64 {}
