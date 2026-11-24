use rayon::prelude::*;

use crate::CodecError;
use crate::allocation::zeroed_samples;
use crate::error::invalid;
use crate::request::{ByteOrder, Chunk, SampleFormat};
use crate::sample_buffer::SampleBuffer;

const LARGE_NATIVE_U16_COPY_CHUNK: usize = 16 * 1024 * 1024;
const SMALL_NATIVE_U16_COPY_CHUNK: usize = 8 * 1024 * 1024;
const LARGE_NATIVE_U16_COPY_THRESHOLD: usize = 64 * 1024 * 1024;

pub struct Geometry {
    pub width: usize,
    pub height: usize,
    pub chunk_width: usize,
    pub chunk_height: usize,
    pub across: usize,
    pub down: usize,
}

impl Geometry {
    pub fn new(
        width: usize,
        height: usize,
        chunk_width: usize,
        chunk_height: usize,
        chunk_count: usize,
    ) -> Result<Self, CodecError> {
        if width == 0 || height == 0 {
            return Err(invalid("image dimensions must not be zero"));
        }
        if chunk_width == 0 || chunk_height == 0 {
            return Err(invalid("chunk dimensions must not be zero"));
        }
        checked_product([chunk_width, chunk_height])
            .ok_or_else(|| invalid("chunk sample count overflow"))?;
        let across = width.div_ceil(chunk_width);
        let down = height.div_ceil(chunk_height);
        let expected = across
            .checked_mul(down)
            .ok_or_else(|| invalid("chunk count overflow"))?;
        if chunk_count != expected {
            return Err(invalid(format!(
                "expected {expected} chunks, found {chunk_count}"
            )));
        }
        Ok(Self {
            width,
            height,
            chunk_width,
            chunk_height,
            across,
            down,
        })
    }

    fn sample_count(&self) -> Result<usize, CodecError> {
        checked_product([self.width, self.height])
            .ok_or_else(|| invalid("decoded sample count overflow"))
    }

    pub fn band_stride(&self) -> Result<usize, CodecError> {
        checked_product([self.chunk_height, self.width])
            .ok_or_else(|| invalid("decoded band size overflow"))
    }

    pub fn chunk_sample_count(&self) -> Result<usize, CodecError> {
        checked_product([self.chunk_width, self.chunk_height])
            .ok_or_else(|| invalid("chunk sample count overflow"))
    }
}

pub fn chunk(source: &[u8], chunk: Chunk) -> Result<&[u8], CodecError> {
    let end = chunk
        .offset
        .checked_add(chunk.byte_count)
        .ok_or_else(|| invalid("chunk range overflow"))?;
    source
        .get(chunk.offset..end)
        .ok_or_else(|| invalid("chunk offset or byte count is out of bounds"))
}

pub fn decode_payloads(
    geometry: &Geometry,
    bits_per_sample: u16,
    sample_format: SampleFormat,
    byte_order: ByteOrder,
    payloads: &[&[u8]],
) -> Result<SampleBuffer, CodecError> {
    if payloads.len() != geometry.across * geometry.down {
        return Err(invalid("decoded chunk count does not match image geometry"));
    }

    match sample_format {
        SampleFormat::Float => decode_float(geometry, bits_per_sample, byte_order, payloads),
        SampleFormat::Integer => decode_integer(geometry, bits_per_sample, byte_order, payloads),
    }
}

fn decode_integer(
    geometry: &Geometry,
    bits: u16,
    byte_order: ByteOrder,
    payloads: &[&[u8]],
) -> Result<SampleBuffer, CodecError> {
    let sample_count = geometry.sample_count()?;
    match bits {
        1..=8 => {
            let mut output = zeroed_samples(sample_count)?;
            if bits == 8 {
                if geometry.across == 1 && geometry.chunk_width == geometry.width {
                    copy_u8_full_strips(geometry, payloads, &mut output)?;
                } else {
                    copy_samples(geometry, byte_order, payloads, &mut output)?;
                }
            } else {
                // `bits <= 8`, so the unpacked value always fits.
                decode_packed(geometry, bits, payloads, low_u8, &mut output)?;
            }
            buffer_u8(geometry.width, geometry.height, output)
        }
        9..=16 => {
            let mut output = zeroed_samples(sample_count)?;
            if bits == 16 {
                if geometry.across == 1 && geometry.chunk_width == geometry.width {
                    unpack_u16_word_rows(
                        payloads,
                        &mut output,
                        geometry.width,
                        geometry.height,
                        geometry.chunk_height,
                        byte_order,
                        u16::MAX,
                    )?;
                } else {
                    copy_samples(geometry, byte_order, payloads, &mut output)?;
                }
            } else if matches!(bits, 10 | 14)
                && geometry.across == 1
                && geometry.chunk_width == geometry.width
            {
                unpack_packed_rows(
                    geometry,
                    bits,
                    geometry
                        .width
                        .checked_mul(usize::from(bits))
                        .ok_or_else(|| invalid("packed-row bit count overflow"))?
                        .div_ceil(8),
                    payloads,
                    &mut output,
                    |source, target| {
                        unpack_row_u16(source, target, bits, byte_order);
                        Ok(())
                    },
                )?;
            } else if matches!(bits, 12 | 14) {
                decode_packed_u16(geometry, bits, byte_order, payloads, &mut output)?;
            } else {
                // `bits <= 16`, so the unpacked value always fits.
                decode_packed(geometry, bits, payloads, low_u16, &mut output)?;
            }
            buffer_u16(geometry.width, geometry.height, output)
        }
        17..=31 => {
            let mut output = zeroed_samples(sample_count)?;
            decode_packed(geometry, bits, payloads, |value| value, &mut output)?;
            buffer_u32(geometry.width, geometry.height, output)
        }
        32 => {
            let mut output = zeroed_samples(sample_count)?;
            copy_samples(geometry, byte_order, payloads, &mut output)?;
            buffer_u32(geometry.width, geometry.height, output)
        }
        _ => Err(invalid(format!("integer sample depth {bits} is invalid"))),
    }
}

