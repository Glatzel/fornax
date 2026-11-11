//! Phase One IIQ sample decoders.

use crate::error::invalid;
use crate::samples::{buffer_u16, chunk};
use crate::{CodecError, DecodeRequest, SampleBuffer, SampleFormat};

const LENGTHS: [u8; 10] = [8, 7, 6, 9, 11, 10, 5, 12, 14, 13];
const SV2_USED_CORR: [u8; 8] = [3, 3, 3, 3, 1, 1, 1, 1];
const SV2_EXTRA_BITS: [u8; 8] = [1, 2, 3, 4, 0, 0, 0, 0];
const SV2_BIT_INDICATOR: [u8; 32] = [
    9, 8, 0, 7, 6, 6, 5, 5, 1, 1, 1, 1, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 2, 2,
];
const SV2_SKIP_BITS: [u8; 32] = [
    5, 5, 5, 5, 4, 4, 4, 4, 3, 3, 3, 3, 3, 3, 3, 3, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2, 2,
];

pub fn decode_uncompressed(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    let shift = validate_request(request, false)?;
    if request.chunks.len() != 1 {
        return Err(invalid(
            "IIQ uncompressed data requires one whole-image chunk",
        ));
    }
    if request.chunk_height < request.height {
        return Err(invalid(
            "IIQ uncompressed chunk must span the whole image height",
        ));
    }
    let samples = checked_samples(request)?;
    let bytes = samples
        .checked_mul(2)
        .ok_or_else(|| invalid("IIQ uncompressed byte count overflow"))?;
    let payload = chunk(request.source, request.chunks[0])?;
    if payload.len() < bytes {
        return Err(invalid(format!(
            "IIQ uncompressed payload is truncated: {} bytes, expected {bytes}",
            payload.len()
        )));
    }
    let output = payload[..bytes]
        .as_chunks::<2>()
        .0
        .iter()
        .map(|bytes| u16::from_le_bytes(*bytes) << shift)
        .collect();
    buffer_u16(request.width, request.height, output)
}

pub fn decode_lossless(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    decode_lossless_rows(request, true)
}

pub fn decode_nonlinear(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    decode_lossless_rows(request, false)
}

fn decode_lossless_rows(
    request: &DecodeRequest<'_>,
    shift_values: bool,
) -> Result<SampleBuffer, CodecError> {
    let shift = validate_request(request, true)?;
    validate_row_chunks(request)?;
    let samples = checked_samples(request)?;
    let mut output = vec![0_u16; samples];
    let maximum = (1_u32 << request.bits_per_sample) - 1;
    for (row, encoded) in request.chunks.iter().copied().enumerate() {
        let payload = chunk(request.source, encoded)?;
        let mut bits = BitReader::new(payload);
        let mut predictors = [0_i64; 2];
        let mut lengths = [0_u8; 2];
        let target = &mut output[row * request.width..][..request.width];
        for (column, sample) in target.iter_mut().enumerate() {
            if column >= (request.width & !7) {
                lengths = [14, 14];
            } else if column.is_multiple_of(8) {
                for length in &mut lengths {
                    let mut zeros = 0_u8;
                    while zeros < 5 && bits.read_bits(1)? == 0 {
                        zeros += 1;
                    }
                    if zeros > 0 {
                        let selector = (zeros - 1) * 2 + bits.read_bits(1)? as u8;
                        *length = LENGTHS[usize::from(selector)];
                    }
                }
            }
            let phase = column & 1;
            let length = lengths[phase];
            if length == 0 {
                return Err(invalid("IIQ lossless code length must not be zero"));
            }
            let predictor = if length == 14 {
                i64::from(bits.read_bits(16)?)
            } else {
                let value = i64::from(bits.read_bits(length)?);
                predictors[phase] + value + 1 - (1_i64 << (length - 1))
            };
            if !(0..=i64::from(maximum)).contains(&predictor) {
                return Err(invalid(format!(
                    "IIQ lossless row {row} column {column} predictor {predictor} exceeds {maximum}"
                )));
            }
            predictors[phase] = predictor;
            let value = u32::try_from(predictor)
                .map_err(|_| invalid("IIQ lossless predictor is negative"))?
                .checked_shl(shift.into())
                .filter(|value| *value <= u32::from(u16::MAX))
                .ok_or_else(|| invalid("IIQ lossless sample normalization overflow"))?;
            *sample = if shift_values {
                value as u16
            } else if predictor < 256 {
                nonlinear_curve(predictor as usize) << shift
            } else {
                predictor as u16
            };
        }
    }
    buffer_u16(request.width, request.height, output)
}

