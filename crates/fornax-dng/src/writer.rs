use std::fs::File;
use std::io::{BufWriter, Seek, Write};
use std::path::Path;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use fornax_core::RawMosaic;
use fornax_tiff::{Tag, TiffWriter as ContainerWriter, Value};
use thiserror::Error;

use crate::ljpeg_writer as ljpeg;
use crate::metadata::{DngMetadata, DngTag, DngValue};

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

struct DngDirectory<'a, W> {
    writer: &'a mut ContainerWriter<W>,
    entries: Vec<(Tag, Value)>,
}

impl<'a, W> DngDirectory<'a, W>
where
    W: Write + Seek,
{
    const fn new(writer: &'a mut ContainerWriter<W>) -> Self {
        Self {
            writer,
            entries: Vec::new(),
        }
    }

    fn write_data(&mut self, data: &[u8]) -> Result<u64, DngWriteError> {
        self.writer.write_data(data).map_err(tiff_write_error)
    }

    fn write_tag(&mut self, tag: Tag, value: impl IntoDngValue) -> Result<(), DngWriteError> {
        self.entries.push((tag, value.into_dng_value()));
        Ok(())
    }

    fn finish(self) -> Result<(), DngWriteError> {
        self.writer.finish(self.entries).map_err(tiff_write_error)
    }
}

trait IntoDngValue {
    fn into_dng_value(self) -> DngValue;
}

impl IntoDngValue for u8 {
    fn into_dng_value(self) -> DngValue { DngValue::Byte(self) }
}

impl IntoDngValue for u16 {
    fn into_dng_value(self) -> DngValue { DngValue::Short(self) }
}

impl IntoDngValue for u32 {
    fn into_dng_value(self) -> DngValue { DngValue::Unsigned(self) }
}

impl IntoDngValue for f32 {
    fn into_dng_value(self) -> DngValue { DngValue::Float(self) }
}

impl IntoDngValue for &str {
    fn into_dng_value(self) -> DngValue { DngValue::Ascii(self.to_owned()) }
}

impl IntoDngValue for [u8; 4] {
    fn into_dng_value(self) -> DngValue {
        DngValue::List(self.into_iter().map(DngValue::Byte).collect())
    }
}

impl IntoDngValue for &[u16] {
    fn into_dng_value(self) -> DngValue {
        DngValue::List(self.iter().copied().map(DngValue::Short).collect())
    }
}

impl IntoDngValue for &[u32] {
    fn into_dng_value(self) -> DngValue {
        DngValue::List(self.iter().copied().map(DngValue::Unsigned).collect())
    }
}

impl IntoDngValue for DngValue {
    fn into_dng_value(self) -> DngValue { self }
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

    /// Returns the selected version used for regenerated output.
    pub const fn version(self) -> DngVersion { self.version }

    pub fn write<P>(
        &self,
        path: P,
        mosaic: &RawMosaic,
        metadata: &DngMetadata,
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
        metadata: &DngMetadata,
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
        }
        Ok(())
    }
}

