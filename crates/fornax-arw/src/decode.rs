use flashraw_codecs::ljpeg::{LjpegHeader, LjpegOptions, decode_rows_par};
use fornax_core::error::DecodeError;
use rayon::prelude::*;

use crate::container::{CompressionFamily, RawLayout, invalid, unsupported};
use crate::metadata;

pub fn samples(
    source: &[u8],
    layout: &RawLayout,
    raw: &fornax_tiff::Directory,
) -> Result<Vec<u16>, DecodeError> {
    match layout.compression {
        CompressionFamily::Uncompressed => decode_uncompressed(source, layout),
        CompressionFamily::Arw1 => decode_arw1(source, layout),
        CompressionFamily::Arw2 => decode_arw2(source, layout, raw),
        CompressionFamily::Arw6 => decode_arw6(source, layout),
        CompressionFamily::SonyLosslessJpeg => decode_ljpeg(source, layout),
    }
}

fn decode_uncompressed(source: &[u8], layout: &RawLayout) -> Result<Vec<u16>, DecodeError> {
    let data = chunk(source, layout, 0)?;
    let sample_count = checked_samples(layout.width, layout.stored_height)?;
    match layout.bits_per_sample {
        12 => {
            let needed = sample_count
                .checked_mul(12)
                .ok_or_else(|| invalid("Sony 12-bit sample count overflows"))?
                .div_ceil(8);
            if data.len() < needed {
                return Err(invalid(format!(
                    "Sony 12-bit strip is truncated: {} bytes, expected {needed}",
                    data.len()
                )));
            }
            let mut output = vec![0_u16; sample_count];
            for (row, values) in output.chunks_exact_mut(layout.width).enumerate() {
                let start = row
                    .checked_mul(layout.width)
                    .and_then(|offset| offset.checked_mul(12))
                    .ok_or_else(|| invalid("Sony 12-bit row offset overflows"))?
                    / 8;
                unpack_12_lsb(&data[start..], values)?;
            }
            Ok(output)
        }
        14 | 16 => {
            let needed = sample_count
                .checked_mul(2)
                .ok_or_else(|| invalid("Sony 16-bit sample count overflows"))?;
            if data.len() < needed {
                return Err(invalid(format!(
                    "Sony 16-bit strip is truncated: {} bytes, expected {needed}",
                    data.len()
                )));
            }
            Ok(data[..needed]
                .as_chunks::<2>()
                .0
                .iter()
                .map(|bytes| u16::from_le_bytes(*bytes))
                .collect())
        }
        bits => Err(unsupported(format!(
            "uncompressed Sony samples with {bits} bits per sample are unsupported"
        ))),
    }
}

fn decode_arw1(source: &[u8], layout: &RawLayout) -> Result<Vec<u16>, DecodeError> {
    let data = chunk(source, layout, 0)?;
    let sample_count = checked_samples(layout.width, layout.stored_height)?;
    let mut output = vec![0_u16; sample_count];
    let mut bits = BitReaderMsb::new(data);
    let mut sum = 0_i32;
    for column in (0..layout.width).rev() {
        let mut row = 0_usize;
        while row <= layout.stored_height {
            if row == layout.stored_height {
                row = 1;
            }
            let mut length = 4_u32 - bits.read(2)?;
            if length == 3 && bits.read(1)? != 0 {
                length = 0;
            } else if length == 4 {
                let zeros = bits.peek(13).leading_zeros() - 19;
                length += zeros;
                bits.skip(usize::try_from(zeros + 1).expect("at most 13 bits"));
            }
            let difference = bits.read_signed(length)?;
            sum = sum.wrapping_add(difference);
            if length > 0 && difference & (1 << (length - 1)) == 0 {
                sum -= (1 << length) - 1;
            }
            output[row * layout.width + column] = sum as u16;
            row += 2;
        }
    }
    Ok(output)
}

fn decode_arw2(
    source: &[u8],
    layout: &RawLayout,
    raw: &fornax_tiff::Directory,
) -> Result<Vec<u16>, DecodeError> {
    let data = chunk(source, layout, 0)?;
    let sample_count = checked_samples(layout.width, layout.stored_height)?;
    let row_bytes = layout.width;
    let needed = sample_count;
    if data.len() < needed {
        return Err(invalid(format!(
            "ARW2 strip is truncated: {} bytes, expected {needed}",
            data.len()
        )));
    }
    let curve = Arw2Curve::new(metadata::curve_knots(raw).unwrap_or([0; 4]));
    let mut output = vec![0_u16; sample_count];
    output
        .par_chunks_exact_mut(row_bytes)
        .enumerate()
        .for_each(|(row, values)| {
            let start = row * row_bytes;
            curve.decode_row(&data[start..start + row_bytes], values);
        });
    Ok(output)
}

