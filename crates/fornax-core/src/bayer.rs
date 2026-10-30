use num_enum::IntoPrimitive;
use strum::Display;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, IntoPrimitive)]
#[repr(u8)]
pub enum BayerChannel {
    R = 0,
    GRedRow = 1,
    B = 2,
    GBlueRow = 3,
}

impl BayerChannel {
    /// The channel's stable index: `R = 0`, `G = 1`, `B = 2`, `G2 = 3`.
    #[inline(always)]
    pub const fn index(self) -> usize { self as usize }

    /// `true` for both `G` and `G2`.
    #[inline(always)]
    pub const fn is_green(self) -> bool { matches!(self, Self::GRedRow | Self::GBlueRow) }
}

/// Common Bayer patterns used in image sensors, named after the 2x2 tile read
/// row by row from the top-left pixel:
/// - `RGGB`: Red, Green, Green, Blue,
/// - `BGGR`: Blue, Green, Green, Red,
/// - `GRBG`: Green, Red, Blue, Green,
/// - `GBRG`: Green, Blue, Red, Green.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Display)]
pub enum BayerPattern {
    #[strum(serialize = "RGGB")]
    RGGB,
    #[strum(serialize = "BGGR")]
    BGGR,
    #[strum(serialize = "GRBG")]
    GRBG,
    #[strum(serialize = "GBRG")]
    GBRG,
}
impl BayerPattern {
    /// Every pattern, handy for exhaustive tests and benchmarks.
    pub const ALL: [Self; 4] = [Self::RGGB, Self::BGGR, Self::GRBG, Self::GBRG];

    /// The channels of the 2x2 tile in row-major order.
    ///
    /// Slot `(x & 1) + 2 * (y & 1)` is the channel at pixel `(x, y)`.
    pub const fn as_mask(&self) -> &[BayerChannel; 4] {
        match self {
            Self::RGGB => &[
                BayerChannel::R,
                BayerChannel::GRedRow,
                BayerChannel::GBlueRow,
                BayerChannel::B,
            ],
            Self::BGGR => &[
                BayerChannel::B,
                BayerChannel::GBlueRow,
                BayerChannel::GRedRow,
                BayerChannel::R,
            ],
            Self::GRBG => &[
                BayerChannel::GRedRow,
                BayerChannel::R,
                BayerChannel::B,
                BayerChannel::GBlueRow,
            ],
            Self::GBRG => &[
                BayerChannel::GBlueRow,
                BayerChannel::B,
                BayerChannel::R,
                BayerChannel::GRedRow,
            ],
        }
    }

    /// The channel at pixel `(x, y)` of an image using this pattern.
    #[inline(always)]
    pub const fn channel_at(&self, x: u32, y: u32) -> BayerChannel {
        self.as_mask()[((x & 1) + 2 * (y & 1)) as usize]
    }

    /// The pattern seen from a window whose origin is offset by `(dx, dy)`.
    ///
    /// Only the parity of the offsets matters. Use this when demosaicing a
    /// crop that starts at odd coordinates:
    /// `p.shifted(dx, dy).channel_at(x, y) == p.channel_at(x + dx, y + dy)`.
    pub const fn shifted(self, dx: u32, dy: u32) -> Self {
        match self.channel_at(dx, dy) {
            BayerChannel::R => Self::RGGB,
            BayerChannel::GRedRow => Self::GRBG,
            BayerChannel::GBlueRow => Self::GBRG,
            BayerChannel::B => Self::BGGR,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn channel_indices_are_stable() {
        assert_eq!(u8::from(BayerChannel::R), 0);
        assert_eq!(u8::from(BayerChannel::GRedRow), 1);
        assert_eq!(u8::from(BayerChannel::B), 2);
        assert_eq!(u8::from(BayerChannel::GBlueRow), 3);
        assert_eq!(BayerChannel::GBlueRow.index(), 3);
    }

    #[test]
    fn channel_at_matches_mask() {
        for p in BayerPattern::ALL {
            let m = p.as_mask();
            for y in 0..6 {
                for x in 0..6 {
                    assert_eq!(p.channel_at(x, y), m[((x & 1) + 2 * (y & 1)) as usize]);
                }
            }
        }
    }

    #[test]
    fn green_is_named_by_row() {
        for p in BayerPattern::ALL {
            for y in 0..4 {
                for x in 0..4 {
                    let row_has_red = (0..2).any(|i| p.channel_at(i, y) == BayerChannel::R);
                    match p.channel_at(x, y) {
                        BayerChannel::GRedRow => assert!(row_has_red, "{p} ({x},{y})"),
                        BayerChannel::GBlueRow => assert!(!row_has_red, "{p} ({x},{y})"),
                        _ => {}
                    }
                }
            }
        }
    }

    #[test]
    fn shifted_matches_offset_lookup() {
        for p in BayerPattern::ALL {
            for dy in 0..4 {
                for dx in 0..4 {
                    let s = p.shifted(dx, dy);
                    for y in 0..4 {
                        for x in 0..4 {
                            assert_eq!(s.channel_at(x, y), p.channel_at(x + dx, y + dy));
                        }
                    }
                }
            }
        }
    }
}
