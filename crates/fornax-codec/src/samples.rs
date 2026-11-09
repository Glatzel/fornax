use rayon::prelude::*;

use crate::CodecError;
use crate::error::invalid;
use crate::request::{ByteOrder, Chunk, SampleFormat};
use crate::sample_buffer::SampleBuffer;

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
            let mut output = vec![0_u8; sample_count];
            if bits == 8 {
                copy_samples(geometry, byte_order, payloads, &mut output)?;
            } else {
                // `bits <= 8`, so the unpacked value always fits.
                decode_packed(geometry, bits, payloads, low_u8, &mut output)?;
            }
            buffer_u8(geometry.width, geometry.height, output)
        }
        9..=16 => {
            let mut output = vec![0_u16; sample_count];
            if bits == 16 {
                copy_samples(geometry, byte_order, payloads, &mut output)?;
            } else {
                // `bits <= 16`, so the unpacked value always fits.
                decode_packed(geometry, bits, payloads, low_u16, &mut output)?;
            }
            buffer_u16(geometry.width, geometry.height, output)
        }
        17..=31 => {
            let mut output = vec![0_u32; sample_count];
            decode_packed(geometry, bits, payloads, |value| value, &mut output)?;
            buffer_u32(geometry.width, geometry.height, output)
        }
        32 => {
            let mut output = vec![0_u32; sample_count];
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
        _ => {
            return Err(invalid(format!(
                "floating-point sample depth {bits} is invalid"
            )));
        }
    };
    let sample_count = geometry.sample_count()?;
    let mut output = vec![0_f32; sample_count];

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

fn decode_float_with<F>(
    geometry: &Geometry,
    bytes_per_sample: usize,
    payloads: &[&[u8]],
    output: &mut [f32],
    read: F,
) -> Result<(), CodecError>
where
    F: Fn(&[u8]) -> f32 + Sync,
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

impl_sample!(u16, 2);
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

const fn low_u8(value: u32) -> u8 { value.to_le_bytes()[0] }

const fn low_u16(value: u32) -> u16 {
    let bytes = value.to_le_bytes();
    u16::from_le_bytes([bytes[0], bytes[1]])
}

const fn low_u32(value: u64) -> u32 {
    let bytes = value.to_le_bytes();
    u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
}

const fn read_u16(bytes: &[u8], byte_order: ByteOrder) -> u16 {
    let bytes = [bytes[0], bytes[1]];
    match byte_order {
        ByteOrder::Little => u16::from_le_bytes(bytes),
        ByteOrder::Big => u16::from_be_bytes(bytes),
    }
}

const fn read_u24(bytes: &[u8], byte_order: ByteOrder) -> u32 {
    match byte_order {
        ByteOrder::Little => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]),
        ByteOrder::Big => u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]),
    }
}

const fn read_u32(bytes: &[u8], byte_order: ByteOrder) -> u32 {
    let bytes = [bytes[0], bytes[1], bytes[2], bytes[3]];
    match byte_order {
        ByteOrder::Little => u32::from_le_bytes(bytes),
        ByteOrder::Big => u32::from_be_bytes(bytes),
    }
}

fn extend_float(raw: u32, fraction_bits: u32, exponent_bits: u32) -> f32 {
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

fn checked_product<const N: usize>(values: [usize; N]) -> Option<usize> {
    values.into_iter().try_fold(1_usize, usize::checked_mul)
}