fn write_uncompressed<W>(
    writer: W,
    mosaic: &RawMosaic,
    metadata: &DngMetadata,
    version: DngVersion,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    let (width, height) = mosaic.dimensions();
    let mut encoder = ContainerWriter::new(writer).map_err(tiff_write_error)?;
    let mut directory = DngDirectory::new(&mut encoder);

    let (strip_offset, strip_byte_count, bits_per_sample, sample_format) = match mosaic {
        RawMosaic::U8(image) => {
            let data = image.as_raw();
            (
                directory.write_data(data.as_slice())?,
                data.len(),
                8_u16,
                1_u16,
            )
        }
        RawMosaic::U16(image) => {
            let data = image.as_raw();
            let bytes = data
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect::<Vec<_>>();
            (directory.write_data(&bytes)?, bytes.len(), 16_u16, 1_u16)
        }
        RawMosaic::U32(image) => {
            let data = image.as_raw();
            let bytes = data
                .iter()
                .flat_map(|value| value.to_le_bytes())
                .collect::<Vec<_>>();
            (directory.write_data(&bytes)?, bytes.len(), 32_u16, 1_u16)
        }
        RawMosaic::F32(image) => {
            let data = image.as_raw();
            let bytes = data
                .iter()
                .flat_map(|value| value.to_bits().to_le_bytes())
                .collect::<Vec<_>>();
            (directory.write_data(&bytes)?, bytes.len(), 32_u16, 3_u16)
        }
    };

    let strip_offset = u32::try_from(strip_offset)
        .map_err(|_| invalid_metadata("uncompressed DNG offset exceeds classic TIFF limits"))?;
    let strip_byte_count = u32::try_from(strip_byte_count)
        .map_err(|_| invalid_metadata("uncompressed DNG strip exceeds classic TIFF limits"))?;

    directory.write_tag(Tag::NEW_SUBFILE_TYPE, 0_u32)?;
    directory.write_tag(Tag::IMAGE_WIDTH, width)?;
    directory.write_tag(Tag::IMAGE_LENGTH, height)?;
    directory.write_tag(Tag::BITS_PER_SAMPLE, bits_per_sample)?;
    directory.write_tag(Tag::COMPRESSION, 1_u16)?;
    directory.write_tag(Tag::PHOTOMETRIC_INTERPRETATION, 32_803_u16)?;
    directory.write_tag(Tag::STRIP_OFFSETS, &[strip_offset][..])?;
    directory.write_tag(Tag::SAMPLES_PER_PIXEL, 1_u16)?;
    directory.write_tag(Tag::ROWS_PER_STRIP, height)?;
    directory.write_tag(Tag::STRIP_BYTE_COUNTS, &[strip_byte_count][..])?;
    directory.write_tag(Tag::PLANAR_CONFIGURATION, 1_u16)?;
    directory.write_tag(Tag::SAMPLE_FORMAT, sample_format)?;

    let dng_version = [version.major, version.minor, version.patch, version.build];
    directory.write_tag(Tag(u16::from(DngTag::DngVersion)), dng_version)?;
    directory.write_tag(Tag(u16::from(DngTag::DngBackwardVersion)), dng_version)?;

    for (tag, value) in &**metadata {
        if structural_metadata(*tag) {
            continue;
        }
        write_metadata_value(&mut directory, metadata_tag(*tag), value)?;
    }

    directory.finish()?;
    Ok(())
}

