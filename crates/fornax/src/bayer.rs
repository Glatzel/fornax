use std::fmt::Display;

/// An enum representing the channels in a Bayer pattern.
///
/// The `BayerChannel` enum defines the four possible color channels in a Bayer
/// pattern:
/// - `R` for Red,
/// - `G` for Green (first channel),
/// - `B` for Blue,
/// - `G2` for Green (second channel).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord)]
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
    pub const fn as_mask(&self) -> &[BayerChannel; 4] {
        match self {
            Self::RGGB => &[
                BayerChannel::R,
                BayerChannel::G,
                BayerChannel::G2,
                BayerChannel::B,
            ],
            Self::BGGR => &[
                BayerChannel::B,
                BayerChannel::G2,
                BayerChannel::G,
                BayerChannel::R,
            ],
            Self::GRBG => &[
                BayerChannel::G,
                BayerChannel::R,
                BayerChannel::B,
                BayerChannel::G2,
            ],
            Self::GBRG => &[
                BayerChannel::G2,
                BayerChannel::B,
                BayerChannel::R,
                BayerChannel::G,
            ],
        }
    }
}
impl Display for BayerPattern {
    fn fmt(&self, f: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        let text = match self {
            Self::RGGB => "RGGB",
            Self::BGGR => "BGGR",
            Self::GRBG => "GRBG",
            Self::GBRG => "GBRG",
        };
        write!(f, "{text}")
    }
}
