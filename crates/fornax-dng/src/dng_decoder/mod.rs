//! DNG container and sample decoding.

mod deflate_codec;
mod jpeg_codec;
mod jpegxl_codec;
mod ljpeg;
mod process_metadata;
mod transforms;

use fornax_core::error::DecodeError;
use fornax_core::metadata::ProcessMetadata;
use fornax_core::{CroppedMosaic, MonoImage, Primitive, RawMosaic};
use fornax_tiff::{ParsedTiff, Tag, Value};
use generic_num::num;
use image::math::Rect;
use process_metadata::process_metadata_from_dng_metadata;
use rayon::prelude::*;

use crate::metadata::{DngMetadata, DngTag};

const PHOTOMETRIC_CFA: u64 = 32_803;
const PHOTOMETRIC_LINEAR_RAW: u64 = 34_892;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompressionMethod {
    Uncompressed,
    LosslessJpeg,
    Deflate,
    LossyJpeg,
    JpegXl,
    Zstd,
    Unknown(u16),
}

impl CompressionMethod {
    pub const fn from_u16(value: u16) -> Self {
        match value {
            1 => Self::Uncompressed,
            7 => Self::LosslessJpeg,
            8 => Self::Deflate,
            34_892 => Self::LossyJpeg,
            52_546 => Self::JpegXl,
            50_000 => Self::Zstd,
            value => Self::Unknown(value),
        }
    }

    pub const fn as_u16(self) -> u16 {
        match self {
            Self::Uncompressed => 1,
            Self::LosslessJpeg => 7,
            Self::Deflate => 8,
            Self::LossyJpeg => 34_892,
            Self::JpegXl => 52_546,
            Self::Zstd => 50_000,
            Self::Unknown(value) => value,
        }
    }
}

#[derive(Clone, Debug)]
pub struct DngImage {
    parsed: ParsedTiff,
    raw_ifd: usize,
}

impl DngImage {
    pub fn parse(source: &[u8]) -> Result<Self, DecodeError> {
        let parsed = fornax_tiff::parse(source)
            .map_err(|error| DecodeError::InvalidInput(error.to_string().into()))?;
        let raw_ifd = select_raw_ifd(&parsed)?;
        Ok(Self { parsed, raw_ifd })
    }
}

fn select_raw_ifd(parsed: &ParsedTiff) -> Result<usize, DecodeError> {
    parsed
        .directories
        .iter()
        .enumerate()
        .filter(|(_, directory)| is_raw_ifd(directory))
        .max_by_key(|(_, directory)| raw_area(directory))
        .map(|(index, _)| index)
        .ok_or_else(|| unsupported_variant("no full-resolution CFA image IFD was found"))
}

fn is_raw_ifd(directory: &fornax_tiff::Directory) -> bool {
    let photometric = dng_value(directory.get(&Tag(u16::from(DngTag::PhotometricInterpretation))));
    let reduced_resolution = dng_value(directory.get(&Tag(u16::from(DngTag::NewSubfileType))))
        .is_some_and(|value| value & 1 != 0);
    !reduced_resolution
        && photometric
            .is_some_and(|value| matches!(value, PHOTOMETRIC_CFA | PHOTOMETRIC_LINEAR_RAW))
}

fn raw_area(directory: &fornax_tiff::Directory) -> u64 {
    let width = dng_value(directory.get(&Tag(u16::from(DngTag::ImageWidth)))).unwrap_or(0);
    let height = dng_value(directory.get(&Tag(u16::from(DngTag::ImageLength)))).unwrap_or(0);
    width.saturating_mul(height)
}

fn dng_value(value: Option<&Value>) -> Option<u64> {
    match value? {
        Value::Byte(value) => Some(u64::from(*value)),
        Value::Short(value) => Some(u64::from(*value)),
        Value::Unsigned(value) | Value::Ifd(value) => Some(u64::from(*value)),
        Value::UnsignedBig(value) | Value::IfdBig(value) => Some(*value),
        Value::List(values) => dng_value(values.first()),
        _ => None,
    }
}

pub fn raw_mosaic(parsed: &DngImage) -> Result<RawMosaic, DecodeError> {
    let metadata = dng_metadata(parsed)?;
    let pixels = transforms::apply(decode_samples(&parsed.parsed.source, &metadata)?, &metadata)?;
    validate_dimensions(&pixels, &metadata)?;
    Ok(pixels)
}

pub fn cropped_mosaic<T>(parsed: &DngImage) -> Result<CroppedMosaic<T>, DecodeError>
where
    T: Primitive,
{
    let pixels = raw_mosaic(parsed)?;
    let metadata = dng_metadata(parsed)?;
    let (width, height) = pixels.dimensions();
    let process_metadata = process_metadata_from_dng_metadata::<T>(width, height, &metadata)?;
    let active_area = process_metadata.active_area;
    let image = match pixels {
        RawMosaic::U8(image) => crop_and_cast(image, active_area)?,
        RawMosaic::U16(image) => crop_and_cast(image, active_area)?,
        RawMosaic::U32(image) => crop_and_cast(image, active_area)?,
        RawMosaic::F32(image) => crop_and_cast(image, active_area)?,
    };
    Ok(CroppedMosaic::new(image))
}

pub fn dng_metadata(parsed: &DngImage) -> Result<DngMetadata, DecodeError> {
    if !parsed
        .parsed
        .root
        .contains_key(&Tag(u16::from(DngTag::DngVersion)))
    {
        return Err(unsupported_variant(
            "TIFF container does not declare a DNGVersion",
        ));
    }
    let raw_metadata = parsed
        .parsed
        .directories
        .get(parsed.raw_ifd)
        .ok_or_else(|| corrupt("selected raw IFD is missing"))?;
    let mut dng_metadata = DngMetadata::default();
    dng_metadata.extend(
        parsed
            .parsed
            .root
            .iter()
            .map(|(tag, value)| (DngTag::from(tag.0), value.clone())),
    );
    dng_metadata.extend(
        raw_metadata
            .iter()
            .map(|(tag, value)| (DngTag::from(tag.0), value.clone())),
    );
    Ok(dng_metadata)
}

