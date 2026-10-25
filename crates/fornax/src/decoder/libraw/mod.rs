use std::path::Path;

use super::IDecoder;
use crate::error::DecodeError;
use crate::{DecodedRaw, Primitive};

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct LibRawDecoder;

impl<T> IDecoder<Path, T> for LibRawDecoder
where
    T: Primitive,
{
    fn decode(&self, _input: &Path) -> Result<DecodedRaw<T>, DecodeError> { todo!() }
}

impl<T> IDecoder<[u8], T> for LibRawDecoder
where
    T: Primitive,
{
    fn decode(&self, _input: &[u8]) -> Result<DecodedRaw<T>, DecodeError> { todo!() }
}
