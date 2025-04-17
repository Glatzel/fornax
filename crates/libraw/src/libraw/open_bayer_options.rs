pub enum ProcFlag {
    _10bit4PixelsIn5Bytes,
    _10bit6PixelsIn8Bytes,
    BigEndianData,
}
impl From<ProcFlag> for u8 {
    fn from(value: ProcFlag) -> Self {
        match value {
            ProcFlag::_10bit4PixelsIn5Bytes => 1,
            ProcFlag::_10bit6PixelsIn8Bytes => 0,
            ProcFlag::BigEndianData => 1,
        }
    }
}