struct Arw2Curve {
    table: Vec<(u16, u16)>,
}

impl Arw2Curve {
    fn new(knots: [u16; 4]) -> Self {
        let mut points = [0_u32; 6];
        points[5] = 4095;
        for (index, knot) in knots.into_iter().enumerate() {
            points[index + 1] = (u32::from(knot) >> 2) & 0xfff;
        }
        let mut curve = (0..4096_u32)
            .map(|value| value.min(u32::from(u16::MAX)) as u16)
            .collect::<Vec<_>>();
        for level in 0..5 {
            let start = points[level] + 1;
            let end = points[level + 1].min(4095);
            for index in start..=end {
                curve[index as usize] = curve[index as usize - 1].wrapping_add((1_u16) << level);
            }
        }
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
        let width = output.len();
        let mut column = 0_usize;
        let mut offset = 0_usize;
        while column + 30 < width {
            let mut block = [0_u8; 18];
            if let Some(bytes) = source.get(offset..offset + 18) {
                block.copy_from_slice(bytes);
            } else if let Some(bytes) = source.get(offset..) {
                block[..bytes.len().min(18)].copy_from_slice(&bytes[..bytes.len().min(18)]);
            }
            let mut pixels = Self::decode_block(&block);
            for pixel in &mut pixels {
                let value = *pixel << 1;
                let (base, delta) = self.table[usize::from(value)];
                let dithered =
                    u32::from(base) + ((u32::from(delta) * (random & 2047) + 1024) >> 12);
                random = 15_700_u32
                    .wrapping_mul(random & 0xffff)
                    .wrapping_add(random >> 16);
                *pixel = dithered.min(u32::from(u16::MAX)) as u16;
            }
            for (index, pixel) in pixels.into_iter().enumerate() {
                output[column + index * 2] = pixel;
            }
            column += 32;
            column -= if column & 1 == 1 { 1 } else { 31 };
            offset += 16;
        }
    }

    fn decode_block(block: &[u8; 18]) -> [u16; 16] {
        let value = u32::from_le_bytes([block[0], block[1], block[2], block[3]]);
        let max = (value & 0x7ff) as i32;
        let min = ((value >> 11) & 0x7ff) as i32;
        let imax = ((value >> 22) & 0x0f) as usize;
        let imin = ((value >> 26) & 0x0f) as usize;
        let mut shift = 0_u32;
        while shift < 4 && (0x80_i32 << shift) <= max - min {
            shift += 1;
        }
        let mut bit = 30_usize;
        let mut pixels = [0_u16; 16];
        for (index, pixel) in pixels.iter_mut().enumerate() {
            let raw = if index == imax {
                max
            } else if index == imin {
                min
            } else {
                let word = u16::from_le_bytes([block[bit >> 3], block[(bit >> 3) + 1]]);
                let delta = i32::from((word >> (bit & 7)) & 0x7f);
                bit += 7;
                ((delta << shift) + min).min(0x7ff)
            };
            *pixel = raw as u16;
        }
        pixels
    }
}

fn decode_arw6(source: &[u8], layout: &RawLayout) -> Result<Vec<u16>, DecodeError> {
    let data = chunk(source, layout, 0)?;
    let sample_count = checked_samples(layout.width, layout.stored_height)?;
    let mut output = vec![0_u16; sample_count];
    flashraw_codecs::sony_arw6::decode(data, &mut output, layout.width, layout.stored_height, true)
        .map_err(codec_error)?;
    Ok(output)
}

