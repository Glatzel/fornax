//! Single-channel JPEG XL decoding.

use fornax_core::RawMosaic;
use jxl_oxide::JxlImage;
use rayon::prelude::*;

use crate::error::invalid;
use crate::request::{DecodeRequest, SampleFormat};
use crate::samples::{Geometry, chunk, mono};
use crate::{CodecError, ICodecDecode, Predictor};

/// Single-channel JPEG XL raw tile decoder.
///
/// # Examples
///
/// ```
/// assert_eq!(fornax_codec::JpegXlCodec, fornax_codec::JpegXlCodec);
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub struct JpegXlCodec;

impl ICodecDecode for JpegXlCodec {
    fn decode(&self, request: &DecodeRequest<'_>) -> Result<RawMosaic, CodecError> {
        if request.predictor != Predictor::None {
            return Err(invalid("JPEG XL samples do not support prediction"));
        }
        if request.sample_format != SampleFormat::Integer
            || !(8..=16).contains(&request.bits_per_sample)
        {
            return Err(invalid(
                "JPEG XL raw tiles require unsigned 8-16-bit storage",
            ));
        }
        let geometry = Geometry::new(
            request.width,
            request.height,
            request.chunk_width,
            request.chunk_height,
            request.chunks.len(),
        )?;
        if request.bits_per_sample == 8 {
            let mut output = vec![
                0_u8;
                geometry
                    .width
                    .checked_mul(geometry.height)
                    .ok_or_else(|| invalid("JPEG XL sample count overflow"))?
            ];
            decode_into(request, &geometry, &mut output, decode_u8)?;
            mono(geometry.width, geometry.height, output).map(RawMosaic::U8)
        } else {
            let mut output = vec![
                0_u16;
                geometry
                    .width
                    .checked_mul(geometry.height)
                    .ok_or_else(|| invalid("JPEG XL sample count overflow"))?
            ];
            let bits = request.bits_per_sample;
            decode_into(
                request,
                &geometry,
                &mut output,
                move |encoded, width, height| decode_u16(encoded, width, height, bits),
            )?;
            mono(geometry.width, geometry.height, output).map(RawMosaic::U16)
        }
    }
}

fn decode_into<T>(
    request: &DecodeRequest<'_>,
    geometry: &Geometry,
    output: &mut [T],
    decode: impl Fn(&[u8], usize, usize) -> Result<Vec<T>, String> + Sync,
) -> Result<(), CodecError>
where
    T: Copy + Send,
{
    output
        .par_chunks_mut(geometry.band_stride()?)
        .enumerate()
        .try_for_each(|(tile_y_index, band)| {
            let tile_y = tile_y_index * geometry.chunk_height;
            let copy_height = geometry.chunk_height.min(geometry.height - tile_y);
            for tile_x_index in 0..geometry.across {
                let index = tile_y_index * geometry.across + tile_x_index;
                let encoded = chunk(request.source, request.chunks[index])?;
                let decoded = decode(encoded, geometry.chunk_width, geometry.chunk_height)
                    .map_err(|detail| invalid(format!("JPEG XL tile {index}: {detail}")))?;
                let expected = geometry
                    .chunk_width
                    .checked_mul(geometry.chunk_height)
                    .ok_or_else(|| invalid("JPEG XL tile size overflow"))?;
                if decoded.len() != expected {
                    return Err(invalid(format!(
                        "JPEG XL tile {index} decoded {} samples, expected {expected}",
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
        })
}

fn decode_u8(encoded: &[u8], width: usize, height: usize) -> Result<Vec<u8>, String> {
    let image = JxlImage::builder()
        .read(encoded)
        .map_err(|error| format!("header is invalid: {error:?}"))?;
    let frame = image
        .render_frame(0)
        .map_err(|error| format!("payload is invalid: {error:?}"))?;
    let mut stream = frame.stream_no_alpha();
    if stream.width() as usize != width
        || stream.height() as usize != height
        || stream.channels() != 1
    {
        return Err(format!(
            "tile is {}x{} with {} channels, expected {width}x{height} with one",
            stream.width(),
            stream.height(),
            stream.channels()
        ));
    }
    let mut output = vec![0_u8; width * height];
    let written = stream.write_to_buffer(&mut output);
    if written != output.len() {
        return Err(format!(
            "decoded {written} samples, expected {}",
            output.len()
        ));
    }
    Ok(output)
}

fn decode_u16(encoded: &[u8], width: usize, height: usize, bits: u16) -> Result<Vec<u16>, String> {
    let image = JxlImage::builder()
        .read(encoded)
        .map_err(|error| format!("header is invalid: {error:?}"))?;
    let frame = image
        .render_frame(0)
        .map_err(|error| format!("payload is invalid: {error:?}"))?;
    let mut stream = frame.stream_no_alpha();
    if stream.width() as usize != width
        || stream.height() as usize != height
        || stream.channels() != 1
    {
        return Err(format!(
            "tile is {}x{} with {} channels, expected {width}x{height} with one",
            stream.width(),
            stream.height(),
            stream.channels()
        ));
    }
    let mut output = vec![0_u16; width * height];
    let written = stream.write_to_buffer(&mut output);
    if written != output.len() {
        return Err(format!(
            "decoded {written} samples, expected {}",
            output.len()
        ));
    }
    if bits < 16 {
        let shift = 16 - bits;
        for value in &mut output {
            *value >>= shift;
        }
    }
    Ok(output)
}
