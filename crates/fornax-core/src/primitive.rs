use num_traits::Float;
/// The type of each channel in a pixel.
/// Including `u8`, `u16`, `f32`, `f64`.
pub trait Primitive: image::Primitive + image::Enlargeable + Float + Send + Sync {}

// impl Primitive for f16 {}
impl Primitive for f32 {}
impl Primitive for f64 {}
// impl Primitive for f128 {}
