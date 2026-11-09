//! DNG container and sample decoding.

mod process_metadata;
mod transforms;

use fornax_codec::{ByteOrder, Chunk, DecodeCompression, DecodeRequest, Predictor, SampleFormat};
use fornax_core::error::DecodeError;
use fornax_core::metadata::ProcessMetadata;
use fornax_core::{CroppedMosaic, MonoImage, Primitive, RawMosaic};
use fornax_tiff::constants::{CompressionMethod, PhotometricInterpretation, TiffTag};
use fornax_tiff::{
    Directory, ParsedTiff, Tag, ValueAccessError, unsigned_scalar,
    unsigned_vec as tiff_unsigned_vec,
};
use generic_num::num;
use image::math::Rect;
pub use process_metadata::process_metadata_from_dng_metadata;
use rayon::prelude::*;

use crate::metadata::DngTag;
use crate::sample_buffer::into_raw_mosaic;

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
    let photometric =
        unsigned_scalar(directory.get(&Tag::from(TiffTag::PhotometricInterpretation)))
            .and_then(|value| u16::try_from(value).ok())
            .map(PhotometricInterpretation::from);
    let reduced_resolution = unsigned_scalar(directory.get(&Tag::from(TiffTag::NewSubfileType)))
        .is_some_and(|value| value & 1 != 0);
    !reduced_resolution
        && photometric.is_some_and(|value| {
            matches!(
                value,
                PhotometricInterpretation::Cfa | PhotometricInterpretation::LinearRaw
            )
        })
}

fn raw_area(directory: &fornax_tiff::Directory) -> u64 {
    let width = unsigned_scalar(directory.get(&Tag::from(TiffTag::ImageWidth))).unwrap_or(0);
    let height = unsigned_scalar(directory.get(&Tag::from(TiffTag::ImageLength))).unwrap_or(0);
    width.saturating_mul(height)
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
    let process_metadata = process_metadata_from_dng_metadata(width, height, &metadata)?;
    let active_area = process_metadata.active_area;
    let image = match pixels {
        RawMosaic::U8(image) => crop_and_cast(image, active_area)?,
        RawMosaic::U16(image) => crop_and_cast(image, active_area)?,
        RawMosaic::U32(image) => crop_and_cast(image, active_area)?,
        RawMosaic::F32(image) => crop_and_cast(image, active_area)?,
    };
    Ok(CroppedMosaic::new(image))
}

pub fn dng_metadata(parsed: &DngImage) -> Result<Directory, DecodeError> {
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
    let mut dng_metadata = Directory::default();
    dng_metadata.extend(parsed.parsed.root.clone());
    dng_metadata.extend(raw_metadata.clone());
    Ok(dng_metadata)
}

