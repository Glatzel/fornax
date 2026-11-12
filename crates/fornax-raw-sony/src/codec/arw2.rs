use fornax_codec::SampleFormat;
use rayon::prelude::*;

use crate::error::invalid;
use crate::samples::{Geometry, buffer_u16, chunk};
use crate::{CodecError, DecodeRequest, SampleBuffer};

pub fn curve(knots: Option<[u16; 4]>) -> Vec<u16> {
    let Some(knots) = knots else {
        return (0..4096_u32)
            .map(|value| value.min(u32::from(u16::MAX)) as u16)
            .collect();
    };
    let mut points = [0_u32; 6];
    points[5] = 4095;
    for (index, knot) in knots.into_iter().enumerate() {
        points[index + 1] = (u32::from(knot) >> 2) & 0x0fff;
    }
    let mut values = (0..4096_u32)
        .map(|value| value.min(u32::from(u16::MAX)) as u16)
        .collect::<Vec<_>>();
    for level in 0..5 {
        let start = points[level] + 1;
        let end = points[level + 1].min(4095);
        for index in start..=end {
            values[index as usize] = values[index as usize - 1].wrapping_add(1_u16 << level);
        }
    }
    values
}

pub fn white_level(knots: Option<[u16; 4]>) -> u16 {
    curve(knots).last().copied().unwrap_or(16_383)
}

pub fn decode(
    request: &DecodeRequest<'_>,
    curve_knots: Option<[u16; 4]>,
) -> Result<SampleBuffer, CodecError> {
    if request.sample_format != SampleFormat::Integer {
        return Err(invalid("Sony ARW2 requires integer samples"));
    }
    let geometry = Geometry::new(
        request.width,
        request.height,
        request.chunk_width,
        request.chunk_height,
        request.chunks.len(),
    )?;
    if geometry.chunk_width != geometry.width || geometry.chunk_height != geometry.height {
        return Err(invalid("Sony ARW2 requires one whole-image chunk"));
    }
    let encoded = chunk(request.source, request.chunks[0])?;
    let sample_count = geometry
        .width
        .checked_mul(geometry.height)
        .ok_or_else(|| invalid("Sony ARW2 sample count overflows"))?;
    if encoded.len() < sample_count {
        return Err(invalid(format!(
            "Sony ARW2 strip is truncated: {} bytes, expected {sample_count}",
            encoded.len()
        )));
    }
    let curve = Curve::new(curve_knots);
    let mut output = vec![0_u16; sample_count];
    output
        .par_chunks_exact_mut(geometry.width)
        .enumerate()
        .for_each(|(row, values)| {
            let start = row * geometry.width;
            curve.decode_row(&encoded[start..start + geometry.width], values);
        });
    buffer_u16(geometry.width, geometry.height, output)
}

struct Curve {
    table: Vec<(u16, u16)>,
}

impl Curve {
    fn new(knots: Option<[u16; 4]>) -> Self {
        let curve = curve(knots);
        let table = curve
            .iter()
            .enumerate()
            .map(|(index, center)| {
                let lower = if index > 0 { curve[index - 1] } else { *center };
                let upper = curve.get(index + 1).copied().unwrap_or(*center);
                let base = if *center == 0 {
                    0
                } else {
                    center - (upper - lower + 2) / 4
                };
                (base, upper - lower)
            })
            .collect();
        Self { table }
    }

    fn decode_row(&self, source: &[u8], output: &mut [u16]) {
        let mut random = u32::from(u16::from_le_bytes([
            source.first().copied().unwrap_or(0),
            source.get(1).copied().unwrap_or(0),
        ]));
        let mut column = 0_usize;
        let mut offset = 0_usize;
        while column + 30 < output.len() {
            let mut block = [0_u8; 18];
            if let Some(bytes) = source.get(offset..offset + 18) {
                block.copy_from_slice(bytes);
            } else if let Some(bytes) = source.get(offset..) {
                let count = bytes.len().min(18);
                block[..count].copy_from_slice(&bytes[..count]);
            }
            let mut pixels = decode_block(&block);
            for pixel in &mut pixels {
                let (base, delta) = self.table[usize::from(*pixel << 1)];
                let value = u32::from(base) + ((u32::from(delta) * (random & 2047) + 1024) >> 12);
                random = 15_700_u32
                    .wrapping_mul(random & 0xffff)
                    .wrapping_add(random >> 16);
                *pixel = value.min(u32::from(u16::MAX)) as u16;
            }
            for (index, pixel) in pixels.into_iter().enumerate() {
                output[column + index * 2] = pixel;
            }
            column += 32;
            column -= if column & 1 == 1 { 1 } else { 31 };
            offset += 16;
        }
    }
}

fn decode_block(block: &[u8; 18]) -> [u16; 16] {
    let header = u32::from_le_bytes([block[0], block[1], block[2], block[3]]);
    let maximum = (header & 0x7ff) as i32;
    let minimum = ((header >> 11) & 0x7ff) as i32;
    let maximum_index = ((header >> 22) & 0x0f) as usize;
    let minimum_index = ((header >> 26) & 0x0f) as usize;
    let mut shift = 0_u32;
    while shift < 4 && (0x80_i32 << shift) <= maximum - minimum {
        shift += 1;
    }
    let mut bit = 30_usize;
    let mut pixels = [0_u16; 16];
    for (index, pixel) in pixels.iter_mut().enumerate() {
        let value = if index == maximum_index {
            maximum
        } else if index == minimum_index {
            minimum
        } else {
            let word = u16::from_le_bytes([block[bit >> 3], block[(bit >> 3) + 1]]);
            let delta = i32::from((word >> (bit & 7)) & 0x7f);
            bit += 7;
            ((delta << shift) + minimum).min(0x7ff)
        };
        *pixel = value as u16;
    }
    pixels
}

#[cfg(test)]
mod tests {
    use super::{curve, decode_block};

    #[test]
    fn curve_applies_knots_and_preserves_the_top_level() {
        let values = curve(Some([8000, 10400, 12900, 14100]));
        assert_eq!(values[0], 0);
        assert_eq!(values[2000], 2000);
        assert_eq!(values[2001], 2002);
        assert_eq!(values[4095], *values.iter().max().unwrap());
        assert!(
            curve(None)
                .iter()
                .enumerate()
                .all(|(i, v)| *v as usize == i)
        );
    }

    #[test]
    fn zero_block_keeps_extrema_and_fills_deltas_with_the_minimum() {
        let pixels = decode_block(&[0; 18]);
        assert_eq!(pixels, [0; 16]);
    }

    #[test]
    fn extrema_indices_replace_delta_slots() {
        let maximum = 100_u32;
        let minimum = 50_u32;
        let maximum_index = 0_u32;
        let minimum_index = 1_u32;
        let mut block = [0_u8; 18];
        let header = maximum | (minimum << 11) | (maximum_index << 22) | (minimum_index << 26);
        block[..4].copy_from_slice(&header.to_le_bytes());

        let pixels = decode_block(&block);

        assert_eq!(pixels[0], 100);
        assert_eq!(pixels[1], 50);
        assert!(pixels[2..].iter().all(|value| *value == 50));
    }
}
