//! Multi-channel and multi-plane TIFF image reading and writing.

use std::fmt;
use std::fs::File;
use std::io::{BufWriter, Seek, Write};
use std::path::Path;

use fornax_codec::{
    ByteOrder, CodecError, DecodeRequest, EncodeCompression, EncodeRequest, ICodecDecode,
    ICodecEncode, Predictor, SampleBuffer, SampleBufferError, SampleFormat,
};
use fornax_core::{MonoImage, MultiPlaneImage, RgbImage, RgbaImage, UvImage};

use crate::constants::{
    CompressionMethod, PhotometricInterpretation, PlanarConfiguration, SampleFormatTag, TiffTag,
};
use crate::{
    Directory, ParsedTiff, RasterError, RasterLayout, Tag, TiffCompressionError, TiffDirectory,
    TiffError, TiffSource, TiffWriter, resolve_tiff_compression, unsigned_scalar,
    unsigned_vec_lookup,
};

/// A typed image buffer matching one of `fornax-core`'s image layouts.
#[derive(Clone, Debug, PartialEq)]
pub enum TypedTiffImage<T: image::Primitive> {
    /// Single-channel image.
    Mono(MonoImage<T>),
    /// Two-channel image (for example UV or grayscale + alpha).
    Uv(UvImage<T>),
    /// Three-channel RGB image.
    Rgb(RgbImage<T>),
    /// Four-channel RGBA image.
    Rgba(RgbaImage<T>),
    /// Arbitrary-channel planar image (`Vec<MonoImage<T>>`).
    MultiPlane(MultiPlaneImage<T>),
}

impl<T: image::Primitive> TypedTiffImage<T> {
    /// Returns the image width in pixels.
    pub fn width(&self) -> u32 {
        match self {
            Self::Mono(image) => image.width(),
            Self::Uv(image) => image.width(),
            Self::Rgb(image) => image.width(),
            Self::Rgba(image) => image.width(),
            Self::MultiPlane(planes) => planes.first().map_or(0, MonoImage::width),
        }
    }

    /// Returns the image height in pixels.
    pub fn height(&self) -> u32 {
        match self {
            Self::Mono(image) => image.height(),
            Self::Uv(image) => image.height(),
            Self::Rgb(image) => image.height(),
            Self::Rgba(image) => image.height(),
            Self::MultiPlane(planes) => planes.first().map_or(0, MonoImage::height),
        }
    }

    /// Returns the number of channels (samples per pixel).
    pub fn samples_per_pixel(&self) -> usize {
        match self {
            Self::Mono(_) => 1,
            Self::Uv(_) => 2,
            Self::Rgb(_) => 3,
            Self::Rgba(_) => 4,
            Self::MultiPlane(planes) => planes.len(),
        }
    }
}

impl<T: image::Primitive> From<MonoImage<T>> for TypedTiffImage<T> {
    fn from(image: MonoImage<T>) -> Self { Self::Mono(image) }
}

impl<T: image::Primitive> From<UvImage<T>> for TypedTiffImage<T> {
    fn from(image: UvImage<T>) -> Self { Self::Uv(image) }
}

impl<T: image::Primitive> From<RgbImage<T>> for TypedTiffImage<T> {
    fn from(image: RgbImage<T>) -> Self { Self::Rgb(image) }
}

impl<T: image::Primitive> From<RgbaImage<T>> for TypedTiffImage<T> {
    fn from(image: RgbaImage<T>) -> Self { Self::Rgba(image) }
}

impl<T: image::Primitive> From<MultiPlaneImage<T>> for TypedTiffImage<T> {
    fn from(planes: MultiPlaneImage<T>) -> Self { Self::MultiPlane(planes) }
}

/// Sample-type-erased TIFF image supporting mono, UV, RGB, RGBA, and
/// multi-plane layouts.
#[derive(Clone, Debug, PartialEq)]
#[non_exhaustive]
pub enum TiffImage {
    /// 8-bit unsigned samples.
    U8(TypedTiffImage<u8>),
    /// 16-bit unsigned samples.
    U16(TypedTiffImage<u16>),
    /// 32-bit unsigned samples.
    U32(TypedTiffImage<u32>),
    /// 32-bit floating-point samples.
    F32(TypedTiffImage<f32>),
    /// 64-bit floating-point samples.
    F64(TypedTiffImage<f64>),
}

impl TiffImage {
    /// Returns the image width in pixels.
    pub fn width(&self) -> u32 {
        match self {
            Self::U8(image) => image.width(),
            Self::U16(image) => image.width(),
            Self::U32(image) => image.width(),
            Self::F32(image) => image.width(),
            Self::F64(image) => image.width(),
        }
    }

    /// Returns the image height in pixels.
    pub fn height(&self) -> u32 {
        match self {
            Self::U8(image) => image.height(),
            Self::U16(image) => image.height(),
            Self::U32(image) => image.height(),
            Self::F32(image) => image.height(),
            Self::F64(image) => image.height(),
        }
    }

    /// Returns the number of channels (samples per pixel).
    pub fn samples_per_pixel(&self) -> usize {
        match self {
            Self::U8(image) => image.samples_per_pixel(),
            Self::U16(image) => image.samples_per_pixel(),
            Self::U32(image) => image.samples_per_pixel(),
            Self::F32(image) => image.samples_per_pixel(),
            Self::F64(image) => image.samples_per_pixel(),
        }
    }
}

