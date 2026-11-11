use crate::error::invalid;
use crate::request::{ByteOrder, DecodeRequest};
use crate::samples::{buffer_u16, chunk};
use crate::{CodecError, SampleBuffer};

pub(crate) fn decode_packed(
    request: &DecodeRequest<'_>,
    row_stride: usize,
    bits: u8,
    byte_order: ByteOrder,
) -> Result<SampleBuffer, CodecError> {
    validate_bits(bits)?;
    if bits == 14 && byte_order == ByteOrder::Big {
        return Err(invalid(
            "big-endian packed 14-bit Nikon data is unsupported",
        ));
    }
    let rows = validate_geometry(request)?;
    let row_bytes = packed_row_bytes(request.width, bits)?;
    if row_stride < row_bytes {
        return Err(invalid(format!(
            "Nikon packed row stride {row_stride} is shorter than required {row_bytes}"
        )));
    }
    validate_chunk_lengths(request, &rows, |count| {
        count
            .checked_mul(row_stride)
            .ok_or_else(|| invalid("Nikon packed strip size overflow"))
    })?;
    let sample_count = checked_sample_count(request.width, request.height)?;
    let mut output = vec![0_u16; sample_count];

    for (tile, strip) in request.chunks.iter().copied().enumerate() {
        let start_row = tile
            .checked_mul(request.chunk_height)
            .ok_or_else(|| invalid("Nikon packed row offset overflow"))?;
        let rows = rows[tile];
        let payload = chunk(request.source, strip)?;
        for row in 0..rows {
            let source = &payload[row * row_stride..][..row_bytes];
            let target = &mut output[(start_row + row) * request.width..][..request.width];
            unpack_row(source, target, bits, byte_order)?;
        }
    }
    buffer_u16(request.width, request.height, output)
}

pub(crate) fn decode_unpacked_words(
    request: &DecodeRequest<'_>,
    bits: u8,
    byte_order: ByteOrder,
) -> Result<SampleBuffer, CodecError> {
    validate_bits(bits)?;
    let rows = validate_geometry(request)?;
    validate_chunk_lengths(request, &rows, |count| {
        count
            .checked_mul(request.width)
            .and_then(|samples| samples.checked_mul(2))
            .ok_or_else(|| invalid("Nikon unpacked-word strip size overflow"))
    })?;
    let sample_count = checked_sample_count(request.width, request.height)?;
    let mut output = vec![0_u16; sample_count];
    let mask = (1_u16 << bits) - 1;

    for (tile, strip) in request.chunks.iter().copied().enumerate() {
        let start_row = tile
            .checked_mul(request.chunk_height)
            .ok_or_else(|| invalid("Nikon unpacked row offset overflow"))?;
        let payload = chunk(request.source, strip)?;
        for row in 0..rows[tile] {
            let source = &payload[row * request.width * 2..][..request.width * 2];
            let target = &mut output[(start_row + row) * request.width..][..request.width];
            for (target, bytes) in target.iter_mut().zip(source.chunks_exact(2)) {
                let bytes = [bytes[0], bytes[1]];
                *target = match byte_order {
                    ByteOrder::Little => u16::from_le_bytes(bytes),
                    ByteOrder::Big => u16::from_be_bytes(bytes),
                } & mask;
            }
        }
    }
    buffer_u16(request.width, request.height, output)
}

pub(crate) fn decode_striped_packed_12(
    request: &DecodeRequest<'_>,
) -> Result<SampleBuffer, CodecError> {
    if request.width == 0 || request.width % 8 != 0 {
        return Err(invalid(
            "Nikon mode-5 rows require a non-zero width divisible by eight",
        ));
    }
    let rows = validate_geometry(request)?;
    let row_bytes = request
        .width
        .checked_mul(12)
        .map(|bits| bits / 8)
        .ok_or_else(|| invalid("Nikon mode-5 row size overflow"))?;
    validate_chunk_lengths(request, &rows, |count| {
        count
            .checked_mul(row_bytes)
            .ok_or_else(|| invalid("Nikon mode-5 strip size overflow"))
    })?;
    let sample_count = checked_sample_count(request.width, request.height)?;
    let mut output = vec![0_u16; sample_count];

    for (tile, strip) in request.chunks.iter().copied().enumerate() {
        let start_row = tile
            .checked_mul(request.chunk_height)
            .ok_or_else(|| invalid("Nikon mode-5 row offset overflow"))?;
        let payload = chunk(request.source, strip)?;
        for row in 0..rows[tile] {
            let source = &payload[row * row_bytes..][..row_bytes];
            unpack_word_stream(
                source,
                &mut output[(start_row + row) * request.width..][..request.width],
            )?;
        }
    }
    buffer_u16(request.width, request.height, output)
}