fn decode_float(
    geometry: &Geometry,
    bits: u16,
    byte_order: ByteOrder,
    payloads: &[&[u8]],
) -> Result<SampleBuffer, CodecError> {
    let bytes_per_sample = match bits {
        16 => 2,
        24 => 3,
        32 => 4,
        64 => 8,
        _ => {
            return Err(invalid(format!(
                "floating-point sample depth {bits} is invalid"
            )));
        }
    };
    if bits == 64 {
        let sample_count = geometry.sample_count()?;
        let mut output = zeroed_samples(sample_count)?;
        decode_float_with(geometry, bytes_per_sample, payloads, &mut output, |bytes| {
            f64::from_bits(read_u64(bytes, byte_order))
        })?;
        return buffer_f64(geometry.width, geometry.height, output);
    }
    let sample_count = geometry.sample_count()?;
    let mut output = zeroed_samples(sample_count)?;

    // Width and byte order are resolved once here, not per sample.
    macro_rules! go {
        ($read:expr) => {
            decode_float_with(geometry, bytes_per_sample, payloads, &mut output, $read)?
        };
    }
    match (bits, byte_order) {
        (16, ByteOrder::Little) => {
            go!(|b| extend_float(u32::from(read_u16(b, ByteOrder::Little)), 10, 5));
        }
        (16, ByteOrder::Big) => {
            go!(|b| extend_float(u32::from(read_u16(b, ByteOrder::Big)), 10, 5));
        }
        (24, ByteOrder::Little) => go!(|b| extend_float(read_u24(b, ByteOrder::Little), 16, 7)),
        (24, ByteOrder::Big) => go!(|b| extend_float(read_u24(b, ByteOrder::Big), 16, 7)),
        (32, ByteOrder::Little) => go!(|b| f32::from_bits(read_u32(b, ByteOrder::Little))),
        (32, ByteOrder::Big) => go!(|b| f32::from_bits(read_u32(b, ByteOrder::Big))),
        _ => unreachable!("the sample width is validated above"),
    }
    buffer_f32(geometry.width, geometry.height, output)
}

fn decode_float_with<T, F>(
    geometry: &Geometry,
    bytes_per_sample: usize,
    payloads: &[&[u8]],
    output: &mut [T],
    read: F,
) -> Result<(), CodecError>
where
    T: Send,
    F: Fn(&[u8]) -> T + Sync,
{
    par_tiles(
        geometry,
        bytes_per_sample,
        payloads,
        output,
        |source, target| {
            for (target, bytes) in target.iter_mut().zip(source.chunks_exact(bytes_per_sample)) {
                *target = read(bytes);
            }
        },
    )
}

/// Walks every tile row of the image in parallel (one rayon task per tile
/// band), validates the chunk size, and hands the matching source bytes and
/// destination samples of each row to `row`.
///
/// `source` and `target` passed to `row` always cover exactly `copy_width`
/// samples.
fn par_tiles<T, F>(
    geometry: &Geometry,
    bytes_per_sample: usize,
    payloads: &[&[u8]],
    output: &mut [T],
    row: F,
) -> Result<(), CodecError>
where
    T: Send,
    F: Fn(&[u8], &mut [T]) + Sync,
{
    output
        .par_chunks_mut(geometry.band_stride()?)
        .enumerate()
        .try_for_each(|(tile_y_index, band)| {
            let tile_y = tile_y_index * geometry.chunk_height;
            let copy_height = geometry.chunk_height.min(geometry.height - tile_y);
            let source_stride = geometry.chunk_width * bytes_per_sample;
            for tile_x_index in 0..geometry.across {
                let index = tile_y_index * geometry.across + tile_x_index;
                let encoded = payloads[index];
                let tile_x = tile_x_index * geometry.chunk_width;
                let copy_width = geometry.chunk_width.min(geometry.width - tile_x);
                let required_samples = (copy_height - 1)
                    .checked_mul(geometry.chunk_width)
                    .and_then(|count| count.checked_add(copy_width))
                    .ok_or_else(|| invalid("uncompressed chunk size overflow"))?;
                let required_bytes = required_samples
                    .checked_mul(bytes_per_sample)
                    .ok_or_else(|| invalid("uncompressed chunk size overflow"))?;
                if encoded.len() < required_bytes {
                    return Err(invalid(format!("uncompressed chunk {index} is truncated")));
                }

                let copy_bytes = copy_width * bytes_per_sample;
                for y in 0..copy_height {
                    let source = &encoded[y * source_stride..][..copy_bytes];
                    let target = &mut band[y * geometry.width + tile_x..][..copy_width];
                    row(source, target);
                }
            }
            Ok::<(), CodecError>(())
        })
}

fn decode_packed<T>(
    geometry: &Geometry,
    bits: u16,
    payloads: &[&[u8]],
    convert: impl Fn(u32) -> T + Sync,
    output: &mut [T],
) -> Result<(), CodecError>
where
    T: Send,
{
    let bits = usize::from(bits);
    output
        .par_chunks_mut(geometry.band_stride()?)
        .enumerate()
        .try_for_each(|(tile_y_index, band)| {
            let tile_y = tile_y_index * geometry.chunk_height;
            let copy_height = geometry.chunk_height.min(geometry.height - tile_y);
            // One scratch buffer per band, reused for every tile in it.
            let mut unpacked = vec![0_u32; geometry.chunk_sample_count()?];
            for tile_x_index in 0..geometry.across {
                let index = tile_y_index * geometry.across + tile_x_index;
                unpack_bits(payloads[index], &mut unpacked, bits)?;

                let tile_x = tile_x_index * geometry.chunk_width;
                let copy_width = geometry.chunk_width.min(geometry.width - tile_x);
                for y in 0..copy_height {
                    let source = &unpacked[y * geometry.chunk_width..][..copy_width];
                    let target = &mut band[y * geometry.width + tile_x..][..copy_width];
                    for (target, &value) in target.iter_mut().zip(source) {
                        *target = convert(value);
                    }
                }
            }
            Ok::<(), CodecError>(())
        })
}

