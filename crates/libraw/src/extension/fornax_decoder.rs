use std::path::Path;

use fornax_core::metadata::Metadata;
use fornax_core::{DecodeError, DecodedRawActive, DecodedRawFull, IDecoder, Primitive};

pub struct LibrawDecoder;
impl<T> IDecoder<Path, T> for LibrawDecoder
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

impl<T> IDecoder<[u8], T> for LibrawDecoder
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