fn decode_ljpeg(source: &[u8], layout: &RawLayout) -> Result<Vec<u16>, DecodeError> {
    let tile_width = layout
        .tile_width
        .ok_or_else(|| invalid("Sony LJPEG image has no tile width"))?;
    let tile_height = layout
        .tile_height
        .ok_or_else(|| invalid("Sony LJPEG image has no tile length"))?;
    if tile_width == 0 || tile_height == 0 {
        return Err(invalid("Sony LJPEG tile dimensions must not be zero"));
    }
    let across = layout.width.div_ceil(tile_width);
    let down = layout.stored_height.div_ceil(tile_height);
    let expected = across
        .checked_mul(down)
        .ok_or_else(|| invalid("Sony LJPEG tile count overflows"))?;
    if layout.offsets.len() != expected || layout.byte_counts.len() != expected {
        return Err(invalid(format!(
            "Sony LJPEG geometry requires {expected} tiles, found {} offsets and {} byte counts",
            layout.offsets.len(),
            layout.byte_counts.len()
        )));
    }
    let sample_count = checked_samples(layout.width, layout.stored_height)?;
    let mut output = vec![0_u16; sample_count];
    for tile_y in 0..down {
        for tile_x in 0..across {
            let index = tile_y * across + tile_x;
            let encoded = chunk(source, layout, index)?;
            let header =
                LjpegHeader::parse(encoded, LjpegOptions::default()).map_err(codec_error)?;
            let components = header.components.len();
            let (decoded_width, decoded_height) = match components {
                1 => (header.width as usize, header.height as usize),
                4 => (
                    usize::try_from(header.width)
                        .ok()
                        .and_then(|width| width.checked_mul(2))
                        .ok_or_else(|| invalid("Sony LJPEG output width overflows"))?,
                    usize::try_from(header.height)
                        .ok()
                        .and_then(|height| height.checked_mul(2))
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
            if (decoded_width, decoded_height) != (tile_width, tile_height) {
                return Err(invalid(format!(
                    "Sony LJPEG tile {index} decodes to {decoded_width}x{decoded_height}, \
                     TIFF tile geometry is {tile_width}x{tile_height}"
                )));
            }
            let mut tile = vec![0_u16; decoded_width * decoded_height];
            let expected_row_len = match components {
                1 => decoded_width,
                4 => decoded_width * 2,
                _ => unreachable!("component count was validated above"),
            };
            if header.row_len() != expected_row_len {
                return Err(invalid(format!(
                    "Sony LJPEG tile {index} row has {} samples, expected {expected_row_len}",
                    header.row_len()
                )));
            }
            match components {
                1 => decode_rows_par(encoded, &header, true, |row, values| {
                    debug_assert_eq!(values.len(), decoded_width);
                    tile[row * decoded_width..(row + 1) * decoded_width].copy_from_slice(values);
                    Ok(())
                })
                .map_err(codec_error)?,
                4 => decode_rows_par(encoded, &header, true, |row, values| {
                    debug_assert_eq!(values.len(), decoded_width * 2);
                    let first = row * 2 * decoded_width;
                    let second = first + decoded_width;
                    for (column, pixel) in values.as_chunks::<4>().0.iter().enumerate() {
                        tile[first + column * 2] = pixel[0];
                        tile[first + column * 2 + 1] = pixel[1];
                        tile[second + column * 2] = pixel[2];
                        tile[second + column * 2 + 1] = pixel[3];
                    }
                    Ok(())
                })
                .map_err(codec_error)?,
                _ => unreachable!("component count was validated above"),
            }

            let destination_x = tile_x * tile_width;
            let destination_y = tile_y * tile_height;
            let copy_width = tile_width.min(layout.width - destination_x);
            let copy_height = tile_height.min(layout.stored_height - destination_y);
            for row in 0..copy_height {
                let source = &tile[row * decoded_width..row * decoded_width + copy_width];
                let destination = (destination_y + row) * layout.width + destination_x;
                output[destination..destination + copy_width].copy_from_slice(source);
            }
        }
    }
    Ok(output)
}

fn unpack_12_lsb(source: &[u8], output: &mut [u16]) -> Result<(), DecodeError> {
    let needed = output
        .len()
        .checked_mul(12)
        .ok_or_else(|| invalid("12-bit row bit count overflows"))?
        .div_ceil(8);
    if source.len() < needed {
        return Err(invalid("12-bit row is truncated"));
    }
    let (pairs, remainder) = output.as_chunks_mut::<2>();
    for (ba, bytes) in pairs.iter_mut().zip(source.as_chunks::<3>().0) {
        ba[0] = u16::from(bytes[0]) | (u16::from(bytes[1] & 0x0f) << 8);
        ba[1] = (u16::from(bytes[1]) >> 4) | (u16::from(bytes[2]) << 4);
    }
    if let Some(last) = remainder.first_mut() {
        let offset = pairs.len() * 3;
        *last = u16::from(source[offset]) | (u16::from(source[offset + 1] & 0x0f) << 8);
    }
    Ok(())
}

fn checked_samples(width: usize, height: usize) -> Result<usize, DecodeError> {
    width
        .checked_mul(height)
        .ok_or_else(|| invalid("Sony decoded sample count overflows"))
}

fn chunk<'a>(source: &'a [u8], layout: &RawLayout, index: usize) -> Result<&'a [u8], DecodeError> {
    let offset = *layout
        .offsets
        .get(index)
        .ok_or_else(|| invalid("Sony chunk offset is missing"))?;
    let count = *layout
        .byte_counts
        .get(index)
        .ok_or_else(|| invalid("Sony chunk byte count is missing"))?;
    let end = offset
        .checked_add(count)
        .ok_or_else(|| invalid("Sony chunk range overflows"))?;
    source
        .get(offset..end)
        .ok_or_else(|| invalid("Sony chunk is outside the source file"))
}

