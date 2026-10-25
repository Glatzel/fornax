#[derive(Clone, Copy, Debug, Eq, Hash, PartialEq, PartialOrd, Ord)]
pub enum BayerPattern {
    Rggb,
    Bggr,
    Grbg,
    Gbrg,
}

impl BayerPattern {
    pub(crate) const fn from_codes(codes: [u8; 4]) -> Option<Self> {
        match codes {
            [0, 1, 3, 2] => Some(Self::Rggb),
            [2, 3, 1, 0] => Some(Self::Bggr),
            [1, 0, 2, 3] => Some(Self::Grbg),
            [3, 2, 0, 1] => Some(Self::Gbrg),
            _ => None,
        }
    }

    pub(crate) const fn codes(self) -> [u8; 4] {
        match self {
            Self::Rggb => [0, 1, 3, 2],
            Self::Bggr => [2, 3, 1, 0],
            Self::Grbg => [1, 0, 2, 3],
            Self::Gbrg => [3, 2, 0, 1],
        }
    }

    pub(crate) const fn shift(mut self, mut rows: u32, mut columns: u32) -> Self {
        while rows & 1 != 0 {
            let [r, g1, g2, b] = self.codes();
            self = Self::from_codes([g2, b, r, g1]).expect("row shift preserves Bayer layout");
            rows -= 1;
        }
        while columns & 1 != 0 {
            let [r, g1, g2, b] = self.codes();
            self = Self::from_codes([g1, r, b, g2]).expect("column shift preserves Bayer layout");
            columns -= 1;
        }
        self
    }
}

impl core::fmt::Display for BayerPattern {
    fn fmt(&self, formatter: &mut core::fmt::Formatter<'_>) -> core::fmt::Result {
        formatter.write_str(match self {
            Self::Rggb => "RGGB",
            Self::Bggr => "BGGR",
            Self::Grbg => "GRBG",
            Self::Gbrg => "GBRG",
        })
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn shifts_all_phase_combinations() {
        assert_eq!(BayerPattern::Rggb.shift(0, 0), BayerPattern::Rggb);
        assert_eq!(BayerPattern::Rggb.shift(1, 0), BayerPattern::Gbrg);
        assert_eq!(BayerPattern::Rggb.shift(0, 1), BayerPattern::Grbg);
        assert_eq!(BayerPattern::Rggb.shift(1, 1), BayerPattern::Bggr);
    }
}