fn write_lossless<W>(
    writer: W,
    mosaic: &RawMosaic,
    metadata: &DngMetadata,
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
    metadata: &DngMetadata,
    version: DngVersion,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    const TILE: usize = 256;
    let (width, height) = mosaic.dimensions();
    let width = usize::try_from(width)
        .map_err(|_| invalid_metadata("DNG width exceeds the platform address space"))?;
    let height = usize::try_from(height)
        .map_err(|_| invalid_metadata("DNG height exceeds the platform address space"))?;
    let across = width.div_ceil(TILE);
    let down = height.div_ceil(TILE);
    let bits = if matches!(mosaic, RawMosaic::U8(_)) {
        8_u8
    } else {
        16_u8
    };

    let source: Vec<u16> = match mosaic {
        RawMosaic::U8(image) => image
            .as_raw()
            .iter()
            .map(|value| u16::from(*value))
            .collect(),
        RawMosaic::U16(image) => image.as_raw().clone(),
        _ => unreachable!("the caller selects JPEG-92 only for 8/16-bit integers"),
    };
    let mut tiles = Vec::with_capacity(across * down);
    for tile_y in 0..down {
        for tile_x in 0..across {
            let mut tile = vec![0_u16; TILE * TILE];
            let copy_width = TILE.min(width - tile_x * TILE);
            let copy_height = TILE.min(height - tile_y * TILE);
            for row in 0..copy_height {
                let source_start = (tile_y * TILE + row) * width + tile_x * TILE;
                let target_start = row * TILE;
                tile[target_start..target_start + copy_width]
                    .copy_from_slice(&source[source_start..source_start + copy_width]);
            }
            tiles.push(
                ljpeg::encode(&tile, TILE as u16, TILE as u16, bits).map_err(|detail| {
                    invalid_metadata(format!("lossless JPEG encoding: {detail}"))
                })?,
            );
        }
    }

    let mut encoder = ContainerWriter::new(writer).map_err(tiff_write_error)?;
    let mut directory = DngDirectory::new(&mut encoder);
    let mut tile_offsets = Vec::with_capacity(tiles.len());
    let mut tile_byte_counts = Vec::with_capacity(tiles.len());
    for tile in &tiles {
        let offset = directory.write_data(tile.as_slice())?;
        tile_offsets.push(
            u32::try_from(offset)
                .map_err(|_| invalid_metadata("lossless DNG tile offset exceeds u32"))?,
        );
        tile_byte_counts.push(
            u32::try_from(tile.len())
                .map_err(|_| invalid_metadata("lossless DNG tile exceeds u32 bytes"))?,
        );
    }

    write_common_directory(
        &mut directory,
        u32::try_from(width).expect("source width fits u32"),
        u32::try_from(height).expect("source height fits u32"),
        bits.into(),
        1_u16,
        7,
        version,
    )?;
    directory.write_tag(
        Tag::TILE_WIDTH,
        u32::try_from(TILE).expect("tile width fits u32"),
    )?;
    directory.write_tag(
        Tag::TILE_LENGTH,
        u32::try_from(TILE).expect("tile length fits u32"),
    )?;
    directory.write_tag(Tag::TILE_OFFSETS, tile_offsets.as_slice())?;
    directory.write_tag(Tag::TILE_BYTE_COUNTS, tile_byte_counts.as_slice())?;
    write_preserved_metadata(&mut directory, metadata)?;
    directory.finish()?;
    Ok(())
}

