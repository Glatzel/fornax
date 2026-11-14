use fornax_raw_core::error::DecodeError;
use fornax_raw_core::{MonoImage, RawMosaic};
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{DirectoryLookup, Tag, Value, unsigned_scalar};
use num_traits::ToPrimitive;
use rayon::prelude::*;

use super::{corrupt, unsigned_vec};
use crate::metadata::DngTag;

const MAX_EXACT_F32_INTEGER: u32 = 1 << 24;

pub(super) fn apply<D>(pixels: RawMosaic, metadata: &D) -> Result<RawMosaic, DecodeError>
where
    D: DirectoryLookup + ?Sized,
{
    let pixels = deinterleave(pixels, metadata)?;
    let pixels = apply_opcode_list1(pixels, metadata)?;
    apply_linearization(pixels, metadata)
}

fn deinterleave<D>(pixels: RawMosaic, metadata: &D) -> Result<RawMosaic, DecodeError>
where
    D: DirectoryLookup + ?Sized,
{
    let row_factor =
        unsigned_scalar(metadata.get(&DngTag::RowInterleaveFactor.into())).unwrap_or(1);
    let column_factor =
        unsigned_scalar(metadata.get(&DngTag::ColumnInterleaveFactor.into())).unwrap_or(1);
    if row_factor == 1 && column_factor == 1 {
        return Ok(pixels);
    }
    if row_factor != 2 || column_factor != 2 {
        return Err(corrupt(format!(
            "interleave factors {row_factor}x{column_factor} are invalid"
        )));
    }
    match pixels {
        RawMosaic::U8(image) => deinterleave_image(image).map(RawMosaic::U8),
        RawMosaic::U16(image) => deinterleave_image(image).map(RawMosaic::U16),
        RawMosaic::U32(image) => deinterleave_image(image).map(RawMosaic::U32),
        RawMosaic::F32(image) => deinterleave_image(image).map(RawMosaic::F32),
    }
}

fn deinterleave_image<T>(image: MonoImage<T>) -> Result<MonoImage<T>, DecodeError>
where
    T: image::Primitive + Send + Sync,
{
    let (width_u32, height_u32) = image.dimensions();
    let width = usize::try_from(width_u32)
        .map_err(|_| corrupt("interleaved image width exceeds the address space"))?;
    let height = usize::try_from(height_u32)
        .map_err(|_| corrupt("interleaved image height exceeds the address space"))?;
    if width == 0 || height == 0 || width % 2 != 0 || height % 2 != 0 {
        return Err(corrupt(
            "2x2 interleaved storage requires non-zero even dimensions",
        ));
    }
    let source = image.into_raw();
    let half_width = width / 2;
    let source_row_bytes = width;
    let half_len = source.len() / 2;
    let top_left = &*source;
    let top_right = &source[half_width..];
    let bottom_left = &source[half_len..];
    let bottom_right = &source[half_len + half_width..];
    let mut output = vec![source[0]; source.len()];
    output
        .par_chunks_mut(width)
        .enumerate()
        .try_for_each(|(row, target)| {
            let source_row = row / 2;
            let offset = source_row * source_row_bytes;
            let (first, second) = if row & 1 == 0 {
                (
                    top_left.get(offset..offset + half_width),
                    top_right.get(offset..offset + half_width),
                )
            } else {
                (
                    bottom_left.get(offset..offset + half_width),
                    bottom_right.get(offset..offset + half_width),
                )
            };
            let first = first.ok_or_else(|| corrupt("interleaved image channel is truncated"))?;
            let second = second.ok_or_else(|| corrupt("interleaved image channel is truncated"))?;
            for ((target, first), second) in target
                .as_chunks_mut::<2>()
                .0
                .iter_mut()
                .zip(first.iter().copied())
                .zip(second.iter().copied())
            {
                target[0] = first;
                target[1] = second;
            }
            Ok::<(), DecodeError>(())
        })?;
    MonoImage::from_raw(width_u32, height_u32, output)
        .ok_or_else(|| corrupt("deinterleaved image dimensions are invalid"))
}