pub fn process_metadata(parsed: &DngImage) -> Result<ProcessMetadata, DecodeError> {
    let metadata = dng_metadata(parsed)?;
    let width = required_u32(&metadata, TiffTag::ImageWidth)?;
    let height = required_u32(&metadata, TiffTag::ImageLength)?;
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

fn validate_dimensions(pixels: &RawMosaic, metadata: &Directory) -> Result<(), DecodeError> {
    let width = required_u32(metadata, TiffTag::ImageWidth)?;
    let height = required_u32(metadata, TiffTag::ImageLength)?;
    let (actual_width, actual_height) = pixels.dimensions();
    if (width, height) != (actual_width, actual_height) {
        return Err(corrupt(format!(
            "DNG metadata dimensions are {width}x{height}, mosaic dimensions are \
             {actual_width}x{actual_height}"
        )));
    }
    Ok(())
}

#[derive(Clone, Debug)]
struct DecodePlan {
    compression: DecodeCompression,
    width: usize,
    height: usize,
    chunk_width: usize,
    chunk_height: usize,
    chunks: Vec<Chunk>,
    bits_per_sample: u16,
    sample_format: SampleFormat,
    byte_order: ByteOrder,
    predictor: Predictor,
}

impl DecodePlan {
    fn request<'a>(&'a self, source: &'a [u8]) -> DecodeRequest<'a> {
        DecodeRequest {
            source,
            compression: self.compression,
            width: self.width,
            height: self.height,
            chunk_width: self.chunk_width,
            chunk_height: self.chunk_height,
            chunks: &self.chunks,
            bits_per_sample: self.bits_per_sample,
            sample_format: self.sample_format,
            byte_order: self.byte_order,
            predictor: self.predictor,
        }
    }
}

fn decode_plan(source: &[u8], metadata: &Directory) -> Result<DecodePlan, DecodeError> {
    let width = required_u32(metadata, TiffTag::ImageWidth)?;
    let height = required_u32(metadata, TiffTag::ImageLength)?;
    if width == 0 || height == 0 {
        return Err(corrupt("DNG image dimensions must not be zero"));
    }
    let compression = match CompressionMethod::from(required_u16(metadata, TiffTag::Compression)?) {
        CompressionMethod::Uncompressed => DecodeCompression::Uncompressed,
        CompressionMethod::LosslessJpeg => DecodeCompression::LosslessJpeg,
        CompressionMethod::Deflate => DecodeCompression::Deflate,
        CompressionMethod::LossyJpeg => DecodeCompression::LossyJpeg,
        CompressionMethod::JpegXlLegacy | CompressionMethod::JpegXl => DecodeCompression::JpegXl,
        CompressionMethod::Zstd => DecodeCompression::Zstd,
        CompressionMethod::Unknown(value) => {
            return Err(corrupt(format!("invalid DNG compression {value}")));
        }
    };
    let samples_per_pixel =
        unsigned_scalar(metadata.get(&Tag::from(TiffTag::SamplesPerPixel))).unwrap_or(1);
    if samples_per_pixel != 1 {
        return Err(unsupported_variant(format!(
            "only one raw sample per pixel is supported, found {samples_per_pixel}"
        )));
    }
    let bits_per_sample = required_u16(metadata, TiffTag::BitsPerSample)?;
    let sample_format = match sample_format(metadata) {
        1 => SampleFormat::Integer,
        3 => SampleFormat::Float,
        value => {
            return Err(corrupt(format!("invalid DNG sample format {value}")));
        }
    };
    let (chunk_width, chunk_height, offsets, byte_counts) = layout(metadata)?;
    if offsets.len() != byte_counts.len() {
        return Err(corrupt(format!(
            "DNG chunk offsets and byte counts differ: {} offsets, {} byte counts",
            offsets.len(),
            byte_counts.len()
        )));
    }
    let chunks = offsets
        .into_iter()
        .zip(byte_counts)
        .map(|(offset, byte_count)| Chunk { offset, byte_count })
        .collect();
    let predictor = match compression {
        DecodeCompression::Deflate | DecodeCompression::Zstd => {
            match unsigned_scalar(metadata.get(&Tag::from(TiffTag::Predictor))).unwrap_or(1) {
                1 => Predictor::None,
                2 => Predictor::HorizontalInteger,
                3 => Predictor::FloatingPoint { byte_stride: 1 },
                34_894 => Predictor::FloatingPoint { byte_stride: 2 },
                34_895 => Predictor::FloatingPoint { byte_stride: 4 },
                value => {
                    return Err(corrupt(format!("DNG predictor {value} is unsupported")));
                }
            }
        }
        _ => Predictor::None,
    };
    Ok(DecodePlan {
        compression,
        width: usize::try_from(width).expect("u32 fits usize on supported platforms"),
        height: usize::try_from(height).expect("u32 fits usize on supported platforms"),
        chunk_width,
        chunk_height,
        chunks,
        bits_per_sample,
        sample_format,
        byte_order: byte_order(source),
        predictor,
    })
}

fn decode_samples(source: &[u8], metadata: &Directory) -> Result<RawMosaic, DecodeError> {
    let plan = decode_plan(source, metadata)?;
    let samples = fornax_codec::decode(&plan.request(source)).map_err(codec_decode_error)?;
    into_raw_mosaic(samples)
}

fn codec_decode_error(error: fornax_codec::CodecError) -> DecodeError {
    match error {
        fornax_codec::CodecError::InvalidInput(detail) => DecodeError::InvalidInput(detail),
        fornax_codec::CodecError::Unsupported(detail) => DecodeError::Unsupported(detail),
    }
}

fn layout(metadata: &Directory) -> Result<(usize, usize, Vec<usize>, Vec<usize>), DecodeError> {
    if metadata.contains_key(&Tag::from(TiffTag::TileOffsets)) {
        let width = usize::try_from(required_u32(metadata, TiffTag::TileWidth)?)
            .expect("u32 fits usize on supported platforms");
        let height = usize::try_from(required_u32(metadata, TiffTag::TileLength)?)
            .expect("u32 fits usize on supported platforms");
        if width == 0 || height == 0 {
            return Err(corrupt("DNG tile dimensions must not be zero"));
        }
        return Ok((
            width,
            height,
            unsigned_vec(metadata, TiffTag::TileOffsets)?,
            unsigned_vec(metadata, TiffTag::TileByteCounts)?,
        ));
    }

    let width = usize::try_from(required_u32(metadata, TiffTag::ImageWidth)?)
        .expect("u32 fits usize on supported platforms");
    let height = usize::try_from(required_u32(metadata, TiffTag::RowsPerStrip)?)
        .expect("u32 fits usize on supported platforms");
    if width == 0 || height == 0 {
        return Err(corrupt("DNG strip dimensions must not be zero"));
    }
    Ok((
        width,
        height,
        unsigned_vec(metadata, TiffTag::StripOffsets)?,
        unsigned_vec(metadata, TiffTag::StripByteCounts)?,
    ))
}

fn sample_format(metadata: &Directory) -> u64 {
    unsigned_scalar(metadata.get(&Tag::from(TiffTag::SampleFormat))).unwrap_or(1)
}

fn byte_order(source: &[u8]) -> ByteOrder {
    match source.get(..2) {
        Some(b"II") => ByteOrder::Little,
        _ => ByteOrder::Big,
    }
}

fn required_u16<T>(metadata: &Directory, tag: T) -> Result<u16, DecodeError>
where
    T: Into<Tag>,
{
    let tag = tag.into();
    let value = unsigned_scalar(metadata.get(&tag))
        .ok_or_else(|| corrupt(format!("{tag} is missing or invalid")))?;
    u16::try_from(value).map_err(|_| corrupt(format!("{tag} is out of range")))
}

fn required_u32<T>(metadata: &Directory, tag: T) -> Result<u32, DecodeError>
where
    T: Into<Tag>,
{
    let tag = tag.into();
    let value = unsigned_scalar(metadata.get(&tag))
        .ok_or_else(|| corrupt(format!("{tag} is missing or invalid")))?;
    u32::try_from(value).map_err(|_| corrupt(format!("{tag} is out of range")))
}

fn unsigned_vec<T>(metadata: &Directory, tag: T) -> Result<Vec<usize>, DecodeError>
where
    T: Into<Tag>,
{
    let tag = tag.into();
    tiff_unsigned_vec(metadata, tag).map_err(|error| match error {
        ValueAccessError::Missing => corrupt(format!("{tag} is missing")),
        ValueAccessError::InvalidType => corrupt(format!("{tag} has an invalid value type")),
        ValueAccessError::Overflow => corrupt(format!(
            "{tag} contains a value too large for this platform"
        )),
    })
}

fn unsupported_variant(detail: impl Into<Box<str>>) -> DecodeError {
    DecodeError::Unsupported(detail.into())
}

fn corrupt(detail: impl Into<Box<str>>) -> DecodeError { DecodeError::InvalidInput(detail.into()) }

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use fornax_core::{MonoImage, RawMosaic};
    use fornax_tiff::constants::TiffTag;
    use fornax_tiff::{Tag, TiffWriter, Value};

    use super::{DngImage, process_metadata};

    #[test]
    fn compression_methods_preserve_known_and_unknown_identifiers() {
        assert_eq!(
            super::CompressionMethod::from(7),
            super::CompressionMethod::LosslessJpeg
        );
        assert_eq!(u16::from(super::CompressionMethod::from(7)), 7);
        assert_eq!(
            super::CompressionMethod::from(12_345),
            super::CompressionMethod::Unknown(12_345)
        );
        assert_eq!(u16::from(super::CompressionMethod::from(12_345)), 12_345);
    }

    #[test]
    fn dng_metadata_maps_to_a_codec_decode_request() {
        let source = b"MMxxxxxxxx";
        let mut metadata = metadata(
            4,
            2,
            12,
            8,
            [
                (TiffTag::TileWidth.into(), Value::Unsigned(4)),
                (TiffTag::TileLength.into(), Value::Unsigned(1)),
                (TiffTag::Predictor.into(), Value::Short(2)),
                (TiffTag::SampleFormat.into(), Value::Short(1)),
            ],
        );
        metadata.insert(
            Tag::from(TiffTag::TileOffsets),
            Value::List(vec![Value::Unsigned(2), Value::Unsigned(6)]),
        );
        metadata.insert(
            Tag::from(TiffTag::TileByteCounts),
            Value::List(vec![Value::Unsigned(4), Value::Unsigned(2)]),
        );
        metadata.insert(
            Tag::from(TiffTag::Compression),
            Value::Short(u16::from(super::CompressionMethod::Deflate)),
        );

        let plan = super::decode_plan(source, &metadata).unwrap();

        assert_eq!(plan.compression, fornax_codec::DecodeCompression::Deflate);
        assert_eq!((plan.width, plan.height), (4, 2));
        assert_eq!((plan.chunk_width, plan.chunk_height), (4, 1));
        assert_eq!(plan.bits_per_sample, 12);
        assert_eq!(plan.sample_format, fornax_codec::SampleFormat::Integer);
        assert_eq!(plan.byte_order, fornax_codec::ByteOrder::Big);
        assert_eq!(plan.predictor, fornax_codec::Predictor::HorizontalInteger);
        assert_eq!(
            plan.chunks,
            vec![
                fornax_codec::Chunk {
                    offset: 2,
                    byte_count: 4,
                },
                fornax_codec::Chunk {
                    offset: 6,
                    byte_count: 2,
                },
            ]
        );
    }

    #[test]
    fn zstd_metadata_maps_to_level_free_decode_compression() {
        let source = b"MM";
        let mut metadata = metadata(
            1,
            1,
            8,
            1,
            [
                (TiffTag::TileWidth.into(), Value::Unsigned(1)),
                (TiffTag::TileLength.into(), Value::Unsigned(1)),
            ],
        );
        metadata.insert(
            Tag::from(TiffTag::Compression),
            Value::Short(u16::from(super::CompressionMethod::Zstd)),
        );

        let plan = super::decode_plan(source, &metadata).unwrap();

        assert_eq!(plan.compression, fornax_codec::DecodeCompression::Zstd);
    }

    #[test]
    fn legacy_jpeg_xl_metadata_maps_to_jpeg_xl_decode_compression() {
        let source = b"MM";
        let mut metadata = metadata(
            1,
            1,
            8,
            1,
            [
                (TiffTag::TileWidth.into(), Value::Unsigned(1)),
                (TiffTag::TileLength.into(), Value::Unsigned(1)),
            ],
        );
        metadata.insert(Tag::from(TiffTag::Compression), Value::Short(50_002));

        let plan = super::decode_plan(source, &metadata).unwrap();

        assert_eq!(plan.compression, fornax_codec::DecodeCompression::JpegXl);
    }

    #[test]
    fn dng_metadata_rejects_mismatched_chunk_offsets_and_byte_counts() {
        let source = b"MMxxxxxxxx";
        let mut metadata = metadata(
            4,
            2,
            12,
            8,
            [
                (TiffTag::TileWidth.into(), Value::Unsigned(4)),
                (TiffTag::TileLength.into(), Value::Unsigned(1)),
            ],
        );
        metadata.insert(
            Tag::from(TiffTag::TileOffsets),
            Value::List(vec![Value::Unsigned(2), Value::Unsigned(6)]),
        );
        metadata.insert(Tag::from(TiffTag::TileByteCounts), Value::Unsigned(4));

        let error = super::decode_plan(source, &metadata).unwrap_err();

        assert!(matches!(
            error,
            fornax_core::error::DecodeError::InvalidInput(_)
        ));
    }

    use crate::metadata::{Directory, DngTag};
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
        let metadata = process_metadata(&parsed)
            .expect("metadata access must not depend on the pixel payload");
        assert_eq!(metadata.active_area.width, 4);
        assert_eq!(metadata.active_area.height, 4);
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
    fn zstd_writer_round_trips_u16_samples() {
        let samples = vec![0, 65_535, 30_000, 32_768, 1, 2, 40_000, 12_345];
        let mosaic = RawMosaic::U16(MonoImage::from_raw(4, 2, samples.clone()).unwrap());
        let mut output = Cursor::new(Vec::new());
        DngWriter::new(DngCompression::Zstd)
            .with_zstd_level(3)
            .write_dng_with(&mut output, &mosaic, &writable_metadata(4, 2))
            .unwrap();

        let parsed = DngImage::parse(output.get_ref()).unwrap();
        let RawMosaic::U16(image) = super::raw_mosaic(&parsed).unwrap() else {
            panic!("zstd samples must decode as u16");
        };
        assert_eq!(image.as_raw().as_slice(), samples.as_slice());
    }

    #[test]
    fn zstd_writer_round_trips_u8_u32_and_f32_samples() {
        let u8_samples = vec![0, 1, 2, 255];
        let u32_samples = vec![0, 70_000, 4_000_000_000, 12];
        let f32_samples = vec![-1.5_f32, 0.25, 100.0, f32::from_bits(0x7fc0_1234)];
        for (mosaic, expected) in [
            (
                RawMosaic::U8(MonoImage::from_raw(4, 1, u8_samples).unwrap()),
                "u8",
            ),
            (
                RawMosaic::U32(MonoImage::from_raw(4, 1, u32_samples).unwrap()),
                "u32",
            ),
            (
                RawMosaic::F32(MonoImage::from_raw(4, 1, f32_samples).unwrap()),
                "f32",
            ),
        ] {
            let mut output = Cursor::new(Vec::new());
            DngWriter::new(DngCompression::Zstd)
                .write_dng_with(&mut output, &mosaic, &writable_metadata(4, 1))
                .unwrap();
            let parsed = DngImage::parse(output.get_ref()).unwrap();
            match (mosaic, super::raw_mosaic(&parsed).unwrap()) {
                (RawMosaic::U8(expected), RawMosaic::U8(decoded)) => {
                    assert_eq!(decoded, expected, "{expected:?}");
                }
                (RawMosaic::U32(expected), RawMosaic::U32(decoded)) => {
                    assert_eq!(decoded, expected, "{expected:?}");
                }
                (RawMosaic::F32(expected), RawMosaic::F32(decoded)) => {
                    assert_eq!(
                        decoded
                            .as_raw()
                            .iter()
                            .map(|value| value.to_bits())
                            .collect::<Vec<_>>(),
                        expected
                            .as_raw()
                            .iter()
                            .map(|value| value.to_bits())
                            .collect::<Vec<_>>(),
                        "{expected:?}"
                    );
                }
                (_, decoded) => panic!("{expected} decoded as an unexpected type: {decoded:?}"),
            }
        }
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
            DngTag::LinearizationTable.into(),
            Value::List(vec![
                Value::Short(100),
                Value::Short(200),
                Value::Short(300),
                Value::Short(400),
            ]),
        );
        metadata.insert(Tag::from(DngTag::Unknown(50_975)), Value::Short(2));
        metadata.insert(Tag::from(DngTag::Unknown(50_976)), Value::Short(2));

        let mut output = Cursor::new(Vec::new());
        DngWriter::new(DngCompression::LosslessJpeg)
            .write_dng_with(&mut output, &mosaic, &metadata)
            .unwrap();
        let parsed = DngImage::parse(output.get_ref()).unwrap();
        let generated = super::dng_metadata(&parsed).unwrap();
        assert!(!generated.contains_key(&Tag::from(DngTag::LinearizationTable)));
        assert!(!generated.contains_key(&Tag::from(DngTag::Unknown(50_975))));
        assert!(!generated.contains_key(&Tag::from(DngTag::Unknown(50_976))));
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
    fn writer_does_not_duplicate_structural_tiff_tags_from_metadata() {
        let mosaic =
            RawMosaic::U32(MonoImage::from_raw(3, 1, vec![1_u32, 70_000, 4_000_000_000]).unwrap());
        let mut metadata = writable_metadata(3, 1);
        metadata.insert(TiffTag::SampleFormat.into(), Value::Short(1));
        metadata.insert(TiffTag::Predictor.into(), Value::Short(3));
        let mut output = Cursor::new(Vec::new());

        DngWriter::new(DngCompression::LosslessJpeg)
            .write_dng_with(&mut output, &mosaic, &metadata)
            .unwrap();

        assert_eq!(ifd_tag_count(output.get_ref(), TiffTag::SampleFormat), 1);
        assert_eq!(ifd_tag_count(output.get_ref(), TiffTag::Predictor), 1);
    }

    #[test]
    fn linearization_table_replaces_stored_values() {
        let pixels = RawMosaic::U16(MonoImage::from_raw(4, 1, vec![0, 1, 2, 3]).unwrap());
        let mut metadata = Directory::default();
        metadata.insert(Tag::from(TiffTag::BitsPerSample), Value::Short(2));
        metadata.insert(
            DngTag::LinearizationTable.into(),
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
        let mut metadata = Directory::default();
        metadata.insert(Tag::from(DngTag::Unknown(50_975)), Value::Short(2));
        metadata.insert(Tag::from(DngTag::Unknown(50_976)), Value::Short(2));
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
        let mut metadata = Directory::default();
        metadata.insert(
            DngTag::Unknown(51_008).into(),
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

    fn metadata(
        width: u32,
        height: u32,
        bits: u16,
        byte_count: u32,
        extra: impl IntoIterator<Item = (Tag, Value)>,
    ) -> Directory {
        let mut metadata = Directory::default();
        metadata.insert(Tag::from(TiffTag::ImageWidth), Value::Unsigned(width));
        metadata.insert(Tag::from(TiffTag::ImageLength), Value::Unsigned(height));
        metadata.insert(Tag::from(TiffTag::BitsPerSample), Value::Short(bits));
        metadata.insert(Tag::from(TiffTag::Compression), Value::Short(1));
        metadata.insert(Tag::from(TiffTag::SamplesPerPixel), Value::Short(1));
        metadata.insert(
            Tag::from(TiffTag::PhotometricInterpretation),
            Value::Short(32_803),
        );
        metadata.insert(Tag::from(TiffTag::TileLength), Value::Unsigned(height));
        metadata.insert(Tag::from(TiffTag::TileOffsets), Value::Unsigned(0));
        metadata.insert(
            Tag::from(TiffTag::TileByteCounts),
            Value::Unsigned(byte_count),
        );
        for (tag, value) in extra {
            metadata.insert(tag, value);
        }
        metadata
    }

    fn writable_metadata(width: u32, height: u32) -> Directory {
        let mut metadata = Directory::default();
        metadata.insert(
            DngTag::ActiveArea.into(),
            Value::List(vec![
                Value::Unsigned(0),
                Value::Unsigned(0),
                Value::Unsigned(height),
                Value::Unsigned(width),
            ]),
        );
        metadata.insert(
            DngTag::DefaultCropOrigin.into(),
            Value::List(vec![Value::Unsigned(0), Value::Unsigned(0)]),
        );
        metadata.insert(
            DngTag::DefaultCropSize.into(),
            Value::List(vec![Value::Unsigned(width), Value::Unsigned(height)]),
        );
        metadata.insert(
            DngTag::BlackLevelRepeatDim.into(),
            Value::List(vec![Value::Short(2), Value::Short(2)]),
        );
        metadata.insert(
            DngTag::BlackLevel.into(),
            Value::List(vec![
                Value::Short(0),
                Value::Short(0),
                Value::Short(0),
                Value::Short(0),
            ]),
        );
        metadata.insert(Tag::from(DngTag::WhiteLevel), Value::Unsigned(65_535));
        metadata.insert(
            TiffTag::CfaRepeatPatternDim.into(),
            Value::List(vec![Value::Short(2), Value::Short(2)]),
        );
        metadata.insert(
            TiffTag::CfaPattern.into(),
            Value::List(vec![
                Value::Byte(0),
                Value::Byte(1),
                Value::Byte(1),
                Value::Byte(2),
            ]),
        );
        metadata.insert(
            DngTag::CfaPlaneColor.into(),
            Value::List(vec![Value::Byte(0), Value::Byte(1), Value::Byte(2)]),
        );
        metadata.insert(Tag::from(DngTag::CfaLayout), Value::Short(1));
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

    fn ifd_tag_count(source: &[u8], tag: TiffTag) -> usize {
        assert_eq!(&source[..2], b"II");
        let offset = usize::try_from(u32::from_le_bytes(source[4..8].try_into().unwrap())).unwrap();
        let count = usize::from(u16::from_le_bytes(
            source[offset..offset + 2].try_into().unwrap(),
        ));
        (0..count)
            .filter(|index| {
                let entry = offset + 2 + index * 12;
                u16::from_le_bytes(source[entry..entry + 2].try_into().unwrap()) == u16::from(tag)
            })
            .count()
    }
}