fn write_lossless_deflate<W>(
    writer: W,
    mosaic: &RawMosaic,
    metadata: &DngMetadata,
    version: DngVersion,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    const TILE: usize = 256;
    let (width, height) = mosaic.dimensions();
    let width = usize::try_from(width)
        .map_err(|_| invalid_metadata("DNG width exceeds the platform address space"))?;
    let height = usize::try_from(height)
        .map_err(|_| invalid_metadata("DNG height exceeds the platform address space"))?;
    let (bits, sample_format, predictor) = match mosaic {
        RawMosaic::U32(_) => (32_u16, 1_u16, 1_u16),
        RawMosaic::F32(_) => (32_u16, 3_u16, 3_u16),
        _ => unreachable!("the caller selects Deflate only for wider samples"),
    };
    let across = width.div_ceil(TILE);
    let down = height.div_ceil(TILE);
    let mut tiles = Vec::with_capacity(across * down);
    for tile_y in 0..down {
        for tile_x in 0..across {
            let mut bytes = vec![0_u8; TILE * TILE * size_of::<u32>()];
            let copy_width = TILE.min(width - tile_x * TILE);
            let copy_height = TILE.min(height - tile_y * TILE);
            for row in 0..copy_height {
                let source_start = (tile_y * TILE + row) * width + tile_x * TILE;
                let target_start = row * TILE * size_of::<u32>();
                match mosaic {
                    RawMosaic::U32(image) => {
                        for (value, target) in image.as_raw()
                            [source_start..source_start + copy_width]
                            .iter()
                            .zip(
                                bytes[target_start..]
                                    .as_chunks_mut::<{ size_of::<u32>() }>()
                                    .0
                                    .iter_mut(),
                            )
                        {
                            target.copy_from_slice(&value.to_le_bytes());
                        }
                    }
                    RawMosaic::F32(image) => {
                        for (value, target) in image.as_raw()
                            [source_start..source_start + copy_width]
                            .iter()
                            .zip(
                                bytes[target_start..]
                                    .as_chunks_mut::<{ size_of::<u32>() }>()
                                    .0
                                    .iter_mut(),
                            )
                        {
                            target.copy_from_slice(&value.to_bits().to_be_bytes());
                        }
                    }
                    _ => unreachable!("the caller selects Deflate only for wider samples"),
                }
            }
            if predictor == 3 {
                for row in bytes
                    .as_chunks_mut::<{ TILE * size_of::<u32>() }>()
                    .0
                    .iter_mut()
                {
                    float_horizontal_predictor(row, TILE);
                }
            }
            let mut compressor = ZlibEncoder::new(Vec::new(), Compression::fast());
            compressor
                .write_all(&bytes)
                .map_err(|error| invalid_metadata(format!("deflate setup failed: {error}")))?;
            tiles.push(
                compressor.finish().map_err(|error| {
                    invalid_metadata(format!("deflate encoding failed: {error}"))
                })?,
            );
        }
    }

    let mut encoder = ContainerWriter::new(writer).map_err(tiff_write_error)?;
    let mut directory = DngDirectory::new(&mut encoder);
    let mut tile_offsets = Vec::with_capacity(tiles.len());
    let mut tile_byte_counts = Vec::with_capacity(tiles.len());
    for tile in &tiles {
        let offset = directory.write_data(tile.as_slice())?;
        tile_offsets.push(
            u32::try_from(offset)
                .map_err(|_| invalid_metadata("lossless DNG tile offset exceeds u32"))?,
        );
        tile_byte_counts.push(
            u32::try_from(tile.len())
                .map_err(|_| invalid_metadata("lossless DNG tile exceeds u32 bytes"))?,
        );
    }
    write_common_directory(
        &mut directory,
        u32::try_from(width).expect("source width fits u32"),
        u32::try_from(height).expect("source height fits u32"),
        bits,
        sample_format,
        8,
        version,
    )?;
    directory.write_tag(
        Tag::TILE_WIDTH,
        u32::try_from(TILE).expect("tile width fits u32"),
    )?;
    directory.write_tag(
        Tag::TILE_LENGTH,
        u32::try_from(TILE).expect("tile length fits u32"),
    )?;
    directory.write_tag(Tag::TILE_OFFSETS, tile_offsets.as_slice())?;
    directory.write_tag(Tag::TILE_BYTE_COUNTS, tile_byte_counts.as_slice())?;
    directory.write_tag(Tag::PREDICTOR, predictor)?;
    write_preserved_metadata(&mut directory, metadata)?;
    directory.finish()?;
    Ok(())
}

fn float_horizontal_predictor(row: &mut [u8], width: usize) {
    let interleaved = row.to_vec();
    for sample in 0..width {
        for byte in 0..size_of::<u32>() {
            row[sample + byte * width] = interleaved[sample * size_of::<u32>() + byte];
        }
    }
    for index in (1..row.len()).rev() {
        row[index] = row[index].wrapping_sub(row[index - 1]);
    }
}

fn write_common_directory<W>(
    directory: &mut DngDirectory<'_, W>,
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
    directory.write_tag(Tag::NEW_SUBFILE_TYPE, 0_u32)?;
    directory.write_tag(Tag::IMAGE_WIDTH, width)?;
    directory.write_tag(Tag::IMAGE_LENGTH, height)?;
    directory.write_tag(Tag::BITS_PER_SAMPLE, bits)?;
    directory.write_tag(Tag::COMPRESSION, compression)?;
    directory.write_tag(Tag::PHOTOMETRIC_INTERPRETATION, 32_803_u16)?;
    directory.write_tag(Tag::SAMPLES_PER_PIXEL, 1_u16)?;
    directory.write_tag(Tag::PLANAR_CONFIGURATION, 1_u16)?;
    directory.write_tag(Tag::SAMPLE_FORMAT, sample_format)?;
    let dng_version = [version.major, version.minor, version.patch, version.build];
    directory.write_tag(Tag(u16::from(DngTag::DngVersion)), dng_version)?;
    directory.write_tag(Tag(u16::from(DngTag::DngBackwardVersion)), dng_version)?;
    Ok(())
}