fn copy_u8_full_strips(
    geometry: &Geometry,
    payloads: &[&[u8]],
    output: &mut [u8],
) -> Result<(), CodecError> {
    let sample_count = geometry.sample_count()?;
    if output.len() != sample_count {
        return Err(invalid(format!(
            "8-bit strip output has {} samples, expected {sample_count}",
            output.len()
        )));
    }
    let expected_chunks = geometry.height.div_ceil(geometry.chunk_height);
    if payloads.len() != expected_chunks {
        return Err(invalid(format!(
            "8-bit image requires {expected_chunks} strips, found {}",
            payloads.len()
        )));
    }
    for (index, payload) in payloads.iter().enumerate() {
        let first_row = index * geometry.chunk_height;
        let rows = geometry.chunk_height.min(geometry.height - first_row);
        let samples = rows
            .checked_mul(geometry.width)
            .ok_or_else(|| invalid("8-bit strip size overflow"))?;
        let source = payload
            .get(..samples)
            .ok_or_else(|| invalid(format!("uncompressed chunk {index} is truncated")))?;
        let destination = output
            .get_mut(first_row * geometry.width..first_row * geometry.width + samples)
            .ok_or_else(|| invalid("8-bit strip output range overflow"))?;
        destination.copy_from_slice(source);
    }
    Ok(())
}

fn unpack_packed_rows<T, F>(
    geometry: &Geometry,
    bits: u16,
    row_stride: usize,
    payloads: &[&[u8]],
    output: &mut [T],
    unpack_row: F,
) -> Result<(), CodecError>
where
    T: Send,
    F: Fn(&[u8], &mut [T]) -> Result<(), CodecError> + Sync,
{
    let bits = usize::from(bits);
    if bits == 0 || bits > 32 {
        return Err(invalid("packed sample depth must be between 1 and 32"));
    }
    let sample_count = geometry.sample_count()?;
    if output.len() != sample_count {
        return Err(invalid(format!(
            "packed-row output has {} samples, expected {sample_count}",
            output.len()
        )));
    }
    let expected_chunks = geometry.height.div_ceil(geometry.chunk_height);
    if payloads.len() != expected_chunks {
        return Err(invalid(format!(
            "packed-row image requires {expected_chunks} strips, found {}",
            payloads.len()
        )));
    }
    let row_bytes = geometry
        .width
        .checked_mul(bits)
        .ok_or_else(|| invalid("packed-row bit count overflow"))?
        .div_ceil(8);
    if row_stride < row_bytes {
        return Err(invalid(format!(
            "packed-row stride {row_stride} is shorter than required {row_bytes}"
        )));
    }
    for (index, payload) in payloads.iter().enumerate() {
        let rows = geometry
            .chunk_height
            .min(geometry.height - index * geometry.chunk_height);
        let required = rows
            .checked_mul(row_stride)
            .ok_or_else(|| invalid("packed-row strip size overflow"))?;
        if payload.len() < required {
            return Err(invalid(format!(
                "uncompressed chunk {index} is truncated: {} bytes available, {required} required",
                payload.len()
            )));
        }
    }

    let unpack = |row: usize, target: &mut [T]| -> Result<(), CodecError> {
        let chunk = row / geometry.chunk_height;
        let within_chunk = row % geometry.chunk_height;
        let start = within_chunk
            .checked_mul(row_stride)
            .ok_or_else(|| invalid("packed-row source offset overflow"))?;
        unpack_row(&payloads[chunk][start..start + row_bytes], target)
    };
    let threads = rayon::current_num_threads().max(1);
    if should_parallelize_word_rows(output.len(), threads) {
        let rows_per_item = geometry.height.div_ceil(threads).max(1);
        let samples_per_item = geometry
            .width
            .checked_mul(rows_per_item)
            .ok_or_else(|| invalid("packed-row parallel band size overflow"))?;
        output
            .par_chunks_mut(samples_per_item)
            .enumerate()
            .try_for_each(|(band, rows)| {
                let first_row = band * rows_per_item;
                for (offset, target) in rows.chunks_exact_mut(geometry.width).enumerate() {
                    unpack(first_row + offset, target)?;
                }
                Ok::<(), CodecError>(())
            })?;
    } else {
        for (row, target) in output.chunks_exact_mut(geometry.width).enumerate() {
            unpack(row, target)?;
        }
    }
    Ok(())
}

/// Unpacks full-width packed 16-bit rows directly into the final sample buffer.
///
/// `row_stride` may include alignment padding after each packed row. The helper
/// validates all strips before writing, uses the shared four-million-sample
/// parallel cutoff, and invokes `unpack_row` once per output row.
///
/// # Errors
///
/// Returns an error when the geometry is inconsistent, the stride is too short,
/// a strip is truncated, or the output length does not match the image.
pub fn unpack_packed_u16_rows<F>(
    payloads: &[&[u8]],
    output: &mut [u16],
    width: usize,
    height: usize,
    chunk_height: usize,
    row_stride: usize,
    bits: u16,
    unpack_row: F,
) -> Result<(), CodecError>
where
    F: Fn(&[u8], &mut [u16]) -> Result<(), CodecError> + Sync,
{
    let geometry = Geometry::new(width, height, width, chunk_height, payloads.len())?;
    unpack_packed_rows(&geometry, bits, row_stride, payloads, output, unpack_row)
}

fn unpack_bits(source: &[u8], output: &mut [u32], bits: usize) -> Result<(), CodecError> {
    if !(1..=32).contains(&bits) {
        return Err(invalid("packed sample depth must be between 1 and 32"));
    }
    let needed = output
        .len()
        .checked_mul(bits)
        .ok_or_else(|| invalid("packed sample bit count overflow"))?
        .div_ceil(8);
    if source.len() < needed {
        return Err(invalid("packed sample payload is truncated"));
    }

    let mask = (1_u64 << bits) - 1;
    // Old bits shift out of the top of the accumulator; only the low
    // `buffered_bits + 8` (at most 39) bits are ever read, so no per-sample
    // masking of the buffer is needed.
    let mut buffer = 0_u64;
    let mut buffered_bits = 0_usize;
    let mut bytes = source.iter();
    for value in output {
        while buffered_bits < bits {
            // The length check above guarantees enough bytes for every sample.
            let byte = bytes.next().copied().unwrap_or(0);
            buffer = (buffer << 8) | u64::from(byte);
            buffered_bits += 8;
        }
        buffered_bits -= bits;
        *value = low_u32((buffer >> buffered_bits) & mask);
    }
    Ok(())
}

