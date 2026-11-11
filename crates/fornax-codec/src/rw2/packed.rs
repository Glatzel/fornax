use super::{finish_buffer, pixels, single_payload};
use crate::error::invalid;
use crate::{CodecError, DecodeRequest, SampleBuffer, SampleFormat};

pub fn decode_unpacked_12(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    validate(request, "Panasonic unpacked 12-bit")?;
    let sample_count = pixels(request, "Panasonic unpacked 12-bit")?;
    let required = sample_count
        .checked_mul(2)
        .ok_or_else(|| invalid("Panasonic unpacked 12-bit payload size overflow"))?;
    let payload = single_payload(request, "Panasonic unpacked 12-bit")?;
    if payload.len() < required {
        return Err(invalid(format!(
            "Panasonic unpacked 12-bit payload has {} bytes, {required} required",
            payload.len()
        )));
    }
    let output: Vec<u16> = payload[..required]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|bytes| u16::from_le_bytes([bytes[0], bytes[1]]) >> 4)
        .collect();
    finish_buffer(request, output)
}

pub fn decode_packed_12_control(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    validate(request, "Panasonic packed 12-bit control")?;
    let sample_count = pixels(request, "Panasonic packed 12-bit control")?;
    let mut output = vec![0_u16; sample_count];
    let payload = single_payload(request, "Panasonic packed 12-bit control")?;
    let mut position = 0_usize;

    for row in 0..request.height {
        let row_start = row
            .checked_mul(request.width)
            .ok_or_else(|| invalid("Panasonic packed row offset overflow"))?;
        let mut column = 0_usize;
        while column < request.width {
            let count = (request.width - column).min(10);
            let data_bytes = if count == 10 {
                15
            } else {
                count
                    .checked_mul(3)
                    .map(|bits| bits.div_ceil(2))
                    .ok_or_else(|| invalid("Panasonic packed group size overflow"))?
            };
            let group_bytes = data_bytes
                .checked_add(usize::from(count == 10))
                .ok_or_else(|| invalid("Panasonic packed group size overflow"))?;
            let end = position
                .checked_add(group_bytes)
                .ok_or_else(|| invalid("Panasonic packed payload offset overflow"))?;
            let group = payload
                .get(position..end)
                .ok_or_else(|| invalid("Panasonic packed payload is truncated"))?;
            for (pair_index, pair) in group[..data_bytes].as_chunks::<3>().0.iter().enumerate() {
                let first = column + pair_index * 2;
                if first >= column + count {
                    break;
                }
                let target = row_start + first;
                output[target] = u16::from(pair[0]) | (u16::from(pair[1] & 0x0f) << 8);
                if first + 1 < column + count {
                    output[target + 1] = (u16::from(pair[1]) >> 4) | (u16::from(pair[2]) << 4);
                }
            }
            if count % 2 == 1 {
                let last = column + count - 1;
                output[row_start + last] = u16::from(group[data_bytes - 2])
                    | (u16::from(group[data_bytes - 1] & 0x0f) << 8);
            }
            position = end;
            column += count;
        }
    }
    finish_buffer(request, output)
}

fn validate(request: &DecodeRequest<'_>, format: &str) -> Result<(), CodecError> {
    if request.sample_format != SampleFormat::Integer || request.bits_per_sample != 12 {
        return Err(invalid(format!("{format} requires 12-bit integer samples")));
    }
    Ok(())
}