fn write_preserved_metadata<W>(
    directory: &mut DngDirectory<'_, W>,
    metadata: &DngMetadata,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    for (tag, value) in &**metadata {
        if structural_metadata(*tag) {
            continue;
        }
        write_metadata_value(directory, metadata_tag(*tag), value)?;
    }
    Ok(())
}

const fn structural_metadata(tag: DngTag) -> bool {
    matches!(
        tag,
        DngTag::NewSubfileType
            | DngTag::ImageWidth
            | DngTag::ImageLength
            | DngTag::BitsPerSample
            | DngTag::Compression
            | DngTag::PhotometricInterpretation
            | DngTag::SamplesPerPixel
            | DngTag::RowsPerStrip
            | DngTag::StripOffsets
            | DngTag::StripByteCounts
            | DngTag::PlanarConfiguration
            | DngTag::TileWidth
            | DngTag::TileLength
            | DngTag::TileOffsets
            | DngTag::TileByteCounts
            | DngTag::SubIfds
            | DngTag::DngVersion
            | DngTag::DngBackwardVersion
            | DngTag::LinearizationTable
            | DngTag::Unknown((50_974..=50_976) | 51_008)
    )
}

fn metadata_tag(tag: DngTag) -> Tag { tag.into() }

fn write_metadata_value<W>(
    directory: &mut DngDirectory<'_, W>,
    tag: Tag,
    value: &DngValue,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    match value {
        DngValue::Byte(_)
        | DngValue::SignedByte(_)
        | DngValue::Short(_)
        | DngValue::SignedShort(_)
        | DngValue::Unsigned(_)
        | DngValue::Signed(_)
        | DngValue::UnsignedBig(_)
        | DngValue::SignedBig(_)
        | DngValue::Float(_)
        | DngValue::Double(_)
        | DngValue::Rational(_, _)
        | DngValue::SRational(_, _)
        | DngValue::Ascii(_) => directory.write_tag(tag, value.clone()),
        DngValue::RationalBig(_, _)
        | DngValue::SRationalBig(_, _)
        | DngValue::Ifd(_)
        | DngValue::IfdBig(_) => Ok(()),
        DngValue::List(values) => write_list(directory, tag, values),
    }
}

fn write_list<W>(
    directory: &mut DngDirectory<'_, W>,
    tag: Tag,
    values: &[DngValue],
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
        Some(DngValue::RationalBig(_, _) | DngValue::SRationalBig(_, _)) => {
            return Err(invalid_metadata(
                "BigTIFF rational lists are not supported by the classic writer",
            ));
        }
        Some(DngValue::Ascii(_) | DngValue::Ifd(_) | DngValue::IfdBig(_)) => {
            return Err(invalid_metadata(
                "layout-dependent or string list metadata cannot be serialized",
            ));
        }
        Some(DngValue::List(_)) => {
            return Err(invalid_metadata(
                "nested TIFF metadata lists are unsupported",
            ));
        }
        Some(_) => {}
        None => unreachable!("the list was checked as non-empty"),
    }
    directory.write_tag(tag, DngValue::List(values.to_vec()))
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

    use fornax_tiff::TiffWriter;

    use super::*;

    #[test]
    fn classic_writer_rejects_big_rational_metadata_lists() {
        let mut cursor = Cursor::new(Vec::new());
        let mut container = TiffWriter::new(&mut cursor).unwrap();
        let mut directory = DngDirectory::new(&mut container);

        let error = write_list(
            &mut directory,
            Tag(1),
            &[DngValue::RationalBig(1, 2), DngValue::RationalBig(3, 4)],
        )
        .expect_err("classic TIFF cannot represent BigTIFF rationals");

        assert!(matches!(error, DngWriteError::InvalidMetadata { .. }));
    }
}
