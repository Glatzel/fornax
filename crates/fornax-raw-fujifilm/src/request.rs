#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum FujiCfa {
    Bayer([u8; 4]),
    XTrans([u8; 36]),
}

impl FujiCfa {
    pub const fn color_at(self, row: usize, column: usize) -> u8 {
        match self {
            Self::Bayer(pattern) => pattern[(row % 2) * 2 + column % 2],
            Self::XTrans(pattern) => pattern[(row % 6) * 6 + column % 6],
        }
    }
}
