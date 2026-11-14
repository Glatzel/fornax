//! TIFF Deflate decoding and encoding.

use std::io::{Read, Write};

use flate2::Compression as FlateCompression;
use flate2::read::ZlibDecoder;
use flate2::write::ZlibEncoder;
use rayon::prelude::*;

use crate::encode::encode_image;
use crate::error::invalid;
use crate::request::{ByteOrder, DecodeRequest, EncodeRequest, EncodedChunks, SampleFormat};
use crate::samples::{
    Geometry, chunk, decode_payloads, extend_float, read_u16, read_u24, read_u32,
};
use crate::{CodecError, Predictor, SampleBuffer, predictor};

struct DeflateScratch {
    compressed: Vec<u8>,
    rows: Vec<u8>,
}

pub fn decode(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    let geometry = Geometry::new(
        request.width,
        request.height,
        request.chunk_width,
        request.chunk_height,
        request.chunks.len(),
    )?;
    if request.sample_format == SampleFormat::Float
        && matches!(request.bits_per_sample, 16 | 24 | 32)
        && matches!(
            request.predictor,
            Predictor::None | Predictor::FloatingPoint { .. }
        )
    {
        return decode_float_fused(&geometry, request);
    }
    let mut decoded = Vec::with_capacity(request.chunks.len());
    for (index, range) in request.chunks.iter().copied().enumerate() {
        let encoded = chunk(request.source, range)?;
        let mut payload = inflate(encoded)
            .map_err(|detail| invalid(format!("Deflate chunk {index}: {detail}")))?;
        predictor::reverse(
            &mut payload,
            request.predictor,
            request.chunk_width,
            request.chunk_height,
            request.bits_per_sample,
            request.sample_format,
            request.byte_order,
        )
        .map_err(|error| invalid(format!("Deflate chunk {index}: {error}")))?;
        decoded.push(payload);
    }
    let payloads = decoded.iter().map(Vec::as_slice).collect::<Vec<_>>();
    decode_payloads(
        &geometry,
        request.bits_per_sample,
        request.sample_format,
        request.byte_order,
        &payloads,
    )
}

fn decode_float_fused(
    geometry: &Geometry,
    request: &DecodeRequest<'_>,
) -> Result<SampleBuffer, CodecError> {
    let sample_count = geometry
        .width
        .checked_mul(geometry.height)
        .ok_or_else(|| invalid("decoded sample count overflow"))?;
    let bytes_per_sample = usize::from(request.bits_per_sample.div_ceil(8));
    if geometry.across == 1
        && geometry.down == 1
        && geometry.width == geometry.chunk_width
        && geometry.height == geometry.chunk_height
    {
        let encoded = chunk(request.source, request.chunks[0])?;
        let mut inflated = Vec::new();
        inflate_into(encoded, &mut inflated)
            .map_err(|detail| invalid(format!("Deflate chunk 0: {detail}")))?;
        let mut output = vec![0.0_f32; sample_count];
        decode_float_rows_into(
            &inflated,
            geometry.width,
            geometry.height,
            bytes_per_sample,
            request.predictor,
            request.byte_order,
            &mut output,
        )?;
        return float_buffer(geometry.width, geometry.height, output);
    }
    let row_bytes = geometry
        .chunk_width
        .checked_mul(bytes_per_sample)
        .ok_or_else(|| invalid("Deflate row size overflow"))?;
    let mut output = vec![0.0_f32; sample_count];
    output
        .par_chunks_mut(geometry.band_stride()?)
        .enumerate()
        .try_for_each(|(tile_y_index, band)| {
            let mut scratch = DeflateScratch {
                compressed: Vec::new(),
                rows: vec![0; row_bytes],
            };
            let tile_y = tile_y_index * geometry.chunk_height;
            let copy_height = geometry.chunk_height.min(geometry.height - tile_y);
            for tile_x_index in 0..geometry.across {
                let index = tile_y_index * geometry.across + tile_x_index;
                let encoded = chunk(request.source, request.chunks[index])?;
                inflate_into(encoded, &mut scratch.compressed)
                    .map_err(|detail| invalid(format!("Deflate chunk {index}: {detail}")))?;
                let tile_x = tile_x_index * geometry.chunk_width;
                let copy_width = geometry.chunk_width.min(geometry.width - tile_x);
                let required_bytes = copy_height
                    .checked_mul(row_bytes)
                    .ok_or_else(|| invalid("Deflate tile size overflow"))?;
                if scratch.compressed.len() < required_bytes {
                    return Err(invalid(format!("Deflate chunk {index} is truncated")));
                }
                for y in 0..copy_height {
                    let source = &scratch.compressed[y * row_bytes..][..row_bytes];
                    let target = &mut band[y * geometry.width + tile_x..][..copy_width];
                    decode_float_row_into(
                        source,
                        geometry.chunk_width,
                        copy_width,
                        request.bits_per_sample,
                        request.predictor,
                        request.byte_order,
                        target,
                        &mut scratch.rows,
                    )?;
                }
            }
            Ok::<(), CodecError>(())
        })?;
    float_buffer(geometry.width, geometry.height, output)
}

