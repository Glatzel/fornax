//! Reversible sample predictors used by compressed chunks.

use crate::CodecError;
use crate::error::invalid;
use crate::request::{ByteOrder, SampleFormat};

/// Reversible per-chunk predictor applied before sample reconstruction.
///
/// # Examples
///
/// ```
/// assert_eq!(fornax_codec::Predictor::None, fornax_codec::Predictor::None);
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Predictor {
    /// Samples are not predicted.
    None,
    /// Horizontal integer differencing using the previous sample.
    HorizontalInteger,
    /// TIFF floating-point byte shuffling and differencing.
    FloatingPoint {
        /// Distance between differenced bytes.
        byte_stride: usize,
    },
}

pub(crate) fn reverse(
    payload: &mut [u8],
    predictor: Predictor,
    width: usize,
    height: usize,
    bits: u16,
    sample_format: SampleFormat,
    byte_order: ByteOrder,
) -> Result<(), CodecError> {
    if predictor == Predictor::None {
        return Ok(());
    }
    let bytes_per_sample = usize::from(bits.div_ceil(8));
    let row_bytes = width
        .checked_mul(bytes_per_sample)
        .ok_or_else(|| invalid("predictor row size overflow"))?;
    let needed = row_bytes
        .checked_mul(height)
        .ok_or_else(|| invalid("predictor payload size overflow"))?;
    if payload.len() < needed {
        return Err(invalid("predicted payload is truncated"));
    }

    match predictor {
        Predictor::None => {}
        Predictor::HorizontalInteger => {
            if sample_format != SampleFormat::Integer || !matches!(bits, 8 | 16 | 32) {
                return Err(invalid(
                    "horizontal prediction requires 8/16/32-bit integers",
                ));
            }
            for row in payload[..needed].chunks_exact_mut(row_bytes) {
                for sample in 1..width {
                    let previous = (sample - 1) * bytes_per_sample;
                    let current = sample * bytes_per_sample;
                    match bytes_per_sample {
                        1 => row[current] = row[current].wrapping_add(row[previous]),
                        2 => write_u16(
                            row,
                            current,
                            read_u16(row, previous, byte_order)
                                .wrapping_add(read_u16(row, current, byte_order)),
                            byte_order,
                        ),
                        4 => write_u32(
                            row,
                            current,
                            read_u32(row, previous, byte_order)
                                .wrapping_add(read_u32(row, current, byte_order)),
                            byte_order,
                        ),
                        _ => unreachable!("the sample width is validated above"),
                    }
                }
            }
        }
        Predictor::FloatingPoint { byte_stride } => {
            if sample_format != SampleFormat::Float
                || !matches!(bits, 16 | 24 | 32)
                || !matches!(byte_stride, 1 | 2 | 4)
            {
                return Err(invalid(
                    "floating-point prediction requires 16/24/32-bit floats and stride 1, 2, or 4",
                ));
            }
            for row in payload[..needed].chunks_exact_mut(row_bytes) {
                for index in byte_stride..row_bytes {
                    row[index] = row[index].wrapping_add(row[index - byte_stride]);
                }
                let shuffled = row.to_vec();
                for sample in 0..width {
                    for byte in 0..bytes_per_sample {
                        row[sample * bytes_per_sample + byte] = shuffled[sample + byte * width];
                    }
                    if byte_order == ByteOrder::Little {
                        row[sample * bytes_per_sample..(sample + 1) * bytes_per_sample].reverse();
                    }
                }
            }
        }
    }
    Ok(())
}

pub(crate) fn apply(
    payload: &mut [u8],
    predictor: Predictor,
    width: usize,
    height: usize,
    bits: u16,
    sample_format: SampleFormat,
    byte_order: ByteOrder,
) -> Result<(), CodecError> {
    if predictor == Predictor::None {
        return Ok(());
    }
    let bytes_per_sample = usize::from(bits.div_ceil(8));
    let row_bytes = width
        .checked_mul(bytes_per_sample)
        .ok_or_else(|| invalid("predictor row size overflow"))?;
    let needed = row_bytes
        .checked_mul(height)
        .ok_or_else(|| invalid("predictor payload size overflow"))?;
    if payload.len() != needed {
        return Err(invalid("predictor payload size does not match the tile"));
    }

    match predictor {
        Predictor::None => {}
        Predictor::HorizontalInteger => {
            if sample_format != SampleFormat::Integer || !matches!(bits, 8 | 16 | 32) {
                return Err(invalid(
                    "horizontal prediction requires 8/16/32-bit integers",
                ));
            }
            for row in payload.chunks_exact_mut(row_bytes) {
                for sample in (1..width).rev() {
                    let previous = (sample - 1) * bytes_per_sample;
                    let current = sample * bytes_per_sample;
                    match bytes_per_sample {
                        1 => row[current] = row[current].wrapping_sub(row[previous]),
                        2 => write_u16(
                            row,
                            current,
                            read_u16(row, current, byte_order)
                                .wrapping_sub(read_u16(row, previous, byte_order)),
                            byte_order,
                        ),
                        4 => write_u32(
                            row,
                            current,
                            read_u32(row, current, byte_order)
                                .wrapping_sub(read_u32(row, previous, byte_order)),
                            byte_order,
                        ),
                        _ => unreachable!("the sample width is validated above"),
                    }
                }
            }
        }
        Predictor::FloatingPoint { byte_stride } => {
            if sample_format != SampleFormat::Float
                || !matches!(bits, 16 | 24 | 32)
                || !matches!(byte_stride, 1 | 2 | 4)
            {
                return Err(invalid(
                    "floating-point prediction requires 16/24/32-bit floats and stride 1, 2, or 4",
                ));
            }
            for row in payload.chunks_exact_mut(row_bytes) {
                if byte_order == ByteOrder::Little {
                    for sample in row.chunks_exact_mut(bytes_per_sample) {
                        sample.reverse();
                    }
                }
                let interleaved = row.to_vec();
                for sample in 0..width {
                    for byte in 0..bytes_per_sample {
                        row[sample + byte * width] = interleaved[sample * bytes_per_sample + byte];
                    }
                }
                for index in (byte_stride..row_bytes).rev() {
                    row[index] = row[index].wrapping_sub(row[index - byte_stride]);
                }
            }
        }
    }
    Ok(())
}

const fn read_u16(bytes: &[u8], offset: usize, byte_order: ByteOrder) -> u16 {
    let value = [bytes[offset], bytes[offset + 1]];
    match byte_order {
        ByteOrder::Little => u16::from_le_bytes(value),
        ByteOrder::Big => u16::from_be_bytes(value),
    }
}

fn write_u16(bytes: &mut [u8], offset: usize, value: u16, byte_order: ByteOrder) {
    let value = match byte_order {
        ByteOrder::Little => value.to_le_bytes(),
        ByteOrder::Big => value.to_be_bytes(),
    };
    bytes[offset..offset + 2].copy_from_slice(&value);
}

const fn read_u32(bytes: &[u8], offset: usize, byte_order: ByteOrder) -> u32 {
    let value = [
        bytes[offset],
        bytes[offset + 1],
        bytes[offset + 2],
        bytes[offset + 3],
    ];
    match byte_order {
        ByteOrder::Little => u32::from_le_bytes(value),
        ByteOrder::Big => u32::from_be_bytes(value),
    }
}

fn write_u32(bytes: &mut [u8], offset: usize, value: u32, byte_order: ByteOrder) {
    let value = match byte_order {
        ByteOrder::Little => value.to_le_bytes(),
        ByteOrder::Big => value.to_be_bytes(),
    };
    bytes[offset..offset + 4].copy_from_slice(&value);
}
