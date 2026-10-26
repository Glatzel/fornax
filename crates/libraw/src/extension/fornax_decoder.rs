use std::path::Path;

use fornax_core::metadata::Metadata;
use fornax_core::{DecodeError, DecodedRawActive, DecodedRawFull, IDecoder, Primitive};
use image::math::Rect;

use crate::Libraw;

pub struct LibrawDecoder;

fn backend_err<E>(_: E) -> DecodeError { DecodeError::Backend("".into()) }

impl<T: Primitive> IDecoder<Path, T> for LibrawDecoder {
    fn decode_full(&self, input: &Path) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError> {
        decode(
            |l| {
                l.open_file(input)
                    .map_err(backend_err)?
                    .unpack()
                    .map_err(backend_err)?;
                Ok(())
            },
            |l| l.get_decoded_raw_full().map_err(backend_err),
        )
    }

    fn decode_active(
        &self,
        input: &Path,
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError> {
        decode(
            |l| {
                l.open_file(input)
                    .map_err(backend_err)?
                    .unpack()
                    .map_err(backend_err)?;
                Ok(())
            },
            |l| l.get_decoded_raw_active().map_err(backend_err),
        )
    }
}

impl<T: Primitive> IDecoder<[u8], T> for LibrawDecoder {
    fn decode_full(&self, input: &[u8]) -> Result<(DecodedRawFull<T>, Metadata<T>), DecodeError> {
        decode(
            |l| {
                l.open_buffer(input)
                    .map_err(backend_err)?
                    .unpack()
                    .map_err(backend_err)?;
                Ok(())
            },
            |l| l.get_decoded_raw_full().map_err(backend_err),
        )
    }

    fn decode_active(
        &self,
        input: &[u8],
    ) -> Result<(DecodedRawActive<T>, Metadata<T>), DecodeError> {
        decode(
            |l| {
                l.open_buffer(input)
                    .map_err(backend_err)?
                    .unpack()
                    .map_err(backend_err)?;
                Ok(())
            },
            |l| l.get_decoded_raw_active().map_err(backend_err),
        )
    }
}

fn decode<T: Primitive, R>(
    open: impl FnOnce(&Libraw) -> Result<(), DecodeError>,
    extract: impl FnOnce(&Libraw) -> Result<R, DecodeError>,
) -> Result<(R, Metadata<T>), DecodeError> {
    let libraw = Libraw::default();
    open(&libraw)?;
    let image = extract(&libraw)?;
    Ok((image, get_metadata(&libraw)?))
}

fn get_metadata<T: Primitive>(libraw: &Libraw) -> Result<Metadata<T>, DecodeError> {
    let sizes = libraw.get_image_sizes()?;
    let metadata = Metadata {
        active_area: Rect {
            x: sizes.left_margin() as _,
            y: sizes.top_margin() as _,
            width: sizes.width() as _,
            height: sizes.height() as _,
        },
        default_crop: Rect {
            x: todo!(),
            y: todo!(),
            width: todo!(),
            height: todo!(),
        },
        levels: todo!(),
        bayer_pattern: libraw.bayer_pattern().unwrap(),
    };
    Ok(metadata);
}
