use fornax_codec::SampleFormat;
use rayon::prelude::*;

use crate::error::{invalid, unsupported};
use crate::ljpeg::decode_interleaved;
use crate::samples::{Geometry, buffer_u16, chunk};
use crate::{CodecError, DecodeRequest, Predictor, SampleBuffer};

pub fn decode(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    if request.sample_format != SampleFormat::Integer
        || !(2..=16).contains(&request.bits_per_sample)
    {
        return Err(invalid(
            "Sony lossless JPEG requires 2-16-bit integer samples",
        ));
    }
    if request.predictor != Predictor::None {
        return Err(invalid(
            "Sony lossless JPEG prediction is encoded in the JPEG stream",
        ));
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
        .ok_or_else(|| invalid("Sony lossless JPEG sample count overflows"))?;
    let mut output = vec![0_u16; sample_count];
    let expected_tile_samples = geometry.chunk_sample_count()?;
    let precision =
        u8::try_from(request.bits_per_sample).expect("Sony lossless JPEG precision is at most 16");

    output
        .par_chunks_mut(geometry.band_stride()?)
        .enumerate()
        .try_for_each(|(tile_y_index, band)| {
            let tile_y = tile_y_index * geometry.chunk_height;
            let copy_height = geometry.chunk_height.min(geometry.height - tile_y);
            for tile_x_index in 0..geometry.across {
                let index = tile_y_index * geometry.across + tile_x_index;
                let encoded = chunk(request.source, request.chunks[index])?;
                let decoded = decode_interleaved(encoded, precision).map_err(|detail| {
                    invalid(format!("Sony lossless JPEG tile {index}: {detail}"))
                })?;
                let components = decoded.components;
                let (tile_width, tile_height) = match components {
                    1 => (decoded.width, decoded.height),
                    4 => (
                        decoded
                            .width
                            .checked_div(2)
                            .ok_or_else(|| invalid("Sony LJPEG output width overflows"))?,
                        decoded
                            .height
                            .checked_mul(2)
                            .ok_or_else(|| invalid("Sony LJPEG output height overflows"))?,
                    ),
                    3 => {
                        return Err(unsupported(
                            "Sony reduced-resolution YCbCr LJPEG tiles are unsupported",
                        ));
                    }
                    count => {
                        return Err(unsupported(format!(
                            "Sony LJPEG tiles with {count} components are unsupported"
                        )));
                    }
                };
                if (tile_width, tile_height) != (geometry.chunk_width, geometry.chunk_height) {
                    return Err(invalid(format!(
                        "Sony LJPEG tile {index} decodes to {tile_width}x{tile_height}, \
                         TIFF tile geometry is {}x{}",
                        geometry.chunk_width, geometry.chunk_height
                    )));
                }
                let mut tile = vec![0_u16; expected_tile_samples];
                match components {
                    1 => tile.copy_from_slice(&decoded.samples),
                    4 => unpack_four_components(
                        &decoded.samples,
                        decoded.width,
                        decoded.height,
                        geometry.chunk_width,
                        &mut tile,
                    ),
                    _ => unreachable!("component count was validated above"),
                }

                let destination_x = tile_x_index * geometry.chunk_width;
                let copy_width = geometry.chunk_width.min(geometry.width - destination_x);
                for row in 0..copy_height {
                    let source = row * tile_width;
                    let destination = row * geometry.width + destination_x;
                    band[destination..destination + copy_width]
                        .copy_from_slice(&tile[source..source + copy_width]);
                }
            }
            Ok::<(), CodecError>(())
        })?;

    buffer_u16(geometry.width, geometry.height, output)
}

fn unpack_four_components(
    samples: &[u16],
    row_width: usize,
    height: usize,
    output_width: usize,
    output: &mut [u16],
) {
    const COMPONENTS_PER_PIXEL: usize = 4;
    for row in 0..height {
        let source = &samples[row * row_width..(row + 1) * row_width];
        let first = row * 2 * output_width;
        let second = first + output_width;
        for (column, pixel) in source
            .as_chunks::<COMPONENTS_PER_PIXEL>()
            .0
            .iter()
            .enumerate()
        {
            output[first + column * 2] = pixel[0];
            output[first + column * 2 + 1] = pixel[1];
            output[second + column * 2] = pixel[2];
            output[second + column * 2 + 1] = pixel[3];
        }
    }
}
