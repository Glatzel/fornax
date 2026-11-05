//! DNG container and sample decoding.

mod ljpeg;
mod process_metadata;
mod tiff;

use ::tiff::decoder::ifd::Value;
use fornax_core::error::DecodeError;
use fornax_core::metadata::dng::{DngMetadata, DngTag};
use fornax_core::metadata::process::ProcessMetadata;
use fornax_core::{CroppedMosaic, MonoImage, Primitive, RawMosaic};
use generic_num::num;
use image::math::Rect;
use process_metadata::process_metadata_from_dng_metadata;
use rayon::prelude::*;
pub use tiff::ParsedTiff;

const PHOTOMETRIC_CFA: u64 = 32_803;
const PHOTOMETRIC_LINEAR_RAW: u64 = 34_892;
const COMPRESSION_UNCOMPRESSED: u16 = 1;
const COMPRESSION_LOSSLESS_JPEG: u16 = 7;

pub fn decode(source: &[u8]) -> Result<ParsedTiff, DecodeError> { tiff::parse(source) }

pub fn raw_mosaic(parsed: &ParsedTiff) -> Result<RawMosaic, DecodeError> {
    let metadata = dng_metadata(parsed)?;
    let pixels = decode_samples(&parsed.source, &metadata)?;
    validate_dimensions(&pixels, &metadata)?;
    Ok(pixels)
}

pub fn cropped_mosaic<T>(parsed: &ParsedTiff) -> Result<CroppedMosaic<T>, DecodeError>
where
    T: Primitive,
{
    let pixels = raw_mosaic(parsed)?;
    let metadata = dng_metadata(parsed)?;
    let process_metadata = process_metadata_from_dng_metadata::<T>(&pixels, &metadata)?;
    let active_area = process_metadata.active_area;
    let image = match pixels {
        RawMosaic::U8(image) => crop_and_cast(image, active_area)?,
        RawMosaic::U16(image) => crop_and_cast(image, active_area)?,
        RawMosaic::U32(image) => crop_and_cast(image, active_area)?,
        RawMosaic::F32(image) => crop_and_cast(image, active_area)?,
    };
    Ok(CroppedMosaic::new(image))
}

pub fn dng_metadata(parsed: &ParsedTiff) -> Result<DngMetadata, DecodeError> {
    if !parsed.root.contains_key(&DngTag::DngVersion) {
        return Err(unsupported_variant(
            "TIFF container does not declare a DNGVersion",
        ));
    }
    let raw_metadata = parsed
        .directories
        .iter()
        .filter(|metadata| is_raw_ifd(metadata))
        .max_by_key(|metadata| raw_area(metadata))
        .ok_or_else(|| unsupported_variant("no full-resolution CFA image IFD was found"))?;
    let mut dng_metadata = parsed.root.clone();
    dng_metadata.extend(
        raw_metadata
            .iter()
            .map(|(tag, value)| (*tag, value.clone())),
    );
    Ok(dng_metadata)
}

pub fn process_metadata<T>(parsed: &ParsedTiff) -> Result<ProcessMetadata<T>, DecodeError>
where
    T: Primitive,
{
    let pixels = raw_mosaic(parsed)?;
    let metadata = dng_metadata(parsed)?;
    process_metadata_from_dng_metadata(&pixels, &metadata)
}

fn crop_and_cast<S, T>(mut image: MonoImage<S>, area: Rect) -> Result<MonoImage<T>, DecodeError>
where
    S: image::Primitive + Send + Sync,
    T: Primitive,
{
    image.crop_in_place(area);
    let (width, height) = image.dimensions();
    let samples = image
        .into_raw()
        .into_par_iter()
        .map(|value| num!(value))
        .collect();
    MonoImage::from_raw(width, height, samples)
        .ok_or_else(|| corrupt("cropped DNG dimensions are invalid"))
}

