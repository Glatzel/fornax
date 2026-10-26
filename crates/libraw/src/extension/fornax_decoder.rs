use std::path::Path;

use fornax_core::metadata::Metadata;
use fornax_core::{DecodeError, DecodedRawActive, DecodedRawFull, IDecoder, Primitive};

use crate::Libraw;

pub struct LibrawDecoder;
impl<T> IDecoder<Path, T> for LibrawDecoder
where
    T: Primitive,
{
    fn decode_full(&self, input: &Path) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError> {
        let libraw = Libraw::default();
        libraw
            .open_file(input)
            .map_err(|_e| DecodeError::Backend("".into()))?
            .unpack()
            .map_err(|_e| DecodeError::Backend("".into()))?;
        let image = libraw
            .get_decoded_raw_full()
            .map_err(|_e| DecodeError::Backend("".into()))?;
        let metadata = get_metadata(&libraw)?;
        Ok((image, metadata))
    }

    fn decode_active(
        &self,
        input: &Path,
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError> {
        let libraw = Libraw::default();
        libraw
            .open_file(input)
            .map_err(|_e| DecodeError::Backend("".into()))?
            .unpack()
            .map_err(|_e| DecodeError::Backend("".into()))?;
        let image = libraw
            .get_decoded_raw_active()
            .map_err(|_e| DecodeError::Backend("".into()))?;
        let metadata = get_metadata(&libraw)?;
        Ok((image, metadata))
    }
}

impl<T> IDecoder<[u8], T> for LibrawDecoder
where
    T: Primitive,
{
    fn decode_full(&self, input: &[u8]) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError> {
        let libraw = Libraw::default();
        libraw
            .open_buffer(input)
            .map_err(|_e| DecodeError::Backend("".into()))?
            .unpack()
            .map_err(|_e| DecodeError::Backend("".into()))?;
        let image = libraw
            .get_decoded_raw_full()
            .map_err(|_e| DecodeError::Backend("".into()))?;
        let metadata = get_metadata(&libraw)?;
        Ok((image, metadata))
    }

    fn decode_active(
        &self,
        input: &[u8],
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError> {
        let libraw = Libraw::default();
        libraw
            .open_buffer(input)
            .map_err(|_e| DecodeError::Backend("".into()))?
            .unpack()
            .map_err(|_e| DecodeError::Backend("".into()))?;
        let image = libraw
            .get_decoded_raw_active()
            .map_err(|_e| DecodeError::Backend("".into()))?;
        let metadata = get_metadata(&libraw)?;
        Ok((image, metadata))
    }
}
fn get_metadata<T>(libraw: &Libraw) -> Result<Metadata<T>, DecodeError>
where
    T: Primitive,
{
    todo!()
}