fn decode_packed_u16(
    geometry: &Geometry,
    bits: u16,
    byte_order: ByteOrder,
    payloads: &[&[u8]],
    output: &mut [u16],
) -> Result<(), CodecError> {
    let row_bytes = geometry
        .chunk_width
        .checked_mul(usize::from(bits))
        .ok_or_else(|| invalid("packed row bit count overflow"))?
        .div_ceil(8);
    output
        .par_chunks_mut(geometry.band_stride()?)
        .enumerate()
        .try_for_each(|(tile_y_index, band)| {
            let tile_y = tile_y_index * geometry.chunk_height;
            let copy_height = geometry.chunk_height.min(geometry.height - tile_y);
            for tile_x_index in 0..geometry.across {
                let index = tile_y_index * geometry.across + tile_x_index;
                let encoded = payloads[index];
                let tile_x = tile_x_index * geometry.chunk_width;
                let copy_width = geometry.chunk_width.min(geometry.width - tile_x);
                let copy_row_bytes = copy_width
                    .checked_mul(usize::from(bits))
                    .ok_or_else(|| invalid("packed row bit count overflow"))?
                    .div_ceil(8);
                let required_bytes = (copy_height - 1)
                    .checked_mul(row_bytes)
                    .and_then(|bytes| bytes.checked_add(copy_row_bytes))
                    .ok_or_else(|| invalid("uncompressed chunk size overflow"))?;
                if encoded.len() < required_bytes {
                    return Err(invalid(format!("uncompressed chunk {index} is truncated")));
                }

                for y in 0..copy_height {
                    let source = &encoded[y * row_bytes..][..copy_row_bytes];
                    let target = &mut band[y * geometry.width + tile_x..][..copy_width];
                    unpack_row_u16(source, target, bits, byte_order);
                }
            }
            Ok::<(), CodecError>(())
        })
}

fn unpack_row_u16(source: &[u8], target: &mut [u16], bits: u16, byte_order: ByteOrder) {
    match bits {
        10 => unpack_10_msb(source, target),
        12 => unpack_12_msb(source, target),
        14 => unpack_14_msb(source, target),
        16 if byte_order == native_order() => {
            let byte_count = target.len() * 2;
            bytemuck::cast_slice_mut(target).copy_from_slice(&source[..byte_count]);
        }
        16 => {
            for (target, source) in target.iter_mut().zip(source.as_chunks::<2>().0) {
                *target = read_u16(source, byte_order);
            }
        }
        _ => unpack_scalar(source, target, usize::from(bits), false),
    }
}

fn unpack_10_msb(source: &[u8], target: &mut [u16]) {
    let groups = (target.len() / 4).min(source.len() / 5);
    for group in 0..groups {
        let bytes = &source[group * 5..][..5];
        target[group * 4] = (u16::from(bytes[0]) << 2) | (u16::from(bytes[1]) >> 6);
        target[group * 4 + 1] = (u16::from(bytes[1] & 0x3f) << 4) | (u16::from(bytes[2]) >> 4);
        target[group * 4 + 2] = (u16::from(bytes[2] & 0x0f) << 6) | (u16::from(bytes[3]) >> 2);
        target[group * 4 + 3] = (u16::from(bytes[3] & 0x03) << 8) | u16::from(bytes[4]);
    }
    unpack_scalar(&source[groups * 5..], &mut target[groups * 4..], 10, false);
}

fn unpack_12_msb(source: &[u8], target: &mut [u16]) {
    let groups = (target.len() / 2).min(source.len() / 3);
    for group in 0..groups {
        let bytes = &source[group * 3..][..3];
        target[group * 2] = (u16::from(bytes[0]) << 4) | (u16::from(bytes[1]) >> 4);
        target[group * 2 + 1] = (u16::from(bytes[1] & 0x0f) << 8) | u16::from(bytes[2]);
    }
    unpack_scalar(&source[groups * 3..], &mut target[groups * 2..], 12, false);
}

#[cfg(test)]
fn unpack_12_lsb(source: &[u8], target: &mut [u16]) {
    let groups = (target.len() / 2).min(source.len() / 3);
    for group in 0..groups {
        let bytes = &source[group * 3..][..3];
        target[group * 2] = u16::from(bytes[0]) | (u16::from(bytes[1] & 0x0f) << 8);
        target[group * 2 + 1] = (u16::from(bytes[1]) >> 4) | (u16::from(bytes[2]) << 4);
    }
    unpack_scalar(&source[groups * 3..], &mut target[groups * 2..], 12, true);
}

fn unpack_14_msb(source: &[u8], target: &mut [u16]) { unpack_14_msb_kernel(source, target); }

fn unpack_14_msb_kernel(source: &[u8], target: &mut [u16]) {
    const GROUP_BYTES: usize = 14;
    const GROUP_SAMPLES: usize = 8;
    const MASK: u64 = 0x3fff;

    let groups = (target.len() / GROUP_SAMPLES).min(source.len() / GROUP_BYTES);
    for group in 0..groups {
        let bytes = &source[group * GROUP_BYTES..][..GROUP_BYTES];
        let high = u64::from_be_bytes(
            bytes[..8]
                .try_into()
                .expect("the first half of a 14-bit group is eight bytes"),
        );
        let low = u64::from_be_bytes([
            0, 0, bytes[8], bytes[9], bytes[10], bytes[11], bytes[12], bytes[13],
        ]);
        let target = &mut target[group * GROUP_SAMPLES..][..GROUP_SAMPLES];
        target[0] = ((high >> 50) & MASK) as u16;
        target[1] = ((high >> 36) & MASK) as u16;
        target[2] = ((high >> 22) & MASK) as u16;
        target[3] = ((high >> 8) & MASK) as u16;
        target[4] = (((high & 0xff) << 6) | (low >> 42)) as u16;
        target[5] = ((low >> 28) & MASK) as u16;
        target[6] = ((low >> 14) & MASK) as u16;
        target[7] = (low & MASK) as u16;
    }
    unpack_scalar(
        &source[groups * GROUP_BYTES..],
        &mut target[groups * GROUP_SAMPLES..],
        14,
        false,
    );
}

