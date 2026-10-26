use std::path::Path;

use fornax_core::Primitive;
use fornax_decoder_protocol::{DecodeError, DecodedRawActive, DecodedRawFull, IDecoder};
use fornax_metadata::Metadata;

use crate::Libraw;

impl<T> IDecoder<Path, T> for Libraw
where
    T: Primitive,
{
    fn decode_full(&self, _input: &Path) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError> {
        todo!()
    }

    fn decode_active(
        &self,
        _input: &Path,
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError> {
        todo!()
    }
}

impl<T> IDecoder<[u8], T> for Libraw
where
    T: Primitive,
{
    fn decode_full(&self, _input: &[u8]) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError> {
        todo!()
    }

    fn decode_active(
        &self,
        _input: &[u8],
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError> {
        todo!()
    }
}