fn apply_opcode_list1<D>(pixels: RawMosaic, metadata: &D) -> Result<RawMosaic, DecodeError>
where
    D: DirectoryLookup + ?Sized,
{
    let Some(value) = metadata.get(&DngTag::OpcodeList1.into()) else {
        return Ok(pixels);
    };
    let bytes = opcode_bytes(value)?;
    if bytes.is_empty() {
        return Ok(pixels);
    }
    let (width_u32, height_u32) = pixels.dimensions();
    let width = usize::try_from(width_u32)
        .map_err(|_| corrupt("opcode image width exceeds the address space"))?;
    let height = usize::try_from(height_u32)
        .map_err(|_| corrupt("opcode image height exceeds the address space"))?;
    let kind = match &pixels {
        RawMosaic::U8(_) => SampleKind::U8,
        RawMosaic::U16(_) => SampleKind::U16,
        RawMosaic::U32(image) => {
            if image
                .as_raw()
                .iter()
                .any(|value| *value > MAX_EXACT_F32_INTEGER)
            {
                return Err(corrupt(
                    "OpcodeList1 cannot exactly process u32 samples above 2^24",
                ));
            }
            SampleKind::U32
        }
        RawMosaic::F32(_) => SampleKind::F32,
    };
    let mut samples = match &pixels {
        RawMosaic::U8(image) => image
            .as_raw()
            .iter()
            .map(|value| f32::from(*value))
            .collect(),
        RawMosaic::U16(image) => image
            .as_raw()
            .iter()
            .map(|value| f32::from(*value))
            .collect(),
        RawMosaic::U32(image) => image
            .as_raw()
            .iter()
            .map(|value| value.to_f32().expect("u32 converts to f32"))
            .collect::<Vec<_>>(),
        RawMosaic::F32(image) => image.as_raw().clone(),
    };
    super::opcodes::apply(&bytes, &mut samples, width, height)?;

    match kind {
        SampleKind::U8 => {
            let samples = convert_samples(&samples, 255)?;
            MonoImage::from_raw(width_u32, height_u32, samples)
                .map(RawMosaic::U8)
                .ok_or_else(|| corrupt("opcode image dimensions are invalid"))
        }
        SampleKind::U16 => {
            let samples = convert_samples(&samples, u16::MAX)?;
            MonoImage::from_raw(width_u32, height_u32, samples)
                .map(RawMosaic::U16)
                .ok_or_else(|| corrupt("opcode image dimensions are invalid"))
        }
        SampleKind::U32 => {
            let samples = samples
                .into_iter()
                .map(|value| {
                    value
                        .round()
                        .to_u32()
                        .ok_or_else(|| corrupt("OpcodeList1 output exceeds u32"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            MonoImage::from_raw(width_u32, height_u32, samples)
                .map(RawMosaic::U32)
                .ok_or_else(|| corrupt("opcode image dimensions are invalid"))
        }
        SampleKind::F32 => MonoImage::from_raw(width_u32, height_u32, samples)
            .map(RawMosaic::F32)
            .ok_or_else(|| corrupt("opcode image dimensions are invalid")),
    }
}

enum SampleKind {
    U8,
    U16,
    U32,
    F32,
}

fn convert_samples<T>(samples: &[f32], maximum: T) -> Result<Vec<T>, DecodeError>
where
    T: Copy
        + num_traits::Bounded
        + num_traits::FromPrimitive
        + num_traits::ToPrimitive
        + num_traits::Zero,
{
    samples
        .iter()
        .map(|value| {
            let rounded = value.round();
            if !rounded.is_finite() {
                return Err(corrupt("OpcodeList1 produced a non-finite sample"));
            }
            let zero = T::zero();
            if rounded < zero.to_f32().expect("bounded sample types convert to f32")
                || rounded
                    > maximum
                        .to_f32()
                        .expect("bounded sample types convert to f32")
            {
                return Err(corrupt("OpcodeList1 output exceeds the stored sample type"));
            }
            T::from_f32(rounded).ok_or_else(|| corrupt("OpcodeList1 output is not representable"))
        })
        .collect()
}

fn apply_linearization<D>(pixels: RawMosaic, metadata: &D) -> Result<RawMosaic, DecodeError>
where
    D: DirectoryLookup + ?Sized,
{
    if !metadata.contains_key(&Tag::from(DngTag::LinearizationTable)) {
        return Ok(pixels);
    }
    let table = unsigned_vec(metadata, DngTag::LinearizationTable)?
        .into_iter()
        .map(|value| {
            u16::try_from(value).map_err(|_| corrupt("LinearizationTable value exceeds u16"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    if table.is_empty() {
        return Err(corrupt("LinearizationTable must not be empty"));
    }
    let (width, height) = pixels.dimensions();
    let mapped = match pixels {
        RawMosaic::U8(image) => map_linearization(image.as_raw(), &table)?,
        RawMosaic::U16(image) => {
            let bits = u32::try_from(
                unsigned_scalar(metadata.get(&Tag::from(TiffTag::BitsPerSample)))
                    .ok_or_else(|| corrupt("BitsPerSample is missing or invalid"))?,
            )
            .map_err(|_| corrupt("LinearizationTable bit depth exceeds u32"))?;
            map_u16_linearization(image.as_raw(), width, &table, bits)?
        }
        RawMosaic::U32(image) => {
            let samples = image
                .as_raw()
                .iter()
                .map(|value| {
                    usize::try_from(*value)
                        .map_err(|_| corrupt("linearization index exceeds the address space"))
                })
                .collect::<Result<Vec<_>, _>>()?;
            lookup_linearization(&samples, &table)?
        }
        RawMosaic::F32(_) => {
            return Err(corrupt(
                "LinearizationTable cannot index floating-point samples",
            ));
        }
    };
    MonoImage::from_raw(width, height, mapped)
        .map(RawMosaic::U16)
        .ok_or_else(|| corrupt("linearized image dimensions are invalid"))
}

fn map_u16_linearization(
    samples: &[u16],
    width: u32,
    table: &[u16],
    bits: u32,
) -> Result<Vec<u16>, DecodeError> {
    if bits > 16 {
        return Err(corrupt(
            "LinearizationTable requires at most sixteen-bit sample indexes",
        ));
    }
    let table_size = 1_usize << bits;
    if table.len() < table_size {
        let mut expanded = Vec::with_capacity(table_size);
        expanded.extend_from_slice(table);
        expanded.resize(
            table_size,
            *table
                .last()
                .ok_or_else(|| corrupt("LinearizationTable must not be empty"))?,
        );
        return apply_u16_linearization(samples, width, &expanded);
    }
    apply_u16_linearization(samples, width, table)
}

fn apply_u16_linearization(
    samples: &[u16],
    width: u32,
    table: &[u16],
) -> Result<Vec<u16>, DecodeError> {
    if table.is_empty() {
        return Err(corrupt("LinearizationTable must not be empty"));
    }
    let width = usize::try_from(width)
        .map_err(|_| corrupt("linearized image width exceeds the address space"))?;
    if width == 0 {
        return Err(corrupt("linearized image width must not be zero"));
    }
    let table = table
        .iter()
        .enumerate()
        .map(|(index, center)| {
            let lower = table[index.saturating_sub(1)];
            let upper = table[(index + 1).min(table.len() - 1)];
            let base = if *center == 0 {
                0
            } else {
                let spread = upper
                    .checked_sub(lower)
                    .ok_or_else(|| corrupt("LinearizationTable must be non-decreasing"))?;
                let correction = u16::try_from((u32::from(spread) + 2) / 4)
                    .expect("linearization table delta fits u16");
                center
                    .checked_sub(correction)
                    .ok_or_else(|| corrupt("LinearizationTable base underflows"))?
            };
            let spread = upper
                .checked_sub(lower)
                .ok_or_else(|| corrupt("LinearizationTable must be non-decreasing"))?;
            Ok((base, spread))
        })
        .collect::<Result<Vec<_>, DecodeError>>()?;

    let mut mapped = Vec::with_capacity(samples.len());
    for row in samples.chunks(width) {
        let mut random = row.first().copied().map(u32::from).unwrap_or_default();
        if let Some(second) = row.get(1) {
            random |= u32::from(*second) << 16;
        }
        for &sample in row {
            let (base, delta) = *table
                .get(usize::from(sample))
                .ok_or_else(|| corrupt(format!("LinearizationTable has no entry for {sample}")))?;
            let value = u32::from(base) + ((u32::from(delta) * (random & 2047) + 1024) >> 12);
            mapped.push(
                u16::try_from(value)
                    .map_err(|_| corrupt("LinearizationTable output exceeds u16"))?,
            );
            random = 15_700 * (random & 65_535) + (random >> 16);
        }
    }
    Ok(mapped)
}

fn map_linearization<T>(samples: &[T], table: &[u16]) -> Result<Vec<u16>, DecodeError>
where
    T: Copy,
    usize: TryFrom<T>,
{
    let indexes = samples
        .iter()
        .map(|value| {
            usize::try_from(*value)
                .map_err(|_| corrupt("linearization index exceeds the address space"))
        })
        .collect::<Result<Vec<_>, _>>()?;
    lookup_linearization(&indexes, table)
}

fn lookup_linearization(indexes: &[usize], table: &[u16]) -> Result<Vec<u16>, DecodeError> {
    indexes
        .iter()
        .map(|index| {
            table
                .get(*index)
                .copied()
                .ok_or_else(|| corrupt(format!("LinearizationTable has no entry for {index}")))
        })
        .collect()
}

fn opcode_bytes(value: &Value) -> Result<Vec<u8>, DecodeError> {
    match value {
        Value::Byte(value) => Ok(vec![*value]),
        Value::Bytes(values) => Ok(values.clone()),
        Value::List(values) => values
            .iter()
            .map(|value| match value {
                Value::Byte(value) => Ok(*value),
                _ => Err(corrupt("OpcodeList1 must contain BYTE values")),
            })
            .collect(),
        _ => Err(corrupt("OpcodeList1 must contain BYTE values")),
    }
}