macro_rules! impl_from_image_for_tiff_image {
    ($sample:ty, $variant:ident) => {
        impl From<TypedTiffImage<$sample>> for TiffImage {
            fn from(image: TypedTiffImage<$sample>) -> Self { Self::$variant(image) }
        }

        impl From<MonoImage<$sample>> for TiffImage {
            fn from(image: MonoImage<$sample>) -> Self {
                Self::$variant(TypedTiffImage::Mono(image))
            }
        }

        impl From<UvImage<$sample>> for TiffImage {
            fn from(image: UvImage<$sample>) -> Self { Self::$variant(TypedTiffImage::Uv(image)) }
        }

        impl From<RgbImage<$sample>> for TiffImage {
            fn from(image: RgbImage<$sample>) -> Self { Self::$variant(TypedTiffImage::Rgb(image)) }
        }

        impl From<RgbaImage<$sample>> for TiffImage {
            fn from(image: RgbaImage<$sample>) -> Self {
                Self::$variant(TypedTiffImage::Rgba(image))
            }
        }

        impl From<MultiPlaneImage<$sample>> for TiffImage {
            fn from(planes: MultiPlaneImage<$sample>) -> Self {
                Self::$variant(TypedTiffImage::MultiPlane(planes))
            }
        }
    };
}

impl_from_image_for_tiff_image!(u8, U8);
impl_from_image_for_tiff_image!(u16, U16);
impl_from_image_for_tiff_image!(u32, U32);
impl_from_image_for_tiff_image!(f32, F32);
impl_from_image_for_tiff_image!(f64, F64);

/// Strip or tile storage configuration for [`TiffWriteOptions`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum WriteStorage {
    /// Store image rows in strips. `None` writes the entire image as one strip.
    Strips {
        /// Number of rows per strip, or `None` for a single strip.
        rows_per_strip: Option<u32>,
    },
    /// Store image samples in rectangular tiles.
    Tiles {
        /// Tile width in pixels.
        width: u32,
        /// Tile height in pixels.
        height: u32,
    },
}

/// Configuration for writing a [`TiffImage`].
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct TiffWriteOptions {
    /// Compression algorithm applied to each strip or tile.
    pub compression: EncodeCompression,
    /// Strip or tile chunk layout.
    pub storage: WriteStorage,
    /// Interleaved (`Chunky`) or separated (`Planar`) sample storage.
    pub planar_configuration: PlanarConfiguration,
    /// Reversible predictor applied before compression.
    ///
    /// When [`Predictor::HorizontalInteger`] is selected with
    /// [`PlanarConfiguration::Chunky`] and `samples_per_pixel > 1`, the writer
    /// automatically uses channel-strided horizontal differencing.
    pub predictor: Predictor,
    /// Explicit photometric interpretation override, or `None` to infer from
    /// channel count.
    pub photometric: Option<PhotometricInterpretation>,
}

impl Default for TiffWriteOptions {
    fn default() -> Self {
        Self {
            compression: EncodeCompression::Uncompressed,
            storage: WriteStorage::Strips {
                rows_per_strip: None,
            },
            planar_configuration: PlanarConfiguration::Chunky,
            predictor: Predictor::None,
            photometric: None,
        }
    }
}

impl TiffWriteOptions {
    /// Sets the compression algorithm.
    pub const fn with_compression(mut self, compression: EncodeCompression) -> Self {
        self.compression = compression;
        self
    }

    /// Sets the strip or tile storage layout.
    pub const fn with_storage(mut self, storage: WriteStorage) -> Self {
        self.storage = storage;
        self
    }

    /// Sets the planar configuration.
    pub const fn with_planar_configuration(mut self, planar: PlanarConfiguration) -> Self {
        self.planar_configuration = planar;
        self
    }

    /// Sets the predictor.
    pub const fn with_predictor(mut self, predictor: Predictor) -> Self {
        self.predictor = predictor;
        self
    }

    /// Sets an explicit photometric interpretation.
    pub const fn with_photometric(mut self, photometric: PhotometricInterpretation) -> Self {
        self.photometric = Some(photometric);
        self
    }
}

/// Failure during high-level TIFF image reading or writing.
#[derive(Debug)]
pub enum TiffIoError {
    /// Low-level TIFF container parsing or writing failure.
    Tiff(TiffError),
    /// Raster layout validation failure.
    Raster(RasterError),
    /// Compression identifier resolution failure.
    Compression(TiffCompressionError),
    /// Codec decode or encode failure.
    Codec(CodecError),
    /// File system I/O failure.
    Io(std::io::Error),
    /// Malformed or inconsistent TIFF image metadata.
    Invalid(Box<str>),
    /// Recognized TIFF feature outside the supported image IO scope.
    Unsupported(Box<str>),
}

impl fmt::Display for TiffIoError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::Tiff(error) => write!(formatter, "{error}"),
            Self::Raster(error) => write!(formatter, "{error}"),
            Self::Compression(error) => write!(formatter, "{error}"),
            Self::Codec(error) => write!(formatter, "{error}"),
            Self::Io(error) => write!(formatter, "{error}"),
            Self::Invalid(detail) | Self::Unsupported(detail) => formatter.write_str(detail),
        }
    }
}

