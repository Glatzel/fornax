use core::fmt::{Debug, Display};
/// The type of each channel in a pixel.
pub trait Primitive:
    image::Primitive
    + image::Enlargeable
    + core::marker::Send
    + core::marker::Sync
    + Debug
    + Display
    + 'static
{
}

// impl Primitive for f16 {}
impl Primitive for f32 {}
impl Primitive for f64 {}
// impl Primitive for f128{}
