use std::fs::File;
use std::io::{BufWriter, Seek, Write};
use std::path::Path;

use flate2::Compression;
use flate2::write::ZlibEncoder;
use tiff::encoder::{DirectoryEncoder, Rational, SRational, TiffEncoder, TiffKindStandard};
use tiff::tags::{SampleFormat, Tag};

use crate::RawMosaic;
use crate::error::DngWriteError;
use crate::metadata::dng::{DngMetadata, DngTag, DngValue};

mod ljpeg;

/// DNG compression modes written in the first implementation stage.
///
/// # Examples
///
/// ```
/// use fornax_core::writer::DngCompression;
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
/// use fornax_core::writer::DngVersion;
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
    let mut encoder = TiffEncoder::new(writer).map_err(tiff_write_error)?;
    let mut directory = encoder.image_directory().map_err(tiff_write_error)?;

    let (strip_offset, strip_byte_count, bits_per_sample, sample_format) = match mosaic {
        RawMosaic::U8(image) => {
            let data = image.as_raw();
            (
                directory
                    .write_data(data.as_slice())
                    .map_err(tiff_write_error)?,
                data.len(),
                8_u16,
                SampleFormat::Uint.to_u16(),
            )
        }
        RawMosaic::U16(image) => {
            let data = image.as_raw();
            (
                directory
                    .write_data(data.as_slice())
                    .map_err(tiff_write_error)?,
                data.len() * size_of::<u16>(),
                16_u16,
                SampleFormat::Uint.to_u16(),
            )
        }
        RawMosaic::U32(image) => {
            let data = image.as_raw();
            (
                directory
                    .write_data(data.as_slice())
                    .map_err(tiff_write_error)?,
                data.len() * size_of::<u32>(),
                32_u16,
                SampleFormat::Uint.to_u16(),
            )
        }
        RawMosaic::F32(image) => {
            let data = image.as_raw();
            (
                directory
                    .write_data(data.as_slice())
                    .map_err(tiff_write_error)?,
                data.len() * size_of::<f32>(),
                32_u16,
                SampleFormat::IEEEFP.to_u16(),
            )
        }
    };

    let strip_offset = u32::try_from(strip_offset)
        .map_err(|_| invalid_metadata("uncompressed DNG offset exceeds classic TIFF limits"))?;
    let strip_byte_count = u32::try_from(strip_byte_count)
        .map_err(|_| invalid_metadata("uncompressed DNG strip exceeds classic TIFF limits"))?;

    directory
        .write_tag(Tag::NewSubfileType, 0_u32)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::ImageWidth, width)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::ImageLength, height)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::BitsPerSample, bits_per_sample)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::Compression, 1_u16)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::PhotometricInterpretation, 32_803_u16)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::StripOffsets, &[strip_offset][..])
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::SamplesPerPixel, 1_u16)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::RowsPerStrip, height)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::StripByteCounts, &[strip_byte_count][..])
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::PlanarConfiguration, 1_u16)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::SampleFormat, sample_format)
        .map_err(tiff_write_error)?;

    let dng_version = [version.major, version.minor, version.patch, version.build];
    directory
        .write_tag(Tag::Unknown(DngTag::DngVersion.into()), dng_version)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::Unknown(DngTag::DngBackwardVersion.into()), dng_version)
        .map_err(tiff_write_error)?;

    for (tag, value) in &**metadata {
        if structural_metadata(*tag) {
            continue;
        }
        write_metadata_value(&mut directory, metadata_tag(*tag), value)?;
    }

    directory.finish().map_err(tiff_write_error)?;
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

    let mut encoder = TiffEncoder::new(writer).map_err(tiff_write_error)?;
    let mut directory = encoder.image_directory().map_err(tiff_write_error)?;
    let mut tile_offsets = Vec::with_capacity(tiles.len());
    let mut tile_byte_counts = Vec::with_capacity(tiles.len());
    for tile in &tiles {
        let offset = directory
            .write_data(tile.as_slice())
            .map_err(tiff_write_error)?;
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
        SampleFormat::Uint.to_u16(),
        7,
        version,
    )?;
    directory
        .write_tag(
            Tag::TileWidth,
            u32::try_from(TILE).expect("tile width fits u32"),
        )
        .map_err(tiff_write_error)?;
    directory
        .write_tag(
            Tag::TileLength,
            u32::try_from(TILE).expect("tile length fits u32"),
        )
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::TileOffsets, tile_offsets.as_slice())
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::TileByteCounts, tile_byte_counts.as_slice())
        .map_err(tiff_write_error)?;
    write_preserved_metadata(&mut directory, metadata)?;
    directory.finish().map_err(tiff_write_error)?;
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
        RawMosaic::U32(_) => (32_u16, SampleFormat::Uint.to_u16(), 1_u16),
        RawMosaic::F32(_) => (32_u16, SampleFormat::IEEEFP.to_u16(), 3_u16),
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

    let mut encoder = TiffEncoder::new(writer).map_err(tiff_write_error)?;
    let mut directory = encoder.image_directory().map_err(tiff_write_error)?;
    let mut tile_offsets = Vec::with_capacity(tiles.len());
    let mut tile_byte_counts = Vec::with_capacity(tiles.len());
    for tile in &tiles {
        let offset = directory
            .write_data(tile.as_slice())
            .map_err(tiff_write_error)?;
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
    directory
        .write_tag(
            Tag::TileWidth,
            u32::try_from(TILE).expect("tile width fits u32"),
        )
        .map_err(tiff_write_error)?;
    directory
        .write_tag(
            Tag::TileLength,
            u32::try_from(TILE).expect("tile length fits u32"),
        )
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::TileOffsets, tile_offsets.as_slice())
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::TileByteCounts, tile_byte_counts.as_slice())
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::Predictor, predictor)
        .map_err(tiff_write_error)?;
    write_preserved_metadata(&mut directory, metadata)?;
    directory.finish().map_err(tiff_write_error)?;
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

