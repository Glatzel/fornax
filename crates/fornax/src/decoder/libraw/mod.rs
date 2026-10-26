mod native;

use std::path::Path;

use super::IDecoder;
use crate::error::DecodeError;
use crate::metadata::Metadata;
use crate::{DecodedRawActive, DecodedRawFull, Primitive};

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
