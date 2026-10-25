use crate::error::DecodeError;
use crate::{DecodedRaw, Primitive};

#[cfg(feature = "backend-libraw")]
mod libraw;

#[cfg(feature = "backend-libraw")]
pub use libraw::LibRawDecoder;

pub trait IDecoder<I: ?Sized, T>: Send + Sync
where
    T: Primitive,
{
    fn decode(&self, input: &I) -> Result<DecodedRaw<T>, DecodeError>;
}