impl std::error::Error for TiffIoError {}

impl From<TiffError> for TiffIoError {
    fn from(error: TiffError) -> Self { Self::Tiff(error) }
}

impl From<RasterError> for TiffIoError {
    fn from(error: RasterError) -> Self { Self::Raster(error) }
}

impl From<TiffCompressionError> for TiffIoError {
    fn from(error: TiffCompressionError) -> Self { Self::Compression(error) }
}

impl From<CodecError> for TiffIoError {
    fn from(error: CodecError) -> Self { Self::Codec(error) }
}

impl From<SampleBufferError> for TiffIoError {
    fn from(error: SampleBufferError) -> Self { Self::Codec(error.into()) }
}

impl From<std::io::Error> for TiffIoError {
    fn from(error: std::io::Error) -> Self { Self::Io(error) }
}

/// Decodes the root directory of an in-memory TIFF container into a
/// [`TiffImage`].
///
/// Channel count mapping:
/// - `1` -> [`TypedTiffImage::Mono`]
/// - `2` -> [`TypedTiffImage::Uv`]
/// - `3` -> [`TypedTiffImage::Rgb`]
/// - `4` -> [`TypedTiffImage::Rgba`]
/// - `>= 5` -> [`TypedTiffImage::MultiPlane`]
pub fn read_tiff(source: &[u8]) -> Result<TiffImage, TiffIoError> {
    let parsed = crate::parse(source)?;
    read_directory_image(&parsed, 0)
}

/// Decodes the root directory of a TIFF file into a [`TiffImage`].
pub fn read_tiff_file(path: &Path) -> Result<TiffImage, TiffIoError> {
    let source = TiffSource::from_path(path)?;
    let parsed = crate::parse_source(source)?;
    read_directory_image(&parsed, 0)
}

/// Decodes directory `directory_index` of `parsed` into a [`TiffImage`].
pub fn read_directory_image(
    parsed: &ParsedTiff,
    directory_index: usize,
) -> Result<TiffImage, TiffIoError> {
    read_directory_internal(parsed, directory_index, false)
}

/// Decodes directory `directory_index` of `parsed` as
/// [`TypedTiffImage::MultiPlane`] regardless of channel count.
pub fn read_directory_planes(
    parsed: &ParsedTiff,
    directory_index: usize,
) -> Result<TiffImage, TiffIoError> {
    read_directory_internal(parsed, directory_index, true)
}