fn validate_dimensions(pixels: &RawMosaic, metadata: &DngMetadata) -> Result<(), DecodeError> {
    let width = required_u32(metadata, DngTag::ImageWidth)?;
    let height = required_u32(metadata, DngTag::ImageLength)?;
    let (actual_width, actual_height) = pixels.dimensions();
    if (width, height) != (actual_width, actual_height) {
        return Err(corrupt(format!(
            "DNG metadata dimensions are {width}x{height}, mosaic dimensions are \
             {actual_width}x{actual_height}"
        )));
    }
    Ok(())
}

fn is_raw_ifd(metadata: &DngMetadata) -> bool {
    let photometric = unsigned_scalar(metadata.get(&DngTag::PhotometricInterpretation));
    let reduced_resolution =
        unsigned_scalar(metadata.get(&DngTag::NewSubfileType)).is_some_and(|value| value & 1 != 0);
    !reduced_resolution && matches!(photometric, Some(PHOTOMETRIC_CFA | PHOTOMETRIC_LINEAR_RAW))
}

fn raw_area(metadata: &DngMetadata) -> u64 {
    let width = unsigned_scalar(metadata.get(&DngTag::ImageWidth)).unwrap_or(0);
    let height = unsigned_scalar(metadata.get(&DngTag::ImageLength)).unwrap_or(0);
    width.saturating_mul(height)
}

fn decode_samples(source: &[u8], metadata: &DngMetadata) -> Result<RawMosaic, DecodeError> {
    let width = required_u32(metadata, DngTag::ImageWidth)?;
    let height = required_u32(metadata, DngTag::ImageLength)?;
    if width == 0 || height == 0 {
        return Err(corrupt("DNG image dimensions must not be zero"));
    }
    let compression = required_u16(metadata, DngTag::Compression)?;
    let samples_per_pixel = unsigned_scalar(metadata.get(&DngTag::SamplesPerPixel)).unwrap_or(1);
    if samples_per_pixel != 1 {
        return Err(unsupported_variant(format!(
            "only one raw sample per pixel is supported, found {samples_per_pixel}"
        )));
    }

    match compression {
        COMPRESSION_UNCOMPRESSED => decode_uncompressed(source, metadata, width, height),
        COMPRESSION_LOSSLESS_JPEG => decode_lossless_jpeg(source, metadata, width, height),
        _ => Err(unsupported_variant(format!(
            "DNG compression {compression} is unsupported"
        ))),
    }
}

fn decode_lossless_jpeg(
    source: &[u8],
    metadata: &DngMetadata,
    width: u32,
    height: u32,
) -> Result<RawMosaic, DecodeError> {
    let bits = required_u16(metadata, DngTag::BitsPerSample)?;
    if bits != 16 {
        return Err(unsupported_variant(format!(
            "lossless JPEG CFA samples with {bits} bits are unsupported"
        )));
    }

    let (tile_width, tile_height, offsets, byte_counts) = layout(metadata)?;
    let width = usize::try_from(width).expect("u32 fits usize on supported platforms");
    let height = usize::try_from(height).expect("u32 fits usize on supported platforms");
    let sample_count = width
        .checked_mul(height)
        .ok_or_else(|| corrupt("raw sample count overflow"))?;
    let mut output = vec![0_u16; sample_count];

    let across = width.div_ceil(tile_width);
    let down = height.div_ceil(tile_height);
    if offsets.len() != byte_counts.len() || offsets.len() != across * down {
        return Err(corrupt(format!(
            "expected {} tiles, found {} offsets and {} byte counts",
            across * down,
            offsets.len(),
            byte_counts.len()
        )));
    }

    for (index, (&offset, &byte_count)) in offsets.iter().zip(&byte_counts).enumerate() {
        let encoded = checked_slice(source, offset, byte_count)?;
        let decoded = ljpeg::decode(encoded, tile_width, tile_height)
            .map_err(|detail| corrupt(format!("lossless JPEG tile {index}: {detail}")))?;
        let expected_samples = tile_width
            .checked_mul(tile_height)
            .ok_or_else(|| corrupt("lossless JPEG tile sample count overflow"))?;
        if decoded.len() != expected_samples {
            return Err(corrupt(format!(
                "lossless JPEG tile {index} decoded {} samples, expected {expected_samples}",
                decoded.len()
            )));
        }

        let tile_x = (index % across) * tile_width;
        let tile_y = (index / across) * tile_height;
        let copy_width = tile_width.min(width - tile_x);
        let copy_height = tile_height.min(height - tile_y);
        for row in 0..copy_height {
            let source_start = row * tile_width;
            let target_start = (tile_y + row) * width + tile_x;
            output[target_start..target_start + copy_width]
                .copy_from_slice(&decoded[source_start..source_start + copy_width]);
        }
    }

    to_mosaic(width, height, output)
}