fn float_buffer(width: usize, height: usize, output: Vec<f32>) -> Result<SampleBuffer, CodecError> {
    SampleBuffer::from_f32(
        u32::try_from(width).map_err(|_| invalid("image width exceeds u32"))?,
        u32::try_from(height).map_err(|_| invalid("image height exceeds u32"))?,
        output,
    )
    .map_err(Into::into)
}

fn decode_float_rows_into(
    inflated: &[u8],
    width: usize,
    height: usize,
    bytes_per_sample: usize,
    predictor: Predictor,
    byte_order: ByteOrder,
    output: &mut [f32],
) -> Result<(), CodecError> {
    let bits = u16::try_from(bytes_per_sample * 8)
        .map_err(|_| invalid("floating-point sample width exceeds u16"))?;
    let expected = width
        .checked_mul(height)
        .ok_or_else(|| invalid("fused float geometry overflow"))?;
    if !matches!(bits, 16 | 24 | 32) || output.len() != expected {
        return Err(invalid("invalid fused float row geometry"));
    }
    let row_bytes = width
        .checked_mul(bytes_per_sample)
        .ok_or_else(|| invalid("Deflate row size overflow"))?;
    let expected_bytes = row_bytes
        .checked_mul(height)
        .ok_or_else(|| invalid("fused float payload size overflow"))?;
    if inflated.len() < expected_bytes {
        return Err(invalid("inflated float payload is truncated"));
    }
    let mut scratch = vec![0_u8; row_bytes];
    for y in 0..height {
        let source = &inflated[y * row_bytes..][..row_bytes];
        let target = &mut output[y * width..][..width];
        decode_float_row_into(
            source,
            width,
            width,
            bits,
            predictor,
            byte_order,
            target,
            &mut scratch,
        )?;
    }
    Ok(())
}

fn decode_float_row_into(
    source: &[u8],
    source_width: usize,
    output_width: usize,
    bits: u16,
    predictor: Predictor,
    byte_order: ByteOrder,
    output: &mut [f32],
    scratch: &mut [u8],
) -> Result<(), CodecError> {
    let bytes_per_sample = usize::from(bits.div_ceil(8));
    let row_bytes = source_width
        .checked_mul(bytes_per_sample)
        .ok_or_else(|| invalid("Deflate row size overflow"))?;
    let planes = match predictor {
        Predictor::None => source,
        Predictor::FloatingPoint { byte_stride } => {
            predictor::reverse_floating_row(source, scratch, source_width, bits, byte_stride)?;
            &scratch[..row_bytes]
        }
        Predictor::HorizontalInteger => {
            return Err(invalid(
                "horizontal integer prediction cannot produce float samples",
            ));
        }
    };
    let planar = predictor != Predictor::None;
    if source.len() < row_bytes || output.len() < output_width {
        return Err(invalid("fused float row is truncated"));
    }
    for (x, value) in output.iter_mut().take(output_width).enumerate() {
        let raw = match (bits, planar) {
            (16, true) => (u32::from(planes[x]) << 8) | u32::from(planes[source_width + x]),
            (24, true) => {
                (u32::from(planes[x]) << 16)
                    | (u32::from(planes[source_width + x]) << 8)
                    | u32::from(planes[2 * source_width + x])
            }
            (32, true) => {
                (u32::from(planes[x]) << 24)
                    | (u32::from(planes[source_width + x]) << 16)
                    | (u32::from(planes[2 * source_width + x]) << 8)
                    | u32::from(planes[3 * source_width + x])
            }
            (16, false) => u32::from(read_u16(&planes[x * bytes_per_sample..], byte_order)),
            (24, false) => read_u24(&planes[x * bytes_per_sample..], byte_order),
            (32, false) => read_u32(&planes[x * bytes_per_sample..], byte_order),
            _ => return Err(invalid("unsupported fused float width")),
        };
        *value = match bits {
            16 => extend_float(raw, 10, 5),
            24 => extend_float(raw, 16, 7),
            32 => f32::from_bits(raw),
            _ => unreachable!("the width was validated above"),
        };
    }
    Ok(())
}

pub fn encode(request: &EncodeRequest<'_>) -> Result<EncodedChunks, CodecError> {
    let mut encoded = encode_image(
        request.image,
        request.chunk_width,
        request.chunk_height,
        request.byte_order,
    )?;
    let mut chunks = Vec::with_capacity(encoded.chunks.len());
    for chunk in &mut encoded.chunks {
        predictor::apply(
            chunk,
            request.predictor,
            request.chunk_width,
            request.chunk_height,
            encoded.bits_per_sample,
            encoded.sample_format,
            request.byte_order,
        )?;
        chunks.push(deflate(chunk)?);
    }
    Ok(EncodedChunks { chunks })
}