/// Writes a [`TiffImage`] to a seekable writer.
pub fn write_tiff<W: Write + Seek>(
    writer: W,
    image: &TiffImage,
    options: &TiffWriteOptions,
) -> Result<(), TiffIoError> {
    let width = image.width();
    let height = image.height();
    let samples_per_pixel = image.samples_per_pixel();
    if width == 0 || height == 0 {
        return Err(TiffIoError::Invalid(
            "image dimensions must not be zero".into(),
        ));
    }
    if samples_per_pixel == 0 {
        return Err(TiffIoError::Invalid(
            "image must contain at least one plane".into(),
        ));
    }

    let (chunk_width, chunk_height, is_tiles) = match options.storage {
        WriteStorage::Strips { rows_per_strip } => {
            let rows = rows_per_strip.unwrap_or(height);
            if rows == 0 {
                return Err(TiffIoError::Invalid(
                    "rows_per_strip must not be zero".into(),
                ));
            }
            (width, rows, false)
        }
        WriteStorage::Tiles {
            width: tw,
            height: th,
        } => {
            if tw == 0 || th == 0 {
                return Err(TiffIoError::Invalid(
                    "tile dimensions must not be zero".into(),
                ));
            }
            (tw, th, true)
        }
    };

    let planar = match options.planar_configuration {
        PlanarConfiguration::Chunky => PlanarConfiguration::Chunky,
        PlanarConfiguration::Planar => PlanarConfiguration::Planar,
        PlanarConfiguration::Unknown(id) => {
            return Err(TiffIoError::Unsupported(
                format!("unsupported planar configuration {id}").into(),
            ));
        }
    };

    let mut tiff_writer = TiffWriter::new(writer)?;
    let (bits_per_sample, sample_format_tag, encoded_chunks) =
        encode_tiff_image(image, chunk_width, chunk_height, planar, options)?;

    let mut offsets = Vec::with_capacity(encoded_chunks.len());
    let mut byte_counts = Vec::with_capacity(encoded_chunks.len());
    for chunk in encoded_chunks {
        let offset = tiff_writer.write_data(&chunk)?;
        let offset = u32::try_from(offset)
            .map_err(|_| TiffError::new("classic TIFF chunk offset exceeds u32"))?;
        let byte_count = u32::try_from(chunk.len())
            .map_err(|_| TiffError::new("classic TIFF chunk byte count exceeds u32"))?;
        offsets.push(offset);
        byte_counts.push(byte_count);
    }

    let photometric = options.photometric.unwrap_or({
        if samples_per_pixel <= 2 {
            PhotometricInterpretation::BlackIsZero
        } else {
            PhotometricInterpretation::Rgb
        }
    });

    let mut dir = TiffDirectory::new(&mut tiff_writer);
    dir.write_tag(TiffTag::ImageWidth, width)?;
    dir.write_tag(TiffTag::ImageLength, height)?;

    let spp_u16 = u16::try_from(samples_per_pixel)
        .map_err(|_| TiffIoError::Invalid("samples_per_pixel exceeds u16".into()))?;
    dir.write_tag(TiffTag::SamplesPerPixel, spp_u16)?;
    if samples_per_pixel == 1 {
        dir.write_tag(TiffTag::BitsPerSample, bits_per_sample)?;
        dir.write_tag(TiffTag::SampleFormat, u16::from(sample_format_tag))?;
    } else {
        let bits = vec![bits_per_sample; samples_per_pixel];
        let formats = vec![u16::from(sample_format_tag); samples_per_pixel];
        dir.write_tag(TiffTag::BitsPerSample, bits.as_slice())?;
        dir.write_tag(TiffTag::SampleFormat, formats.as_slice())?;
    }

    dir.write_tag(
        TiffTag::Compression,
        u16::from(compression_method(options.compression)),
    )?;
    dir.write_tag(TiffTag::PhotometricInterpretation, u16::from(photometric))?;
    dir.write_tag(TiffTag::PlanarConfiguration, u16::from(planar))?;

    if matches!(
        image,
        TiffImage::U8(TypedTiffImage::Uv(_) | TypedTiffImage::Rgba(_))
            | TiffImage::U16(TypedTiffImage::Uv(_) | TypedTiffImage::Rgba(_))
            | TiffImage::U32(TypedTiffImage::Uv(_) | TypedTiffImage::Rgba(_))
            | TiffImage::F32(TypedTiffImage::Uv(_) | TypedTiffImage::Rgba(_))
            | TiffImage::F64(TypedTiffImage::Uv(_) | TypedTiffImage::Rgba(_))
    ) {
        // Unassociated alpha (2) is the standard default for general RGBA/GA.
        dir.write_tag(TiffTag::ExtraSamples, 2_u16)?;
    }

    let predictor_tag = match options.predictor {
        Predictor::None => 1_u16,
        Predictor::HorizontalInteger | Predictor::HorizontalIntegerStrided { .. } => 2_u16,
        Predictor::FloatingPoint { .. } => 3_u16,
    };
    if predictor_tag != 1 {
        dir.write_tag(TiffTag::Predictor, predictor_tag)?;
    }

    if is_tiles {
        dir.write_tag(TiffTag::TileWidth, chunk_width)?;
        dir.write_tag(TiffTag::TileLength, chunk_height)?;
        dir.write_tag(TiffTag::TileOffsets, offsets.as_slice())?;
        dir.write_tag(TiffTag::TileByteCounts, byte_counts.as_slice())?;
    } else {
        dir.write_tag(TiffTag::RowsPerStrip, chunk_height)?;
        dir.write_tag(TiffTag::StripOffsets, offsets.as_slice())?;
        dir.write_tag(TiffTag::StripByteCounts, byte_counts.as_slice())?;
    }

    dir.finish()?;
    Ok(())
}

/// Writes a [`TiffImage`] to a file at `path`.
pub fn write_tiff_file(
    path: &Path,
    image: &TiffImage,
    options: &TiffWriteOptions,
) -> Result<(), TiffIoError> {
    let file = File::create(path)?;
    write_tiff(BufWriter::new(file), image, options)
}

