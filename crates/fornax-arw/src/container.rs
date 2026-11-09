use fornax_core::error::DecodeError;
use fornax_tiff::constants::{PhotometricInterpretation, TiffTag};
use fornax_tiff::{Directory, ParsedTiff, Tag, Value};

pub const DNG_PRIVATE_DATA: Tag = Tag(50_740);
pub const SONY_CURVE: Tag = Tag(0x7010);
pub const SONY_RAW_IMAGE_SIZE: Tag = Tag(0x7038);
pub const SONY_OFFSET: Tag = Tag(0x7200);
pub const SONY_LENGTH: Tag = Tag(0x7201);
pub const SONY_KEY: Tag = Tag(0x7221);
pub const SONY_GRBG_WHITE_BALANCE: Tag = Tag(0x7303);
pub const SONY_BLACK_LEVEL_1: Tag = Tag(0x7300);
pub const SONY_RGGB_BLACK_LEVEL: Tag = Tag(0x7310);
pub const SONY_RGGB_WHITE_BALANCE: Tag = Tag(0x7313);
pub const SONY_WHITE_LEVEL: Tag = Tag(0x787f);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum CompressionFamily {
    Uncompressed,
    Arw1,
    Arw2,
    Arw6,
    SonyLosslessJpeg,
}

#[derive(Clone, Debug)]
pub struct RawLayout {
    pub compression: CompressionFamily,
    pub width: usize,
    pub stored_height: usize,
    pub bits_per_sample: u16,
    pub offsets: Vec<usize>,
    pub byte_counts: Vec<usize>,
    pub tile_width: Option<usize>,
    pub tile_height: Option<usize>,
}

impl RawLayout {
    pub fn parse(parsed: &ParsedTiff) -> Result<(usize, Self), DecodeError> {
        let make = parsed
            .root
            .get(&Tag::from(TiffTag::Make))
            .and_then(|value| match value {
                Value::Ascii(value) => Some(value.trim()),
                _ => None,
            })
            .unwrap_or_default();
        if !make.eq_ignore_ascii_case("SONY") && !make.eq_ignore_ascii_case("Hasselblad") {
            return Err(unsupported(format!(
                "ARW/NRW container is not a Sony image (Make={make:?})"
            )));
        }
        let (raw_ifd, directory) = select_raw_ifd(parsed)?;
        let width = required_usize(directory, TiffTag::ImageWidth)?;
        let height = required_usize(directory, TiffTag::ImageLength)?;
        if width == 0 || height == 0 {
            return Err(invalid("ARW image dimensions must not be zero"));
        }
        let photometric = PhotometricInterpretation::from(required_u16(
            directory,
            TiffTag::PhotometricInterpretation,
        )?);
        if photometric == PhotometricInterpretation::LinearRaw {
            return Err(unsupported(
                "reduced-resolution LinearRaw/YCbCr Sony images are unsupported",
            ));
        }
        if photometric != PhotometricInterpretation::Cfa {
            return Err(unsupported(format!(
                "Sony image uses unsupported photometric interpretation {photometric:?}"
            )));
        }

        let compression_id = required_u16(directory, TiffTag::Compression)?;
        let bits_per_sample =
            required_u16(directory, TiffTag::BitsPerSample).unwrap_or(match compression_id {
                32_767 => 8,
                _ => 16,
            });
        let samples_per_pixel = optional_u16(directory, TiffTag::SamplesPerPixel).unwrap_or(1);
        let (tile_width, tile_height, offsets, byte_counts) = layout(directory)?;
        if samples_per_pixel == 4 {
            return Err(unsupported(
                "Sony ARQ multi-sample pixel-shift images are unsupported",
            ));
        }
        if samples_per_pixel != 1 {
            return Err(unsupported(format!(
                "Sony image has {samples_per_pixel} samples per pixel; one CFA sample is required"
            )));
        }

        let total_bytes = byte_counts.iter().try_fold(0_usize, |total, count| {
            total
                .checked_add(*count)
                .ok_or_else(|| invalid("Sony compressed byte count overflows"))
        })?;
        let expected_bits = width
            .checked_mul(height)
            .and_then(|samples| samples.checked_mul(usize::from(bits_per_sample)))
            .ok_or_else(|| invalid("Sony image bit count overflows"))?;
        let stored_bytes = total_bytes
            .checked_mul(8)
            .ok_or_else(|| invalid("Sony compressed bit count overflows"))?;

        let (compression, stored_height) = match compression_id {
            1 => (CompressionFamily::Uncompressed, height),
            7 => (CompressionFamily::SonyLosslessJpeg, height),
            32_766 => (CompressionFamily::Arw6, height),
            32_767 if total_bytes == width.saturating_mul(height) => {
                (CompressionFamily::Arw2, height)
            }
            32_767 if bits_per_sample == 12 && expected_bits == stored_bytes => {
                (CompressionFamily::Uncompressed, height)
            }
            32_767 if bits_per_sample == 8 || bits_per_sample == 12 || bits_per_sample == 16 => {
                let height = height
                    .checked_add(8)
                    .ok_or_else(|| invalid("ARW1 height overflows"))?;
                (CompressionFamily::Arw1, height)
            }
            32_767 => {
                return Err(unsupported(format!(
                    "ARW compression 32767 with {bits_per_sample} bits per sample is unsupported"
                )));
            }
            value => {
                return Err(unsupported(format!(
                    "Sony ARW compression {value} is unsupported"
                )));
            }
        };

        if matches!(
            compression,
            CompressionFamily::Uncompressed | CompressionFamily::Arw1 | CompressionFamily::Arw2
        ) && offsets.len() != 1
        {
            return Err(unsupported(format!(
                "Sony compression {compression_id} uses {} strips; one strip is required",
                offsets.len()
            )));
        }

        Ok((
            raw_ifd,
            Self {
                compression,
                width,
                stored_height,
                bits_per_sample,
                offsets,
                byte_counts,
                tile_width,
                tile_height,
            },
        ))
    }
}