#[allow(
    clippy::too_many_arguments,
    reason = "the directory writer mirrors the TIFF fields shared by both encodings"
)]
fn write_common_directory<W>(
    directory: &mut DirectoryEncoder<'_, W, TiffKindStandard>,
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
    directory
        .write_tag(Tag::NewSubfileType, 0_u32)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::ImageWidth, width)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::ImageLength, height)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::BitsPerSample, bits)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::Compression, compression)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::PhotometricInterpretation, 32_803_u16)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::SamplesPerPixel, 1_u16)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::PlanarConfiguration, 1_u16)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::SampleFormat, sample_format)
        .map_err(tiff_write_error)?;
    let dng_version = [version.major, version.minor, version.patch, version.build];
    directory
        .write_tag(Tag::Unknown(DngTag::DngVersion.into()), dng_version)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::Unknown(DngTag::DngBackwardVersion.into()), dng_version)
        .map_err(tiff_write_error)?;
    Ok(())
}

fn write_preserved_metadata<W>(
    directory: &mut DirectoryEncoder<'_, W, TiffKindStandard>,
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
    directory: &mut DirectoryEncoder<'_, W, TiffKindStandard>,
    tag: Tag,
    value: &DngValue,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    match value {
        DngValue::Byte(value) => write_tag(directory, tag, [*value]),
        DngValue::SignedByte(value) => write_tag(directory, tag, [*value]),
        DngValue::Short(value) => write_tag(directory, tag, [*value]),
        DngValue::SignedShort(value) => write_tag(directory, tag, [*value]),
        DngValue::Unsigned(value) => write_tag(directory, tag, [*value]),
        DngValue::Signed(value) => write_tag(directory, tag, [*value]),
        DngValue::UnsignedBig(value) => write_tag(directory, tag, [*value]),
        DngValue::SignedBig(value) => write_tag(directory, tag, [*value]),
        DngValue::Float(value) => write_tag(directory, tag, [*value]),
        DngValue::Double(value) => write_tag(directory, tag, [*value]),
        DngValue::Rational(numerator, denominator) => write_tag(
            directory,
            tag,
            [Rational {
                n: *numerator,
                d: *denominator,
            }],
        ),
        DngValue::SRational(numerator, denominator) => write_tag(
            directory,
            tag,
            [SRational {
                n: *numerator,
                d: *denominator,
            }],
        ),
        #[expect(deprecated, reason = "tiff exposes legacy BigTIFF rational variants")]
        DngValue::RationalBig(_, _)
        | DngValue::SRationalBig(_, _)
        | DngValue::Ifd(_)
        | DngValue::IfdBig(_) => Ok(()),
        DngValue::Ascii(value) => write_tag(directory, tag, value.as_str()),
        DngValue::List(values) => write_list(directory, tag, values),
        _ => Err(invalid_metadata(
            "unknown TIFF metadata value cannot be serialized",
        )),
    }
}

