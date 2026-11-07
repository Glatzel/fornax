use fornax_core::{MonoImage, RawMosaic};
use rayon::prelude::*;

use crate::request::{ByteOrder, Chunk, SampleFormat};
use crate::{CodecError, error::invalid};

pub(crate) struct Geometry {
    pub(crate) width: usize,
    pub(crate) height: usize,
    pub(crate) chunk_width: usize,
    pub(crate) chunk_height: usize,
    pub(crate) across: usize,
    pub(crate) down: usize,
}

impl Geometry {
    pub(crate) fn new(
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

    fn band_stride(&self) -> Result<usize, CodecError> {
        checked_product([self.chunk_height, self.width])
            .ok_or_else(|| invalid("decoded band size overflow"))
    }
}

pub(crate) fn chunk(source: &[u8], chunk: Chunk) -> Result<&[u8], CodecError> {
    let end = chunk
        .offset
        .checked_add(chunk.byte_count)
        .ok_or_else(|| invalid("chunk range overflow"))?;
    source
        .get(chunk.offset..end)
        .ok_or_else(|| invalid("chunk offset or byte count is out of bounds"))
}

pub(crate) fn decode_payloads(
    geometry: &Geometry,
    bits_per_sample: u16,
    sample_format: SampleFormat,
    byte_order: ByteOrder,
    payloads: &[&[u8]],
) -> Result<RawMosaic, CodecError> {
    if payloads.len() != geometry.across * geometry.down {
        return Err(invalid("decoded chunk count does not match image geometry"));
    }

    match sample_format {
        SampleFormat::Float => decode_float(geometry, bits_per_sample, byte_order, payloads),
        SampleFormat::Integer => decode_integer(
            geometry,
            bits_per_sample,
            byte_order,
            payloads,
        ),
    }
}

fn decode_integer(
    geometry: &Geometry,
    bits: u16,
    byte_order: ByteOrder,
    payloads: &[&[u8]],
) -> Result<RawMosaic, CodecError> {
    let sample_count = geometry.sample_count()?;
    match bits {
        1..=8 => {
            let mut output = vec![0_u8; sample_count];
            if bits == 8 {
                output = copy_buffer(geometry, 1, payloads)?;
            } else {
                decode_packed(geometry, bits, payloads, |value| {
                    u8::try_from(value).map_err(|_| invalid("packed sample exceeds u8 storage"))
                }, &mut output)?;
            }
            mono(geometry.width, geometry.height, output).map(RawMosaic::U8)
        }
        9..=16 => {
            let mut output = vec![0_u16; sample_count];
            if bits == 16 {
                copy_samples(geometry, 2, byte_order, payloads, &mut output)?;
            } else {
                decode_packed(geometry, bits, payloads, |value| {
                    u16::try_from(value).map_err(|_| invalid("packed sample exceeds u16 storage"))
                }, &mut output)?;
            }
            mono(geometry.width, geometry.height, output).map(RawMosaic::U16)
        }
        17..=31 => {
            let mut output = vec![0_u32; sample_count];
            decode_packed(geometry, bits, payloads, Ok, &mut output)?;
            mono(geometry.width, geometry.height, output).map(RawMosaic::U32)
        }
        32 => {
            let mut output = vec![0_u32; sample_count];
            copy_samples(geometry, 4, byte_order, payloads, &mut output)?;
            mono(geometry.width, geometry.height, output).map(RawMosaic::U32)
        }
        _ => Err(invalid(format!(
            "integer sample depth {bits} is invalid"
        ))),
    }
}

fn decode_float(
    geometry: &Geometry,
    bits: u16,
    byte_order: ByteOrder,
    payloads: &[&[u8]],
) -> Result<RawMosaic, CodecError> {
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
                let required_samples = (copy_height - 1)
                    .checked_mul(geometry.chunk_width)
                    .and_then(|count| count.checked_add(copy_width))
                    .ok_or_else(|| invalid("floating-point chunk size overflow"))?;
                let required_bytes = required_samples
                    .checked_mul(bytes_per_sample)
                    .ok_or_else(|| invalid("floating-point chunk size overflow"))?;
                if encoded.len() < required_bytes {
                    return Err(invalid(format!(
                        "uncompressed chunk {index} is truncated"
                    )));
                }

                for row in 0..copy_height {
                    let source_start = row * geometry.chunk_width * bytes_per_sample;
                    let target_start = row * geometry.width + tile_x;
                    for (sample_index, target) in band[target_start..target_start + copy_width]
                        .iter_mut()
                        .enumerate()
                    {
                        let offset = source_start + sample_index * bytes_per_sample;
                        let bytes = &encoded[offset..offset + bytes_per_sample];
                        *target = match bits {
                            16 => {
                                let raw = read_u16(bytes, byte_order);
                                extend_float(u32::from(raw), 10, 5)
                            }
                            24 => extend_float(read_u24(bytes, byte_order), 16, 7),
                            32 => f32::from_bits(read_u32(bytes, byte_order)),
                            _ => unreachable!("the sample width is validated above"),
                        };
                    }
                }
            }
            Ok::<(), CodecError>(())
        })?;
    mono(geometry.width, geometry.height, output).map(RawMosaic::F32)
}

