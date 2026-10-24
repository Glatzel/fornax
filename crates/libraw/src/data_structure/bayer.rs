/// A conventional 2x2 Bayer color order understood by LibRaw's `open_bayer`.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BayerPattern {
    /// Red, green, green, blue.
    RGGB,
    /// Blue, green, green, red.
    BGGR,
    /// Green, red, blue, green.
    GRBG,
    /// Green, blue, red, green.
    GBRG,
}

/// A LibRaw Bayer color index used by the camera-multiplier accessors.
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum BayerChannel {
    /// Red channel.
    R,
    /// First green channel.
    G,
    /// Blue channel.
    B,
    /// Second green channel.
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
