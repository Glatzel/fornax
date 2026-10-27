use image::{ImageBuffer, Luma, Rgb};

pub type RgbImage<T> = ImageBuffer<Rgb<T>, Vec<T>>;

pub type MonoImage<T> = ImageBuffer<Luma<T>, Vec<T>>;
