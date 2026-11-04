use std::io::{Seek, Write};

use tiff::encoder::{DirectoryEncoder, Rational, SRational, TiffEncoder, TiffKindStandard};
use tiff::tags::{SampleFormat, Tag};

use crate::{DecodedRaw, DngTag, DngValue, DngWriteError, MosaicData};

/// DNG compression modes written in the first implementation stage.
///
/// # Examples
///
/// ```
/// use fornax_decoder::DngCompression;
///
/// assert_eq!(DngCompression::default(), DngCompression::LosslessJpeg);
/// ```
#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
pub enum DngCompression {
    /// Uncompressed strip storage.
    Uncompressed,
    /// DNG lossless JPEG-92 strip or tile storage.
    #[default]
    LosslessJpeg,
}

/// Four-component DNG specification version stored in `DNGVersion`.
///
/// # Examples
///
/// ```
/// use fornax_decoder::DngVersion;
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

/// DNG writer configuration.
///
/// # Examples
///
/// ```
/// use fornax_decoder::{DngCompression, DngVersion, DngWriteOptions};
///
/// let options = DngWriteOptions::default();
/// assert_eq!(options.compression(), DngCompression::LosslessJpeg);
/// assert_eq!(options.version(), DngVersion::latest());
/// ```
#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DngWriteOptions {
    version: DngVersion,
    compression: DngCompression,
}

impl Default for DngWriteOptions {
    fn default() -> Self { Self::new(DngCompression::default()) }
}

impl DngWriteOptions {
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
}

pub fn write_uncompressed<W>(
    decoded: &DecodedRaw,
    writer: W,
    options: DngWriteOptions,
) -> Result<(), DngWriteError>
where
    W: Write + Seek,
{
    let (width, height) = decoded.dimensions();
    let mut encoder = TiffEncoder::new(writer).map_err(tiff_write_error)?;
    let mut directory = encoder.image_directory().map_err(tiff_write_error)?;

    let (strip_offset, strip_byte_count, bits_per_sample, sample_format) = match decoded.pixels() {
        MosaicData::U8(image) => {
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
        MosaicData::U16(image) => {
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
        MosaicData::U32(image) => {
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
        MosaicData::F32(image) => {
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

    let version = options.version();
    let dng_version = [version.major, version.minor, version.patch, version.build];
    directory
        .write_tag(Tag::Unknown(DngTag::DngVersion.into()), dng_version)
        .map_err(tiff_write_error)?;
    directory
        .write_tag(Tag::Unknown(DngTag::DngBackwardVersion.into()), dng_version)
        .map_err(tiff_write_error)?;

    for (tag, value) in decoded.metadata() {
        if is_regenerated_tag(*tag) {
            continue;
        }
        write_metadata_value(&mut directory, metadata_tag(*tag), value)?;
    }

    directory.finish().map_err(tiff_write_error)?;
    Ok(())
}

const fn is_regenerated_tag(tag: DngTag) -> bool {
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
    ) || matches!(
        tag,
        DngTag::Unknown(339 | 34_665 | 34_853 | 40_965 | 50_941)
    )
}

#[allow(
    clippy::missing_const_for_fn,
    reason = "the tiff Tag conversion is not a const function"
)]
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
