use super::bits::LsbBits;
use super::{finish_buffer, pixels, require_integer_12_or_14, single_payload};
use crate::error::invalid;
use crate::{CodecError, DecodeRequest, SampleBuffer};

const BLOCK_BYTES: usize = 0x4000;
const PACKET_BYTES: usize = 16;
const SPLIT_OFFSET: usize = 0x1ff8;

pub fn decode(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    let bits = require_integer_12_or_14(request, "Panasonic V5")?;
    let sample_count = pixels(request, "Panasonic V5")?;
    let pixels_per_packet = if bits == 12 { 10 } else { 9 };
    let packets_per_block = BLOCK_BYTES / PACKET_BYTES;
    let pixels_per_block = packets_per_block
        .checked_mul(pixels_per_packet)
        .ok_or_else(|| invalid("Panasonic V5 block pixel count overflow"))?;
    let block_count = sample_count.div_ceil(pixels_per_block);
    let required = block_count
        .checked_mul(BLOCK_BYTES)
        .ok_or_else(|| invalid("Panasonic V5 payload size overflow"))?;
    let payload = single_payload(request, "Panasonic V5")?;
    if payload.len() < required {
        return Err(invalid(format!(
            "Panasonic V5 payload has {} bytes, {required} required",
            payload.len()
        )));
    }

    let mut output = Vec::with_capacity(sample_count);
    'blocks: for block_index in 0..block_count {
        let block_start = block_index
            .checked_mul(BLOCK_BYTES)
            .ok_or_else(|| invalid("Panasonic V5 block offset overflow"))?;
        let block = &payload[block_start..block_start + BLOCK_BYTES];
        for packet in block[SPLIT_OFFSET..]
            .as_chunks::<PACKET_BYTES>()
            .0
            .iter()
            .chain(block[..SPLIT_OFFSET].as_chunks::<PACKET_BYTES>().0.iter())
        {
            let mut stream = LsbBits::new(packet, PACKET_BYTES * 8)?;
            for _ in 0..pixels_per_packet {
                if output.len() == sample_count {
                    break 'blocks;
                }
                output.push(stream.read(u32::from(bits))? as u16);
            }
        }
    }

    finish_buffer(request, output)
}