#[cfg(test)]
fn unpack_14_lsb(source: &[u8], target: &mut [u16]) {
    let groups = (target.len() / 4).min(source.len() / 7);
    for group in 0..groups {
        let bytes = &source[group * 7..][..7];
        let word = u64::from_le_bytes([
            bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], 0,
        ]);
        for index in 0..4 {
            target[group * 4 + index] = ((word >> (14 * index)) & 0x3fff) as u16;
        }
    }
    unpack_scalar(&source[groups * 7..], &mut target[groups * 4..], 14, true);
}

fn unpack_scalar(source: &[u8], target: &mut [u16], bits: usize, lsb: bool) {
    if bits == 0 || target.is_empty() {
        return;
    }
    let mask = (1_u64 << bits) - 1;
    let mut buffer = 0_u64;
    let mut buffered = 0_usize;
    let mut bytes = source.iter().copied();
    if lsb {
        for value in target {
            while buffered < bits {
                buffer |= u64::from(bytes.next().unwrap_or(0)) << buffered;
                buffered += 8;
            }
            *value = (buffer & mask) as u16;
            buffer >>= bits;
            buffered -= bits;
        }
    } else {
        for value in target {
            while buffered < bits {
                buffer = (buffer << 8) | u64::from(bytes.next().unwrap_or(0));
                buffered += 8;
            }
            buffered -= bits;
            *value = ((buffer >> buffered) & mask) as u16;
        }
    }
}

/// Samples that can be read straight from a tile row of raw bytes.
trait Sample: Copy + Send {
    const BYTES: usize;

    /// `source.len() == target.len() * Self::BYTES`.
    fn read_row(source: &[u8], target: &mut [Self], byte_order: ByteOrder);
}

impl Sample for u8 {
    const BYTES: usize = 1;

    fn read_row(source: &[u8], target: &mut [Self], _byte_order: ByteOrder) {
        target.copy_from_slice(source);
    }
}

macro_rules! impl_sample {
    ($ty:ty, $bytes:expr) => {
        impl Sample for $ty {
            const BYTES: usize = $bytes;

            fn read_row(source: &[u8], target: &mut [Self], byte_order: ByteOrder) {
                let chunks = source.chunks_exact($bytes);
                match byte_order {
                    ByteOrder::Little => {
                        for (target, bytes) in target.iter_mut().zip(chunks) {
                            *target = <$ty>::from_le_bytes(bytes.try_into().expect("fixed width"));
                        }
                    }
                    ByteOrder::Big => {
                        for (target, bytes) in target.iter_mut().zip(chunks) {
                            *target = <$ty>::from_be_bytes(bytes.try_into().expect("fixed width"));
                        }
                    }
                }
            }
        }
    };
}

impl Sample for u16 {
    const BYTES: usize = 2;

    fn read_row(source: &[u8], target: &mut [Self], byte_order: ByteOrder) {
        if byte_order == native_order() {
            bytemuck::cast_slice_mut(target).copy_from_slice(source);
        } else {
            for (target, bytes) in target.iter_mut().zip(source.as_chunks::<2>().0) {
                *target = read_u16(bytes, byte_order);
            }
        }
    }
}

impl_sample!(u32, 4);

/// Copies tiles straight into the typed output, with no intermediate byte
/// buffer.
fn copy_samples<T>(
    geometry: &Geometry,
    byte_order: ByteOrder,
    payloads: &[&[u8]],
    output: &mut [T],
) -> Result<(), CodecError>
where
    T: Sample,
{
    par_tiles(geometry, T::BYTES, payloads, output, |source, target| {
        T::read_row(source, target, byte_order);
    })
}

// Keep small 8 MiB-or-less mosaics on the calling thread. This avoids Rayon
// setup for low-millisecond fixed-cost decodes while still parallelizing every
// current large uncompressed camera path.
const PARALLEL_ROW_MIN_SAMPLES: usize = 1 << 22;

/// Unpacks full-width strips of 16-bit words directly into `output`.
///
/// Strips are validated before decoding. Large images are parallelized by row,
/// while small images stay on the calling thread to avoid scheduling overhead.
///
/// # Errors
///
/// Returns an error when the geometry is invalid, the strip count does not
/// match the image height, `output` has the wrong length, or a strip is
/// truncated.
pub fn unpack_u16_word_rows(
    payloads: &[&[u8]],
    output: &mut [u16],
    width: usize,
    height: usize,
    chunk_height: usize,
    byte_order: ByteOrder,
    mask: u16,
) -> Result<(), CodecError> {
    if width == 0 || height == 0 || chunk_height == 0 {
        return Err(invalid("word-row dimensions must not be zero"));
    }
    let sample_count = width
        .checked_mul(height)
        .ok_or_else(|| invalid("word-row sample count overflow"))?;
    if output.len() != sample_count {
        return Err(invalid(format!(
            "word-row output has {} samples, expected {sample_count}",
            output.len()
        )));
    }
    let expected_chunks = height.div_ceil(chunk_height);
    if payloads.len() != expected_chunks {
        return Err(invalid(format!(
            "word-row image requires {expected_chunks} strips, found {}",
            payloads.len()
        )));
    }
    let row_bytes = width
        .checked_mul(2)
        .ok_or_else(|| invalid("word-row byte count overflow"))?;
    for (index, payload) in payloads.iter().enumerate() {
        let rows = chunk_height.min(height - index * chunk_height);
        let required = rows
            .checked_mul(row_bytes)
            .ok_or_else(|| invalid("word-row strip size overflow"))?;
        if payload.len() < required {
            return Err(invalid(format!(
                "word-row strip {index} is truncated: {} bytes available, {required} required",
                payload.len()
            )));
        }
    }

    let threads = rayon::current_num_threads();
    let byte_count = sample_count
        .checked_mul(2)
        .ok_or_else(|| invalid("word-row byte count overflow"))?;
    if byte_order == native_order()
        && mask == u16::MAX
        && payloads.len() == 1
        && should_parallelize_word_rows(output.len(), threads)
    {
        let source = payloads[0]
            .get(..byte_count)
            .ok_or_else(|| invalid("word-row strip is truncated"))?;
        let chunk_bytes = if byte_count >= LARGE_NATIVE_U16_COPY_THRESHOLD {
            LARGE_NATIVE_U16_COPY_CHUNK
        } else {
            SMALL_NATIVE_U16_COPY_CHUNK
        };
        let samples_per_item = chunk_bytes / 2;
        output
            .par_chunks_mut(samples_per_item)
            .zip(source.par_chunks(chunk_bytes))
            .for_each(|(target, source)| {
                bytemuck::cast_slice_mut(target).copy_from_slice(source);
            });
        return Ok(());
    }

    let unpack_row = |row: usize, target: &mut [u16]| {
        let chunk = row / chunk_height;
        let within_chunk = row % chunk_height;
        let start = within_chunk * row_bytes;
        let source = &payloads[chunk][start..start + row_bytes];
        unpack_u16_word_row(source, target, byte_order, mask);
    };
    if output.len() >= PARALLEL_ROW_MIN_SAMPLES && threads > 1 {
        let rows_per_item = height.div_ceil(threads * 4).max(1);
        let samples_per_item = width
            .checked_mul(rows_per_item)
            .ok_or_else(|| invalid("word-row parallel band size overflow"))?;
        output
            .par_chunks_mut(samples_per_item)
            .enumerate()
            .for_each(|(band, rows)| {
                let first_row = band * rows_per_item;
                for (offset, target) in rows.chunks_exact_mut(width).enumerate() {
                    unpack_row(first_row + offset, target);
                }
            });
    } else {
        for (row, target) in output.chunks_exact_mut(width).enumerate() {
            unpack_row(row, target);
        }
    }
    Ok(())
}

