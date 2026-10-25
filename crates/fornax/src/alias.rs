use image::{ImageBuffer, Luma, Rgb};

pub type RgbImage = ImageBuffer<Rgb<f32>, Vec<f32>>;
pub type LumaImage = ImageBuffer<Luma<f32>, Vec<f32>>;
