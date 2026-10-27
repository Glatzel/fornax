use std::path::Path;

use fornax_core::{IDecoder, Primitive};

pub struct RawlerDecoder;
impl<T> IDecoder<Path, T> for RawlerDecoder
where
    T: Primitive,
{
    fn decode_full(
        &self,
        _input: &Path,
    ) -> Result<
        (
            fornax_core::DecodedRawFull<T>,
            fornax_core::metadata::Metadata<T>,
        ),
        fornax_core::error::DecodeError,
    > {
        todo!()
    }

    fn decode_active(
        &self,
        _input: &Path,
    ) -> Result<
        (
            fornax_core::DecodedRawActive<T>,
            fornax_core::metadata::Metadata<T>,
        ),
        fornax_core::error::DecodeError,
    > {
        todo!()
    }
}