fn read_directory_internal(
    parsed: &ParsedTiff,
    directory_index: usize,
    force_multiplane: bool,
) -> Result<TiffImage, TiffIoError> {
    let directory = parsed.directories.get(directory_index).ok_or_else(|| {
        TiffIoError::Invalid(format!("TIFF directory {directory_index} does not exist").into())
    })?;

    let samples_per_pixel = unsigned_scalar(directory.get(&TiffTag::SamplesPerPixel.into()))
        .map(|value| {
            usize::try_from(value)
                .map_err(|_| TiffIoError::Invalid("SamplesPerPixel exceeds usize".into()))
        })
        .transpose()?
        .unwrap_or(1);
    if samples_per_pixel == 0 {
        return Err(TiffIoError::Invalid(
            "SamplesPerPixel must not be zero".into(),
        ));
    }

    let planar_raw = unsigned_scalar(directory.get(&TiffTag::PlanarConfiguration.into()))
        .map(|value| {
            u16::try_from(value)
                .map_err(|_| TiffIoError::Invalid("PlanarConfiguration exceeds u16".into()))
        })
        .transpose()?
        .unwrap_or(1);
    let planar = match PlanarConfiguration::from(planar_raw) {
        PlanarConfiguration::Unknown(id) => {
            return Err(TiffIoError::Unsupported(
                format!("unsupported PlanarConfiguration {id}").into(),
            ));
        }
        other => other,
    };

    let photometric_raw =
        unsigned_scalar(directory.get(&TiffTag::PhotometricInterpretation.into())).map(|value| {
            u16::try_from(value)
                .map_err(|_| TiffIoError::Invalid("PhotometricInterpretation exceeds u16".into()))
        });
    let photometric = match photometric_raw {
        Some(Ok(id)) => PhotometricInterpretation::from(id),
        Some(Err(err)) => return Err(err),
        None => {
            if samples_per_pixel <= 2 {
                PhotometricInterpretation::BlackIsZero
            } else {
                PhotometricInterpretation::Rgb
            }
        }
    };
    match photometric {
        PhotometricInterpretation::WhiteIsZero
        | PhotometricInterpretation::BlackIsZero
        | PhotometricInterpretation::Rgb => {}
        other => {
            return Err(TiffIoError::Unsupported(
                format!("unsupported PhotometricInterpretation {other:?}").into(),
            ));
        }
    }

    let bits_per_sample = parse_uniform_u16_tag(directory, TiffTag::BitsPerSample, 1)?;
    let sample_format_raw = parse_uniform_u16_tag(directory, TiffTag::SampleFormat, 1)?;
    let sample_format = match SampleFormatTag::from(sample_format_raw) {
        SampleFormatTag::Unsigned => SampleFormat::Integer,
        SampleFormatTag::Float => SampleFormat::Float,
        other => {
            return Err(TiffIoError::Unsupported(
                format!("unsupported SampleFormat {other:?}").into(),
            ));
        }
    };

    let compression_id = unsigned_scalar(directory.get(&TiffTag::Compression.into()))
        .map(|value| {
            u16::try_from(value)
                .map_err(|_| TiffIoError::Invalid("Compression tag exceeds u16".into()))
        })
        .transpose()?
        .unwrap_or(1);
    let compression = resolve_tiff_compression(compression_id)?;

    let predictor_id = unsigned_scalar(directory.get(&TiffTag::Predictor.into()))
        .map(|value| {
            u16::try_from(value)
                .map_err(|_| TiffIoError::Invalid("Predictor tag exceeds u16".into()))
        })
        .transpose()?
        .unwrap_or(1);
    let predictor = resolve_predictor(predictor_id, planar, samples_per_pixel)?;

    let layout = RasterLayout::from_lookup_with_planes(directory, samples_per_pixel, planar)?;
    let source = parsed.source.as_ref();
    validate_chunk_bounds(&layout, source.len())?;
    let byte_order = match parsed.byte_order() {
        crate::ByteOrder::Little => ByteOrder::Little,
        crate::ByteOrder::Big => ByteOrder::Big,
    };

    let invert_white_is_zero = photometric == PhotometricInterpretation::WhiteIsZero
        && sample_format == SampleFormat::Integer;

    if planar == PlanarConfiguration::Planar || samples_per_pixel == 1 {
        let mut buffers = Vec::with_capacity(samples_per_pixel);
        for plane in 0..samples_per_pixel {
            let chunks = layout
                .plane_chunks(plane)
                .ok_or_else(|| TiffIoError::Invalid("missing planar chunk slice".into()))?;
            let request = DecodeRequest {
                source,
                width: layout.width(),
                height: layout.height(),
                chunk_width: layout.chunk_width(),
                chunk_height: layout.chunk_height(),
                chunks,
                bits_per_sample,
                sample_format,
                byte_order,
                predictor,
            };
            buffers.push(compression.decode(&request)?);
        }
        buffers_to_tiff_image(
            buffers,
            force_multiplane,
            invert_white_is_zero,
            bits_per_sample,
        )
    } else {
        let interleaved_width = layout
            .width()
            .checked_mul(samples_per_pixel)
            .ok_or_else(|| TiffIoError::Invalid("interleaved image width overflow".into()))?;
        let interleaved_chunk_width = layout
            .chunk_width()
            .checked_mul(samples_per_pixel)
            .ok_or_else(|| TiffIoError::Invalid("interleaved chunk width overflow".into()))?;
        let request = DecodeRequest {
            source,
            width: interleaved_width,
            height: layout.height(),
            chunk_width: interleaved_chunk_width,
            chunk_height: layout.chunk_height(),
            chunks: layout.chunks(),
            bits_per_sample,
            sample_format,
            byte_order,
            predictor,
        };
        let buffer = compression.decode(&request)?;
        chunky_buffer_to_tiff_image(
            buffer,
            layout.width(),
            layout.height(),
            samples_per_pixel,
            force_multiplane,
            invert_white_is_zero,
            bits_per_sample,
        )
    }
}

fn parse_uniform_u16_tag(
    directory: &Directory,
    tag: TiffTag,
    default: u16,
) -> Result<u16, TiffIoError> {
    let tiff_tag = Tag::from(tag);
    if !directory.contains_key(&tiff_tag) {
        return Ok(default);
    }
    let values = unsigned_vec_lookup::<_, _, u16>(directory, tiff_tag)
        .map_err(|_| TiffIoError::Invalid(format!("{tag} has an invalid unsigned value").into()))?;
    let first = *values
        .first()
        .ok_or_else(|| TiffIoError::Invalid(format!("{tag} is empty").into()))?;
    if values.iter().any(|&value| value != first) {
        return Err(TiffIoError::Unsupported(
            format!("non-uniform {tag} across channels is unsupported").into(),
        ));
    }
    Ok(first)
}

