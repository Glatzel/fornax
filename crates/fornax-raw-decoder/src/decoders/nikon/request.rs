pub use fornax_codec::{ByteOrder, DecodeRequest};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum NikonTreeId {
    Lossless12,
    Lossless14,
}

impl NikonTreeId {
    pub const fn bits(self) -> u8 {
        match self {
            Self::Lossless12 => 12,
            Self::Lossless14 => 14,
        }
    }
}