pub fn decode_sv2(request: &DecodeRequest<'_>) -> Result<SampleBuffer, CodecError> {
    let shift = validate_request(request, true)?;
    validate_row_chunks(request)?;
    let samples = checked_samples(request)?;
    let mut output = vec![0_u16; samples];
    for (row, encoded) in request.chunks.iter().copied().enumerate() {
        let payload = chunk(request.source, encoded)?;
        let mut bits = BitReader::new(payload);
        let base_bits = bits.read_bits(16)? as u8 & 7;
        let mut checks = [0_u32; 2];
        let mut predictors = [0_u32; 2];
        let subtraction = 17 - u32::from(base_bits);
        let target = &mut output[row * request.width..][..request.width];
        for cluster in target.chunks_mut(8) {
            if cluster.len() < 8 {
                for sample in cluster {
                    *sample = (bits.read_bits(14)? as u16) << shift;
                }
                continue;
            }
            for check in &mut checks {
                let index = bits.peek_bits(7)? as usize;
                bits.consume_bits(2)?;
                if index >= 32 {
                    *check = (index as u32 >> 5)
                        .checked_add(*check)
                        .and_then(|value| value.checked_sub(2))
                        .ok_or_else(|| invalid("IIQ-Sv2 bit indicator underflow"))?;
                } else {
                    *check = u32::from(SV2_BIT_INDICATOR[index]);
                    bits.consume_bits(u32::from(SV2_SKIP_BITS[index]))?;
                }
            }
            let savepoint = bits.clone();
            let selector = bits.peek_bits(3)? as usize;
            bits.consume_bits(u32::from(SV2_USED_CORR[selector]))?;
            let extra = u32::from(SV2_EXTRA_BITS[selector]);
            let bit_count = u32::from(base_bits) + extra;
            for (index, sample) in cluster.iter_mut().enumerate() {
                let phase = index & 1;
                let check = checks[phase];
                if check > subtraction {
                    return Err(invalid("IIQ-Sv2 bit indicator exceeds the base shift"));
                }
                let shift_amount = check.saturating_sub(extra);
                let subtract = 0xffff_u32 >> (subtraction - check);
                let value = if check == 9 {
                    bits.read_bits(14)?
                } else {
                    let encoded = bits.read_bits(
                        u8::try_from(bit_count)
                            .map_err(|_| invalid("IIQ-Sv2 sample bit count exceeds u8"))?,
                    )?;
                    predictors[phase]
                        .checked_add(encoded << shift_amount)
                        .and_then(|value| value.checked_sub(subtract))
                        .ok_or_else(|| invalid("IIQ-Sv2 predictor underflow"))?
                };
                predictors[phase] = value;
                *sample = (value as u16) << shift;
            }
            if predictors.iter().any(|value| value >> 14 != 0) {
                bits = savepoint;
                for (index, sample) in cluster.iter_mut().enumerate() {
                    let phase = index & 1;
                    let check = checks[phase];
                    if check > subtraction {
                        return Err(invalid("IIQ-Sv2 bit indicator exceeds the base shift"));
                    }
                    let shift_amount = check.saturating_sub(extra);
                    let subtract = 0xffff_u32 >> (subtraction - check);
                    let value = if check == 9 {
                        bits.read_bits(14)?
                    } else {
                        let encoded = bits.read_bits(bit_count as u8)?;
                        predictors[phase]
                            .saturating_add(encoded << shift_amount)
                            .saturating_sub(subtract)
                            .min(0x3fff)
                    };
                    predictors[phase] = value;
                    *sample = (value as u16) << shift;
                }
            }
        }
    }
    buffer_u16(request.width, request.height, output)
}