const fn should_parallelize_word_rows(sample_count: usize, threads: usize) -> bool {
    sample_count >= PARALLEL_ROW_MIN_SAMPLES && threads > 1
}

fn unpack_u16_word_row(source: &[u8], target: &mut [u16], byte_order: ByteOrder, mask: u16) {
    if byte_order == native_order() && mask == u16::MAX {
        bytemuck::cast_slice_mut(target).copy_from_slice(source);
        return;
    }
    match byte_order {
        ByteOrder::Little => {
            for (target, bytes) in target.iter_mut().zip(source.as_chunks::<2>().0) {
                *target = u16::from_le_bytes(*bytes) & mask;
            }
        }
        ByteOrder::Big => {
            for (target, bytes) in target.iter_mut().zip(source.as_chunks::<2>().0) {
                *target = u16::from_be_bytes(*bytes) & mask;
            }
        }
    }
}

const fn low_u8(value: u32) -> u8 { value.to_le_bytes()[0] }

const fn low_u16(value: u32) -> u16 {
    let bytes = value.to_le_bytes();
    u16::from_le_bytes([bytes[0], bytes[1]])
}

const fn low_u32(value: u64) -> u32 {
    let bytes = value.to_le_bytes();
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

const fn native_order() -> ByteOrder {
    if cfg!(target_endian = "little") {
        ByteOrder::Little
    } else {
        ByteOrder::Big
    }
}

pub const fn read_u16(bytes: &[u8], byte_order: ByteOrder) -> u16 {
    let bytes = [bytes[0], bytes[1]];
    match byte_order {
        ByteOrder::Little => u16::from_le_bytes(bytes),
        ByteOrder::Big => u16::from_be_bytes(bytes),
    }
}

pub const fn read_u24(bytes: &[u8], byte_order: ByteOrder) -> u32 {
    match byte_order {
        ByteOrder::Little => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]),
        ByteOrder::Big => u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]),
    }
}

pub const fn read_u32(bytes: &[u8], byte_order: ByteOrder) -> u32 {
    let bytes = [bytes[0], bytes[1], bytes[2], bytes[3]];
    match byte_order {
        ByteOrder::Little => u32::from_le_bytes(bytes),
        ByteOrder::Big => u32::from_be_bytes(bytes),
    }
}

const fn read_u64(bytes: &[u8], byte_order: ByteOrder) -> u64 {
    let bytes = [
        bytes[0], bytes[1], bytes[2], bytes[3], bytes[4], bytes[5], bytes[6], bytes[7],
    ];
    match byte_order {
        ByteOrder::Little => u64::from_le_bytes(bytes),
        ByteOrder::Big => u64::from_be_bytes(bytes),
    }
}

pub fn extend_float(raw: u32, fraction_bits: u32, exponent_bits: u32) -> f32 {
    let storage_bits = 1 + exponent_bits + fraction_bits;
    let sign = (raw >> (storage_bits - 1)) & 1;
    let exponent_mask = (1_u32 << exponent_bits) - 1;
    let exponent = (raw >> fraction_bits) & exponent_mask;
    let fraction = raw & ((1_u32 << fraction_bits) - 1);
    let bias = i32::try_from(exponent_mask >> 1).expect("float exponent bias fits i32");

    let (wide_exponent, wide_fraction) = if exponent == exponent_mask {
        (u32::from(u8::MAX), fraction << (23 - fraction_bits))
    } else if exponent == 0 {
        if fraction == 0 {
            (0, 0)
        } else {
            let mut wide_exponent = 1 - bias + 127;
            let mut wide_fraction = fraction << (23 - fraction_bits);
            while wide_fraction & (1 << 23) == 0 {
                wide_exponent -= 1;
                wide_fraction <<= 1;
            }
            (
                u32::try_from(wide_exponent).expect("widened exponent fits u32"),
                wide_fraction & ((1 << 23) - 1),
            )
        }
    } else {
        (
            u32::try_from(i32::try_from(exponent).expect("exponent fits i32") - bias + 127)
                .expect("widened exponent fits u32"),
            fraction << (23 - fraction_bits),
        )
    };
    f32::from_bits((sign << 31) | (wide_exponent << 23) | wide_fraction)
}

fn dimensions(width: usize, height: usize) -> Result<(u32, u32), CodecError> {
    let width = u32::try_from(width).map_err(|_| invalid("image width exceeds u32"))?;
    let height = u32::try_from(height).map_err(|_| invalid("image height exceeds u32"))?;
    Ok((width, height))
}

