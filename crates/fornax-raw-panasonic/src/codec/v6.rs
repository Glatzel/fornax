use super::bits::LsbBits;
use super::{finish_buffer, pixels, require_integer_12_or_14, single_payload};
use crate::error::invalid;
use crate::{CodecError, DecodeRequest, SampleBuffer};

const BLOCK_BYTES: usize = 16;

pub fn decode(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    let bits = require_integer_12_or_14(request, "Panasonic V6")?;
    let sample_count = pixels(request, "Panasonic V6")?;
    let pixels_per_block = if bits == 12 { 14 } else { 11 };
    if !request.width.is_multiple_of(pixels_per_block) {
        return Err(invalid(format!(
            "Panasonic V6 {bits}-bit width {} must be divisible by {pixels_per_block}",
            request.width
        )));
    }
    let blocks_per_row = request.width / pixels_per_block;
    let row_bytes = blocks_per_row
        .checked_mul(BLOCK_BYTES)
        .ok_or_else(|| invalid("Panasonic V6 row size overflow"))?;
    let required = row_bytes
        .checked_mul(request.height)
        .ok_or_else(|| invalid("Panasonic V6 payload size overflow"))?;
    let payload = single_payload(request, "Panasonic V6")?;
    if payload.len() < required {
        return Err(invalid(format!(
            "Panasonic V6 payload has {} bytes, {required} required",
            payload.len()
        )));
    }
    let mut output = vec![0_u16; sample_count];
    for row in 0..request.height {
        let source_start = row
            .checked_mul(row_bytes)
            .ok_or_else(|| invalid("Panasonic V6 row offset overflow"))?;
        let source_row = &payload[source_start..source_start + row_bytes];
        let output_start = row
            .checked_mul(request.width)
            .ok_or_else(|| invalid("Panasonic V6 output offset overflow"))?;
        let output_row = &mut output[output_start..output_start + request.width];
        for (block_index, block) in source_row.as_chunks::<BLOCK_BYTES>().0.iter().enumerate() {
            decode_block(
                block,
                bits,
                &mut output_row[block_index * pixels_per_block..][..pixels_per_block],
            )?;
        }
    }
    finish_buffer(request, output)
}

fn decode_block(block: &[u8], bits: u16, output: &mut [u16]) -> Result<(), CodecError> {
    let mut stream = LsbBits::new(block, BLOCK_BYTES * 8)?;
    let mut pixel = [0_u16; 18];
    if bits == 12 {
        pixel[17] = stream.read(8)? as u16;
        pixel[16] = stream.read(8)? as u16;
        pixel[15] = stream.read(8)? as u16;
        pixel[14] = stream.read(2)? as u16;
        pixel[13] = stream.read(8)? as u16;
        pixel[12] = stream.read(8)? as u16;
        pixel[11] = stream.read(8)? as u16;
        pixel[10] = stream.read(2)? as u16;
        pixel[9] = stream.read(8)? as u16;
        pixel[8] = stream.read(8)? as u16;
        pixel[7] = stream.read(8)? as u16;
        pixel[6] = stream.read(2)? as u16;
        pixel[5] = stream.read(8)? as u16;
        pixel[4] = stream.read(8)? as u16;
        pixel[3] = stream.read(8)? as u16;
        pixel[2] = stream.read(2)? as u16;
        pixel[1] = stream.read(12)? as u16;
        pixel[0] = stream.read(12)? as u16;
    } else {
        stream.skip(4)?;
        pixel[13] = stream.read(10)? as u16;
        pixel[12] = stream.read(10)? as u16;
        pixel[11] = stream.read(10)? as u16;
        pixel[10] = stream.read(2)? as u16;
        pixel[9] = stream.read(10)? as u16;
        pixel[8] = stream.read(10)? as u16;
        pixel[7] = stream.read(10)? as u16;
        pixel[6] = stream.read(2)? as u16;
        pixel[5] = stream.read(10)? as u16;
        pixel[4] = stream.read(10)? as u16;
        pixel[3] = stream.read(10)? as u16;
        pixel[2] = stream.read(2)? as u16;
        pixel[1] = stream.read(14)? as u16;
        pixel[0] = stream.read(14)? as u16;
    }

    let (base_start, compare, spix_mask, pixel_mask) = if bits == 12 {
        (0x80_i32, 0x800_i32, 0x3fff_i32, 0xfff_i32)
    } else {
        (0x200_i32, 0x2000_i32, 0xffff_i32, 0x3fff_i32)
    };
    let mut odd_even = [0_i32; 2];
    let mut nonzero = [0_i32; 2];
    let mut multiplier = 0_i32;
    let mut pixel_base = 0_i32;
    let mut field = 0_usize;

    for (index, target) in output.iter_mut().enumerate() {
        if index % 3 == 2 {
            let mut base = i32::from(pixel[field]);
            field += 1;
            if base == 3 {
                base = 4;
            }
            pixel_base = base_start << base;
            multiplier = 1 << base;
        }
        let mut encoded = i32::from(pixel[field]);
        field += 1;
        let phase = index & 1;
        if odd_even[phase] != 0 {
            encoded *= multiplier;
            if pixel_base < compare && nonzero[phase] > pixel_base {
                encoded += nonzero[phase] - pixel_base;
            }
            nonzero[phase] = encoded;
        } else {
            odd_even[phase] = encoded;
            if encoded != 0 {
                nonzero[phase] = encoded;
            } else {
                encoded = nonzero[phase];
            }
        }
        let sample = encoded.wrapping_sub(0xf);
        *target = if sample <= spix_mask {
            (sample & spix_mask) as u16
        } else {
            ((encoded.wrapping_add(0x7fff_fff1) >> 31) & pixel_mask) as u16
        };
    }
    Ok(())
}