fn write_list<W>(
    directory: &mut DirectoryEncoder<'_, W, TiffKindStandard>,
    tag: Tag,
    values: &[DngValue],
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    let Some(first) = values.first() else {
        return Err(invalid_metadata(
            "empty metadata lists cannot be serialized",
        ));
    };
    match first {
        DngValue::Byte(_) => write_tag(
            directory,
            tag,
            collect_values(values, |value| match value {
                DngValue::Byte(value) => Some(*value),
                _ => None,
            })?
            .as_slice(),
        ),
        DngValue::SignedByte(_) => write_tag(
            directory,
            tag,
            collect_values(values, |value| match value {
                DngValue::SignedByte(value) => Some(*value),
                _ => None,
            })?
            .as_slice(),
        ),
        DngValue::Short(_) => write_tag(
            directory,
            tag,
            collect_values(values, |value| match value {
                DngValue::Short(value) => Some(*value),
                _ => None,
            })?
            .as_slice(),
        ),
        DngValue::SignedShort(_) => write_tag(
            directory,
            tag,
            collect_values(values, |value| match value {
                DngValue::SignedShort(value) => Some(*value),
                _ => None,
            })?
            .as_slice(),
        ),
        DngValue::Unsigned(_) => write_tag(
            directory,
            tag,
            collect_values(values, |value| match value {
                DngValue::Unsigned(value) => Some(*value),
                _ => None,
            })?
            .as_slice(),
        ),
        DngValue::Signed(_) => write_tag(
            directory,
            tag,
            collect_values(values, |value| match value {
                DngValue::Signed(value) => Some(*value),
                _ => None,
            })?
            .as_slice(),
        ),
        DngValue::UnsignedBig(_) => write_tag(
            directory,
            tag,
            collect_values(values, |value| match value {
                DngValue::UnsignedBig(value) => Some(*value),
                _ => None,
            })?
            .as_slice(),
        ),
        DngValue::SignedBig(_) => write_tag(
            directory,
            tag,
            collect_values(values, |value| match value {
                DngValue::SignedBig(value) => Some(*value),
                _ => None,
            })?
            .as_slice(),
        ),
        DngValue::Float(_) => write_tag(
            directory,
            tag,
            collect_values(values, |value| match value {
                DngValue::Float(value) => Some(*value),
                _ => None,
            })?
            .as_slice(),
        ),
        DngValue::Double(_) => write_tag(
            directory,
            tag,
            collect_values(values, |value| match value {
                DngValue::Double(value) => Some(*value),
                _ => None,
            })?
            .as_slice(),
        ),
        DngValue::Rational(_, _) => {
            let mut result = Vec::with_capacity(values.len());
            for value in values {
                let DngValue::Rational(numerator, denominator) = value else {
                    return Err(invalid_metadata("mixed TIFF metadata list types"));
                };
                result.push(Rational {
                    n: *numerator,
                    d: *denominator,
                });
            }
            write_tag(directory, tag, result.as_slice())
        }
        DngValue::SRational(_, _) => {
            let mut result = Vec::with_capacity(values.len());
            for value in values {
                let DngValue::SRational(numerator, denominator) = value else {
                    return Err(invalid_metadata("mixed TIFF metadata list types"));
                };
                result.push(SRational {
                    n: *numerator,
                    d: *denominator,
                });
            }
            write_tag(directory, tag, result.as_slice())
        }
        #[expect(deprecated, reason = "tiff exposes legacy BigTIFF rational variants")]
        DngValue::RationalBig(_, _) | DngValue::SRationalBig(_, _) => Err(invalid_metadata(
            "BigTIFF rational lists are not supported by the classic writer",
        )),
        DngValue::Ascii(_) | DngValue::Ifd(_) | DngValue::IfdBig(_) => Err(invalid_metadata(
            "layout-dependent or string list metadata cannot be serialized",
        )),
        DngValue::List(_) => Err(invalid_metadata(
            "nested TIFF metadata lists are unsupported",
        )),
        _ => Err(invalid_metadata(
            "unknown TIFF metadata list cannot be serialized",
        )),
    }
}

fn write_tag<W, T>(
    directory: &mut DirectoryEncoder<'_, W, TiffKindStandard>,
    tag: Tag,
    value: T,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
    T: tiff::encoder::TiffValue,
{
    directory.write_tag(tag, value).map_err(tiff_write_error)
}

fn collect_values<T>(
    values: &[DngValue],
    extract: impl Fn(&DngValue) -> Option<T>,
) -> Result<Vec<T>, DngWriteError> {
    values
        .iter()
        .map(|value| {
            extract(value).ok_or_else(|| invalid_metadata("mixed TIFF metadata list types"))
        })
        .collect()
}

fn invalid_metadata(detail: impl Into<Box<str>>) -> DngWriteError {
    DngWriteError::InvalidMetadata {
        detail: detail.into(),
    }
}

fn tiff_write_error(error: tiff::TiffError) -> DngWriteError {
    DngWriteError::Io {
        operation: "encoding DNG".into(),
        detail: error.to_string().into(),
    }
}