fn decode_uncompressed(
    source: &[u8],
    metadata: &DngMetadata,
    width: u32,
    height: u32,
) -> Result<RawMosaic, DecodeError> {
    let bits = required_u16(metadata, DngTag::BitsPerSample)?;
    let (tile_width, tile_height, offsets, byte_counts) = layout(metadata)?;
    let width = usize::try_from(width).expect("u32 fits usize on supported platforms");
    let height = usize::try_from(height).expect("u32 fits usize on supported platforms");
    let across = width.div_ceil(tile_width);
    let down = height.div_ceil(tile_height);
    if offsets.len() != byte_counts.len() || offsets.len() != across * down {
        return Err(corrupt(format!(
            "expected {} chunks, found {} offsets and {} byte counts",
            across * down,
            offsets.len(),
            byte_counts.len()
        )));
    }

    match bits {
        8 => {
            let sample_count = width
                .checked_mul(height)
                .ok_or_else(|| corrupt("uncompressed DNG sample count overflow"))?;
            let mut output = vec![0_u8; sample_count];
            copy_bytes(
                source,
                &offsets,
                &byte_counts,
                width,
                height,
                tile_width,
                tile_height,
                1,
                &mut output,
            )?;
            MonoImage::from_raw(width as u32, height as u32, output)
                .map(RawMosaic::U8)
                .ok_or_else(|| corrupt("uncompressed DNG dimensions are invalid"))
        }
        16 => {
            let mut output = vec![0_u16; width * height];
            copy_samples(
                source,
                &offsets,
                &byte_counts,
                width,
                height,
                tile_width,
                tile_height,
                2,
                byte_order(source),
                &mut output,
            )?;
            to_mosaic(width, height, output)
        }
        32 => {
            if sample_format(metadata) == 3 {
                let mut output = vec![0_f32; width * height];
                copy_float_samples(
                    source,
                    &offsets,
                    &byte_counts,
                    width,
                    height,
                    tile_width,
                    tile_height,
                    byte_order(source),
                    &mut output,
                )?;
                MonoImage::from_raw(width as u32, height as u32, output)
                    .map(RawMosaic::F32)
                    .ok_or_else(|| {
                        corrupt("uncompressed floating-point DNG dimensions are invalid")
                    })
            } else {
                let mut output = vec![0_u32; width * height];
                copy_samples(
                    source,
                    &offsets,
                    &byte_counts,
                    width,
                    height,
                    tile_width,
                    tile_height,
                    4,
                    byte_order(source),
                    &mut output,
                )?;
                MonoImage::from_raw(width as u32, height as u32, output)
                    .map(RawMosaic::U32)
                    .ok_or_else(|| corrupt("uncompressed DNG dimensions are invalid"))
            }
        }
        _ => Err(unsupported_variant(format!(
            "uncompressed DNG samples with {bits} bits are unsupported"
        ))),
    }
}