fn decode_packed<T>(
    geometry: &Geometry,
    bits: u16,
    payloads: &[&[u8]],
    convert: impl Fn(u32) -> Result<T, CodecError> + Sync,
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
            let mut unpacked = vec![0_u32; geometry.chunk_width * geometry.chunk_height];
            for tile_x_index in 0..geometry.across {
                let index = tile_y_index * geometry.across + tile_x_index;
                unpack_bits(payloads[index], &mut unpacked, bits)?;

                let tile_x = tile_x_index * geometry.chunk_width;
                let copy_width = geometry.chunk_width.min(geometry.width - tile_x);
                for row in 0..copy_height {
                    let source_start = row * geometry.chunk_width;
                    let target_start = row * geometry.width + tile_x;
                    for (target, value) in band[target_start..target_start + copy_width]
                        .iter_mut()
                        .zip(&unpacked[source_start..source_start + copy_width])
                    {
                        *target = convert(*value)?;
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
    let mut buffer = 0_u64;
    let mut buffered_bits = 0_usize;
    let mut position = 0_usize;
    for value in output {
        while buffered_bits < bits {
            buffer = (buffer << 8) | u64::from(source[position]);
            position += 1;
            buffered_bits += 8;
        }
        buffered_bits -= bits;
        *value = u32::try_from((buffer >> buffered_bits) & mask)
            .map_err(|_| invalid("packed sample exceeds u32"))?;
        buffer &= (1_u64 << buffered_bits) - 1;
    }
    Ok(())
}

fn copy_buffer(
    geometry: &Geometry,
    bytes_per_sample: usize,
    payloads: &[&[u8]],
) -> Result<Vec<u8>, CodecError> {
    let expected = geometry
        .sample_count()?
        .checked_mul(bytes_per_sample)
        .ok_or_else(|| invalid("sample byte count overflow"))?;
    let mut output = vec![0_u8; expected];

    for (index, encoded) in payloads.iter().enumerate() {
        let tile_x = (index % geometry.across) * geometry.chunk_width;
        let tile_y = (index / geometry.across) * geometry.chunk_height;
        let copy_width = geometry.chunk_width.min(geometry.width - tile_x);
        let copy_height = geometry.chunk_height.min(geometry.height - tile_y);
        let required_samples = (copy_height - 1)
            .checked_mul(geometry.chunk_width)
            .and_then(|count| count.checked_add(copy_width))
            .ok_or_else(|| invalid("uncompressed chunk size overflow"))?;
        let required = required_samples
            .checked_mul(bytes_per_sample)
            .ok_or_else(|| invalid("uncompressed chunk size overflow"))?;
        if encoded.len() < required {
            return Err(invalid(format!("uncompressed chunk {index} is truncated")));
        }
        let copy_bytes = copy_width * bytes_per_sample;
        for row in 0..copy_height {
            let source_start = row * geometry.chunk_width * bytes_per_sample;
            let target_start =
                (tile_y + row) * geometry.width * bytes_per_sample + tile_x * bytes_per_sample;
            let source = &encoded[source_start..source_start + copy_bytes];
            let target = &mut output[target_start..target_start + copy_bytes];
            target.copy_from_slice(source);
        }
    }
    Ok(output)
}

fn copy_samples<T>(
    geometry: &Geometry,
    bytes_per_sample: usize,
    byte_order: ByteOrder,
    payloads: &[&[u8]],
    output: &mut [T],
) -> Result<(), CodecError>
where
    T: FromSampleBytes + Send + Sync,
{
    let bytes = copy_buffer(geometry, bytes_per_sample, payloads)?;
    for (source, target) in bytes.chunks_exact(bytes_per_sample).zip(output.iter_mut()) {
        *target = T::from_bytes(source, byte_order);
    }
    Ok(())
}

trait FromSampleBytes: Copy {
    fn from_bytes(bytes: &[u8], byte_order: ByteOrder) -> Self;
}

impl FromSampleBytes for u16 {
    fn from_bytes(bytes: &[u8], byte_order: ByteOrder) -> Self {
        let bytes: [u8; 2] = bytes.try_into().expect("sample width is fixed");
        match byte_order {
            ByteOrder::Little => Self::from_le_bytes(bytes),
            ByteOrder::Big => Self::from_be_bytes(bytes),
        }
    }
}

impl FromSampleBytes for u32 {
    fn from_bytes(bytes: &[u8], byte_order: ByteOrder) -> Self {
        let bytes: [u8; 4] = bytes.try_into().expect("sample width is fixed");
        match byte_order {
            ByteOrder::Little => Self::from_le_bytes(bytes),
            ByteOrder::Big => Self::from_be_bytes(bytes),
        }
    }
}

fn read_u16(bytes: &[u8], byte_order: ByteOrder) -> u16 {
    let bytes = [bytes[0], bytes[1]];
    match byte_order {
        ByteOrder::Little => u16::from_le_bytes(bytes),
        ByteOrder::Big => u16::from_be_bytes(bytes),
    }
}

fn read_u24(bytes: &[u8], byte_order: ByteOrder) -> u32 {
    match byte_order {
        ByteOrder::Little => u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0]),
        ByteOrder::Big => u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]]),
    }
}

fn read_u32(bytes: &[u8], byte_order: ByteOrder) -> u32 {
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

fn mono<T>(
    width: usize,
    height: usize,
    samples: Vec<T>,
) -> Result<MonoImage<T>, CodecError>
where
    T: image::Primitive,
{
    let width = u32::try_from(width).map_err(|_| invalid("image width exceeds u32"))?;
    let height = u32::try_from(height).map_err(|_| invalid("image height exceeds u32"))?;
    MonoImage::from_raw(width, height, samples)
        .ok_or_else(|| invalid("decoded sample count does not match dimensions"))
}

fn checked_product<const N: usize>(values: [usize; N]) -> Option<usize> {
    values.into_iter().try_fold(1_usize, usize::checked_mul)
}
