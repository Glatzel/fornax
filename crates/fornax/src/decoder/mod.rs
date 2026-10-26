use crate::error::DecodeError;
use crate::metadata::Metadata;
use crate::{DecodedRawActive, DecodedRawFull, Primitive};

#[cfg(feature = "backend-libraw")]
mod libraw;
#[cfg(feature = "backend-libraw")]
pub use libraw::LibRawDecoder;

#[cfg(all(feature = "backend-dnc", not(target_os = "linux")))]
mod dnc;
#[cfg(all(feature = "backend-dnc", not(target_os = "linux")))]
pub use dnc::DncDecoder;

pub trait IDecoder<I: ?Sized, T>: Send + Sync
where
    T: Primitive,
{
    fn decode_full(&self, input: &I) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError>;
    fn decode_active(&self, input: &I) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError>;
}