pub fn process_metadata<T>(parsed: &DngImage) -> Result<ProcessMetadata<T>, DecodeError>
where
    T: Primitive,
{
    let metadata = dng_metadata(parsed)?;
    let width = required_u32(&metadata, DngTag::ImageWidth)?;
    let height = required_u32(&metadata, DngTag::ImageLength)?;
    process_metadata_from_dng_metadata(width, height, &metadata)
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

fn decode_samples(source: &[u8], metadata: &DngMetadata) -> Result<RawMosaic, DecodeError> {
    let width = required_u32(metadata, DngTag::ImageWidth)?;
    let height = required_u32(metadata, DngTag::ImageLength)?;
    if width == 0 || height == 0 {
        return Err(corrupt("DNG image dimensions must not be zero"));
    }
    let compression = CompressionMethod::from_u16(required_u16(metadata, DngTag::Compression)?);
    let samples_per_pixel = unsigned_scalar(metadata.get(&DngTag::SamplesPerPixel)).unwrap_or(1);
    if samples_per_pixel != 1 {
        return Err(unsupported_variant(format!(
            "only one raw sample per pixel is supported, found {samples_per_pixel}"
        )));
    }

    match compression {
        CompressionMethod::Uncompressed => decode_uncompressed(source, metadata, width, height),
        CompressionMethod::LosslessJpeg => decode_lossless_jpeg(source, metadata, width, height),
        CompressionMethod::Deflate => decode_deflate(source, metadata, width, height),
        CompressionMethod::LossyJpeg => decode_lossy_jpeg(source, metadata, width, height),
        CompressionMethod::JpegXl => decode_jpeg_xl(source, metadata, width, height),
        CompressionMethod::Zstd => Err(unsupported_variant(
            "Zstandard compression requires the fornax-dng zstd feature",
        )),
        CompressionMethod::Unknown(value) => {
            Err(corrupt(format!("invalid DNG compression {value}")))
        }
    }
}

fn decode_deflate(
    source: &[u8],
    metadata: &DngMetadata,
    width: u32,
    height: u32,
) -> Result<RawMosaic, DecodeError> {
    let bits = required_u16(metadata, DngTag::BitsPerSample)?;
    let (tile_width, tile_height, offsets, byte_counts) = layout(metadata)?;
    let little_endian = matches!(byte_order(source), ByteOrder::Little);
    let predictor = unsigned_scalar(metadata.get(&DngTag::Unknown(317))).unwrap_or(1);
    let predictor =
        u16::try_from(predictor).map_err(|_| corrupt("Deflate predictor is out of range"))?;
    let mut inflated = Vec::new();
    inflated.extend_from_slice(if little_endian { b"II" } else { b"MM" });
    let mut inflated_offsets = Vec::with_capacity(offsets.len());
    let mut inflated_counts = Vec::with_capacity(byte_counts.len());
    for (index, (&offset, &byte_count)) in offsets.iter().zip(&byte_counts).enumerate() {
        let encoded = checked_slice(source, offset, byte_count)?;
        let mut payload = deflate_codec::inflate(encoded)
            .map_err(|detail| corrupt(format!("Deflate chunk {index}: {detail}")))?;
        deflate_codec::reverse_predictor(
            &mut payload,
            predictor,
            tile_width,
            tile_height,
            bits,
            sample_format(metadata),
            little_endian,
        )
        .map_err(|detail| corrupt(format!("Deflate chunk {index}: {detail}")))?;
        inflated_offsets.push(
            u32::try_from(inflated.len())
                .map_err(|_| corrupt("inflated Deflate source exceeds classic offsets"))?,
        );
        inflated_counts.push(
            u32::try_from(payload.len())
                .map_err(|_| corrupt("inflated Deflate chunk exceeds u32"))?,
        );
        inflated.extend_from_slice(&payload);
    }

    let mut metadata = metadata.clone();
    if metadata.contains_key(&DngTag::TileOffsets) {
        metadata.insert(DngTag::TileOffsets, dng_values(inflated_offsets));
        metadata.insert(DngTag::TileByteCounts, dng_values(inflated_counts));
    } else {
        metadata.insert(DngTag::StripOffsets, dng_values(inflated_offsets));
        metadata.insert(DngTag::StripByteCounts, dng_values(inflated_counts));
    }
    decode_uncompressed(&inflated, &metadata, width, height)
}

fn decode_lossy_jpeg(
    source: &[u8],
    metadata: &DngMetadata,
    width: u32,
    height: u32,
) -> Result<RawMosaic, DecodeError> {
    let bits = required_u16(metadata, DngTag::BitsPerSample)?;
    if bits != 8 {
        return Err(corrupt(format!(
            "lossy JPEG DNG samples require 8 bits, found {bits}"
        )));
    }
    let (tile_width, tile_height, offsets, byte_counts) = layout(metadata)?;
    let width = usize::try_from(width).expect("u32 fits usize on supported platforms");
    let height = usize::try_from(height).expect("u32 fits usize on supported platforms");
    validate_chunk_count(
        width,
        height,
        tile_width,
        tile_height,
        &offsets,
        &byte_counts,
    )?;
    let mut output = vec![0_u8; width * height];
    output
        .par_chunks_mut(tile_height * width)
        .enumerate()
        .try_for_each(|(tile_y_index, band)| {
            let tile_y = tile_y_index * tile_height;
            let copy_height = tile_height.min(height - tile_y);
            for tile_x_index in 0..width.div_ceil(tile_width) {
                let index = tile_y_index * width.div_ceil(tile_width) + tile_x_index;
                let encoded = checked_slice(source, offsets[index], byte_counts[index])?;
                let decoded = jpeg_codec::decode_luma(encoded)
                    .map_err(|detail| corrupt(format!("lossy JPEG tile {index}: {detail}")))?;
                let expected = tile_width
                    .checked_mul(tile_height)
                    .ok_or_else(|| corrupt("lossy JPEG tile size overflow"))?;
                if decoded.len() != expected {
                    return Err(corrupt(format!(
                        "lossy JPEG tile {index} decoded {} samples, expected {expected}",
                        decoded.len()
                    )));
                }
                let tile_x = tile_x_index * tile_width;
                let copy_width = tile_width.min(width - tile_x);
                for row in 0..copy_height {
                    let source_start = row * tile_width;
                    let target_start = row * width + tile_x;
                    band[target_start..target_start + copy_width]
                        .copy_from_slice(&decoded[source_start..source_start + copy_width]);
                }
            }
            Ok::<(), DecodeError>(())
        })?;
    MonoImage::from_raw(width as u32, height as u32, output)
        .map(RawMosaic::U8)
        .ok_or_else(|| corrupt("lossy JPEG DNG dimensions are invalid"))
}

fn decode_jpeg_xl(
    source: &[u8],
    metadata: &DngMetadata,
    width: u32,
    height: u32,
) -> Result<RawMosaic, DecodeError> {
    let bits = required_u16(metadata, DngTag::BitsPerSample)?;
    if !(8..=16).contains(&bits) {
        return Err(corrupt(format!(
            "JPEG XL Bayer samples with {bits} bits are invalid"
        )));
    }
    let (tile_width, tile_height, offsets, byte_counts) = layout(metadata)?;
    let width = usize::try_from(width).expect("u32 fits usize on supported platforms");
    let height = usize::try_from(height).expect("u32 fits usize on supported platforms");
    validate_chunk_count(
        width,
        height,
        tile_width,
        tile_height,
        &offsets,
        &byte_counts,
    )?;
    let across = width.div_ceil(tile_width);

    if bits == 8 {
        let mut output = vec![0_u8; width * height];
        output
            .par_chunks_mut(tile_height * width)
            .enumerate()
            .try_for_each(|(tile_y_index, band)| {
                let tile_y = tile_y_index * tile_height;
                let copy_height = tile_height.min(height - tile_y);
                for tile_x_index in 0..across {
                    let index = tile_y_index * across + tile_x_index;
                    let encoded = checked_slice(source, offsets[index], byte_counts[index])?;
                    let decoded = jpegxl_codec::decode_u8(encoded, tile_width, tile_height)
                        .map_err(|detail| corrupt(format!("JPEG XL tile {index}: {detail}")))?;
                    let tile_x = tile_x_index * tile_width;
                    let copy_width = tile_width.min(width - tile_x);
                    for row in 0..copy_height {
                        let source_start = row * tile_width;
                        let target_start = row * width + tile_x;
                        band[target_start..target_start + copy_width]
                            .copy_from_slice(&decoded[source_start..source_start + copy_width]);
                    }
                }
                Ok::<(), DecodeError>(())
            })?;
        MonoImage::from_raw(width as u32, height as u32, output)
            .map(RawMosaic::U8)
            .ok_or_else(|| corrupt("JPEG XL DNG dimensions are invalid"))
    } else {
        let mut output = vec![0_u16; width * height];
        output
            .par_chunks_mut(tile_height * width)
            .enumerate()
            .try_for_each(|(tile_y_index, band)| {
                let tile_y = tile_y_index * tile_height;
                let copy_height = tile_height.min(height - tile_y);
                for tile_x_index in 0..across {
                    let index = tile_y_index * across + tile_x_index;
                    let encoded = checked_slice(source, offsets[index], byte_counts[index])?;
                    let decoded = jpegxl_codec::decode_u16(encoded, tile_width, tile_height, bits)
                        .map_err(|detail| corrupt(format!("JPEG XL tile {index}: {detail}")))?;
                    let tile_x = tile_x_index * tile_width;
                    let copy_width = tile_width.min(width - tile_x);
                    for row in 0..copy_height {
                        let source_start = row * tile_width;
                        let target_start = row * width + tile_x;
                        band[target_start..target_start + copy_width]
                            .copy_from_slice(&decoded[source_start..source_start + copy_width]);
                    }
                }
                Ok::<(), DecodeError>(())
            })?;
        to_mosaic(width, height, output)
    }
}

fn decode_lossless_jpeg(
    source: &[u8],
    metadata: &DngMetadata,
    width: u32,
    height: u32,
) -> Result<RawMosaic, DecodeError> {
    let bits = required_u16(metadata, DngTag::BitsPerSample)?;
    if !(2..=16).contains(&bits) {
        return Err(corrupt(format!(
            "lossless JPEG CFA sample depth {bits} is invalid"
        )));
    }
    let precision = u8::try_from(bits).expect("lossless JPEG precision is at most 16");

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

    let expected_samples = tile_width
        .checked_mul(tile_height)
        .ok_or_else(|| corrupt("lossless JPEG tile sample count overflow"))?;
    output
        .par_chunks_mut(tile_height * width)
        .enumerate()
        .try_for_each(|(tile_y_index, band)| {
            let mut decoded = vec![0_u16; expected_samples];
            for tile_x_index in 0..across {
                let index = tile_y_index * across + tile_x_index;
                let encoded = checked_slice(source, offsets[index], byte_counts[index])?;
                ljpeg::decode_into(encoded, tile_width, tile_height, precision, &mut decoded)
                    .map_err(|detail| corrupt(format!("lossless JPEG tile {index}: {detail}")))?;

                let tile_x = tile_x_index * tile_width;
                let tile_y = tile_y_index * tile_height;
                let copy_width = tile_width.min(width - tile_x);
                let copy_height = tile_height.min(height - tile_y);
                for row in 0..copy_height {
                    let source_start = row * tile_width;
                    let target_start = row * width + tile_x;
                    band[target_start..target_start + copy_width]
                        .copy_from_slice(&decoded[source_start..source_start + copy_width]);
                }
            }
            Ok::<(), DecodeError>(())
        })?;

    if bits <= 8 {
        let samples = output
            .into_iter()
            .map(|value| u8::try_from(value).expect("lossless JPEG samples fit eight bits"))
            .collect();
        MonoImage::from_raw(width as u32, height as u32, samples)
            .map(RawMosaic::U8)
            .ok_or_else(|| corrupt("lossless JPEG DNG dimensions are invalid"))
    } else {
        to_mosaic(width, height, output)
    }
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

    let sample_format = sample_format(metadata);
    if sample_format == 3 {
        let samples = decode_float_samples(
            source,
            &offsets,
            &byte_counts,
            width,
            height,
            tile_width,
            tile_height,
            bits,
            byte_order(source),
        )?;
        return MonoImage::from_raw(width as u32, height as u32, samples)
            .map(RawMosaic::F32)
            .ok_or_else(|| corrupt("uncompressed DNG dimensions are invalid"));
    }
    if sample_format != 1 {
        return Err(corrupt(format!(
            "invalid uncompressed DNG sample format {sample_format}"
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
        9..=15 => {
            let samples = decode_packed_samples(
                source,
                &offsets,
                &byte_counts,
                width,
                height,
                tile_width,
                tile_height,
                bits,
            )?;
            to_mosaic(width, height, samples)
        }
        1..=7 => {
            let samples = decode_packed_samples(
                source,
                &offsets,
                &byte_counts,
                width,
                height,
                tile_width,
                tile_height,
                bits,
            )?;
            let samples = samples
                .into_iter()
                .map(|value| u8::try_from(value).expect("packed values fit their declared depth"))
                .collect();
            MonoImage::from_raw(width as u32, height as u32, samples)
                .map(RawMosaic::U8)
                .ok_or_else(|| corrupt("uncompressed DNG dimensions are invalid"))
        }
        17..=31 => {
            let samples = decode_packed_u32_samples(
                source,
                &offsets,
                &byte_counts,
                width,
                height,
                tile_width,
                tile_height,
                bits,
            )?;
            MonoImage::from_raw(width as u32, height as u32, samples)
                .map(RawMosaic::U32)
                .ok_or_else(|| corrupt("uncompressed DNG dimensions are invalid"))
        }
        _ => Err(corrupt(format!(
            "invalid uncompressed DNG sample depth {bits}"
        ))),
    }
}

fn decode_float_samples(
    source: &[u8],
    offsets: &[usize],
    byte_counts: &[usize],
    width: usize,
    height: usize,
    tile_width: usize,
    tile_height: usize,
    bits: u16,
    byte_order: ByteOrder,
) -> Result<Vec<f32>, DecodeError> {
    let bytes_per_sample = match bits {
        16 => 2,
        24 => 3,
        32 => 4,
        _ => {
            return Err(corrupt(format!(
                "IEEE floating-point DNG samples with {bits} bits are invalid"
            )));
        }
    };
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

    let mut output = vec![0_f32; width * height];
    output
        .par_chunks_mut(tile_height * width)
        .enumerate()
        .try_for_each(|(tile_y_index, band)| {
            let tile_y = tile_y_index * tile_height;
            let copy_height = tile_height.min(height - tile_y);
            for tile_x_index in 0..across {
                let index = tile_y_index * across + tile_x_index;
                let encoded = checked_slice(source, offsets[index], byte_counts[index])?;
                let tile_x = tile_x_index * tile_width;
                let copy_width = tile_width.min(width - tile_x);
                let required_samples = (copy_height - 1)
                    .checked_mul(tile_width)
                    .and_then(|count| count.checked_add(copy_width))
                    .ok_or_else(|| corrupt("floating-point DNG chunk size overflow"))?;
                let required_bytes = required_samples
                    .checked_mul(bytes_per_sample)
                    .ok_or_else(|| corrupt("floating-point DNG chunk size overflow"))?;
                if encoded.len() < required_bytes {
                    return Err(corrupt(format!(
                        "uncompressed DNG chunk {index} is truncated"
                    )));
                }

                for row in 0..copy_height {
                    let source_start = row * tile_width * bytes_per_sample;
                    let target_start = row * width + tile_x;
                    for (sample_index, target) in band[target_start..target_start + copy_width]
                        .iter_mut()
                        .enumerate()
                    {
                        let offset = source_start + sample_index * bytes_per_sample;
                        let bytes = &encoded[offset..offset + bytes_per_sample];
                        *target = match bits {
                            16 => {
                                let raw = match byte_order {
                                    ByteOrder::Little => u16::from_le_bytes([bytes[0], bytes[1]]),
                                    ByteOrder::Big => u16::from_be_bytes([bytes[0], bytes[1]]),
                                };
                                extend_float(u32::from(raw), 10, 5)
                            }
                            24 => {
                                let raw = match byte_order {
                                    ByteOrder::Little => {
                                        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], 0])
                                    }
                                    ByteOrder::Big => {
                                        u32::from_be_bytes([0, bytes[0], bytes[1], bytes[2]])
                                    }
                                };
                                extend_float(raw, 16, 7)
                            }
                            32 => {
                                let raw = match byte_order {
                                    ByteOrder::Little => {
                                        u32::from_le_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                                    }
                                    ByteOrder::Big => {
                                        u32::from_be_bytes([bytes[0], bytes[1], bytes[2], bytes[3]])
                                    }
                                };
                                f32::from_bits(raw)
                            }
                            _ => unreachable!("the sample width is validated above"),
                        };
                    }
                }
            }
            Ok::<(), DecodeError>(())
        })?;
    Ok(output)
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
            u32::try_from(i32::try_from(exponent).unwrap() - bias + 127)
                .expect("widened exponent fits u32"),
            fraction << (23 - fraction_bits),
        )
    };
    f32::from_bits((sign << 31) | (wide_exponent << 23) | wide_fraction)
}