fn validate_request(request: &DecodeRequest<'_>, compressed: bool) -> Result<u8, CodecError> {
    if request.sample_format != SampleFormat::Integer {
        return Err(invalid("IIQ decoding requires integer samples"));
    }
    if request.width == 0 || request.height == 0 {
        return Err(invalid("IIQ dimensions must not be zero"));
    }
    if request.chunk_width != request.width {
        return Err(invalid("IIQ chunks must span the full image width"));
    }
    if request.byte_order != crate::ByteOrder::Little {
        return Err(invalid("IIQ samples are little-endian"));
    }
    if compressed && request.chunk_height != 1 {
        return Err(invalid("compressed IIQ chunks must contain one row"));
    }
    match request.bits_per_sample {
        14 => Ok(2),
        16 => Ok(0),
        bits => Err(invalid(format!("IIQ sample depth {bits} is unsupported"))),
    }
}

fn validate_row_chunks(request: &DecodeRequest<'_>) -> Result<(), CodecError> {
    if request.chunks.len() != request.height {
        return Err(invalid(format!(
            "IIQ compressed data requires {} row chunks, found {}",
            request.height,
            request.chunks.len()
        )));
    }
    if request
        .chunks
        .iter()
        .any(|chunk| !chunk.byte_count.is_multiple_of(4))
    {
        return Err(invalid(
            "IIQ compressed row chunks must contain complete 32-bit words",
        ));
    }
    Ok(())
}

fn checked_samples(request: &DecodeRequest<'_>) -> Result<usize, CodecError> {
    request
        .width
        .checked_mul(request.height)
        .ok_or_else(|| invalid("IIQ sample count overflow"))
}

fn nonlinear_curve(index: usize) -> u16 { ((index as f32 * index as f32) / 3.969 + 0.5) as u16 }

#[derive(Clone)]
struct BitReader<'a> {
    source: &'a [u8],
    next_word: usize,
    buffer: u64,
    buffered: u8,
}

impl<'a> BitReader<'a> {
    const fn new(source: &'a [u8]) -> Self {
        Self {
            source,
            next_word: 0,
            buffer: 0,
            buffered: 0,
        }
    }

    fn read_bits(&mut self, count: u8) -> Result<u32, CodecError> {
        let value = self.peek_bits(count)?;
        self.consume_bits(u32::from(count))?;
        Ok(value)
    }

    fn peek_bits(&mut self, count: u8) -> Result<u32, CodecError> {
        if count > 32 {
            return Err(invalid("IIQ bit read exceeds 32 bits"));
        }
        if count == 0 {
            return Ok(0);
        }
        self.fill(count);
        if self.buffered < count {
            return Err(invalid("IIQ bitstream is exhausted"));
        }
        Ok(((self.buffer << (64 - self.buffered)) >> (64 - count)) as u32)
    }

    fn consume_bits(&mut self, count: u32) -> Result<(), CodecError> {
        let count = u8::try_from(count).map_err(|_| invalid("IIQ bit count exceeds u8"))?;
        if count > self.buffered {
            return Err(invalid("IIQ bitstream is exhausted"));
        }
        self.buffered -= count;
        Ok(())
    }

    fn fill(&mut self, required: u8) {
        while self.buffered < required && self.next_word < self.source.len() {
            let end = (self.next_word + 4).min(self.source.len());
            let mut bytes = [0_u8; 4];
            bytes[..end - self.next_word].copy_from_slice(&self.source[self.next_word..end]);
            let word = u32::from_le_bytes(bytes);
            self.buffer = (self.buffer << 32) | u64::from(word);
            self.next_word = end;
            self.buffered = self.buffered.saturating_add(32);
        }
    }
}
