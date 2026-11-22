use bytemuck::Pod;
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::{MonoImage, RawMosaic};
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{DirectoryLookup, Tag, Value, unsigned_scalar};
use num_traits::ToPrimitive;
use rayon::prelude::*;

use super::{corrupt, unsigned_vec};
use crate::metadata::DngTag;

const MAX_EXACT_F32_INTEGER: u32 = 1 << 24;
const OPCODE_BAND_SAMPLES: usize = 32 * 1024;
const BAD_PIXEL_HALO_ROWS: usize = 2;

fn zeroed_samples<T>(len: usize) -> Result<Vec<T>, DecodeError>
where
    T: Pod,
{
    fornax_large_alloc::zeroed_samples(len)
        .map_err(|error| corrupt(format!("DNG transform allocation failed: {error}")))
}

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
    T: image::Primitive + Send + Sync + Pod,
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
    let opcodes = super::opcodes::parse(&bytes)?;
    match (kind, pixels) {
        (SampleKind::U8, RawMosaic::U8(image)) => {
            let source = image.into_raw();
            let converted =
                apply_opcode_bands(&source, width, height, &opcodes, f32::from, |value| {
                    convert_sample(value, u8::MAX)
                })?;
            MonoImage::from_raw(width_u32, height_u32, converted)
                .map(RawMosaic::U8)
                .ok_or_else(|| corrupt("opcode image dimensions are invalid"))
        }
        (SampleKind::U16, RawMosaic::U16(image)) => {
            let source = image.into_raw();
            let converted =
                apply_opcode_bands(&source, width, height, &opcodes, f32::from, |value| {
                    convert_sample(value, u16::MAX)
                })?;
            MonoImage::from_raw(width_u32, height_u32, converted)
                .map(RawMosaic::U16)
                .ok_or_else(|| corrupt("opcode image dimensions are invalid"))
        }
        (SampleKind::U32, RawMosaic::U32(image)) => {
            let source = image.into_raw();
            let converted = apply_opcode_bands(
                &source,
                width,
                height,
                &opcodes,
                |value| value.to_f32().expect("u32 converts to f32"),
                |value| {
                    value
                        .round()
                        .to_u32()
                        .ok_or_else(|| corrupt("OpcodeList1 output exceeds u32"))
                },
            )?;
            MonoImage::from_raw(width_u32, height_u32, converted)
                .map(RawMosaic::U32)
                .ok_or_else(|| corrupt("opcode image dimensions are invalid"))
        }
        (SampleKind::F32, RawMosaic::F32(image)) => {
            let source = image.into_raw();
            let converted =
                apply_opcode_bands(&source, width, height, &opcodes, |value| value, Ok)?;
            MonoImage::from_raw(width_u32, height_u32, converted)
                .map(RawMosaic::F32)
                .ok_or_else(|| corrupt("opcode image dimensions are invalid"))
        }
        _ => unreachable!("sample kind matches the source mosaic"),
    }
}

enum SampleKind {
    U8,
    U16,
    U32,
    F32,
}

fn convert_sample<T>(value: f32, maximum: T) -> Result<T, DecodeError>
where
    T: Copy + num_traits::FromPrimitive + num_traits::ToPrimitive + num_traits::Zero,
{
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
}