#[allow(
    clippy::too_many_arguments,
    reason = "the checked tile-copy parameters mirror the validated TIFF layout"
)]
fn copy_bytes(
    source: &[u8],
    offsets: &[usize],
    byte_counts: &[usize],
    width: usize,
    height: usize,
    tile_width: usize,
    tile_height: usize,
    bytes_per_sample: usize,
    output: &mut [u8],
) -> Result<(), DecodeError> {
    let across = width.div_ceil(tile_width);
    for (index, (&offset, &byte_count)) in offsets.iter().zip(byte_counts).enumerate() {
        let encoded = checked_slice(source, offset, byte_count)?;
        let tile_x = (index % across) * tile_width;
        let tile_y = (index / across) * tile_height;
        let copy_width = tile_width.min(width - tile_x);
        let copy_height = tile_height.min(height - tile_y);
        let expected = copy_width
            .checked_mul(copy_height)
            .and_then(|count| count.checked_mul(bytes_per_sample))
            .ok_or_else(|| corrupt("uncompressed DNG chunk size overflow"))?;
        if encoded.len() < expected {
            return Err(corrupt(format!(
                "uncompressed DNG chunk {index} is truncated"
            )));
        }
        let copy_bytes = copy_width * bytes_per_sample;
        for row in 0..copy_height {
            let source_start = row * tile_width * bytes_per_sample;
            let target_start =
                (tile_y + row) * width * bytes_per_sample + tile_x * bytes_per_sample;
            output[target_start..target_start + copy_bytes]
                .copy_from_slice(&encoded[source_start..source_start + copy_bytes]);
        }
    }
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "the typed copy wrapper forwards the complete validated tile layout"
)]
fn copy_samples<T>(
    source: &[u8],
    offsets: &[usize],
    byte_counts: &[usize],
    width: usize,
    height: usize,
    tile_width: usize,
    tile_height: usize,
    bytes_per_sample: usize,
    byte_order: ByteOrder,
    output: &mut [T],
) -> Result<(), DecodeError>
where
    T: FromSampleBytes,
{
    let byte_count = width
        .checked_mul(height)
        .and_then(|count| count.checked_mul(bytes_per_sample))
        .ok_or_else(|| corrupt("uncompressed DNG sample buffer size overflow"))?;
    let mut bytes = vec![0_u8; byte_count];
    copy_bytes(
        source,
        offsets,
        byte_counts,
        width,
        height,
        tile_width,
        tile_height,
        bytes_per_sample,
        &mut bytes,
    )?;
    for (source, target) in bytes.chunks_exact(bytes_per_sample).zip(output.iter_mut()) {
        *target = T::from_bytes(source, byte_order);
    }
    Ok(())
}

#[allow(
    clippy::too_many_arguments,
    reason = "the float wrapper forwards the complete validated tile layout"
)]
fn copy_float_samples(
    source: &[u8],
    offsets: &[usize],
    byte_counts: &[usize],
    width: usize,
    height: usize,
    tile_width: usize,
    tile_height: usize,
    byte_order: ByteOrder,
    output: &mut [f32],
) -> Result<(), DecodeError> {
    copy_samples(
        source,
        offsets,
        byte_counts,
        width,
        height,
        tile_width,
        tile_height,
        4,
        byte_order,
        output,
    )
}

#[derive(Clone, Copy)]
enum ByteOrder {
    Little,
    Big,
}

