use std::fmt::{Debug, Display};

use image::Luma;

#[derive(Debug, PartialEq)]
pub enum BayerChannel {
    R,
    G,
    B,
    G2,
}

#[derive(Debug, PartialEq)]
pub enum BayerPattern {
    RGGB,
    BGGR,
    GRBG,
    GBRG,
}
impl BayerPattern {
    pub fn as_mask(&self) -> &[BayerChannel; 4] {
        match self {
            BayerPattern::RGGB => &[
                BayerChannel::R,
                BayerChannel::G,
                BayerChannel::G2,
                BayerChannel::B,
            ],
            BayerPattern::BGGR => &[
                BayerChannel::B,
                BayerChannel::G2,
                BayerChannel::G,
                BayerChannel::R,
            ],
            BayerPattern::GRBG => &[
                BayerChannel::G,
                BayerChannel::R,
                BayerChannel::B,
                BayerChannel::G2,
            ],
            BayerPattern::GBRG => &[
                BayerChannel::G2,
                BayerChannel::B,
                BayerChannel::R,
                BayerChannel::G,
            ],
        }
    }
}
impl Display for BayerPattern {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        let text = match self {
            BayerPattern::RGGB => "RGGB",
            BayerPattern::BGGR => "BGGR",
            BayerPattern::GRBG => "GRBG",
            BayerPattern::GBRG => "GBRG",
        };
        write!(f, "{}", text)
    }
}
pub struct BayerImage<T>
where
    T: BayerPrimitive,
{
    bayer_image: image::ImageBuffer<Luma<T>, Vec<T>>,
    pattern: BayerPattern,
}
impl<T> BayerImage<T>
where
    T: BayerPrimitive,
{
    pub fn new(bayer_image: image::ImageBuffer<Luma<T>, Vec<T>>, pattern: BayerPattern) -> Self {
        Self {
            bayer_image,
            pattern,
        }
    }
    pub fn mosaic(&self) -> &image::ImageBuffer<Luma<T>, Vec<T>> {
        &self.bayer_image
    }
    pub fn pattern(&self) -> &BayerPattern {
        &self.pattern
    }
}

pub trait BayerPrimitive:
    image::Primitive + image::Enlargeable + std::marker::Send + std::marker::Sync + Debug + Display
{
}
impl BayerPrimitive for usize {}
impl BayerPrimitive for u8 {}
impl BayerPrimitive for u16 {}
impl BayerPrimitive for u32 {}
impl BayerPrimitive for u64 {}

impl BayerPrimitive for isize {}
impl BayerPrimitive for i8 {}
impl BayerPrimitive for i16 {}
impl BayerPrimitive for i32 {}
impl BayerPrimitive for i64 {}
impl BayerPrimitive for f32 {}
impl BayerPrimitive for f64 {}
pub trait IBayerImage<T>
where
    T: BayerPrimitive,
{
}