pub fn buffer_u8(
    width: usize,
    height: usize,
    samples: Vec<u8>,
) -> Result<SampleBuffer, CodecError> {
    let (width, height) = dimensions(width, height)?;
    SampleBuffer::from_u8(width, height, samples).map_err(Into::into)
}

pub fn buffer_u16(
    width: usize,
    height: usize,
    samples: Vec<u16>,
) -> Result<SampleBuffer, CodecError> {
    let (width, height) = dimensions(width, height)?;
    SampleBuffer::from_u16(width, height, samples).map_err(Into::into)
}

fn buffer_u32(width: usize, height: usize, samples: Vec<u32>) -> Result<SampleBuffer, CodecError> {
    let (width, height) = dimensions(width, height)?;
    SampleBuffer::from_u32(width, height, samples).map_err(Into::into)
}

fn buffer_f32(width: usize, height: usize, samples: Vec<f32>) -> Result<SampleBuffer, CodecError> {
    let (width, height) = dimensions(width, height)?;
    SampleBuffer::from_f32(width, height, samples).map_err(Into::into)
}

fn buffer_f64(width: usize, height: usize, samples: Vec<f64>) -> Result<SampleBuffer, CodecError> {
    let (width, height) = dimensions(width, height)?;
    SampleBuffer::from_f64(width, height, samples).map_err(Into::into)
}

