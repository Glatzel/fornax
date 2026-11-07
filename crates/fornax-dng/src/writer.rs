use std::fs::File;
use std::io::{BufWriter, Seek, Write};
use std::path::Path;

use fornax_codec::{
    ByteOrder as CodecByteOrder, Compression as CodecCompression, EncodeRequest,
    Predictor as CodecPredictor, encode as encode_codec,
};
#[cfg(feature = "zstd")]
use fornax_codec::{ICodecEncode, ZstdCodec};
use fornax_core::RawMosaic;
use fornax_tiff::constants::{PhotometricInterpretation, TiffTag};
use fornax_tiff::{Tag, TiffDirectory, TiffWriter as ContainerWriter, Value};
use thiserror::Error;

use crate::metadata::{Directory, DngTag};

/// Failure while writing DNG data.
#[derive(Clone, Debug, Eq, Error, PartialEq)]
pub enum DngWriteError {
    #[error("unsupported decoder operation: {operation}")]
    Unsupported { operation: Box<str> },
    #[error("{operation} failed: {detail}")]
    Io {
        operation: Box<str>,
        detail: Box<str>,
    },
    #[error("invalid DNG metadata: {detail}")]
    InvalidMetadata { detail: Box<str> },
}

impl From<fornax_tiff::TiffError> for DngWriteError {
    fn from(error: fornax_tiff::TiffError) -> Self { tiff_write_error(error) }
}

/// DNG compression modes written in the first implementation stage.
///
/// # Examples
///
/// ```
/// use fornax_dng::writer::DngCompression;
///
/// assert_eq!(DngCompression::default(), DngCompression::LosslessJpeg);
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DngCompression {
    /// Uncompressed strip storage.
    Uncompressed,
    /// Lossless DNG storage: JPEG-92 for U8/U16 and Deflate for wider samples.
    #[default]
    LosslessJpeg,
    /// Deflate for every supported mosaic sample type.
    Deflate,
    /// Experimental TIFF compression 50000 using pure-Rust Zstandard tiles.
    Zstd,
}

/// Four-component DNG specification version stored in `DNGVersion`.
///
/// # Examples
///
/// ```
/// use fornax_dng::writer::DngVersion;
///
/// assert_eq!(DngVersion::latest().minor, 7);
/// ```
#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub struct DngVersion {
    /// Major version.
    pub major: u8,
    /// Minor version.
    pub minor: u8,
    /// Patch version.
    pub patch: u8,
    /// Build version.
    pub build: u8,
}

impl DngVersion {
    /// Returns DNG specification 1.7.1.0, September 2023.
    pub const fn latest() -> Self {
        Self {
            major: 1,
            minor: 7,
            patch: 1,
            build: 0,
        }
    }

    /// Creates a DNG specification version.
    pub const fn new(major: u8, minor: u8, patch: u8, build: u8) -> Self {
        Self {
            major,
            minor,
            patch,
            build,
        }
    }
}

impl Default for DngVersion {
    fn default() -> Self { Self::latest() }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DngWriter {
    version: DngVersion,
    compression: DngCompression,
    zstd_level: i32,
}

impl Default for DngWriter {
    fn default() -> Self { Self::new(DngCompression::default()) }
}

impl DngWriter {
    /// Creates writer options for the current DNG specification and requested
    /// compression mode.
    pub const fn new(compression: DngCompression) -> Self {
        Self {
            version: DngVersion::latest(),
            compression,
            zstd_level: 3,
        }
    }

    /// Selects the DNG version used when regenerating a container.
    ///
    /// Source-preserving lossless export retains the decoded file's original
    /// `DNGVersion` rather than rewriting its container.
    pub const fn with_version(mut self, version: DngVersion) -> Self {
        self.version = version;
        self
    }

    /// Returns the selected compression mode.
    pub const fn compression(self) -> DngCompression { self.compression }

    /// Selects the experimental Zstandard compression level.
    pub const fn with_zstd_level(mut self, level: i32) -> Self {
        self.zstd_level = level;
        self
    }

    /// Returns the selected version used for regenerated output.
    pub const fn version(self) -> DngVersion { self.version }

