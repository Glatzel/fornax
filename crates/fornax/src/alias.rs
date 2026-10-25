use image::{ImageBuffer, Luma, Rgb};

use crate::Primitive;

pub type RgbImage<T>
where
    T: Primitive,
= ImageBuffer<Rgb<T>, Vec<T>>;

pub type LumaImage<T>
where
    T: Primitive,
= ImageBuffer<Luma<T>, Vec<T>>;
