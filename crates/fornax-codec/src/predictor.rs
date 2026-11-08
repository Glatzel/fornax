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

/// Row geometry shared by `reverse` and `apply`.
struct Layout {
    bytes_per_sample: usize,
    row_bytes: usize,
    needed: usize,
}

fn layout(width: usize, height: usize, bits: u16) -> Result<Layout, CodecError> {
    let bytes_per_sample = usize::from(bits.div_ceil(8));
    let row_bytes = width
        .checked_mul(bytes_per_sample)
        .ok_or_else(|| invalid("predictor row size overflow"))?;
    let needed = row_bytes
        .checked_mul(height)
        .ok_or_else(|| invalid("predictor payload size overflow"))?;
    Ok(Layout {
        bytes_per_sample,
        row_bytes,
        needed,
    })
}

fn check_horizontal(sample_format: SampleFormat, bits: u16) -> Result<(), CodecError> {
    if sample_format != SampleFormat::Integer || !matches!(bits, 8 | 16 | 32) {
        return Err(invalid(
            "horizontal prediction requires 8/16/32-bit integers",
        ));
    }
    Ok(())
}

fn check_floating_point(
    sample_format: SampleFormat,
    bits: u16,
    byte_stride: usize,
) -> Result<(), CodecError> {
    if sample_format != SampleFormat::Float
        || !matches!(bits, 16 | 24 | 32)
        || !matches!(byte_stride, 1 | 2 | 4)
    {
        return Err(invalid(
            "floating-point prediction requires 16/24/32-bit floats and stride 1, 2, or 4",
        ));
    }
    Ok(())
}

pub fn reverse(
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
    let Layout {
        bytes_per_sample,
        row_bytes,
        needed,
    } = layout(width, height, bits)?;
    if payload.len() < needed {
        return Err(invalid("predicted payload is truncated"));
    }

    match predictor {
        Predictor::None => {}
        Predictor::HorizontalInteger => {
            check_horizontal(sample_format, bits)?;
            if needed == 0 {
                return Ok(());
            }
            integer_rows::<true>(
                &mut payload[..needed],
                row_bytes,
                bytes_per_sample,
                byte_order,
            );
        }
        Predictor::FloatingPoint { byte_stride } => {
            check_floating_point(sample_format, bits, byte_stride)?;
            if needed == 0 {
                return Ok(());
            }
            let little = byte_order == ByteOrder::Little;
            // One scratch buffer for the whole payload instead of one per row.
            let mut scratch = vec![0u8; row_bytes];
            for row in payload[..needed].chunks_exact_mut(row_bytes) {
                // Undo the byte-wise delta (inherently serial).
                for index in byte_stride..row_bytes {
                    row[index] = row[index].wrapping_add(row[index - byte_stride]);
                }
                // Un-shuffle byte planes into interleaved samples. The outer
                // loop walks planes so reads are sequential; the endian swap is
                // folded into `dst`.
                scratch.copy_from_slice(row);
                for byte in 0..bytes_per_sample {
                    let dst = if little {
                        bytes_per_sample - 1 - byte
                    } else {
                        byte
                    };
                    let plane = &scratch[byte * width..][..width];
                    for (sample, &value) in row.chunks_exact_mut(bytes_per_sample).zip(plane) {
                        sample[dst] = value;
                    }
                }
            }
        }
    }
    Ok(())
}