    pub fn write<P>(
        &self,
        path: P,
        mosaic: &RawMosaic,
        metadata: &Directory,
    ) -> Result<(), DngWriteError>
    where
        P: AsRef<Path>,
    {
        let file = File::create(path.as_ref()).map_err(|error| DngWriteError::Io {
            operation: format!("creating {}", path.as_ref().display()).into(),
            detail: error.to_string().into(),
        })?;
        self.write_dng_with(BufWriter::new(file), mosaic, metadata)
    }
    pub fn write_dng_with<W>(
        &self,
        writer: W,
        mosaic: &RawMosaic,
        metadata: &Directory,
    ) -> Result<(), DngWriteError>
    where
        W: Write + Seek,
    {
        match self.compression() {
            DngCompression::Uncompressed => {
                write_uncompressed(writer, mosaic, metadata, self.version())?;
            }
            DngCompression::LosslessJpeg => {
                write_lossless(writer, mosaic, metadata, self.version())?;
            }
            DngCompression::Deflate => {
                write_deflate_all(writer, mosaic, metadata, self.version())?;
            }
            DngCompression::Zstd => {
                write_zstd(writer, mosaic, metadata, self.version(), self.zstd_level)?;
            }
        }
        Ok(())
    }
}

fn write_uncompressed<W>(
    writer: W,
    mosaic: &RawMosaic,
    metadata: &Directory,
    version: DngVersion,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    let (width, height) = mosaic.dimensions();
    let mut encoder = ContainerWriter::new(writer).map_err(tiff_write_error)?;
    let mut directory = TiffDirectory::new(&mut encoder);

    let (bits_per_sample, sample_format) = sample_layout(mosaic);
    let encoded = encode_for_chunks(
        mosaic,
        usize::try_from(width).expect("u32 fits usize on supported platforms"),
        usize::try_from(height).expect("u32 fits usize on supported platforms"),
        CodecCompression::Uncompressed,
        CodecPredictor::None,
    )?;
    let [data] = encoded.as_slice() else {
        return Err(invalid_metadata(
            "uncompressed encoding must produce exactly one strip",
        ));
    };
    let strip_offset = directory.write_data(data)?;
    let strip_byte_count = data.len();

    let strip_offset = u32::try_from(strip_offset)
        .map_err(|_| invalid_metadata("uncompressed DNG offset exceeds classic TIFF limits"))?;
    let strip_byte_count = u32::try_from(strip_byte_count)
        .map_err(|_| invalid_metadata("uncompressed DNG strip exceeds classic TIFF limits"))?;

    directory.write_tag(Tag::from(TiffTag::NewSubfileType), 0_u32)?;
    directory.write_tag(Tag::from(TiffTag::ImageWidth), width)?;
    directory.write_tag(Tag::from(TiffTag::ImageLength), height)?;
    directory.write_tag(Tag::from(TiffTag::BitsPerSample), bits_per_sample)?;
    directory.write_tag(Tag::from(TiffTag::Compression), 1_u16)?;
    directory.write_tag(
        TiffTag::PhotometricInterpretation,
        u16::from(PhotometricInterpretation::Cfa),
    )?;
    directory.write_tag(Tag::from(TiffTag::StripOffsets), &[strip_offset][..])?;
    directory.write_tag(Tag::from(TiffTag::SamplesPerPixel), 1_u16)?;
    directory.write_tag(Tag::from(TiffTag::RowsPerStrip), height)?;
    directory.write_tag(Tag::from(TiffTag::StripByteCounts), &[strip_byte_count][..])?;
    directory.write_tag(Tag::from(TiffTag::PlanarConfiguration), 1_u16)?;
    directory.write_tag(Tag::from(TiffTag::SampleFormat), sample_format)?;

    let dng_version = [version.major, version.minor, version.patch, version.build];
    directory.write_tag(Tag(u16::from(DngTag::DngVersion)), dng_version)?;
    directory.write_tag(Tag(u16::from(DngTag::DngBackwardVersion)), dng_version)?;

    for (tag, value) in metadata {
        if structural_metadata(*tag) {
            continue;
        }
        write_metadata_value(&mut directory, *tag, value)?;
    }

    directory.finish()?;
    Ok(())
}

fn write_lossless<W>(
    writer: W,
    mosaic: &RawMosaic,
    metadata: &Directory,
    version: DngVersion,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    match mosaic {
        RawMosaic::U8(_) | RawMosaic::U16(_) => {
            write_lossless_jpeg(writer, mosaic, metadata, version)
        }
        RawMosaic::U32(_) | RawMosaic::F32(_) => {
            write_lossless_deflate(writer, mosaic, metadata, version)
        }
    }
}

fn write_lossless_jpeg<W>(
    writer: W,
    mosaic: &RawMosaic,
    metadata: &Directory,
    version: DngVersion,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    const TILE: usize = 256;
    let (width, height) = mosaic.dimensions();
    let (bits, sample_format) = sample_layout(mosaic);
    let chunks = encode_for_chunks(
        mosaic,
        TILE,
        TILE,
        CodecCompression::LosslessJpeg,
        CodecPredictor::None,
    )?;
    write_tiled_payload(
        writer,
        width,
        height,
        metadata,
        version,
        7,
        bits,
        sample_format,
        None,
        &chunks,
    )
}

fn write_lossless_deflate<W>(
    writer: W,
    mosaic: &RawMosaic,
    metadata: &Directory,
    version: DngVersion,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    const TILE: usize = 256;
    let (width, height) = mosaic.dimensions();
    let (bits, sample_format) = sample_layout(mosaic);
    let (predictor, predictor_tag) = match mosaic {
        RawMosaic::U32(_) => (CodecPredictor::None, 1_u16),
        RawMosaic::F32(_) => (CodecPredictor::FloatingPoint { byte_stride: 1 }, 3_u16),
        _ => unreachable!("the caller selects Deflate only for wider samples"),
    };
    let chunks = encode_for_chunks(mosaic, TILE, TILE, CodecCompression::Deflate, predictor)?;
    write_tiled_payload(
        writer,
        width,
        height,
        metadata,
        version,
        8,
        bits,
        sample_format,
        Some(predictor_tag),
        &chunks,
    )
}

#[cfg(feature = "zstd")]
fn write_zstd<W>(
    writer: W,
    mosaic: &RawMosaic,
    metadata: &Directory,
    version: DngVersion,
    level: i32,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    let (width, height) = mosaic.dimensions();
    let (bits, sample_format) = sample_layout(mosaic);
    const TILE: usize = 256;
    let request = encode_request(mosaic, TILE, TILE, CodecPredictor::None);
    let chunks = ZstdCodec::new(level)
        .encode(&request)
        .map_err(codec_write_error)?
        .chunks;
    write_tiled_payload(
        writer,
        width,
        height,
        metadata,
        version,
        50_000,
        bits,
        sample_format,
        Some(1),
        &chunks,
    )
}

#[cfg(not(feature = "zstd"))]
fn write_zstd<W>(
    _writer: W,
    _mosaic: &RawMosaic,
    _metadata: &Directory,
    _version: DngVersion,
    _level: i32,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    Err(DngWriteError::Unsupported {
        operation: "Zstandard DNG writing requires the fornax-dng zstd feature".into(),
    })
}

fn write_deflate_all<W>(
    writer: W,
    mosaic: &RawMosaic,
    metadata: &Directory,
    version: DngVersion,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    let (width, height) = mosaic.dimensions();
    let (bits, sample_format) = sample_layout(mosaic);
    let chunks = encode_for_chunks(
        mosaic,
        256,
        256,
        CodecCompression::Deflate,
        CodecPredictor::None,
    )?;
    write_tiled_payload(
        writer,
        width,
        height,
        metadata,
        version,
        8,
        bits,
        sample_format,
        Some(1),
        &chunks,
    )
}

fn encode_for_chunks(
    mosaic: &RawMosaic,
    chunk_width: usize,
    chunk_height: usize,
    compression: CodecCompression,
    predictor: CodecPredictor,
) -> Result<Vec<Vec<u8>>, DngWriteError> {
    let request = encode_request(mosaic, chunk_width, chunk_height, predictor);
    encode_codec(compression, &request)
        .map(|encoded| encoded.chunks)
        .map_err(codec_write_error)
}

const fn encode_request(
    mosaic: &RawMosaic,
    chunk_width: usize,
    chunk_height: usize,
    predictor: CodecPredictor,
) -> EncodeRequest<'_> {
    EncodeRequest {
        image: mosaic,
        chunk_width,
        chunk_height,
        byte_order: CodecByteOrder::Little,
        predictor,
    }
}

