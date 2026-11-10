//! Single-component lossy JPEG decoding.

use rayon::prelude::*;
use zune_jpeg::JpegDecoder;
use zune_jpeg::zune_core::bytestream::ZCursor;
use zune_jpeg::zune_core::colorspace::ColorSpace;
use zune_jpeg::zune_core::options::DecoderOptions;

use crate::error::invalid;
use crate::request::{DecodeRequest, SampleFormat};
use crate::samples::{Geometry, buffer_u8, chunk};
use crate::{CodecError, Predictor, SampleBuffer};

pub fn decode(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    if request.predictor != Predictor::None {
        return Err(invalid("lossy JPEG samples do not support prediction"));
    }
    if request.bits_per_sample != 8 || request.sample_format != SampleFormat::Integer {
        return Err(invalid("lossy JPEG samples require unsigned 8-bit storage"));
    }
    let geometry = Geometry::new(
        request.width,
        request.height,
        request.chunk_width,
        request.chunk_height,
        request.chunks.len(),
    )?;
    let sample_count = geometry
        .width
        .checked_mul(geometry.height)
        .ok_or_else(|| invalid("lossy JPEG sample count overflow"))?;
    let mut output = vec![0_u8; sample_count];
    output
        .par_chunks_mut(geometry.band_stride()?)
        .enumerate()
        .try_for_each(|(tile_y_index, band)| {
            let tile_y = tile_y_index * geometry.chunk_height;
            let copy_height = geometry.chunk_height.min(geometry.height - tile_y);
            for tile_x_index in 0..geometry.across {
                let index = tile_y_index * geometry.across + tile_x_index;
                let encoded = chunk(request.source, request.chunks[index])?;
                let decoded = decode_luma(encoded)
                    .map_err(|detail| invalid(format!("lossy JPEG tile {index}: {detail}")))?;
                let expected = geometry
                    .chunk_width
                    .checked_mul(geometry.chunk_height)
                    .ok_or_else(|| invalid("lossy JPEG tile size overflow"))?;
                if decoded.len() != expected {
                    return Err(invalid(format!(
                        "lossy JPEG tile {index} decoded {} samples, expected {expected}",
                        decoded.len()
                    )));
                }
                let tile_x = tile_x_index * geometry.chunk_width;
                let copy_width = geometry.chunk_width.min(geometry.width - tile_x);
                for row in 0..copy_height {
                    let source_start = row * geometry.chunk_width;
                    let target_start = row * geometry.width + tile_x;
                    band[target_start..target_start + copy_width]
                        .copy_from_slice(&decoded[source_start..source_start + copy_width]);
                }
            }
            Ok::<(), CodecError>(())
        })?;
    buffer_u8(geometry.width, geometry.height, output)
}

fn decode_luma(encoded: &[u8]) -> Result<Vec<u8>, String> {
    let options = DecoderOptions::default().jpeg_set_out_colorspace(ColorSpace::Luma);
    let mut decoder = JpegDecoder::new_with_options(ZCursor::new(encoded), options);
    decoder
        .decode_headers()
        .map_err(|error| format!("header is invalid: {error:?}"))?;
    let info = decoder
        .info()
        .ok_or_else(|| "JPEG has no image information".to_owned())?;
    if info.components != 1 {
        return Err(format!(
            "JPEG has {} components; a raw tile requires one",
            info.components
        ));
    }
    let expected = usize::from(info.width)
        .checked_mul(usize::from(info.height))
        .ok_or_else(|| "JPEG dimensions overflow".to_owned())?;
    let pixels = decoder
        .decode()
        .map_err(|error| format!("payload is invalid: {error:?}"))?;
    if pixels.len() != expected {
        return Err(format!(
            "JPEG decoded {} samples, expected {expected}",
            pixels.len()
        ));
    }
    Ok(pixels)
}