fn decode_packed_samples(
    source: &[u8],
    offsets: &[usize],
    byte_counts: &[usize],
    width: usize,
    height: usize,
    tile_width: usize,
    tile_height: usize,
    bits: u16,
) -> Result<Vec<u16>, DecodeError> {
    let values = decode_packed_u32_samples(
        source,
        offsets,
        byte_counts,
        width,
        height,
        tile_width,
        tile_height,
        bits,
    )?;
    values
        .into_iter()
        .map(|value| {
            u16::try_from(value)
                .map_err(|_| corrupt(format!("packed sample exceeds {bits}-bit storage")))
        })
        .collect()
}

fn decode_packed_u32_samples(
    source: &[u8],
    offsets: &[usize],
    byte_counts: &[usize],
    width: usize,
    height: usize,
    tile_width: usize,
    tile_height: usize,
    bits: u16,
) -> Result<Vec<u32>, DecodeError> {
    let bits = usize::from(bits);
    if !(1..=32).contains(&bits) {
        return Err(corrupt("packed sample depth must be between 1 and 32"));
    }
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
    let mut output = vec![0_u32; width * height];
    output
        .par_chunks_mut(tile_height * width)
        .enumerate()
        .try_for_each(|(tile_y_index, band)| {
            let tile_y = tile_y_index * tile_height;
            let copy_height = tile_height.min(height - tile_y);
            let mut unpacked = vec![0_u32; tile_width * tile_height];
            for tile_x_index in 0..across {
                let index = tile_y_index * across + tile_x_index;
                let encoded = checked_slice(source, offsets[index], byte_counts[index])?;
                let needed = (tile_width * tile_height * bits).div_ceil(8);
                if encoded.len() < needed {
                    return Err(corrupt(format!(
                        "uncompressed DNG chunk {index} is truncated"
                    )));
                }
                unpack_bits(encoded, &mut unpacked, bits)?;

                let tile_x = tile_x_index * tile_width;
                let copy_width = tile_width.min(width - tile_x);
                for row in 0..copy_height {
                    let source_start = row * tile_width;
                    let target_start = row * width + tile_x;
                    for (target, value) in band[target_start..target_start + copy_width]
                        .iter_mut()
                        .zip(&unpacked[source_start..source_start + copy_width])
                    {
                        *target = *value;
                    }
                }
            }
            Ok(())
        })?;
    Ok(output)
}