fn resolve_predictor(
    id: u16,
    planar: PlanarConfiguration,
    samples_per_pixel: usize,
) -> Result<Predictor, TiffIoError> {
    match id {
        1 => Ok(Predictor::None),
        2 => {
            if planar == PlanarConfiguration::Planar || samples_per_pixel == 1 {
                Ok(Predictor::HorizontalInteger)
            } else {
                Ok(Predictor::HorizontalIntegerStrided { samples_per_pixel })
            }
        }
        3 => {
            let byte_stride = if planar == PlanarConfiguration::Planar {
                1
            } else {
                samples_per_pixel
            };
            Ok(Predictor::FloatingPoint { byte_stride })
        }
        _ => Err(TiffIoError::Unsupported(
            format!("unsupported TIFF predictor {id}").into(),
        )),
    }
}

fn validate_chunk_bounds(layout: &RasterLayout, source_len: usize) -> Result<(), TiffIoError> {
    for (index, chunk) in layout.chunks().iter().copied().enumerate() {
        let end = chunk
            .offset
            .checked_add(chunk.byte_count)
            .ok_or(RasterError::ChunkRangeOverflow { index })?;
        if end > source_len {
            return Err(RasterError::ChunkOutOfBounds {
                index,
                offset: chunk.offset,
                byte_count: chunk.byte_count,
                source_len,
            }
            .into());
        }
    }
    Ok(())
}

fn invert_white_is_zero_u8(data: &mut [u8], bits: u16) {
    let max_val = ((1_u16 << bits) - 1) as u8;
    for value in data {
        *value = max_val.saturating_sub(*value);
    }
}

fn invert_white_is_zero_u16(data: &mut [u16], bits: u16) {
    let max_val = if bits == 16 {
        u16::MAX
    } else {
        (1_u16 << bits) - 1
    };
    for value in data {
        *value = max_val.saturating_sub(*value);
    }
}

fn invert_white_is_zero_u32(data: &mut [u32], bits: u16) {
    let max_val = if bits == 32 {
        u32::MAX
    } else {
        (1_u32 << bits) - 1
    };
    for value in data {
        *value = max_val.saturating_sub(*value);
    }
}

fn chunky_buffer_to_tiff_image(
    buffer: SampleBuffer,
    width: usize,
    height: usize,
    samples_per_pixel: usize,
    force_multiplane: bool,
    invert_white: bool,
    bits: u16,
) -> Result<TiffImage, TiffIoError> {
    let w = u32::try_from(width).map_err(|_| TiffIoError::Invalid("width exceeds u32".into()))?;
    let h = u32::try_from(height).map_err(|_| TiffIoError::Invalid("height exceeds u32".into()))?;

    macro_rules! convert {
        ($samples:expr, $variant:ident, $invert:expr) => {{
            let mut raw = $samples;
            if invert_white {
                $invert(&mut raw, bits);
            }
            if force_multiplane || samples_per_pixel >= 5 {
                let planes = deinterleave_planes(w, h, samples_per_pixel, &raw)?;
                Ok(TiffImage::$variant(TypedTiffImage::MultiPlane(planes)))
            } else {
                Ok(TiffImage::$variant(match samples_per_pixel {
                    1 => TypedTiffImage::Mono(
                        MonoImage::from_raw(w, h, raw)
                            .ok_or_else(|| TiffIoError::Invalid("invalid mono buffer".into()))?,
                    ),
                    2 => TypedTiffImage::Uv(
                        UvImage::from_raw(w, h, raw)
                            .ok_or_else(|| TiffIoError::Invalid("invalid UV buffer".into()))?,
                    ),
                    3 => TypedTiffImage::Rgb(
                        RgbImage::from_raw(w, h, raw)
                            .ok_or_else(|| TiffIoError::Invalid("invalid RGB buffer".into()))?,
                    ),
                    4 => TypedTiffImage::Rgba(
                        RgbaImage::from_raw(w, h, raw)
                            .ok_or_else(|| TiffIoError::Invalid("invalid RGBA buffer".into()))?,
                    ),
                    _ => unreachable!(),
                }))
            }
        }};
    }

    match buffer {
        SampleBuffer::U8(buf) => convert!(buf.into_raw(), U8, invert_white_is_zero_u8),
        SampleBuffer::U16(buf) => convert!(buf.into_raw(), U16, invert_white_is_zero_u16),
        SampleBuffer::U32(buf) => convert!(buf.into_raw(), U32, invert_white_is_zero_u32),
        SampleBuffer::F32(buf) => convert!(buf.into_raw(), F32, |_: &mut [f32], _| {}),
        SampleBuffer::F64(buf) => convert!(buf.into_raw(), F64, |_: &mut [f64], _| {}),
        _ => Err(TiffIoError::Unsupported(
            "unsupported decoded sample buffer variant".into(),
        )),
    }
}