fn unpack_row(
    source: &[u8],
    target: &mut [u16],
    bits: u8,
    byte_order: ByteOrder,
) -> Result<(), CodecError> {
    let bits = u32::from(bits);
    let mask = (1_u64 << bits) - 1;
    let mut buffer = 0_u64;
    let mut buffered = 0_u32;
    let mut bytes = source.iter().copied();
    match byte_order {
        ByteOrder::Little => {
            for value in target {
                while buffered < bits {
                    buffer |= u64::from(
                        bytes
                            .next()
                            .ok_or_else(|| invalid("Nikon packed row is truncated"))?,
                    ) << buffered;
                    buffered += 8;
                }
                *value = (buffer & mask) as u16;
                buffer >>= bits;
                buffered -= bits;
            }
        }
        ByteOrder::Big => {
            for value in target {
                while buffered < bits {
                    buffer = (buffer << 8)
                        | u64::from(
                            bytes
                                .next()
                                .ok_or_else(|| invalid("Nikon packed row is truncated"))?,
                        );
                    buffered += 8;
                }
                buffered -= bits;
                *value = ((buffer >> buffered) & mask) as u16;
            }
        }
    }
    Ok(())
}

fn unpack_word_stream(source: &[u8], target: &mut [u16]) -> Result<(), CodecError> {
    let mut words = source.chunks_exact(4);
    let mut buffer = 0_u64;
    let mut buffered = 0_u32;
    for value in target {
        while buffered < 12 {
            let bytes: [u8; 4] = words
                .next()
                .ok_or_else(|| invalid("Nikon mode-5 row is truncated"))?
                .try_into()
                .expect("chunks_exact produced four bytes");
            let word = u32::from_le_bytes(bytes);
            buffer = (buffer << 32) | u64::from(word);
            buffered += 32;
        }
        buffered -= 12;
        *value = ((buffer >> buffered) & 0x0fff) as u16;
    }
    Ok(())
}

fn validate_geometry(request: &DecodeRequest<'_>) -> Result<Vec<usize>, CodecError> {
    if request.width == 0 || request.height == 0 {
        return Err(invalid("Nikon image dimensions must not be zero"));
    }
    if request.chunk_width != request.width || request.chunk_height == 0 {
        return Err(invalid(
            "Nikon strips must span the full image width and have non-zero height",
        ));
    }
    let count = request.height.div_ceil(request.chunk_height);
    if request.chunks.len() != count {
        return Err(invalid(format!(
            "Nikon image requires {count} strips, found {}",
            request.chunks.len()
        )));
    }
    Ok((0..count)
        .map(|index| {
            let start = index * request.chunk_height;
            request.chunk_height.min(request.height - start)
        })
        .collect())
}

fn validate_chunk_lengths(
    request: &DecodeRequest<'_>,
    rows: &[usize],
    required: impl Fn(usize) -> Result<usize, CodecError>,
) -> Result<(), CodecError> {
    for (strip, rows) in request.chunks.iter().zip(rows) {
        let required = required(*rows)?;
        if strip.byte_count < required {
            return Err(invalid(format!(
                "Nikon strip is truncated: {} bytes available, {required} required",
                strip.byte_count
            )));
        }
        chunk(request.source, *strip)?;
    }
    Ok(())
}

fn checked_sample_count(width: usize, height: usize) -> Result<usize, CodecError> {
    width
        .checked_mul(height)
        .ok_or_else(|| invalid("Nikon sample count overflow"))
}

fn packed_row_bytes(width: usize, bits: u8) -> Result<usize, CodecError> {
    width
        .checked_mul(usize::from(bits))
        .map(|bits| bits.div_ceil(8))
        .ok_or_else(|| invalid("Nikon packed row size overflow"))
}

fn validate_bits(bits: u8) -> Result<(), CodecError> {
    if matches!(bits, 12 | 14) {
        Ok(())
    } else {
        Err(invalid(format!(
            "Nikon packed sample depth {bits} is unsupported"
        )))
    }
}