pub fn inflate(encoded: &[u8]) -> Result<Vec<u8>, String> {
    let mut decoded = Vec::new();
    inflate_into(encoded, &mut decoded)?;
    Ok(decoded)
}

fn inflate_into(encoded: &[u8], decoded: &mut Vec<u8>) -> Result<(), String> {
    decoded.clear();
    let mut decoder = ZlibDecoder::new(encoded);
    decoder
        .read_to_end(decoded)
        .map_err(|error| format!("payload is invalid: {error}"))?;
    Ok(())
}

pub fn deflate(data: &[u8]) -> Result<Vec<u8>, CodecError> {
    let mut encoder = ZlibEncoder::new(Vec::new(), FlateCompression::fast());
    encoder
        .write_all(data)
        .map_err(|error| invalid(format!("Deflate setup failed: {error}")))?;
    encoder
        .finish()
        .map_err(|error| invalid(format!("Deflate encoding failed: {error}")))
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::request::SampleFormat;
    use crate::samples::{Geometry, decode_payloads};
    use crate::{ByteOrder, Predictor};

    #[test]
    fn deflate_fused_float_rows_match_predictor_reverse_and_typed_conversion() {
        let width = 5;
        let height = 3;
        for bits in [16_u16, 24, 32] {
            for byte_order in [ByteOrder::Little, ByteOrder::Big] {
                let source = float_storage(width * height, bits, byte_order);
                for predictor in [
                    Predictor::None,
                    Predictor::FloatingPoint { byte_stride: 1 },
                    Predictor::FloatingPoint { byte_stride: 2 },
                    Predictor::FloatingPoint { byte_stride: 4 },
                ] {
                    let mut inflated = source.clone();
                    predictor::apply(
                        &mut inflated,
                        predictor,
                        width,
                        height,
                        bits,
                        SampleFormat::Float,
                        byte_order,
                    )
                    .unwrap();

                    let mut oracle_payload = inflated.clone();
                    predictor::reverse(
                        &mut oracle_payload,
                        predictor,
                        width,
                        height,
                        bits,
                        SampleFormat::Float,
                        byte_order,
                    )
                    .unwrap();
                    let geometry = Geometry::new(width, height, width, height, 1).unwrap();
                    let SampleBuffer::F32(oracle) = decode_payloads(
                        &geometry,
                        bits,
                        SampleFormat::Float,
                        byte_order,
                        &[&oracle_payload],
                    )
                    .unwrap() else {
                        panic!("float storage must decode as f32");
                    };

                    let mut fused = vec![0.0_f32; width * height];
                    decode_float_rows_into(
                        &inflated,
                        width,
                        height,
                        usize::from(bits.div_ceil(8)),
                        predictor,
                        byte_order,
                        &mut fused,
                    )
                    .unwrap();

                    assert_eq!(
                        fused
                            .iter()
                            .map(|value| value.to_bits())
                            .collect::<Vec<_>>(),
                        oracle
                            .as_raw()
                            .iter()
                            .map(|value| value.to_bits())
                            .collect::<Vec<_>>(),
                        "{bits}-bit {byte_order:?} {predictor:?}"
                    );
                }
            }
        }
    }

    fn float_storage(sample_count: usize, bits: u16, byte_order: ByteOrder) -> Vec<u8> {
        let raw = [
            0x0000_u32, 0x8000, 0x3c00, 0x7c00, 0xfc00, 0x0001, 0x03ff, 0x1234, 0xabcd, 0x7fff,
            0x00ff, 0x5555, 0x2a2a, 0xbeef, 0x4000,
        ];
        let bytes_per_sample = usize::from(bits.div_ceil(8));
        let mut bytes = Vec::with_capacity(sample_count * bytes_per_sample);
        for index in 0..sample_count {
            let value = raw[index % raw.len()]
                & if bits == 32 {
                    u32::MAX
                } else {
                    (1_u32 << bits) - 1
                };
            match (bytes_per_sample, byte_order) {
                (2, ByteOrder::Little) => {
                    bytes.extend(u16::try_from(value).unwrap().to_le_bytes());
                }
                (2, ByteOrder::Big) => {
                    bytes.extend(u16::try_from(value).unwrap().to_be_bytes());
                }
                (3, ByteOrder::Little) => {
                    bytes.extend_from_slice(&value.to_le_bytes()[..3]);
                }
                (3, ByteOrder::Big) => {
                    bytes.extend_from_slice(&value.to_be_bytes()[1..]);
                }
                (4, ByteOrder::Little) => bytes.extend(value.to_le_bytes()),
                (4, ByteOrder::Big) => bytes.extend(value.to_be_bytes()),
                _ => unreachable!("float width is 16, 24, or 32 bits"),
            }
        }
        bytes
    }
}