fn apply_opcode_bands<S, T, ToF32, FromF32>(
    source: &[S],
    width: usize,
    height: usize,
    opcodes: &super::opcodes::OpcodeList,
    to_f32: ToF32,
    from_f32: FromF32,
) -> Result<Vec<T>, DecodeError>
where
    S: Copy + Send + Sync + Pod,
    T: Copy + Send + Sync + Pod,
    ToF32: Fn(S) -> f32 + Send + Sync,
    FromF32: Fn(f32) -> Result<T, DecodeError> + Send + Sync,
{
    let expected = width
        .checked_mul(height)
        .ok_or_else(|| corrupt("opcode image sample count overflows"))?;
    if source.len() != expected {
        return Err(corrupt("opcode image sample count is invalid"));
    }
    let barriers = opcodes.neighborhood_barriers();
    if barriers > 1 {
        let mut scratch = zeroed_samples::<f32>(expected)?;
        scratch
            .par_iter_mut()
            .zip(source)
            .for_each(|(target, value)| *target = to_f32(*value));
        opcodes.apply_full(&mut scratch, width, height);
        let mut output = zeroed_samples::<T>(expected)?;
        output
            .par_iter_mut()
            .zip(scratch)
            .try_for_each(|(target, value)| {
                *target = from_f32(value)?;
                Ok::<(), DecodeError>(())
            })?;
        return Ok(output);
    }

    let band_rows = (OPCODE_BAND_SAMPLES / width).max(1);
    let band_len = band_rows * width;
    let halo_rows = usize::from(barriers == 1) * BAD_PIXEL_HALO_ROWS;
    let mut output = zeroed_samples::<T>(expected)?;
    output
        .par_chunks_mut(band_len)
        .enumerate()
        .map_init(Vec::new, |scratch, (band_index, output_band)| {
            let start = band_index * band_rows;
            let rows = output_band.len() / width;
            debug_assert!(start + rows <= height);
            let top_halo = start.min(halo_rows);
            let bottom_halo = (height - start - rows).min(halo_rows);
            let scratch_start = start - top_halo;
            let scratch_rows = top_halo + rows + bottom_halo;
            scratch.resize(scratch_rows * width, 0.0);
            let scratch_len = scratch.len();
            let source_start = scratch_start * width;
            for (target, value) in scratch
                .iter_mut()
                .zip(&source[source_start..source_start + scratch_len])
            {
                *target = to_f32(*value);
            }
            opcodes.apply_band(scratch, width, height, scratch_start, top_halo, rows);
            let target = &scratch[top_halo * width..(top_halo + rows) * width];
            for (target, value) in output_band.iter_mut().zip(target) {
                *target = from_f32(*value)?;
            }
            Ok::<(), DecodeError>(())
        })
        .try_for_each(std::convert::identity)?;
    Ok(output)
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
        RawMosaic::U32(image) => map_linearization(image.as_raw(), &table)?,
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

    let mut mapped = zeroed_samples::<u16>(samples.len())?;
    let mut mapped_index = 0;
    for row in samples.chunks(width) {
        let mut random = row.first().copied().map_or_default(u32::from);
        if let Some(second) = row.get(1) {
            random |= u32::from(*second) << 16;
        }
        for &sample in row {
            let (base, delta) = *table
                .get(usize::from(sample))
                .ok_or_else(|| corrupt(format!("LinearizationTable has no entry for {sample}")))?;
            let value = u32::from(base) + ((u32::from(delta) * (random & 2047) + 1024) >> 12);
            mapped[mapped_index] = u16::try_from(value)
                .map_err(|_| corrupt("LinearizationTable output exceeds u16"))?;
            mapped_index += 1;
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
    let mut output = zeroed_samples::<u16>(samples.len())?;
    for (target, value) in output.iter_mut().zip(samples) {
        let index = usize::try_from(*value)
            .map_err(|_| corrupt("linearization index exceeds the address space"))?;
        *target = table
            .get(index)
            .copied()
            .ok_or_else(|| corrupt(format!("LinearizationTable has no entry for {index}")))?;
    }
    Ok(output)
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

#[cfg(test)]
mod tests {
    use super::apply_opcode_bands;
    use crate::dng_decoder::opcodes;

    #[test]
    fn opcode_bands_cover_wide_images_and_partial_tail() {
        let width = 4496;
        let height = 100;
        let table = (0..1024).flat_map(u16::to_be_bytes).collect::<Vec<_>>();
        let mut list = Vec::new();
        for value in [1_u32, 7, 1, 0, 36 + u32::try_from(table.len()).unwrap()] {
            list.extend_from_slice(&value.to_be_bytes());
        }
        for value in [0_u32, 0, u32::MAX, u32::MAX, 0, 1, 1, 1] {
            list.extend_from_slice(&value.to_be_bytes());
        }
        list.extend_from_slice(&1024_u32.to_be_bytes());
        list.extend_from_slice(&table);
        let parsed = opcodes::parse(&list).unwrap();
        let source = (0..width * height)
            .map(|index| u16::try_from(index % 1024).unwrap())
            .collect::<Vec<_>>();

        let output = apply_opcode_bands(&source, width, height, &parsed, f32::from, |value| {
            super::convert_sample(value, u16::MAX)
        })
        .unwrap();

        assert_eq!(output, source);
    }
}
