use fornax_codec::SampleFormat;
use rayon::prelude::*;

use crate::decoders::sony::error::{invalid, unsupported};
use crate::decoders::sony::ljpeg::{decode_interleaved, decode_interleaved_rows};
use crate::decoders::sony::samples::{Geometry, buffer_u16, chunk};
use crate::decoders::sony::{CodecError, DecodeRequest, Predictor, SampleBuffer};

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
    let mut tile_views = tile_output_views(
        &mut output,
        geometry.width,
        geometry.height,
        geometry.chunk_width,
        geometry.chunk_height,
        geometry.across,
    )?;

    tile_views
        .par_iter_mut()
        .enumerate()
        .try_for_each(|(index, tile_rows)| {
            let copy_width = tile_rows
                .first()
                .map(|row| row.len())
                .ok_or_else(|| invalid("Sony LJPEG output tile is empty"))?;
            let encoded = chunk(request.source, request.chunks[index])?;
            let decoded_directly = decode_interleaved_rows(
                encoded,
                precision,
                geometry.chunk_width,
                geometry.chunk_height,
                |row, samples| {
                    if samples.len() != geometry.chunk_width * 2 {
                        return Err("Sony four-component LJPEG row has the wrong width".into());
                    }
                    for (output_row, components) in [0, 2].into_iter().enumerate() {
                        let output_row = row * 2 + output_row;
                        let Some(target) = tile_rows.get_mut(output_row) else {
                            return Ok(());
                        };
                        for (column, pixel) in samples.as_chunks::<4>().0.iter().enumerate() {
                            let first = column * 2;
                            if first < target.len() {
                                target[first] = pixel[components];
                            }
                            if first + 1 < target.len() {
                                target[first + 1] = pixel[components + 1];
                            }
                        }
                    }
                    Ok(())
                },
            )
            .map_err(|detail| ljpeg_error(index, detail))?;
            if decoded_directly {
                return Ok(());
            }

            let decoded = decode_interleaved(encoded, precision)
                .map_err(|detail| ljpeg_error(index, detail))?;
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

            for (row, target) in tile_rows.iter_mut().enumerate() {
                let source = row * tile_width;
                target.copy_from_slice(&tile[source..source + copy_width]);
            }
            Ok::<(), CodecError>(())
        })?;

    buffer_u16(geometry.width, geometry.height, output)
}

fn ljpeg_error(index: usize, detail: String) -> CodecError {
    let message = format!("Sony lossless JPEG tile {index}: {detail}");
    if detail.contains("restart intervals are unsupported") {
        unsupported(message)
    } else {
        invalid(message)
    }
}

fn tile_output_views<'a>(
    output: &'a mut [u16],
    width: usize,
    height: usize,
    chunk_width: usize,
    chunk_height: usize,
    across: usize,
) -> Result<Vec<Vec<&'a mut [u16]>>, CodecError> {
    let sample_count = width
        .checked_mul(height)
        .ok_or_else(|| invalid("Sony LJPEG output size overflows"))?;
    let down = height.div_ceil(chunk_height);
    if output.len() != sample_count || across == 0 || down == 0 || chunk_width == 0 {
        return Err(invalid("Sony LJPEG output geometry is invalid"));
    }
    let mut views = (0..across * down)
        .map(|_| Vec::with_capacity(chunk_height))
        .collect::<Vec<Vec<&'a mut [u16]>>>();
    for (row, mut remaining) in output.chunks_mut(width).enumerate() {
        let tile_row = row / chunk_height;
        if tile_row >= down {
            return Err(invalid("Sony LJPEG output tile row is invalid"));
        }
        for tile_column in 0..across {
            if remaining.is_empty() {
                return Err(invalid("Sony LJPEG output tile width is invalid"));
            }
            let take = chunk_width.min(remaining.len());
            let (segment, rest) = remaining.split_at_mut(take);
            views[tile_row * across + tile_column].push(segment);
            remaining = rest;
        }
        if !remaining.is_empty() {
            return Err(invalid("Sony LJPEG output row exceeds its tiles"));
        }
    }
    if views.iter().any(Vec::is_empty) {
        return Err(invalid("Sony LJPEG output tile is empty"));
    }
    Ok(views)
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
