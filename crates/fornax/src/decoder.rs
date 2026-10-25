use crate::{DecodedRaw, error::DecodeError};

pub trait IDecoder<I: ?Sized>: Send + Sync {
    fn decode(&self, input: &I) -> Result<DecodedRaw, DecodeError>;
}

#[cfg(test)]
mod tests {
    use std::path::Path;

    use super::*;

    struct Unsupported;

    impl IDecoder<[u8]> for Unsupported {
        fn decode(&self, _input: &[u8]) -> Result<DecodedRaw, DecodeError> {
            Err(DecodeError::Unsupported("test".into()))
        }
    }

    #[test]
    fn decoder_input_can_be_memory_or_a_file_path() {
        fn accepts_memory(_: &dyn IDecoder<[u8]>) {}
        fn accepts_path(_: &dyn IDecoder<Path>) {}

        let decoder = Unsupported;
        accepts_memory(&decoder);
        let _: fn(&dyn IDecoder<Path>) = accepts_path;
        assert!(decoder.decode(b"raw").is_err());
    }
}