fn buffers_to_tiff_image(
    buffers: Vec<SampleBuffer>,
    force_multiplane: bool,
    invert_white: bool,
    bits: u16,
) -> Result<TiffImage, TiffIoError> {
    let first = buffers
        .first()
        .ok_or_else(|| TiffIoError::Invalid("decoded image has no planes".into()))?;
    let w = first.width();
    let h = first.height();

    macro_rules! assemble {
        ($variant:ident, $into_raw:ident, $invert:expr) => {{
            let mut planes = Vec::with_capacity(buffers.len());
            for (index, buf) in buffers.into_iter().enumerate() {
                let mut raw = buf.$into_raw().ok_or_else(|| {
                    TiffIoError::Invalid("mixed sample types across planes".into())
                })?;
                if invert_white && index == 0 {
                    $invert(&mut raw, bits);
                }
                let plane = MonoImage::from_raw(w, h, raw)
                    .ok_or_else(|| TiffIoError::Invalid("invalid plane dimensions".into()))?;
                planes.push(plane);
            }
            if force_multiplane || planes.len() >= 5 {
                Ok(TiffImage::$variant(TypedTiffImage::MultiPlane(planes)))
            } else {
                Ok(TiffImage::$variant(match planes.len() {
                    1 => TypedTiffImage::Mono(planes.into_iter().next().unwrap()),
                    2 => TypedTiffImage::Uv(
                        UvImage::from_raw(w, h, interleave_planes(&planes))
                            .ok_or_else(|| TiffIoError::Invalid("invalid UV buffer".into()))?,
                    ),
                    3 => TypedTiffImage::Rgb(
                        RgbImage::from_raw(w, h, interleave_planes(&planes))
                            .ok_or_else(|| TiffIoError::Invalid("invalid RGB buffer".into()))?,
                    ),
                    4 => TypedTiffImage::Rgba(
                        RgbaImage::from_raw(w, h, interleave_planes(&planes))
                            .ok_or_else(|| TiffIoError::Invalid("invalid RGBA buffer".into()))?,
                    ),
                    _ => unreachable!(),
                }))
            }
        }};
    }

    match first {
        SampleBuffer::U8(_) => assemble!(U8, into_raw_u8, invert_white_is_zero_u8),
        SampleBuffer::U16(_) => assemble!(U16, into_raw_u16, invert_white_is_zero_u16),
        SampleBuffer::U32(_) => assemble!(U32, into_raw_u32, invert_white_is_zero_u32),
        SampleBuffer::F32(_) => assemble!(F32, into_raw_f32, |_: &mut [f32], _| {}),
        SampleBuffer::F64(_) => assemble!(F64, into_raw_f64, |_: &mut [f64], _| {}),
        _ => Err(TiffIoError::Unsupported(
            "unsupported decoded sample buffer variant".into(),
        )),
    }
}

fn deinterleave_planes<T: image::Primitive>(
    width: u32,
    height: u32,
    samples_per_pixel: usize,
    interleaved: &[T],
) -> Result<MultiPlaneImage<T>, TiffIoError> {
    let pixel_count = usize::try_from(width)
        .ok()
        .and_then(|w| usize::try_from(height).ok().and_then(|h| w.checked_mul(h)))
        .ok_or_else(|| TiffIoError::Invalid("pixel count overflow".into()))?;
    let mut planes = (0..samples_per_pixel)
        .map(|_| Vec::with_capacity(pixel_count))
        .collect::<Vec<_>>();
    for pixel in interleaved.chunks_exact(samples_per_pixel) {
        for (plane, &sample) in planes.iter_mut().zip(pixel) {
            plane.push(sample);
        }
    }
    planes
        .into_iter()
        .map(|raw| {
            MonoImage::from_raw(width, height, raw)
                .ok_or_else(|| TiffIoError::Invalid("invalid deinterleaved plane buffer".into()))
        })
        .collect()
}

fn interleave_planes<T: image::Primitive>(planes: &[MonoImage<T>]) -> Vec<T> {
    let pixel_count = planes.first().map_or(0, |p| p.as_raw().len());
    let mut interleaved = Vec::with_capacity(pixel_count * planes.len());
    for idx in 0..pixel_count {
        for plane in planes {
            interleaved.push(plane.as_raw()[idx]);
        }
    }
    interleaved
}

