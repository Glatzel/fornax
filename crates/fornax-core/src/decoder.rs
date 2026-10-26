use crate::metadata::Metadata;
use crate::{DecodeError, DecodedRawActive, DecodedRawFull, Primitive};

pub trait IDecoder<I: ?Sized, T>: Send + Sync
where
    T: Primitive,
{
    fn decode_full(&self, input: &I) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError>;
    fn decode_active(&self, input: &I) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError>;
}
