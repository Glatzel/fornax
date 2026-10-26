mod native;
pub use native::*;

use std::path::Path;

use fornax_core::Primitive;
use fornax_decoder_protocol::{DecodeError, DecodedRawActive, DecodedRawFull, IDecoder};
use fornax_metadata::Metadata;

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LibRawDecoder;

impl<T> IDecoder<Path, T> for LibRawDecoder
where
    T: Primitive,
{
    fn decode_full(&self, input: &Path) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError> {
        todo!()
    }

    fn decode_active(
        &self,
        input: &Path,
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError> {
        todo!()
    }
}

impl<T> IDecoder<[u8], T> for LibRawDecoder
where
    T: Primitive,
{
    fn decode_full(&self, input: &[u8]) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError> {
        todo!()
    }

    fn decode_active(
        &self,
        input: &[u8],
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError> {
        todo!()
    }
}
