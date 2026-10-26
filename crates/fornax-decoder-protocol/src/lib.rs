mod decoded_raw;
mod error;

pub use decoded_raw::{DecodedRawActive, DecodedRawFull};
pub use error::DecodeError;
use fornax_core::Primitive;
use fornax_metadata::Metadata;

pub trait IDecoder<I: ?Sized, T>: Send + Sync
where
    T: Primitive,
{
    fn decode_full(&self, input: &I) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError>;
    fn decode_active(&self, input: &I) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError>;
}