fn write_tiled_payload<W>(
    writer: W,
    width: u32,
    height: u32,
    metadata: &Directory,
    version: DngVersion,
    compression_tag: u16,
    bits_per_sample: u16,
    sample_format: u16,
    predictor_tag: Option<u16>,
    chunks: &[Vec<u8>],
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    const TILE: usize = 256;
    let mut encoder = ContainerWriter::new(writer).map_err(tiff_write_error)?;
    let mut directory = TiffDirectory::new(&mut encoder);
    let mut tile_offsets = Vec::with_capacity(chunks.len());
    let mut tile_byte_counts = Vec::with_capacity(chunks.len());
    for tile in chunks {
        let offset = directory.write_data(tile)?;
        tile_offsets.push(
            u32::try_from(offset).map_err(|_| invalid_metadata("DNG tile offset exceeds u32"))?,
        );
        tile_byte_counts.push(
            u32::try_from(tile.len())
                .map_err(|_| invalid_metadata("DNG tile exceeds u32 bytes"))?,
        );
    }
    write_common_directory(
        &mut directory,
        width,
        height,
        bits_per_sample,
        sample_format,
        compression_tag,
        version,
    )?;
    directory.write_tag(
        Tag::from(TiffTag::TileWidth),
        u32::try_from(TILE).expect("tile width fits u32"),
    )?;
    directory.write_tag(
        Tag::from(TiffTag::TileLength),
        u32::try_from(TILE).expect("tile length fits u32"),
    )?;
    directory.write_tag(Tag::from(TiffTag::TileOffsets), tile_offsets.as_slice())?;
    directory.write_tag(
        Tag::from(TiffTag::TileByteCounts),
        tile_byte_counts.as_slice(),
    )?;
    if let Some(predictor) = predictor_tag {
        directory.write_tag(Tag::from(TiffTag::Predictor), predictor)?;
    }
    write_preserved_metadata(&mut directory, metadata)?;
    directory.finish().map_err(DngWriteError::from)
}