fn checked_product<const N: usize>(values: [usize; N]) -> Option<usize> {
    values.into_iter().try_fold(1_usize, usize::checked_mul)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn direct_packed_row_kernels_preserve_msb_order_at_unaligned_row_ends() {
        let mut eight = [0_u8; 3];
        let geometry = Geometry::new(3, 1, 3, 1, 1).unwrap();
        copy_u8_full_strips(&geometry, &[&[0x00, 0xff, 0x7f]], &mut eight).unwrap();
        assert_eq!(eight, [0x00, 0xff, 0x7f]);

        let mut ten = [0_u16; 3];
        unpack_10_msb(&[0xff, 0xc0, 0x1a, 0xa8], &mut ten);
        assert_eq!(ten, [0x3ff, 0x001, 0x2aa]);

        let mut fourteen = [0_u16; 3];
        unpack_14_msb(&[0xff, 0xfc, 0x00, 0x1a, 0xaa, 0x80], &mut fourteen);
        assert_eq!(fourteen, [0x3fff, 0x0001, 0x2aaa]);
    }

    #[test]
    fn parallel_native_u16_copy_preserves_a_partial_final_chunk() {
        let width = 8_190;
        let height = 1_025;
        let expected = (0..width * height)
            .map(|index| (index as u16).wrapping_mul(29))
            .collect::<Vec<_>>();
        let source = bytemuck::cast_slice(&expected).to_vec();
        let mut actual = vec![0_u16; expected.len()];

        rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap()
            .install(|| {
                unpack_u16_word_rows(
                    &[&source],
                    &mut actual,
                    width,
                    height,
                    height,
                    ByteOrder::Little,
                    u16::MAX,
                )
                .unwrap();
            });

        assert_eq!(actual, expected);
    }

    #[test]
    fn direct_fourteen_bit_kernel_handles_full_groups_and_row_tail() {
        for len in [8, 11, 31] {
            let expected = (0..len)
                .map(|index| ((index * 2_411 + 97) & 0x3fff) as u16)
                .collect::<Vec<_>>();
            let encoded = pack(&expected, 14, false);
            let mut decoded = vec![0_u16; len];
            unpack_14_msb_kernel(&encoded, &mut decoded);
            assert_eq!(decoded, expected, "14-bit kernel length {len}");
        }
    }

    #[test]
    fn direct_packed_rows_match_the_generic_path_for_short_widths() {
        for bits in [8_u16, 10, 14] {
            for width in [1_usize, 2, 3, 5, 7, 9, 17] {
                let mask = (1_u32 << bits) - 1;
                let expected = (0..width)
                    .map(|index| ((index as u32 * 2_411 + 97) & mask) as u16)
                    .collect::<Vec<_>>();
                let encoded = pack(&expected, usize::from(bits), false);
                let geometry = Geometry::new(width, 1, width, 1, 1).unwrap();
                let decoded = decode_payloads(
                    &geometry,
                    bits,
                    SampleFormat::Integer,
                    ByteOrder::Little,
                    &[&encoded],
                )
                .unwrap();

                let generic_expected = expected
                    .iter()
                    .map(|value| u32::from(*value))
                    .collect::<Vec<_>>();
                let mut generic = vec![0_u32; width];
                unpack_bits(&encoded, &mut generic, usize::from(bits)).unwrap();
                assert_eq!(generic, generic_expected, "{bits}-bit oracle width {width}");
                match bits {
                    8 => assert_eq!(
                        decoded.into_raw_u8().unwrap(),
                        expected
                            .iter()
                            .map(|value| *value as u8)
                            .collect::<Vec<_>>()
                    ),
                    _ => assert_eq!(decoded.into_raw_u16().unwrap(), expected),
                }
            }
        }
    }

    #[test]
    fn grouped_unpack_12_kernels_match_the_scalar_oracle() {
        for len in [1, 2, 3, 4, 5, 7, 8, 9, 17, 31] {
            let samples = (0..len)
                .map(|index| ((index * 977 + 123) & 0x0fff) as u16)
                .collect::<Vec<_>>();
            for lsb in [false, true] {
                let bytes = pack(&samples, 12, lsb);
                let mut decoded = vec![0_u16; len];
                if lsb {
                    unpack_12_lsb(&bytes, &mut decoded);
                } else {
                    unpack_12_msb(&bytes, &mut decoded);
                }
                assert_eq!(decoded, samples, "12-bit {lsb:?} len {len}");
            }
        }
    }

    #[test]
    fn grouped_unpack_14_kernels_match_the_scalar_oracle() {
        for len in [1, 2, 3, 4, 5, 7, 8, 9, 17, 31] {
            let samples = (0..len)
                .map(|index| ((index * 1_597 + 31) & 0x3fff) as u16)
                .collect::<Vec<_>>();
            for lsb in [false, true] {
                let bytes = pack(&samples, 14, lsb);
                let mut decoded = vec![0_u16; len];
                if lsb {
                    unpack_14_lsb(&bytes, &mut decoded);
                } else {
                    unpack_14_msb(&bytes, &mut decoded);
                }
                assert_eq!(decoded, samples, "14-bit {lsb:?} len {len}");
            }
        }
    }

    #[test]
    fn unpack_row_u16_copies_native_endian_and_swaps_opposite_endian_16_bit_rows() {
        let samples = [0x0102_u16, 0x0304, 0xabcd, 0xff00, 0x1234];
        for byte_order in [ByteOrder::Little, ByteOrder::Big] {
            let bytes = samples
                .iter()
                .flat_map(|value| match byte_order {
                    ByteOrder::Little => value.to_le_bytes(),
                    ByteOrder::Big => value.to_be_bytes(),
                })
                .collect::<Vec<_>>();
            let mut decoded = [0_u16; 5];
            unpack_row_u16(&bytes, &mut decoded, 16, byte_order);
            assert_eq!(decoded, samples);
        }
    }

    #[test]
    fn row_word_unpack_handles_non_multiple_widths_and_partial_last_strips() {
        let width = 5;
        let height = 5;
        let chunk_height = 2;
        let payloads = [
            little_payload(&[1, 2, 3, 4, 5, 6, 7, 8, 9, 10]),
            little_payload(&[11, 12, 13, 14, 15, 16, 17, 18, 19, 20]),
            little_payload(&[21, 22, 23, 24, 25]),
        ];
        let payload_refs = payloads.iter().map(Vec::as_slice).collect::<Vec<_>>();
        let mut output = [0_u16; 25];

        super::unpack_u16_word_rows(
            &payload_refs,
            &mut output,
            width,
            height,
            chunk_height,
            ByteOrder::Little,
            u16::MAX,
        )
        .unwrap();

        assert_eq!(output, std::array::from_fn(|index| index as u16 + 1));
    }

    #[test]
    fn row_word_unpack_handles_single_row_and_tiny_images() {
        let single =
            unpack_u16_word_rows_to_vec(&[&[1, 0, 2, 0, 3, 0]], 3, 1, 1, ByteOrder::Little);
        assert_eq!(single, [1, 2, 3]);

        let tiny = unpack_u16_word_rows_to_vec(&[&[0x34, 0x12]], 1, 1, 1, ByteOrder::Little);
        assert_eq!(tiny, [0x1234]);
    }

    #[test]
    fn word_row_parallelism_keeps_small_two_k_mosaics_serial() {
        assert!(!super::should_parallelize_word_rows(1_936 * 1_090, 16));
        assert!(super::should_parallelize_word_rows(3_968 * 2_648, 16));
        assert!(!super::should_parallelize_word_rows(3_968 * 2_648, 1));
    }

    #[test]
    fn row_word_unpack_rejects_truncated_chunks() {
        let mut output = [0_u16; 6];
        let error = super::unpack_u16_word_rows(
            &[&[1, 0, 2, 0]],
            &mut output,
            3,
            2,
            2,
            ByteOrder::Little,
            u16::MAX,
        )
        .unwrap_err();
        assert!(error.to_string().contains("truncated"), "{error}");
    }

    #[test]
    fn row_word_unpack_honors_both_byte_orders_and_the_sample_mask() {
        let samples = [0x0123_u16, 0x0abc, 0x0def];
        for (byte_order, mask) in [
            (ByteOrder::Little, u16::MAX),
            (ByteOrder::Little, 0x0fff),
            (ByteOrder::Big, u16::MAX),
            (ByteOrder::Big, 0x0fff),
        ] {
            let bytes = samples
                .iter()
                .flat_map(|value| match byte_order {
                    ByteOrder::Little => value.to_le_bytes(),
                    ByteOrder::Big => value.to_be_bytes(),
                })
                .collect::<Vec<_>>();
            let mut decoded = [0_u16; 3];
            super::unpack_u16_word_rows(
                &[&bytes],
                &mut decoded,
                samples.len(),
                1,
                1,
                byte_order,
                mask,
            )
            .unwrap();
            assert_eq!(decoded, samples.map(|value| value & mask));
        }
    }

    #[test]
    fn row_word_unpack_parallel_bands_preserve_row_order() {
        let width = 1025;
        let height = 300;
        let expected = (0..width * height)
            .map(|index| (index as u16).wrapping_mul(29))
            .collect::<Vec<_>>();
        let payload = little_payload(&expected);
        rayon::ThreadPoolBuilder::new()
            .num_threads(4)
            .build()
            .unwrap()
            .install(|| {
                let decoded = unpack_u16_word_rows_to_vec(
                    &[&payload],
                    width,
                    height,
                    height,
                    ByteOrder::Little,
                );
                assert_eq!(decoded, expected);
            });
    }

    fn unpack_u16_word_rows_to_vec(
        payloads: &[&[u8]],
        width: usize,
        height: usize,
        chunk_height: usize,
        byte_order: ByteOrder,
    ) -> Vec<u16> {
        let mut output = vec![0_u16; width * height];
        super::unpack_u16_word_rows(
            payloads,
            &mut output,
            width,
            height,
            chunk_height,
            byte_order,
            u16::MAX,
        )
        .unwrap();
        output
    }

    fn little_payload(samples: &[u16]) -> Vec<u8> {
        samples
            .iter()
            .flat_map(|value| value.to_le_bytes())
            .collect()
    }

    fn pack(samples: &[u16], bits: usize, lsb: bool) -> Vec<u8> {
        let mut bytes = vec![0_u8; (samples.len() * bits).div_ceil(8)];
        for (sample_index, sample) in samples.iter().copied().enumerate() {
            for bit in 0..bits {
                if sample & (1 << bit) == 0 {
                    continue;
                }
                let stream_bit = sample_index * bits + if lsb { bit } else { bits - 1 - bit };
                let byte = stream_bit / 8;
                let within = stream_bit % 8;
                bytes[byte] |= if lsb { 1 << within } else { 1 << (7 - within) };
            }
        }
        bytes
    }
}
