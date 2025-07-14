use std::fmt::{Debug, Display};

use image::Luma;
/// An enum representing the channels in a Bayer pattern.
///
/// The `BayerChannel` enum defines the four possible color channels in a Bayer
/// pattern:
/// - `R` for Red,
/// - `G` for Green (first channel),
/// - `B` for Blue,
/// - `G2` for Green (second channel).
#[derive(Debug, PartialEq)]
pub enum BayerChannel {
    R,
    G,
    B,
    G2,
}
impl From<BayerChannel> for u8 {
    fn from(value: BayerChannel) -> Self {
        match value {
            BayerChannel::R => 0,
            BayerChannel::G => 1,
            BayerChannel::B => 2,
            BayerChannel::G2 => 3,
        }
    }
}
/// An enum representing common Bayer patterns used in image sensors.
///
/// The `BayerPattern` enum defines the layout of color channels in a Bayer
/// filter:
/// - `RGGB`: Red, Green, Green, Blue,
/// - `BGGR`: Blue, Green, Green, Red,
/// - `GRBG`: Green, Red, Blue, Green,
/// - `GBRG`: Green, Blue, Red, Green.
///
/// The order of the channels affects how the image is processed and decoded.
#[derive(Debug, PartialEq)]
pub enum BayerPattern {
    RGGB,
    BGGR,
    GRBG,
    GBRG,
}
impl BayerPattern {
    /// Returns the color channel mask associated with the Bayer pattern.
    ///
    /// The mask is an ordered list of `BayerChannel` values, representing the
    /// sequence of color channels in the pattern.
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
        write!(f, "{text}")
    }
}

/// A struct that represents a Bayer image and its associated Bayer pattern.
///
/// `BayerImage` contains the raw image data in a Bayer pattern along with the
/// `BayerPattern` that specifies how the color channels are arranged in the
/// image. The image data is stored as a single-channel grayscale (`Luma`)
/// image, where each pixel contains a value from one of the Bayer pattern's
/// color channels.
///
/// # Type Parameters
/// - `T`: The type of the pixel data in the image (e.g., `u8`, `f32`). It must
///   implement the `FornaxPrimitive` trait.
///
/// # Methods
/// - `new(bayer_image: image::ImageBuffer<Luma<T>, Vec<T>>, pattern:
///   BayerPattern)`: Creates a new `BayerImage` with the provided image data
///   and pattern.
/// - `mosaic(&self)`: Returns the raw Bayer image data.
/// - `pattern(&self)`: Returns the Bayer pattern associated with the image.
pub struct BayerImage<T>
where
    T: crate::FornaxPrimitive,
{
    bayer_image: image::ImageBuffer<Luma<T>, Vec<T>>,
    pattern: BayerPattern,
}
impl<T> BayerImage<T>
where
    T: crate::FornaxPrimitive,
{
    /// Creates a new `BayerImage` from the given image data and Bayer pattern.
    ///
    /// # Arguments
    /// - `bayer_image`: The raw image data in the Bayer pattern.
    /// - `pattern`: The Bayer pattern that specifies the arrangement of color
    ///   channels.
    ///
    /// # Returns
    /// A new `BayerImage` instance.
    pub fn new(bayer_image: image::ImageBuffer<Luma<T>, Vec<T>>, pattern: BayerPattern) -> Self {
        Self {
            bayer_image,
            pattern,
        }
    }
    /// Returns a reference to the raw Bayer image data.
    ///
    /// This method provides access to the original Bayer image data in the form
    /// of a grayscale image (`Luma<T>`), where each pixel value represents
    /// a color channel from the Bayer pattern.
    pub fn mosaic(&self) -> &image::ImageBuffer<Luma<T>, Vec<T>> { &self.bayer_image }
    /// Returns a reference to the Bayer pattern associated with the image.
    ///
    /// This method allows you to access the pattern used to arrange the color
    /// channels in the Bayer image.
    pub fn pattern(&self) -> &BayerPattern { &self.pattern }
}