fn write_common_directory<W>(
    directory: &mut TiffDirectory<'_, W>,
    width: u32,
    height: u32,
    bits: u16,
    sample_format: u16,
    compression: u16,
    version: DngVersion,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    directory.write_tag(Tag::from(TiffTag::NewSubfileType), 0_u32)?;
    directory.write_tag(Tag::from(TiffTag::ImageWidth), width)?;
    directory.write_tag(Tag::from(TiffTag::ImageLength), height)?;
    directory.write_tag(Tag::from(TiffTag::BitsPerSample), bits)?;
    directory.write_tag(Tag::from(TiffTag::Compression), compression)?;
    directory.write_tag(
        TiffTag::PhotometricInterpretation,
        u16::from(PhotometricInterpretation::Cfa),
    )?;
    directory.write_tag(Tag::from(TiffTag::SamplesPerPixel), 1_u16)?;
    directory.write_tag(Tag::from(TiffTag::PlanarConfiguration), 1_u16)?;
    directory.write_tag(Tag::from(TiffTag::SampleFormat), sample_format)?;
    let dng_version = [version.major, version.minor, version.patch, version.build];
    directory.write_tag(Tag(u16::from(DngTag::DngVersion)), dng_version)?;
    directory.write_tag(Tag(u16::from(DngTag::DngBackwardVersion)), dng_version)?;
    Ok(())
}

const fn sample_layout(mosaic: &RawMosaic) -> (u16, u16) {
    match mosaic {
        RawMosaic::U8(_) => (8, 1),
        RawMosaic::U16(_) => (16, 1),
        RawMosaic::U32(_) => (32, 1),
        RawMosaic::F32(_) => (32, 3),
    }
}

fn codec_write_error(error: fornax_codec::CodecError) -> DngWriteError {
    match error {
        fornax_codec::CodecError::InvalidInput(detail) => DngWriteError::InvalidMetadata { detail },
        fornax_codec::CodecError::Unsupported(detail) => {
            DngWriteError::Unsupported { operation: detail }
        }
    }
}

fn write_preserved_metadata<W>(
    directory: &mut TiffDirectory<'_, W>,
    metadata: &Directory,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    for (tag, value) in metadata {
        if structural_metadata(*tag) {
            continue;
        }
        write_metadata_value(directory, *tag, value)?;
    }
    Ok(())
}