fn byte_order(source: &[u8]) -> ByteOrder {
    match source.get(..2) {
        Some(b"II") => ByteOrder::Little,
        _ => ByteOrder::Big,
    }
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

impl FromSampleBytes for f32 {
    fn from_bytes(bytes: &[u8], byte_order: ByteOrder) -> Self {
        Self::from_bits(u32::from_bytes(bytes, byte_order))
    }
}

fn layout(metadata: &DngMetadata) -> Result<(usize, usize, Vec<usize>, Vec<usize>), DecodeError> {
    if metadata.contains_key(&DngTag::TileOffsets) {
        let width = usize::try_from(required_u32(metadata, DngTag::TileWidth)?)
            .expect("u32 fits usize on supported platforms");
        let height = usize::try_from(required_u32(metadata, DngTag::TileLength)?)
            .expect("u32 fits usize on supported platforms");
        if width == 0 || height == 0 {
            return Err(corrupt("DNG tile dimensions must not be zero"));
        }
        return Ok((
            width,
            height,
            unsigned_vec(metadata, DngTag::TileOffsets)?,
            unsigned_vec(metadata, DngTag::TileByteCounts)?,
        ));
    }

    let width = usize::try_from(required_u32(metadata, DngTag::ImageWidth)?)
        .expect("u32 fits usize on supported platforms");
    let height = usize::try_from(required_u32(metadata, DngTag::RowsPerStrip)?)
        .expect("u32 fits usize on supported platforms");
    if width == 0 || height == 0 {
        return Err(corrupt("DNG strip dimensions must not be zero"));
    }
    Ok((
        width,
        height,
        unsigned_vec(metadata, DngTag::StripOffsets)?,
        unsigned_vec(metadata, DngTag::StripByteCounts)?,
    ))
}

fn to_mosaic(width: usize, height: usize, samples: Vec<u16>) -> Result<RawMosaic, DecodeError> {
    MonoImage::from_raw(
        u32::try_from(width).expect("source width came from u32"),
        u32::try_from(height).expect("source height came from u32"),
        samples,
    )
    .map(RawMosaic::U16)
    .ok_or_else(|| corrupt("decoded DNG dimensions are invalid"))
}

fn sample_format(metadata: &DngMetadata) -> u64 {
    unsigned_scalar(metadata.get(&DngTag::Unknown(339))).unwrap_or(1)
}

fn checked_slice(source: &[u8], offset: usize, length: usize) -> Result<&[u8], DecodeError> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| corrupt("DNG data offset overflow"))?;
    source
        .get(offset..end)
        .ok_or_else(|| corrupt("DNG data offset or byte count is out of bounds"))
}

fn required_u16(metadata: &DngMetadata, tag: DngTag) -> Result<u16, DecodeError> {
    let value = unsigned_scalar(metadata.get(&tag))
        .ok_or_else(|| corrupt(format!("{tag} is missing or invalid")))?;
    u16::try_from(value).map_err(|_| corrupt(format!("{tag} is out of range")))
}

fn required_u32(metadata: &DngMetadata, tag: DngTag) -> Result<u32, DecodeError> {
    let value = unsigned_scalar(metadata.get(&tag))
        .ok_or_else(|| corrupt(format!("{tag} is missing or invalid")))?;
    u32::try_from(value).map_err(|_| corrupt(format!("{tag} is out of range")))
}

fn unsigned_scalar(value: Option<&Value>) -> Option<u64> {
    match value? {
        Value::Byte(value) => Some(u64::from(*value)),
        Value::Short(value) => Some(u64::from(*value)),
        Value::Unsigned(value) | Value::Ifd(value) => Some(u64::from(*value)),
        Value::UnsignedBig(value) | Value::IfdBig(value) => Some(*value),
        Value::List(values) => unsigned_scalar(Some(values.first()?)),
        _ => None,
    }
}

fn unsigned_vec(metadata: &DngMetadata, tag: DngTag) -> Result<Vec<usize>, DecodeError> {
    let value = metadata
        .get(&tag)
        .ok_or_else(|| corrupt(format!("{tag} is missing")))?;
    let values = value
        .clone()
        .into_u64_vec()
        .map_err(|_| corrupt(format!("{tag} has an invalid value type")))?;
    values
        .into_iter()
        .map(|value| {
            usize::try_from(value).map_err(|_| {
                corrupt(format!(
                    "{tag} contains a value too large for this platform"
                ))
            })
        })
        .collect()
}

fn unsupported_variant(detail: impl Into<Box<str>>) -> DecodeError {
    DecodeError::Unsupported(detail.into())
}

fn corrupt(detail: impl Into<Box<str>>) -> DecodeError { DecodeError::InvalidInput(detail.into()) }
