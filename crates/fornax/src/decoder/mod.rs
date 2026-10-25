use crate::DecodedRaw;
use crate::error::DecodeError;

#[cfg(feature = "backend-libraw")]
mod libraw;
// TODO(IMPL-DEC-004..005): add rawler and dnc member modules.

#[cfg(feature = "backend-libraw")]
pub use libraw::LibRawDecoder;

pub trait IRawDecoder<I: ?Sized>: Send + Sync {
    fn decode(&self, input: &I) -> Result<DecodedRaw, DecodeError>;
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    struct Unsupported;

    impl IRawDecoder<[u8]> for Unsupported {
        fn decode(&self, _input: &[u8]) -> Result<DecodedRaw, DecodeError> {
            Err(DecodeError::Unsupported("test".into()))
        }
    }

    #[test]
    fn decoder_input_can_be_memory_or_a_file_path() {
        fn accepts_memory(_: &dyn IRawDecoder<[u8]>) {}
        fn accepts_path(_: &dyn IRawDecoder<Path>) {}

        let decoder = Unsupported;
        accepts_memory(&decoder);
        let _: fn(&dyn IRawDecoder<Path>) = accepts_path;
        assert!(decoder.decode(b"raw").is_err());
    }
}