fn unpack_bits(source: &[u8], output: &mut [u32], bits: usize) -> Result<(), DecodeError> {
    let needed = output
        .len()
        .checked_mul(bits)
        .ok_or_else(|| corrupt("packed sample bit count overflow"))?
        .div_ceil(8);
    if source.len() < needed {
        return Err(corrupt("packed sample payload is truncated"));
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
            .map_err(|_| corrupt("packed sample exceeds u32"))?;
        buffer &= (1_u64 << buffered_bits) - 1;
    }
    Ok(())
}

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

fn validate_chunk_count(
    width: usize,
    height: usize,
    tile_width: usize,
    tile_height: usize,
    offsets: &[usize],
    byte_counts: &[usize],
) -> Result<(), DecodeError> {
    let expected = width.div_ceil(tile_width) * height.div_ceil(tile_height);
    if offsets.len() != byte_counts.len() || offsets.len() != expected {
        return Err(corrupt(format!(
            "expected {expected} chunks, found {} offsets and {} byte counts",
            offsets.len(),
            byte_counts.len()
        )));
    }
    Ok(())
}

fn dng_values(values: Vec<u32>) -> Value {
    if values.len() == 1 {
        Value::Unsigned(values[0])
    } else {
        Value::List(values.into_iter().map(Value::Unsigned).collect())
    }
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

#[cfg(test)]
mod tests {
    use std::io::{Cursor, Write};

    use flate2::Compression;
    use flate2::write::ZlibEncoder;
    use fornax_core::{MonoImage, RawMosaic};
    use fornax_tiff::{Tag, TiffWriter, Value};
    use image::ImageEncoder;

    use super::{DngImage, decode_uncompressed, process_metadata};

    #[test]
    fn compression_methods_preserve_known_and_unknown_identifiers() {
        assert_eq!(
            super::CompressionMethod::from_u16(7),
            super::CompressionMethod::LosslessJpeg
        );
        assert_eq!(super::CompressionMethod::from_u16(7).as_u16(), 7);
        assert_eq!(
            super::CompressionMethod::from_u16(12_345),
            super::CompressionMethod::Unknown(12_345)
        );
        assert_eq!(super::CompressionMethod::from_u16(12_345).as_u16(), 12_345);
    }
    use crate::metadata::{DngMetadata, DngTag};
    use crate::writer::{DngCompression, DngWriter};

    #[test]
    fn process_metadata_does_not_decode_invalid_pixel_payload() {
        let mut cursor = Cursor::new(Vec::new());
        {
            let mut writer = TiffWriter::new(&mut cursor).unwrap();
            let data_offset = writer.write_data(b"invalid lossless JPEG").unwrap();
            let data_offset =
                u32::try_from(data_offset).expect("the synthetic TIFF fits classic offsets");
            let byte_count =
                u32::try_from(b"invalid lossless JPEG".len()).expect("the tile size fits u32");
            writer
                .finish([
                    (
                        Tag(50_706),
                        Value::List(vec![
                            Value::Byte(1),
                            Value::Byte(7),
                            Value::Byte(1),
                            Value::Byte(0),
                        ]),
                    ),
                    (Tag(254), Value::Unsigned(0)),
                    (Tag(256), Value::Unsigned(4)),
                    (Tag(257), Value::Unsigned(4)),
                    (Tag(258), Value::Short(16)),
                    (Tag(259), Value::Short(7)),
                    (Tag(262), Value::Short(32_803)),
                    (Tag(277), Value::Short(1)),
                    (Tag(284), Value::Short(1)),
                    (Tag(322), Value::Unsigned(4)),
                    (Tag(323), Value::Unsigned(4)),
                    (Tag(324), Value::Unsigned(data_offset)),
                    (Tag(325), Value::Unsigned(byte_count)),
                    (
                        Tag(50_829),
                        Value::List(vec![
                            Value::Unsigned(0),
                            Value::Unsigned(0),
                            Value::Unsigned(4),
                            Value::Unsigned(4),
                        ]),
                    ),
                    (
                        Tag(50_719),
                        Value::List(vec![Value::Unsigned(0), Value::Unsigned(0)]),
                    ),
                    (
                        Tag(50_720),
                        Value::List(vec![Value::Unsigned(4), Value::Unsigned(4)]),
                    ),
                    (
                        Tag(50_713),
                        Value::List(vec![Value::Short(2), Value::Short(2)]),
                    ),
                    (
                        Tag(50_714),
                        Value::List(vec![
                            Value::Short(0),
                            Value::Short(0),
                            Value::Short(0),
                            Value::Short(0),
                        ]),
                    ),
                    (Tag(50_717), Value::Unsigned(1)),
                    (
                        Tag(33_421),
                        Value::List(vec![Value::Short(2), Value::Short(2)]),
                    ),
                    (
                        Tag(33_422),
                        Value::List(vec![
                            Value::Byte(0),
                            Value::Byte(1),
                            Value::Byte(1),
                            Value::Byte(2),
                        ]),
                    ),
                    (Tag(50_711), Value::Short(1)),
                ])
                .unwrap();
        };

        let parsed = DngImage::parse(cursor.get_ref()).expect("the synthetic TIFF should parse");
        let metadata = process_metadata::<f32>(&parsed)
            .expect("metadata access must not depend on the pixel payload");
        assert_eq!(metadata.active_area.width, 4);
        assert_eq!(metadata.active_area.height, 4);
    }

    #[test]
    fn uncompressed_ten_bit_tile_is_unpacked_msb_first() {
        let metadata = metadata(4, 1, 10, 5, [(DngTag::TileWidth, Value::Unsigned(4))]);
        let source = [0_u8, 0, 0x10, 0x08, 0x03];
        let mosaic = decode_uncompressed(&source, &metadata, 4, 1).unwrap();
        let RawMosaic::U16(image) = mosaic else {
            panic!("ten-bit samples must widen to u16");
        };
        assert_eq!(image.as_raw().as_slice(), &[0, 1, 2, 3]);
    }

    #[test]
    fn uncompressed_big_endian_strips_copy_the_final_partial_row() {
        let mut metadata = metadata(2, 3, 16, 12, []);
        metadata.remove(&DngTag::TileOffsets);
        metadata.remove(&DngTag::TileByteCounts);
        metadata.remove(&DngTag::TileLength);
        metadata.remove(&DngTag::TileWidth);
        metadata.insert(DngTag::ImageLength, Value::Unsigned(3));
        metadata.insert(DngTag::RowsPerStrip, Value::Unsigned(2));
        metadata.insert(
            DngTag::StripOffsets,
            Value::List(vec![Value::Unsigned(2), Value::Unsigned(10)]),
        );
        metadata.insert(
            DngTag::StripByteCounts,
            Value::List(vec![Value::Unsigned(8), Value::Unsigned(4)]),
        );
        let source = [b'M', b'M', 0, 1, 0, 2, 0, 3, 0, 4, 0, 5, 0, 6];
        let mosaic = decode_uncompressed(&source, &metadata, 2, 3).unwrap();
        let RawMosaic::U16(image) = mosaic else {
            panic!("sixteen-bit samples must decode as u16");
        };
        assert_eq!(image.as_raw().as_slice(), &[1, 2, 3, 4, 5, 6]);
    }

    #[test]
    fn uncompressed_float16_tile_widens_to_f32() {
        let metadata = metadata(
            4,
            1,
            16,
            8,
            [
                (DngTag::TileWidth, Value::Unsigned(4)),
                (DngTag::Unknown(339), Value::Short(3)),
            ],
        );
        let source = [0x3c, 0x00, 0x40, 0x00, 0x38, 0x00, 0xbc, 0x00];
        let mosaic = decode_uncompressed(&source, &metadata, 4, 1).unwrap();
        let RawMosaic::F32(image) = mosaic else {
            panic!("IEEE half samples must decode as f32");
        };
        assert_eq!(image.as_raw().as_slice(), &[1.0, 2.0, 0.5, -1.0]);
    }

    #[test]
    fn lossless_jpeg_decodes_parallel_tile_rows() {
        let tile = [
            0xff, 0xd8, 0xff, 0xc3, 0x00, 0x0b, 0x10, 0x00, 0x02, 0x00, 0x02, 0x01, 0x01, 0x11,
            0x00, 0xff, 0xc4, 0x00, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xda, 0x00, 0x08, 0x01,
            0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0xff, 0xd9,
        ];
        let mut source = Vec::new();
        source.extend_from_slice(&tile);
        let second_offset = source.len();
        source.extend_from_slice(&tile);

        let mut metadata = metadata(
            2,
            4,
            16,
            0,
            [
                (DngTag::TileWidth, Value::Unsigned(2)),
                (DngTag::TileLength, Value::Unsigned(2)),
            ],
        );
        metadata.insert(
            DngTag::TileOffsets,
            Value::List(vec![
                Value::Unsigned(0),
                Value::Unsigned(u32::try_from(second_offset).unwrap()),
            ]),
        );
        metadata.insert(
            DngTag::TileByteCounts,
            Value::List(vec![
                Value::Unsigned(u32::try_from(tile.len()).unwrap()),
                Value::Unsigned(u32::try_from(tile.len()).unwrap()),
            ]),
        );

        let mosaic = super::decode_lossless_jpeg(&source, &metadata, 2, 4).unwrap();
        let RawMosaic::U16(image) = mosaic else {
            panic!("lossless JPEG samples must decode as u16");
        };
        assert_eq!(image.as_raw().as_slice(), &[32_768; 8]);
    }

    #[test]
    fn lossless_jpeg_decodes_twelve_bit_tiles() {
        let tile = [
            0xff, 0xd8, 0xff, 0xc3, 0x00, 0x0b, 0x0c, 0x00, 0x02, 0x00, 0x02, 0x01, 0x01, 0x11,
            0x00, 0xff, 0xc4, 0x00, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xda, 0x00, 0x08, 0x01,
            0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0xff, 0xd9,
        ];
        let metadata = metadata(
            2,
            2,
            16,
            u32::try_from(tile.len()).unwrap(),
            [
                (DngTag::TileWidth, Value::Unsigned(2)),
                (DngTag::TileLength, Value::Unsigned(2)),
            ],
        );
        let mosaic = super::decode_lossless_jpeg(&tile, &metadata, 2, 2).unwrap();
        let RawMosaic::U16(image) = mosaic else {
            panic!("lossless JPEG samples must decode as u16");
        };
        assert_eq!(image.as_raw().as_slice(), &[2_048; 4]);
    }

    #[test]
    fn lossless_jpeg_applies_point_transform() {
        let mut tile = [
            0xff, 0xd8, 0xff, 0xc3, 0x00, 0x0b, 0x10, 0x00, 0x02, 0x00, 0x02, 0x01, 0x01, 0x11,
            0x00, 0xff, 0xc4, 0x00, 0x14, 0x00, 0x01, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
            0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00, 0xff, 0xda, 0x00, 0x08, 0x01,
            0x01, 0x00, 0x01, 0x00, 0x00, 0x00, 0xff, 0xd9,
        ];
        tile[46] = 1;
        let metadata = metadata(
            2,
            2,
            16,
            u32::try_from(tile.len()).unwrap(),
            [
                (DngTag::TileWidth, Value::Unsigned(2)),
                (DngTag::TileLength, Value::Unsigned(2)),
            ],
        );
        let mosaic = super::decode_lossless_jpeg(&tile, &metadata, 2, 2).unwrap();
        let RawMosaic::U16(image) = mosaic else {
            panic!("lossless JPEG samples must decode as u16");
        };
        assert_eq!(image.as_raw().as_slice(), &[32_768; 4]);
    }

    #[test]
    fn deflate_predictor_restores_horizontal_integer_differences() {
        let values = [1_u8, 2, 4, 8];
        let differences = [1_u8, 1, 2, 4];
        let mut encoder = ZlibEncoder::new(Vec::new(), Compression::fast());
        encoder.write_all(&differences).unwrap();
        let compressed = encoder.finish().unwrap();
        let metadata = metadata(
            4,
            1,
            8,
            u32::try_from(compressed.len()).unwrap(),
            [
                (DngTag::TileWidth, Value::Unsigned(4)),
                (DngTag::Unknown(317), Value::Short(2)),
            ],
        );

        let mosaic = super::decode_deflate(&compressed, &metadata, 4, 1).unwrap();
        let RawMosaic::U8(image) = mosaic else {
            panic!("8-bit deflate samples must remain u8");
        };
        assert_eq!(image.as_raw().as_slice(), &values);
    }

    #[test]
    fn lossy_jpeg_decodes_a_single_component_tile() {
        let pixels = [
            0_u8, 32, 64, 96, 96, 64, 32, 0, 16, 48, 80, 112, 112, 80, 48, 16,
        ];
        let mut encoded = Vec::new();
        image::codecs::jpeg::JpegEncoder::new_with_quality(&mut encoded, 100)
            .write_image(&pixels, 4, 4, image::ExtendedColorType::L8)
            .unwrap();
        let metadata = metadata(
            4,
            4,
            8,
            u32::try_from(encoded.len()).unwrap(),
            [
                (DngTag::TileWidth, Value::Unsigned(4)),
                (DngTag::TileLength, Value::Unsigned(4)),
            ],
        );

        let mosaic = super::decode_lossy_jpeg(&encoded, &metadata, 4, 4).unwrap();
        let RawMosaic::U8(image) = mosaic else {
            panic!("lossy JPEG samples must decode as u8");
        };
        assert_eq!(image.dimensions(), (4, 4));
    }

    #[test]
    fn lossless_writer_round_trips_u16_samples() {
        let samples = vec![0, 65_535, 30_000, 32_768, 1, 2, 40_000, 12_345];
        let mosaic = RawMosaic::U16(MonoImage::from_raw(4, 2, samples.clone()).unwrap());
        let metadata = writable_metadata(4, 2);
        let mut output = Cursor::new(Vec::new());
        DngWriter::new(DngCompression::LosslessJpeg)
            .write_dng_with(&mut output, &mosaic, &metadata)
            .unwrap();

        let parsed = DngImage::parse(output.get_ref()).unwrap();
        let decoded = super::raw_mosaic(&parsed).unwrap();
        let RawMosaic::U16(image) = decoded else {
            panic!("written 16-bit samples must decode as u16");
        };
        assert_eq!(image.as_raw().as_slice(), samples.as_slice());
    }

    #[test]
    fn lossless_writer_round_trips_u8_samples() {
        let samples = vec![0, 255, 128, 1, 254, 64, 200, 7];
        let mosaic = RawMosaic::U8(MonoImage::from_raw(4, 2, samples.clone()).unwrap());
        let metadata = writable_metadata(4, 2);
        let mut output = Cursor::new(Vec::new());
        DngWriter::new(DngCompression::LosslessJpeg)
            .write_dng_with(&mut output, &mosaic, &metadata)
            .unwrap();

        let parsed = DngImage::parse(output.get_ref()).unwrap();
        let decoded = super::raw_mosaic(&parsed).unwrap();
        let RawMosaic::U8(image) = decoded else {
            panic!("written 8-bit samples must decode as u8");
        };
        assert_eq!(image.as_raw().as_slice(), samples.as_slice());
    }

    #[test]
    fn lossless_writer_omits_consumed_raw_transforms() {
        let samples = vec![100, 200, 300, 400];
        let mosaic = RawMosaic::U16(MonoImage::from_raw(4, 1, samples.clone()).unwrap());
        let mut metadata = writable_metadata(4, 1);
        metadata.insert(
            DngTag::LinearizationTable,
            Value::List(vec![
                Value::Short(100),
                Value::Short(200),
                Value::Short(300),
                Value::Short(400),
            ]),
        );
        metadata.insert(DngTag::Unknown(50_975), Value::Short(2));
        metadata.insert(DngTag::Unknown(50_976), Value::Short(2));

        let mut output = Cursor::new(Vec::new());
        DngWriter::new(DngCompression::LosslessJpeg)
            .write_dng_with(&mut output, &mosaic, &metadata)
            .unwrap();
        let parsed = DngImage::parse(output.get_ref()).unwrap();
        let generated = super::dng_metadata(&parsed).unwrap();
        assert!(!generated.contains_key(&DngTag::LinearizationTable));
        assert!(!generated.contains_key(&DngTag::Unknown(50_975)));
        assert!(!generated.contains_key(&DngTag::Unknown(50_976)));
        let RawMosaic::U16(image) = super::raw_mosaic(&parsed).unwrap() else {
            panic!("written samples must remain u16");
        };
        assert_eq!(image.as_raw().as_slice(), samples.as_slice());
    }

    #[test]
    fn lossless_writer_round_trips_wide_integer_and_float_samples() {
        let integers = vec![1_u32, 70_000, 4_000_000_000];
        let integer_mosaic = RawMosaic::U32(MonoImage::from_raw(3, 1, integers.clone()).unwrap());
        let mut integer_output = Cursor::new(Vec::new());
        DngWriter::new(DngCompression::LosslessJpeg)
            .write_dng_with(
                &mut integer_output,
                &integer_mosaic,
                &writable_metadata(3, 1),
            )
            .unwrap();
        let integer_parsed = DngImage::parse(integer_output.get_ref()).unwrap();
        let RawMosaic::U32(decoded_integers) = super::raw_mosaic(&integer_parsed).unwrap() else {
            panic!("wide integers must remain u32");
        };
        assert_eq!(decoded_integers.as_raw().as_slice(), integers.as_slice());

        let floats = vec![-1.5_f32, 0.25, f32::from_bits(0x7fc0_1234)];
        let float_mosaic = RawMosaic::F32(MonoImage::from_raw(3, 1, floats.clone()).unwrap());
        let mut float_output = Cursor::new(Vec::new());
        DngWriter::new(DngCompression::LosslessJpeg)
            .write_dng_with(&mut float_output, &float_mosaic, &writable_metadata(3, 1))
            .unwrap();
        let float_parsed = DngImage::parse(float_output.get_ref()).unwrap();
        let RawMosaic::F32(decoded_floats) = super::raw_mosaic(&float_parsed).unwrap() else {
            panic!("floating-point samples must remain f32");
        };
        assert_eq!(
            decoded_floats
                .as_raw()
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>(),
            floats
                .iter()
                .map(|value| value.to_bits())
                .collect::<Vec<_>>()
        );
    }

    #[test]
    fn linearization_table_replaces_stored_values() {
        let pixels = RawMosaic::U16(MonoImage::from_raw(4, 1, vec![0, 1, 2, 3]).unwrap());
        let mut metadata = DngMetadata::default();
        metadata.insert(DngTag::BitsPerSample, Value::Short(2));
        metadata.insert(
            DngTag::LinearizationTable,
            Value::List(vec![
                Value::Short(100),
                Value::Short(200),
                Value::Short(300),
                Value::Short(400),
            ]),
        );
        let transformed = super::transforms::apply(pixels, &metadata).unwrap();
        let RawMosaic::U16(image) = transformed else {
            panic!("linearization output must be u16");
        };
        assert_eq!(image.as_raw().as_slice(), &[75, 150, 316, 397]);
    }

    #[test]
    fn row_and_column_interleave_restore_two_by_two_storage() {
        let pixels = RawMosaic::U16(
            MonoImage::from_raw(
                4,
                4,
                vec![1, 3, 2, 4, 9, 11, 10, 12, 5, 7, 6, 8, 13, 15, 14, 16],
            )
            .unwrap(),
        );
        let mut metadata = DngMetadata::default();
        metadata.insert(DngTag::Unknown(50_975), Value::Short(2));
        metadata.insert(DngTag::Unknown(50_976), Value::Short(2));
        let transformed = super::transforms::apply(pixels, &metadata).unwrap();
        let RawMosaic::U16(image) = transformed else {
            panic!("interleave output must remain u16");
        };
        assert_eq!(
            image.as_raw().as_slice(),
            &[1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15, 16]
        );
    }

    #[test]
    fn opcode_list_one_maps_stored_sample_values() {
        let pixels = RawMosaic::U16(MonoImage::from_raw(4, 1, vec![0, 1, 2, 3]).unwrap());
        let mut metadata = DngMetadata::default();
        metadata.insert(
            DngTag::Unknown(51_008),
            Value::List(
                map_table_opcode_list(&[10, 20, 30, 40])
                    .into_iter()
                    .map(Value::Byte)
                    .collect(),
            ),
        );
        let transformed = super::transforms::apply(pixels, &metadata).unwrap();
        let RawMosaic::U16(image) = transformed else {
            panic!("opcode output must remain u16");
        };
        assert_eq!(image.as_raw().as_slice(), &[10, 20, 30, 40]);
    }

    #[test]
    fn supported_codecs_reject_truncated_payloads_without_panicking() {
        assert!(super::ljpeg::decode_into(&[0xff, 0xd8], 1, 1, 16, &mut [0]).is_err());
        assert!(super::deflate_codec::inflate(&[0x78]).is_err());
        assert!(super::jpeg_codec::decode_luma(&[0xff, 0xd8]).is_err());
        assert!(super::jpegxl_codec::decode_u16(&[0xff], 1, 1, 16).is_err());
    }

    fn metadata(
        width: u32,
        height: u32,
        bits: u16,
        byte_count: u32,
        extra: impl IntoIterator<Item = (DngTag, Value)>,
    ) -> DngMetadata {
        let mut metadata = DngMetadata::default();
        metadata.insert(DngTag::ImageWidth, Value::Unsigned(width));
        metadata.insert(DngTag::ImageLength, Value::Unsigned(height));
        metadata.insert(DngTag::BitsPerSample, Value::Short(bits));
        metadata.insert(DngTag::Compression, Value::Short(1));
        metadata.insert(DngTag::SamplesPerPixel, Value::Short(1));
        metadata.insert(DngTag::PhotometricInterpretation, Value::Short(32_803));
        metadata.insert(DngTag::TileLength, Value::Unsigned(height));
        metadata.insert(DngTag::TileOffsets, Value::Unsigned(0));
        metadata.insert(DngTag::TileByteCounts, Value::Unsigned(byte_count));
        for (tag, value) in extra {
            metadata.insert(tag, value);
        }
        metadata
    }

    fn writable_metadata(width: u32, height: u32) -> DngMetadata {
        let mut metadata = DngMetadata::default();
        metadata.insert(
            DngTag::ActiveArea,
            Value::List(vec![
                Value::Unsigned(0),
                Value::Unsigned(0),
                Value::Unsigned(height),
                Value::Unsigned(width),
            ]),
        );
        metadata.insert(
            DngTag::DefaultCropOrigin,
            Value::List(vec![Value::Unsigned(0), Value::Unsigned(0)]),
        );
        metadata.insert(
            DngTag::DefaultCropSize,
            Value::List(vec![Value::Unsigned(width), Value::Unsigned(height)]),
        );
        metadata.insert(
            DngTag::BlackLevelRepeatDim,
            Value::List(vec![Value::Short(2), Value::Short(2)]),
        );
        metadata.insert(
            DngTag::BlackLevel,
            Value::List(vec![
                Value::Short(0),
                Value::Short(0),
                Value::Short(0),
                Value::Short(0),
            ]),
        );
        metadata.insert(DngTag::WhiteLevel, Value::Unsigned(65_535));
        metadata.insert(
            DngTag::CfaRepeatPatternDim,
            Value::List(vec![Value::Short(2), Value::Short(2)]),
        );
        metadata.insert(
            DngTag::CfaPattern,
            Value::List(vec![
                Value::Byte(0),
                Value::Byte(1),
                Value::Byte(1),
                Value::Byte(2),
            ]),
        );
        metadata.insert(
            DngTag::CfaPlaneColor,
            Value::List(vec![Value::Byte(0), Value::Byte(1), Value::Byte(2)]),
        );
        metadata.insert(DngTag::CfaLayout, Value::Short(1));
        metadata
    }

    fn map_table_opcode_list(table: &[u16]) -> Vec<u8> {
        let mut bytes = Vec::new();
        for value in [1_u32, 7, 1, 0, 44] {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        for value in [0_u32, 0, 1, u32::try_from(table.len()).unwrap(), 0, 1, 1, 1] {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        bytes.extend_from_slice(&u32::try_from(table.len()).unwrap().to_be_bytes());
        for value in table {
            bytes.extend_from_slice(&value.to_be_bytes());
        }
        bytes
    }
}