fn encode_tiff_image(
    image: &TiffImage,
    chunk_width: u32,
    chunk_height: u32,
    planar: PlanarConfiguration,
    options: &TiffWriteOptions,
) -> Result<(u16, SampleFormatTag, Vec<Vec<u8>>), TiffIoError> {
    let cw = usize::try_from(chunk_width)
        .map_err(|_| TiffIoError::Invalid("chunk_width exceeds usize".into()))?;
    let ch = usize::try_from(chunk_height)
        .map_err(|_| TiffIoError::Invalid("chunk_height exceeds usize".into()))?;

    macro_rules! encode_typed {
        ($typed:expr, $bits:expr, $format:expr, $ctor:expr) => {{
            let samples_per_pixel = $typed.samples_per_pixel();
            let predictor = encode_predictor(options.predictor, planar, samples_per_pixel);
            match planar {
                PlanarConfiguration::Chunky => {
                    let w = $typed.width();
                    let h = $typed.height();
                    let interleaved_w = w
                        .checked_mul(u32::try_from(samples_per_pixel).map_err(|_| {
                            TiffIoError::Invalid("samples_per_pixel exceeds u32".into())
                        })?)
                        .ok_or_else(|| {
                            TiffIoError::Invalid("interleaved image width exceeds u32".into())
                        })?;
                    let interleaved_cw = cw.checked_mul(samples_per_pixel).ok_or_else(|| {
                        TiffIoError::Invalid("interleaved chunk width exceeds usize".into())
                    })?;
                    let raw = typed_to_interleaved($typed)?;
                    let buf = $ctor(interleaved_w, h, raw)?;
                    let encoded = options.compression.encode(&EncodeRequest {
                        image: &buf,
                        chunk_width: interleaved_cw,
                        chunk_height: ch,
                        byte_order: ByteOrder::Little,
                        predictor,
                    })?;
                    Ok(($bits, $format, encoded.chunks))
                }
                PlanarConfiguration::Planar => {
                    let planes = typed_to_planes($typed)?;
                    let mut all_chunks = Vec::new();
                    for plane in planes {
                        let buf = $ctor(plane.width(), plane.height(), plane.into_raw())?;
                        let encoded = options.compression.encode(&EncodeRequest {
                            image: &buf,
                            chunk_width: cw,
                            chunk_height: ch,
                            byte_order: ByteOrder::Little,
                            predictor,
                        })?;
                        all_chunks.extend(encoded.chunks);
                    }
                    Ok(($bits, $format, all_chunks))
                }
                PlanarConfiguration::Unknown(_) => unreachable!(),
            }
        }};
    }

    match image {
        TiffImage::U8(typed) => {
            encode_typed!(typed, 8, SampleFormatTag::Unsigned, SampleBuffer::from_u8)
        }
        TiffImage::U16(typed) => {
            encode_typed!(typed, 16, SampleFormatTag::Unsigned, SampleBuffer::from_u16)
        }
        TiffImage::U32(typed) => {
            encode_typed!(typed, 32, SampleFormatTag::Unsigned, SampleBuffer::from_u32)
        }
        TiffImage::F32(typed) => {
            encode_typed!(typed, 32, SampleFormatTag::Float, SampleBuffer::from_f32)
        }
        TiffImage::F64(typed) => {
            encode_typed!(typed, 64, SampleFormatTag::Float, SampleBuffer::from_f64)
        }
    }
}

fn encode_predictor(
    predictor: Predictor,
    planar: PlanarConfiguration,
    samples_per_pixel: usize,
) -> Predictor {
    match predictor {
        Predictor::HorizontalInteger
            if planar == PlanarConfiguration::Chunky && samples_per_pixel > 1 =>
        {
            Predictor::HorizontalIntegerStrided { samples_per_pixel }
        }
        Predictor::FloatingPoint { .. } => Predictor::FloatingPoint {
            byte_stride: if planar == PlanarConfiguration::Planar {
                1
            } else {
                samples_per_pixel
            },
        },
        other => other,
    }
}

fn typed_to_interleaved<T: image::Primitive>(
    typed: &TypedTiffImage<T>,
) -> Result<Vec<T>, TiffIoError> {
    match typed {
        TypedTiffImage::Mono(img) => Ok(img.as_raw().clone()),
        TypedTiffImage::Uv(img) => Ok(img.as_raw().clone()),
        TypedTiffImage::Rgb(img) => Ok(img.as_raw().clone()),
        TypedTiffImage::Rgba(img) => Ok(img.as_raw().clone()),
        TypedTiffImage::MultiPlane(planes) => {
            validate_multiplane_dimensions(planes)?;
            Ok(interleave_planes(planes))
        }
    }
}

fn typed_to_planes<T: image::Primitive>(
    typed: &TypedTiffImage<T>,
) -> Result<Vec<MonoImage<T>>, TiffIoError> {
    match typed {
        TypedTiffImage::Mono(img) => Ok(vec![img.clone()]),
        TypedTiffImage::Uv(img) => deinterleave_planes(img.width(), img.height(), 2, img.as_raw()),
        TypedTiffImage::Rgb(img) => deinterleave_planes(img.width(), img.height(), 3, img.as_raw()),
        TypedTiffImage::Rgba(img) => {
            deinterleave_planes(img.width(), img.height(), 4, img.as_raw())
        }
        TypedTiffImage::MultiPlane(planes) => {
            validate_multiplane_dimensions(planes)?;
            Ok(planes.clone())
        }
    }
}

fn validate_multiplane_dimensions<T: image::Primitive>(
    planes: &[MonoImage<T>],
) -> Result<(), TiffIoError> {
    let first = planes
        .first()
        .ok_or_else(|| TiffIoError::Invalid("multi-plane image has no planes".into()))?;
    let dims = first.dimensions();
    if dims.0 == 0 || dims.1 == 0 {
        return Err(TiffIoError::Invalid(
            "multi-plane image dimensions must not be zero".into(),
        ));
    }
    if planes.iter().any(|plane| plane.dimensions() != dims) {
        return Err(TiffIoError::Invalid(
            "all planes in a multi-plane image must share the same dimensions".into(),
        ));
    }
    Ok(())
}

const fn compression_method(compression: EncodeCompression) -> CompressionMethod {
    match compression {
        EncodeCompression::Uncompressed => CompressionMethod::Uncompressed,
        EncodeCompression::LosslessJpeg => CompressionMethod::LosslessJpeg,
        EncodeCompression::Deflate => CompressionMethod::Deflate,
        #[cfg(feature = "lzw")]
        EncodeCompression::Lzw => CompressionMethod::Lzw,
        #[cfg(feature = "packbits")]
        EncodeCompression::PackBits => CompressionMethod::PackBits,
        #[cfg(feature = "zstd")]
        EncodeCompression::Zstd { .. } => CompressionMethod::Zstd,
    }
}
