//! Decoder boundary and backend input contract.
//!
//! Concrete LibRaw, rawler/dnglab, and DNC members are left to
//! `IMPL-DEC-002` through `IMPL-DEC-004`.
//!
//! # Examples
//!
//! ```
//! use fornax::decoder::IRawDecoder;
//! use fornax::error::DecodeError;
//!
//! struct Unsupported;
//!
//! impl IRawDecoder<[u8]> for Unsupported {
//!     fn decode(&self, _input: &[u8]) -> Result<fornax::DecodedRaw, DecodeError> {
//!         Err(DecodeError::Unsupported("example".into()))
//!     }
//! }
//!
//! assert!(Unsupported.decode(b"raw").is_err());
//! ```

use crate::DecodedRaw;
use crate::error::DecodeError;

// TODO(IMPL-DEC-002..004): add libraw, rawler, and dnc member modules.

/// A decoder backend that produces the universal decoded representation.
///
/// `I` is the backend input type. It may be an in-memory `[u8]` slice, a
/// filesystem path, or another source-specific input.
///
/// # Examples
///
/// ```
/// use std::path::Path;
///
/// use fornax::decoder::IRawDecoder;
///
/// fn accepts_memory_decoder(_decoder: &dyn IRawDecoder<[u8]>) {}
/// fn accepts_file_decoder(_decoder: &dyn IRawDecoder<Path>) {}
/// ```
pub trait IRawDecoder<I: ?Sized>: Send + Sync {
    /// Decodes one input into a validated [`DecodedRaw`].
    ///
    /// # Errors
    ///
    /// Returns [`DecodeError`] for malformed, unsupported, or backend-specific
    /// failures.
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