pub fn apply(
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
    let Layout {
        bytes_per_sample,
        row_bytes,
        needed,
    } = layout(width, height, bits)?;
    if payload.len() != needed {
        return Err(invalid("predictor payload size does not match the tile"));
    }

    match predictor {
        Predictor::None => {}
        Predictor::HorizontalInteger => {
            check_horizontal(sample_format, bits)?;
            if needed == 0 {
                return Ok(());
            }
            integer_rows::<false>(payload, row_bytes, bytes_per_sample, byte_order);
        }
        Predictor::FloatingPoint { byte_stride } => {
            check_floating_point(sample_format, bits, byte_stride)?;
            if needed == 0 {
                return Ok(());
            }
            let little = byte_order == ByteOrder::Little;
            let head = byte_stride.min(row_bytes);
            let mut scratch = vec![0u8; row_bytes];
            for row in payload.chunks_exact_mut(row_bytes) {
                // Shuffle interleaved samples into byte planes (endian swap
                // folded in).
                for byte in 0..bytes_per_sample {
                    let src = if little {
                        bytes_per_sample - 1 - byte
                    } else {
                        byte
                    };
                    let plane = &mut scratch[byte * width..][..width];
                    for (out, sample) in plane.iter_mut().zip(row.chunks_exact(bytes_per_sample)) {
                        *out = sample[src];
                    }
                }
                // Byte-wise delta as a pure element-wise subtract from
                // read-only scratch, so there is no loop-carried dependency.
                row[..head].copy_from_slice(&scratch[..head]);
                if row_bytes > byte_stride {
                    for (out, (&current, &previous)) in row[byte_stride..]
                        .iter_mut()
                        .zip(scratch[byte_stride..].iter().zip(scratch.iter()))
                    {
                        *out = current.wrapping_sub(previous);
                    }
                }
            }
        }
    }
    Ok(())
}

/// Dispatches once on sample width and byte order, then runs a tight per-row
/// loop.
///
/// `ADD = true` reverses horizontal differencing, `ADD = false` applies it.
fn integer_rows<const ADD: bool>(
    payload: &mut [u8],
    row_bytes: usize,
    bytes_per_sample: usize,
    byte_order: ByteOrder,
) {
    macro_rules! run {
        ($zero:expr, $from:expr, $to:expr, $add:expr, $sub:expr) => {
            for row in payload.chunks_exact_mut(row_bytes) {
                if ADD {
                    undiff_row(row, $zero, $from, $to, $add);
                } else {
                    diff_row(row, $zero, $from, $to, $sub);
                }
            }
        };
    }

    match (bytes_per_sample, byte_order) {
        (1, _) => run!(
            0u8,
            u8::from_le_bytes,
            u8::to_le_bytes,
            u8::wrapping_add,
            u8::wrapping_sub
        ),
        (2, ByteOrder::Little) => run!(
            0u16,
            u16::from_le_bytes,
            u16::to_le_bytes,
            u16::wrapping_add,
            u16::wrapping_sub
        ),
        (2, ByteOrder::Big) => run!(
            0u16,
            u16::from_be_bytes,
            u16::to_be_bytes,
            u16::wrapping_add,
            u16::wrapping_sub
        ),
        (4, ByteOrder::Little) => run!(
            0u32,
            u32::from_le_bytes,
            u32::to_le_bytes,
            u32::wrapping_add,
            u32::wrapping_sub
        ),
        (4, ByteOrder::Big) => run!(
            0u32,
            u32::from_be_bytes,
            u32::to_be_bytes,
            u32::wrapping_add,
            u32::wrapping_sub
        ),
        _ => unreachable!("the sample width is validated by the caller"),
    }
}

/// Running prefix sum over one row (decode). The first sample has a virtual
/// predecessor of 0.
#[inline(always)]
fn undiff_row<T: Copy, const N: usize>(
    row: &mut [u8],
    zero: T,
    from: impl Fn([u8; N]) -> T,
    to: impl Fn(T) -> [u8; N],
    add: impl Fn(T, T) -> T,
) {
    let mut acc = zero;
    for chunk in row.chunks_exact_mut(N) {
        acc = add(acc, from(<[u8; N]>::try_from(&*chunk).unwrap()));
        chunk.copy_from_slice(&to(acc));
    }
}

/// Forward difference over one row (encode), carrying the previous original
/// value in a local.
#[inline(always)]
fn diff_row<T: Copy, const N: usize>(
    row: &mut [u8],
    zero: T,
    from: impl Fn([u8; N]) -> T,
    to: impl Fn(T) -> [u8; N],
    sub: impl Fn(T, T) -> T,
) {
    let mut previous = zero;
    for chunk in row.chunks_exact_mut(N) {
        let current = from(<[u8; N]>::try_from(&*chunk).unwrap());
        chunk.copy_from_slice(&to(sub(current, previous)));
        previous = current;
    }
}
