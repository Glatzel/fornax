use bytemuck::Pod;
use fornax_core::MonoImage;
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{DirectoryLookup, Tag, Value, unsigned_scalar};
use num_traits::ToPrimitive;
use rayon::prelude::*;

use super::opcodes::SampleDomain;
use super::{corrupt, unsigned_vec};
use crate::RawMosaic;
use crate::dng::constants::DngCompressionMethod;
use crate::dng::metadata::DngTag;
use crate::error::DecodeError;

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
    let domain = sample_domain(kind, metadata);
    if opcodes.is_exact_identity(domain, width, height) {
        return Ok(pixels);
    }
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
            let converted = if opcodes.is_single_bad_pixels_constant(0) {
                apply_zero_bad_pixels_constant(&source, width, height)?
            } else {
                apply_opcode_bands(&source, width, height, &opcodes, f32::from, |value| {
                    convert_sample(value, u16::MAX)
                })?
            };
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

#[derive(Clone, Copy)]
enum SampleKind {
    U8,
    U16,
    U32,
    F32,
}

fn sample_domain<D>(kind: SampleKind, metadata: &D) -> SampleDomain
where
    D: DirectoryLookup + ?Sized,
{
    let bits = unsigned_scalar(metadata.get(&Tag::from(TiffTag::BitsPerSample)));
    let compression = unsigned_scalar(metadata.get(&Tag::from(TiffTag::Compression)));
    // Only codecs that mask/unpack strictly to `bits` without post-mask point
    // shifts guarantee `sample < (1 << bits)`. For JPEG XL, only `decode_u16`
    // right-shifts values back down to `bits`; `decode_u8` does not shift.
    let integer_bit_bounded =
        compression.is_some_and(|id| matches!(id, 1 | 5 | 8 | 32_773 | 32_946 | 50_000));
    match kind {
        SampleKind::U8 => {
            let max_sample = match bits {
                Some(b) if integer_bit_bounded && (1..=8).contains(&b) => (1_u32 << b) - 1,
                _ => u32::from(u8::MAX),
            };
            SampleDomain::Integer { max_sample }
        }
        SampleKind::U16 => {
            let jxl_bounded =
                compression == Some(u64::from(u16::from(DngCompressionMethod::JpegXl)));
            let max_sample = match bits {
                Some(b) if (integer_bit_bounded || jxl_bounded) && (1..=16).contains(&b) => {
                    (1_u32 << b) - 1
                }
                _ => u32::from(u16::MAX),
            };
            SampleDomain::Integer { max_sample }
        }
        SampleKind::U32 => SampleDomain::Integer {
            max_sample: MAX_EXACT_F32_INTEGER,
        },
        SampleKind::F32 => SampleDomain::Float,
    }
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

fn apply_zero_bad_pixels_constant(
    source: &[u16],
    width: usize,
    height: usize,
) -> Result<Vec<u16>, DecodeError> {
    let expected = width
        .checked_mul(height)
        .ok_or_else(|| corrupt("opcode image sample count overflows"))?;
    if width == 0 || height == 0 || source.len() != expected {
        return Err(corrupt("opcode image sample count is invalid"));
    }
    let band_rows = (OPCODE_BAND_SAMPLES / width).max(1);
    let band_len = band_rows * width;
    let mut output = zeroed_samples::<u16>(expected)?;
    output.copy_from_slice(source);
    output
        .par_chunks_mut(band_len)
        .enumerate()
        .for_each(|(band_index, output_band)| {
            let start_row = band_index * band_rows;
            for (local_row, output_row) in output_band.chunks_mut(width).enumerate() {
                let row = start_row + local_row;
                for column in 0..width {
                    if source[row * width + column] != 0 {
                        continue;
                    }
                    let mut sum = 0.0_f32;
                    let mut count = 0.0_f32;
                    for (dx, dy) in [
                        (-2_isize, 0_isize),
                        (2, 0),
                        (0, -2),
                        (0, 2),
                        (-2, -2),
                        (2, 2),
                        (-2, 2),
                        (2, -2),
                    ] {
                        let Some(neighbor_row) = row.checked_add_signed(dy) else {
                            continue;
                        };
                        let Some(neighbor_column) = column.checked_add_signed(dx) else {
                            continue;
                        };
                        if neighbor_row >= height || neighbor_column >= width {
                            continue;
                        }
                        let value = source[neighbor_row * width + neighbor_column];
                        if value != 0 {
                            sum += f32::from(value);
                            count += 1.0;
                        }
                    }
                    if count > 0.0 {
                        output_row[column] = (sum / count).round() as u16;
                    }
                }
            }
        });
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
    use crate::dng::dng_decoder::opcodes;

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

    #[test]
    fn zero_bad_pixel_fast_path_matches_f32_oracle() {
        let width = 8;
        let height = 6;
        let mut source = (0..width * height)
            .map(|index| u16::try_from(index + 1).unwrap())
            .collect::<Vec<_>>();
        source[3 * width + 4] = 0;
        let mut list = Vec::new();
        for value in [1_u32, 4, 1, 0, 4] {
            list.extend_from_slice(&value.to_be_bytes());
        }
        list.extend_from_slice(&0_u32.to_be_bytes());
        let parsed = opcodes::parse(&list).unwrap();

        let expected = apply_opcode_bands(&source, width, height, &parsed, f32::from, |value| {
            super::convert_sample(value, u16::MAX)
        })
        .unwrap();
        let actual = super::apply_zero_bad_pixels_constant(&source, width, height).unwrap();

        assert_eq!(actual, expected);
    }

    #[test]
    fn identity_opcode_elision_matches_full_apply_and_preserves_errors() {
        use fornax_core::MonoImage;
        use fornax_tiff::constants::TiffTag;
        use fornax_tiff::{Directory, Tag, Value};

        use super::{apply_opcode_bands, apply_opcode_list1, convert_sample};
        use crate::RawMosaic;

        let mut table_bytes = Vec::new();
        let table = (0_u16..1024).flat_map(u16::to_be_bytes).collect::<Vec<_>>();
        table_bytes.extend_from_slice(&4_u32.to_be_bytes());
        for (top, left) in [(0_u32, 0_u32), (0, 1), (1, 0), (1, 1)] {
            table_bytes.extend_from_slice(&7_u32.to_be_bytes());
            table_bytes.extend_from_slice(&0x0103_0000_u32.to_be_bytes());
            table_bytes.extend_from_slice(&0_u32.to_be_bytes());
            table_bytes.extend_from_slice(&u32::try_from(36 + table.len()).unwrap().to_be_bytes());
            for v in [top, left, 8, 8, 0, 1, 2, 2, 1024] {
                table_bytes.extend_from_slice(&v.to_be_bytes());
            }
            table_bytes.extend_from_slice(&table);
        }

        let mut metadata = Directory::default();
        metadata.insert(Tag::from(TiffTag::Compression), Value::Short(1));
        metadata.insert(Tag::from(TiffTag::BitsPerSample), Value::Short(10));
        metadata.insert(
            Tag::from(crate::dng::metadata::DngTag::OpcodeList1),
            Value::Bytes(table_bytes.clone()),
        );

        let samples: Vec<u16> = (0..64).map(|i| (i * 17) & 0x3ff).collect();
        let parsed = opcodes::parse(&table_bytes).unwrap();
        assert!(parsed.is_exact_identity(
            opcodes::SampleDomain::Integer { max_sample: 1023 },
            8,
            8
        ));
        // If samples could exceed table length (e.g. 16-bit container), it is
        // NOT identity.
        assert!(!parsed.is_exact_identity(
            opcodes::SampleDomain::Integer { max_sample: 65535 },
            8,
            8
        ));
        assert!(!parsed.is_exact_identity(opcodes::SampleDomain::Float, 8, 8));

        let expected = apply_opcode_bands(&samples, 8, 8, &parsed, f32::from, |v| {
            convert_sample(v, u16::MAX)
        })
        .unwrap();
        let mosaic = RawMosaic::U16(MonoImage::from_raw(8, 8, samples).unwrap());
        let RawMosaic::U16(actual) = apply_opcode_list1(mosaic, &metadata).unwrap() else {
            panic!("expected U16 mosaic");
        };
        assert_eq!(actual.into_raw(), expected);

        // U32 > 2^24 must still error even when the opcode list is an identity.
        let mut scale_one = Vec::new();
        scale_one.extend_from_slice(&1_u32.to_be_bytes());
        scale_one.extend_from_slice(&12_u32.to_be_bytes());
        scale_one.extend_from_slice(&0x0103_0000_u32.to_be_bytes());
        scale_one.extend_from_slice(&0_u32.to_be_bytes());
        scale_one.extend_from_slice(&40_u32.to_be_bytes());
        for v in [0_u32, 0, u32::MAX, u32::MAX, 0, 1, 1, 1, 1] {
            scale_one.extend_from_slice(&v.to_be_bytes());
        }
        scale_one.extend_from_slice(&1.0_f32.to_bits().to_be_bytes());
        metadata.insert(
            Tag::from(crate::dng::metadata::DngTag::OpcodeList1),
            Value::Bytes(scale_one),
        );
        let oversized = RawMosaic::U32(MonoImage::from_raw(1, 1, vec![(1 << 24) + 1]).unwrap());
        assert!(apply_opcode_list1(oversized, &metadata).is_err());

        // Float edge cases (-0.0, subnormals, NaN) must stay conservative and
        // preserve full-apply behavior.
        let f32_samples = [-0.0_f32, f32::from_bits(1), f32::NAN, 42.5];
        let f32_mosaic = RawMosaic::F32(MonoImage::from_raw(4, 1, f32_samples.to_vec()).unwrap());
        let RawMosaic::F32(f32_out) = apply_opcode_list1(f32_mosaic, &metadata).unwrap() else {
            panic!("expected F32 mosaic");
        };
        assert!(f32_out.as_raw()[0].is_sign_negative());
        assert_eq!(f32_out.as_raw()[1].to_bits(), 1);
        assert!(f32_out.as_raw()[2].is_nan());
        assert_eq!(f32_out.as_raw()[3], 42.5);
    }
}
