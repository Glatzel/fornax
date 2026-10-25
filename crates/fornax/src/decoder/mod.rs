use crate::error::DecodeError;
use crate::{DecodedRaw, Primitive};

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
    fn decode(&self, input: &I) -> Result<DecodedRaw<T>, DecodeError>;
}