fn structural_metadata(tag: Tag) -> bool {
    matches!(
        TiffTag::from(tag),
        TiffTag::NewSubfileType
            | TiffTag::ImageWidth
            | TiffTag::ImageLength
            | TiffTag::BitsPerSample
            | TiffTag::Compression
            | TiffTag::PhotometricInterpretation
            | TiffTag::SamplesPerPixel
            | TiffTag::RowsPerStrip
            | TiffTag::StripOffsets
            | TiffTag::StripByteCounts
            | TiffTag::PlanarConfiguration
            | TiffTag::Predictor
            | TiffTag::TileWidth
            | TiffTag::TileLength
            | TiffTag::TileOffsets
            | TiffTag::TileByteCounts
            | TiffTag::SubIfds
            | TiffTag::SampleFormat
    ) || matches!(
        DngTag::from(tag),
        DngTag::DngVersion
            | DngTag::DngBackwardVersion
            | DngTag::LinearizationTable
            | DngTag::Unknown((50_974..=50_976) | 51_008)
    )
}

fn write_metadata_value<W>(
    directory: &mut TiffDirectory<'_, W>,
    tag: Tag,
    value: &Value,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    match value {
        Value::Byte(_)
        | Value::SignedByte(_)
        | Value::Short(_)
        | Value::SignedShort(_)
        | Value::Unsigned(_)
        | Value::Signed(_)
        | Value::UnsignedBig(_)
        | Value::SignedBig(_)
        | Value::Float(_)
        | Value::Double(_)
        | Value::Rational(_, _)
        | Value::SRational(_, _)
        | Value::Ascii(_) => directory
            .write_tag(tag, value.clone())
            .map_err(DngWriteError::from),
        Value::RationalBig(_, _) | Value::SRationalBig(_, _) | Value::Ifd(_) | Value::IfdBig(_) => {
            Ok(())
        }
        Value::List(values) => write_list(directory, tag, values),
    }
}

fn write_list<W>(
    directory: &mut TiffDirectory<'_, W>,
    tag: Tag,
    values: &[Value],
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    if values.is_empty() {
        return Err(invalid_metadata(
            "empty metadata lists cannot be serialized",
        ));
    }
    match values.first() {
        Some(Value::RationalBig(_, _) | Value::SRationalBig(_, _)) => {
            return Err(invalid_metadata(
                "BigTIFF rational lists are not supported by the classic writer",
            ));
        }
        Some(Value::Ascii(_) | Value::Ifd(_) | Value::IfdBig(_)) => {
            return Err(invalid_metadata(
                "layout-dependent or string list metadata cannot be serialized",
            ));
        }
        Some(Value::List(_)) => {
            return Err(invalid_metadata(
                "nested TIFF metadata lists are unsupported",
            ));
        }
        Some(_) => {}
        None => unreachable!("the list was checked as non-empty"),
    }
    directory
        .write_tag(tag, Value::List(values.to_vec()))
        .map_err(DngWriteError::from)
}

fn invalid_metadata(detail: impl Into<Box<str>>) -> DngWriteError {
    DngWriteError::InvalidMetadata {
        detail: detail.into(),
    }
}

fn tiff_write_error(error: fornax_tiff::TiffError) -> DngWriteError {
    DngWriteError::Io {
        operation: "encoding DNG".into(),
        detail: error.to_string().into(),
    }
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    #[cfg(not(feature = "zstd"))]
    use fornax_core::MonoImage;
    use fornax_tiff::TiffWriter;

    use super::*;

    #[test]
    fn classic_writer_rejects_big_rational_metadata_lists() {
        let mut cursor = Cursor::new(Vec::new());
        let mut container = TiffWriter::new(&mut cursor).unwrap();
        let mut directory = TiffDirectory::new(&mut container);

        let error = write_list(
            &mut directory,
            Tag(1),
            &[Value::RationalBig(1, 2), Value::RationalBig(3, 4)],
        )
        .expect_err("classic TIFF cannot represent BigTIFF rationals");

        assert!(matches!(error, DngWriteError::InvalidMetadata { .. }));
    }

    #[cfg(not(feature = "zstd"))]
    #[test]
    fn zstd_write_without_feature_returns_unsupported() {
        let mosaic = RawMosaic::U8(MonoImage::from_raw(1, 1, vec![0]).unwrap());
        let mut output = Cursor::new(Vec::new());

        let error = DngWriter::new(DngCompression::Zstd)
            .write_dng_with(&mut output, &mosaic, &Directory::default())
            .expect_err("zstd feature is disabled");

        assert!(matches!(error, DngWriteError::Unsupported { .. }));
    }
}