fn codec_error(error: impl std::fmt::Display) -> DecodeError { invalid(error.to_string()) }

struct BitReaderMsb<'a> {
    bytes: &'a [u8],
    bit: usize,
}

impl<'a> BitReaderMsb<'a> {
    const fn new(bytes: &'a [u8]) -> Self { Self { bytes, bit: 0 } }

    fn peek(&self, count: u8) -> u32 {
        let mut value = 0_u32;
        for index in 0..usize::from(count) {
            value = (value << 1) | u32::from(self.bit_at(self.bit + index));
        }
        value
    }

    fn read(&mut self, count: u8) -> Result<u32, DecodeError> {
        let value = self.peek(count);
        self.skip(usize::from(count));
        Ok(value)
    }

    fn read_signed(&mut self, count: u32) -> Result<i32, DecodeError> {
        let count = u8::try_from(count).map_err(|_| invalid("ARW1 difference exceeds 8 bits"))?;
        Ok(self.read(count)? as i32)
    }

    const fn skip(&mut self, count: usize) { self.bit = self.bit.saturating_add(count); }

    fn bit_at(&self, index: usize) -> u8 {
        self.bytes
            .get(index / 8)
            .map_or(0, |byte| (byte >> (7 - index % 8)) & 1)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unpack_12_lsb_reads_two_samples_per_three_bytes() {
        let source = [0xbc, 0x3a, 0x12, 0x56, 0x04];
        let mut output = [0_u16; 3];

        unpack_12_lsb(&source, &mut output).unwrap();

        assert_eq!(output, [0xabc, 0x123, 0x456]);
    }

    #[test]
    fn arw2_block_extrema_replace_delta_slots() {
        let max = 100_u32;
        let min = 50_u32;
        let imax = 0_u32;
        let imin = 1_u32;
        let mut block = [0_u8; 18];
        let header = max | (min << 11) | (imax << 22) | (imin << 26);
        block[..4].copy_from_slice(&header.to_le_bytes());

        let pixels = Arw2Curve::decode_block(&block);

        assert_eq!(pixels[0], 100);
        assert_eq!(pixels[1], 50);
        assert!(pixels[2..].iter().all(|value| *value == 50));
    }

    #[test]
    fn sony_ljpeg_four_components_expand_one_jpeg_row_to_two_bayer_rows() {
        let jpeg_width = 4;
        let jpeg_height = 2;
        let samples = (0_u16..(jpeg_width * jpeg_height * 4) as u16).collect::<Vec<_>>();
        let stream =
            flashraw_codecs::ljpeg::encoder::encode(jpeg_width, jpeg_height, 4, 12, 1, &samples);
        let layout = RawLayout {
            compression: CompressionFamily::SonyLosslessJpeg,
            width: jpeg_width * 2,
            stored_height: jpeg_height * 2,
            bits_per_sample: 12,
            offsets: vec![0],
            byte_counts: vec![stream.len()],
            tile_width: Some(jpeg_width * 2),
            tile_height: Some(jpeg_height * 2),
        };

        let output = decode_ljpeg(&stream, &layout).unwrap();

        for row in 0..jpeg_height {
            let jpeg = &samples[row * jpeg_width * 4..(row + 1) * jpeg_width * 4];
            let first = &output[row * 2 * layout.width..(row * 2 + 1) * layout.width];
            let second = &output[(row * 2 + 1) * layout.width..(row * 2 + 2) * layout.width];
            for (column, pixel) in jpeg.as_chunks::<4>().0.iter().enumerate() {
                assert_eq!(&first[column * 2..column * 2 + 2], &pixel[..2]);
                assert_eq!(&second[column * 2..column * 2 + 2], &pixel[2..]);
            }
        }
    }
}