fn select_raw_ifd(parsed: &ParsedTiff) -> Result<(usize, &Directory), DecodeError> {
    parsed
        .directories
        .iter()
        .enumerate()
        .filter(|(_, directory)| {
            let reduced = optional_u64(directory.get(&Tag::from(TiffTag::NewSubfileType)))
                .is_some_and(|value| value & 1 != 0);
            let has_pixels = directory.contains_key(&Tag::from(TiffTag::StripOffsets))
                || directory.contains_key(&Tag::from(TiffTag::TileOffsets));
            !reduced && has_pixels
        })
        .max_by_key(|(_, directory)| {
            optional_usize(directory, TiffTag::ImageWidth)
                .unwrap_or(0)
                .saturating_mul(optional_usize(directory, TiffTag::ImageLength).unwrap_or(0))
        })
        .ok_or_else(|| unsupported("no full-resolution Sony CFA image IFD was found"))
}

type ImageLayout = (Option<usize>, Option<usize>, Vec<usize>, Vec<usize>);

fn layout(directory: &Directory) -> Result<ImageLayout, DecodeError> {
    if directory.contains_key(&Tag::from(TiffTag::TileOffsets)) {
        return Ok((
            Some(required_usize(directory, TiffTag::TileWidth)?),
            Some(required_usize(directory, TiffTag::TileLength)?),
            unsigned_vec(directory, TiffTag::TileOffsets)?,
            unsigned_vec(directory, TiffTag::TileByteCounts)?,
        ));
    }
    Ok((
        None,
        None,
        unsigned_vec(directory, TiffTag::StripOffsets)?,
        unsigned_vec(directory, TiffTag::StripByteCounts)?,
    ))
}

pub fn required_usize<T>(directory: &Directory, tag: T) -> Result<usize, DecodeError>
where
    T: Into<Tag>,
{
    let tag = tag.into();
    let value = unsigned_scalar(directory.get(&tag))
        .ok_or_else(|| invalid(format!("{tag} is missing or invalid")))?;
    usize::try_from(value).map_err(|_| invalid(format!("{tag} exceeds the platform address space")))
}

pub fn optional_usize<T>(directory: &Directory, tag: T) -> Option<usize>
where
    T: Into<Tag>,
{
    let value = unsigned_scalar(directory.get(&tag.into()))?;
    usize::try_from(value).ok()
}

fn required_u16<T>(directory: &Directory, tag: T) -> Result<u16, DecodeError>
where
    T: Into<Tag>,
{
    let tag = tag.into();
    let value = unsigned_scalar(directory.get(&tag))
        .ok_or_else(|| invalid(format!("{tag} is missing or invalid")))?;
    u16::try_from(value).map_err(|_| invalid(format!("{tag} is out of range")))
}

pub fn optional_u16<T>(directory: &Directory, tag: T) -> Option<u16>
where
    T: Into<Tag>,
{
    let value = unsigned_scalar(directory.get(&tag.into()))?;
    u16::try_from(value).ok()
}

pub fn unsigned_scalar(value: Option<&Value>) -> Option<u64> {
    match value? {
        Value::Byte(value) => Some(u64::from(*value)),
        Value::Short(value) => Some(u64::from(*value)),
        Value::Unsigned(value) | Value::Ifd(value) => Some(u64::from(*value)),
        Value::UnsignedBig(value) | Value::IfdBig(value) => Some(*value),
        Value::List(values) => unsigned_scalar(values.first()),
        _ => None,
    }
}

pub fn unsigned_vec<T>(directory: &Directory, tag: T) -> Result<Vec<usize>, DecodeError>
where
    T: Into<Tag>,
{
    let tag = tag.into();
    let value = directory
        .get(&tag)
        .ok_or_else(|| invalid(format!("{tag} is missing")))?;
    value
        .clone()
        .into_u64_vec()
        .map_err(|_| invalid(format!("{tag} has an invalid numeric type")))?
        .into_iter()
        .map(|value| {
            usize::try_from(value)
                .map_err(|_| invalid(format!("{tag} exceeds the platform address space")))
        })
        .collect()
}

pub fn bytes(value: &Value) -> Option<Vec<u8>> {
    match value {
        Value::Byte(value) => Some(vec![*value]),
        Value::List(values) => values.iter().try_fold(Vec::new(), |mut output, value| {
            output.extend(bytes(value)?);
            Some(output)
        }),
        _ => None,
    }
}

pub fn invalid<T>(detail: T) -> DecodeError
where
    T: Into<Box<str>>,
{
    DecodeError::InvalidInput(detail.into())
}

pub fn unsupported<T>(detail: T) -> DecodeError
where
    T: Into<Box<str>>,
{
    DecodeError::Unsupported(detail.into())
}

fn optional_u64(value: Option<&Value>) -> Option<u64> { unsigned_scalar(value) }
