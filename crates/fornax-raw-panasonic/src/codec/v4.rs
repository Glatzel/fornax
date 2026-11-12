use super::bits::PanasonicV4Bits;
use super::{finish_buffer, pixels, single_payload};
use crate::error::invalid;
use crate::{CodecError, DecodeRequest, SampleBuffer, SampleFormat};

const GROUP_PIXELS: usize = 14;
const BLOCK_BYTES: usize = 0x4000;

pub fn decode(request: &DecodeRequest<'_>, split_blocks: bool) -> Result<SampleBuffer, CodecError> {
    if request.sample_format != SampleFormat::Integer {
        return Err(invalid("Panasonic V4 requires integer samples"));
    }
    if request.bits_per_sample != 12 {
        return Err(invalid("Panasonic V4 stores 12-bit samples"));
    }
    let sample_count = pixels(request, "Panasonic V4")?;
    if !request.width.is_multiple_of(GROUP_PIXELS) {
        return Err(invalid(format!(
            "Panasonic V4 width {} must be divisible by {GROUP_PIXELS}",
            request.width
        )));
    }
    let payload = single_payload(request, "Panasonic V4")?;
    let mut output = vec![0_u16; sample_count];

    for row in 0..request.height {
        let row_bits = request
            .width
            .checked_mul(row)
            .and_then(|value| value.checked_mul(9))
            .and_then(|value| {
                let extra = request
                    .width
                    .checked_div(GROUP_PIXELS)?
                    .checked_mul(2)?
                    .checked_mul(row)?;
                value.checked_add(extra)
            })
            .ok_or_else(|| invalid("Panasonic V4 row offset overflow"))?;
        let skip = row_bits / 8;
        let block = skip / BLOCK_BYTES;
        let start = block
            .checked_mul(BLOCK_BYTES)
            .ok_or_else(|| invalid("Panasonic V4 block offset overflow"))?;
        let source = payload
            .get(start..)
            .ok_or_else(|| invalid("Panasonic V4 payload is truncated"))?;
        let mut bits = PanasonicV4Bits::new(source, split_blocks);
        for _ in 0..(skip % BLOCK_BYTES) {
            bits.read(8)?;
        }

        let width_offset = row
            .checked_mul(request.width)
            .ok_or_else(|| invalid("Panasonic V4 output offset overflow"))?;
        let row_output = output
            .get_mut(width_offset..width_offset + request.width)
            .ok_or_else(|| invalid("Panasonic V4 output row is out of bounds"))?;
        let mut shift = 0_i32;
        for group in row_output.as_chunks_mut::<GROUP_PIXELS>().0 {
            let mut predictor = [0_i32; 2];
            let mut nonzero = [0_i32; 2];
            for (index, sample) in group.iter_mut().enumerate() {
                if index % 3 == 2 {
                    let selector = bits.read(2)?;
                    shift = 4 >> (3 - selector);
                }
                let phase = index & 1;
                if nonzero[phase] != 0 {
                    let value = bits.read(8)? as i32;
                    if value != 0 {
                        predictor[phase] -= 0x80 << shift;
                        if predictor[phase] < 0 || shift == 4 {
                            predictor[phase] &= !(-1_i32 << shift);
                        }
                        predictor[phase] += value << shift;
                    }
                } else {
                    nonzero[phase] = bits.read(8)? as i32;
                    if nonzero[phase] != 0 || index > 11 {
                        predictor[phase] = (nonzero[phase] << 4) | (bits.read(4)? as i32);
                    }
                }
                *sample = predictor[phase] as u16;
            }
        }
    }

    finish_buffer(request, output)
}
