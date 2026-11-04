use std::path::Path;

use fornax_core::error::DecodeError as CoreDecodeError;
use fornax_core::metadata::Metadata;
use fornax_core::protocol::IDecoder;
use fornax_core::{DecodedRawActive, DecodedRawFull, Primitive};

use crate::Decoder;

impl<T> IDecoder<Path, T> for Decoder
where
    T: Primitive,
{
    fn decode_full(
        &self,
        _input: &Path,
    ) -> Result<(DecodedRawFull<T>, Metadata<T>), CoreDecodeError> {
        todo!()
    }

    fn decode_active(
        &self,
        _input: &Path,
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), CoreDecodeError> {
        todo!()
    }
}
