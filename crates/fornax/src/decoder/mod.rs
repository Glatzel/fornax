//! Decoder boundary and backend input contract.
//!
//! Concrete rawler/dnglab and DNC members are left to `IMPL-DEC-004` and
//! `IMPL-DEC-005`.
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

#[cfg(feature = "backend-libraw")]
mod libraw;
// TODO(IMPL-DEC-004..005): add rawler and dnc member modules.

#[cfg(feature = "backend-libraw")]
pub use libraw::LibRawDecoder;

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
