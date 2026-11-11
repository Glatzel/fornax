use fornax_codec::{Chunk, DecodeCompression, PanasonicV8Params, PanasonicV8Strip};
use fornax_raw_core::error::DecodeError;
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{ByteOrder, Directory, ParsedTiff, Tag, Value, parse_ifd_at, unsigned_scalar};

const MAX_SINGLE_SHOT_PIXELS: usize = 64_000_000;
const PANASONIC_COMPRESSION: u16 = 34_316;

const SENSOR_WIDTH: Tag = Tag(0x0002);
const SENSOR_HEIGHT: Tag = Tag(0x0003);
const CFA_PATTERN: Tag = Tag(0x0009);
const BITS_PER_SAMPLE: Tag = Tag(0x000a);
const COMPRESSION: Tag = Tag(0x000b);
const RAW_FORMAT: Tag = Tag(0x002d);
const RAW_DATA_OFFSET: Tag = Tag(0x0118);
const STRIP_OFFSETS: Tag = Tag(0x0111);
const STRIP_BYTE_COUNTS: Tag = Tag(0x0117);
const CAMERA_IFD: Tag = Tag(0x0120);
const MULTISHOT: Tag = Tag(0x0121);
const CAMERA_MULTISHOT: Tag = Tag(0x1001);

#[derive(Clone, Debug)]
pub enum Family {
    V4 { split_blocks: bool },
    V5,
    V6,
    V8(PanasonicV8Params),
    Unpacked12,
    Packed12Control,
}

impl Family {
    pub(crate) fn compression(&self) -> DecodeCompression {
        match self {
            Self::V4 { split_blocks } => DecodeCompression::PanasonicV4 {
                split_blocks: *split_blocks,
            },
            Self::V5 => DecodeCompression::PanasonicV5,
            Self::V6 => DecodeCompression::PanasonicV6,
            Self::V8(params) => DecodeCompression::PanasonicV8 {
                params: params.clone(),
            },
            Self::Unpacked12 => DecodeCompression::PanasonicUnpacked12,
            Self::Packed12Control => DecodeCompression::PanasonicPacked12Control,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RawLayout {
    pub width: usize,
    pub height: usize,
    pub bits: u16,
    pub chunk: Chunk,
    pub family: Family,
}

#[derive(Clone, Debug)]
pub struct Container {
    pub parsed: ParsedTiff,
    pub raw_ifd: usize,
    pub layout: RawLayout,
    pub cfa: [u8; 4],
    pub version: u16,
}

pub fn parse(source: &[u8]) -> Result<Container, DecodeError> {
    if byte_order(source)? != ByteOrder::Little {
        return Err(DecodeError::Unsupported(
            "big-endian Panasonic RW2 is unsupported".into(),
        ));
    }
    let parsed = fornax_tiff::parse(source)
        .map_err(|error| invalid(format!("parsing RW2 TIFF failed: {error}")))?;
    let raw_ifd = parsed
        .directories
        .iter()
        .position(|directory| directory.contains_key(&RAW_DATA_OFFSET))
        .or_else(|| {
            parsed
                .directories
                .iter()
                .position(|directory| directory.contains_key(&STRIP_OFFSETS))
        })
        .ok_or_else(|| {
            DecodeError::Unsupported("RW2 contains no raw-data-offset or strip-offset IFD".into())
        })?;
    let raw = &parsed.directories[raw_ifd];

    if multishot(raw)? {
        return Err(DecodeError::Unsupported(
            "Panasonic multi-exposure or pixel-shift RW2 is unsupported".into(),
        ));
    }

    let version = required_u16(raw, RAW_FORMAT, "Panasonic raw-format version")?;
    let width = required_u16(raw, SENSOR_WIDTH, "Panasonic sensor width")?;
    let height = required_u16(raw, SENSOR_HEIGHT, "Panasonic sensor height")?;
    if width == 0 || height == 0 {
        return Err(invalid("RW2 sensor dimensions must not be zero"));
    }
    let pixels = usize::from(width)
        .checked_mul(usize::from(height))
        .ok_or_else(|| invalid("RW2 pixel count overflow"))?;
    if pixels > MAX_SINGLE_SHOT_PIXELS {
        return Err(DecodeError::Unsupported(
            format!(
                "Panasonic {width}x{height} high-resolution composite exceeds the \
             {MAX_SINGLE_SHOT_PIXELS}-pixel single-shot limit"
            )
            .into(),
        ));
    }
    let bits = required_u16(raw, BITS_PER_SAMPLE, "Panasonic bits per sample")?;
    if !matches!(bits, 12 | 14 | 16) {
        return Err(DecodeError::Unsupported(
            format!("Panasonic RW2 {bits}-bit samples are unsupported").into(),
        ));
    }
    let cfa = cfa(raw)?;
    let compression = optional_u16(raw, COMPRESSION, "Panasonic compression")?;
    let has_raw_offset = raw.contains_key(&RAW_DATA_OFFSET);
    let (offset, byte_count) = data_range(parsed.source.as_ref(), raw, has_raw_offset)?;

    let family = dispatch(
        raw,
        parsed.source.as_ref(),
        version,
        compression,
        bits,
        has_raw_offset,
        byte_count,
        usize::from(width) * usize::from(height),
    )?;
    Ok(Container {
        parsed,
        raw_ifd,
        layout: RawLayout {
            width: usize::from(width),
            height: usize::from(height),
            bits,
            chunk: Chunk { offset, byte_count },
            family,
        },
        cfa,
        version,
    })
}

pub fn byte_order(source: &[u8]) -> Result<ByteOrder, DecodeError> {
    match source.get(..2) {
        Some(b"II") => Ok(ByteOrder::Little),
        Some(b"MM") => Ok(ByteOrder::Big),
        _ => Err(invalid("RW2 TIFF byte-order marker is missing")),
    }
}

fn dispatch(
    raw: &Directory,
    source: &[u8],
    version: u16,
    compression: Option<u16>,
    bits: u16,
    has_raw_offset: bool,
    byte_count: usize,
    pixels: usize,
) -> Result<Family, DecodeError> {
    match version {
        4..=6 => {
            if compression != Some(PANASONIC_COMPRESSION) {
                return Err(DecodeError::Unsupported(
                    format!("Panasonic RW2 V{version} compression {compression:?} is unsupported")
                        .into(),
                ));
            }
            if !has_raw_offset && bits == 12 {
                let unpacked = pixels
                    .checked_mul(2)
                    .ok_or_else(|| invalid("RW2 unpacked payload size overflow"))?;
                let packed = pixels
                    .checked_mul(3)
                    .ok_or_else(|| invalid("RW2 packed payload size overflow"))?
                    .div_ceil(2);
                if byte_count >= unpacked {
                    return Ok(Family::Unpacked12);
                }
                if byte_count >= packed {
                    return Ok(Family::Packed12Control);
                }
            }
            match version {
                4 => Ok(Family::V4 {
                    split_blocks: has_raw_offset,
                }),
                5 if bits == 12 || bits == 14 => Ok(Family::V5),
                6 if bits == 12 || bits == 14 => Ok(Family::V6),
                _ => Err(DecodeError::Unsupported(
                    format!("Panasonic RW2 V{version} at {bits} bits is unsupported").into(),
                )),
            }
        }
        7 => Err(DecodeError::Unsupported(
            "Panasonic RW2 V7 is deferred until a single-shot V7 fixture is available".into(),
        )),
        8 => {
            if compression.is_some_and(|value| value != PANASONIC_COMPRESSION) {
                return Err(DecodeError::Unsupported(
                    format!("Panasonic RW2 V8 compression {compression:?} is unsupported").into(),
                ));
            }
            if !matches!(bits, 12 | 14 | 16) {
                return Err(DecodeError::Unsupported(
                    format!("Panasonic RW2 V8 at {bits} bits is unsupported").into(),
                ));
            }
            let params = v8_params(raw, source)?;
            if is_s1rm2_multishot_layout(raw, &params) {
                return Err(DecodeError::Unsupported(
                    "Panasonic S1R2 multi-exposure strip layout is unsupported".into(),
                ));
            }
            Ok(Family::V8(params))
        }
        _ => Err(DecodeError::Unsupported(
            format!("Panasonic RW2 raw-format version {version} is unsupported").into(),
        )),
    }
}

fn is_s1rm2_multishot_layout(raw: &Directory, params: &PanasonicV8Params) -> bool {
    let model = raw
        .get(&Tag::from(TiffTag::Model))
        .and_then(|value| match value {
            Value::Ascii(model) => Some(model.as_str()),
            _ => None,
        });
    let [first, second] = params.strips() else {
        return false;
    };
    model == Some("DC-S1RM2") && first.width() == 4144 && second.width() == 4008
}

fn data_range(
    source: &[u8],
    raw: &Directory,
    has_raw_offset: bool,
) -> Result<(usize, usize), DecodeError> {
    let (tag, value) = if has_raw_offset {
        (RAW_DATA_OFFSET, raw.get(&RAW_DATA_OFFSET))
    } else {
        (STRIP_OFFSETS, raw.get(&STRIP_OFFSETS))
    };
    let offset = unsigned_scalar(value)
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| invalid(format!("RW2 {} is invalid", tag_name(tag))))?;
    let byte_count = if has_raw_offset {
        source
            .len()
            .checked_sub(offset)
            .ok_or_else(|| invalid("RW2 raw-data offset is outside the file"))?
    } else if let Some(value) = raw.get(&STRIP_BYTE_COUNTS) {
        unsigned_scalar(Some(value))
            .and_then(|value| usize::try_from(value).ok())
            .filter(|value| *value > 0)
            .ok_or_else(|| invalid("RW2 StripByteCounts is invalid"))?
    } else {
        source
            .len()
            .checked_sub(offset)
            .ok_or_else(|| invalid("RW2 strip offset is outside the file"))?
    };
    let end = offset
        .checked_add(byte_count)
        .ok_or_else(|| invalid("RW2 payload range overflow"))?;
    if end > source.len() {
        return Err(invalid("RW2 payload is outside the file"));
    }
    if byte_count == 0 {
        return Err(invalid(
            "RW2 payload is too short for the declared dimensions",
        ));
    }
    Ok((offset, byte_count))
}

fn multishot(raw: &Directory) -> Result<bool, DecodeError> {
    if unsigned_scalar(raw.get(&MULTISHOT)).unwrap_or(0) != 0 {
        return Ok(true);
    }
    let Some(blob) = bytes_value(raw.get(&CAMERA_IFD)) else {
        return Ok(false);
    };
    let Ok(order) = byte_order(&blob) else {
        return Ok(false);
    };
    let Ok(first) = first_ifd(&blob, order) else {
        return Ok(false);
    };
    let Ok(camera) = parse_ifd_at(&blob, first, order, 0) else {
        return Ok(false);
    };
    Ok(unsigned_scalar(camera.get(&CAMERA_MULTISHOT)).unwrap_or(0) != 0)
}

fn v8_params(raw: &Directory, source: &[u8]) -> Result<PanasonicV8Params, DecodeError> {
    let order = byte_order(source)?;
    if order != ByteOrder::Little {
        return Err(DecodeError::Unsupported(
            "big-endian Panasonic RW2 V8 is unsupported".into(),
        ));
    }
    let gamma_points = count_prefixed_u32(raw, Tag(0x0039), "V8 gamma points")?;
    let gamma_slopes = count_prefixed_u32(raw, Tag(0x003a), "V8 gamma slopes")?;
    let clip = required_u16(raw, Tag(0x003b), "V8 gamma clip")?;
    let initial = [
        required_u16(raw, Tag(0x003c), "V8 predictor 0")?,
        required_u16(raw, Tag(0x003d), "V8 predictor 1")?,
        required_u16(raw, Tag(0x003e), "V8 predictor 2")?,
        required_u16(raw, Tag(0x003f), "V8 predictor 3")?,
    ];
    let huffman = count_prefixed_u8_u16(raw, Tag(0x0040), "V8 Huffman table")?;
    let shifts = count_prefixed_u16(raw, Tag(0x0041), "V8 shift-down table")?;
    let strips_h = usize::from(required_u16(raw, Tag(0x0042), "V8 horizontal strips")?);
    let strips_v = usize::from(required_u16(raw, Tag(0x0043), "V8 vertical strips")?);
    let strip_count = strips_h
        .checked_mul(strips_v)
        .ok_or_else(|| invalid("RW2 V8 strip count overflow"))?;
    let offsets = count_prefixed_u32(raw, Tag(0x0044), "V8 strip offsets")?;
    let line_offsets = count_prefixed_pairs::<u16>(raw, Tag(0x0045), "V8 strip line offsets")?;
    let data_sizes = count_prefixed_u32(raw, Tag(0x0046), "V8 strip sizes")?;
    let widths = count_prefixed_u16(raw, Tag(0x0047), "V8 strip widths")?;
    let heights = count_prefixed_u16(raw, Tag(0x0048), "V8 strip heights")?;
    if [
        offsets.len(),
        line_offsets.len(),
        data_sizes.len(),
        widths.len(),
        heights.len(),
    ]
    .into_iter()
    .any(|length| length != strip_count)
    {
        return Err(invalid("RW2 V8 strip tables have inconsistent lengths"));
    }
    let strips = (0..strip_count)
        .map(|index| {
            PanasonicV8Strip::new(
                usize::try_from(offsets[index])
                    .map_err(|_| invalid("V8 strip offset exceeds usize"))?,
                data_sizes[index],
                usize::from(line_offsets[index].0),
                usize::from(line_offsets[index].1),
                usize::from(widths[index]),
                usize::from(heights[index]),
            )
            .map_err(codec_error)
        })
        .collect::<Result<Vec<_>, _>>()?;
    PanasonicV8Params::new(
        gamma_points,
        gamma_slopes,
        clip,
        initial,
        huffman,
        shifts,
        strips,
    )
    .map_err(codec_error)
}

fn cfa(raw: &Directory) -> Result<[u8; 4], DecodeError> {
    let value = required_u16(raw, CFA_PATTERN, "Panasonic CFA pattern")?;
    let pattern = match value {
        1 => [0, 1, 1, 2],
        2 => [1, 0, 2, 1],
        3 => [1, 2, 0, 1],
        4 => [2, 1, 1, 0],
        _ => {
            return Err(DecodeError::Unsupported(
                format!("Panasonic RW2 CFA pattern {value} is unsupported").into(),
            ));
        }
    };
    Ok(pattern)
}

fn first_ifd(blob: &[u8], order: ByteOrder) -> Result<u64, DecodeError> {
    let bytes = blob
        .get(4..8)
        .ok_or_else(|| invalid("Panasonic CameraIFD header is truncated"))?;
    let value = match order {
        ByteOrder::Little => u32::from_le_bytes(bytes.try_into().expect("four bytes")),
        ByteOrder::Big => u32::from_be_bytes(bytes.try_into().expect("four bytes")),
    };
    Ok(u64::from(value))
}

fn count_prefixed_u32(raw: &Directory, tag: Tag, name: &str) -> Result<Vec<u32>, DecodeError> {
    let blob = required_bytes(raw, tag, name)?;
    let count = usize::from(read_u16(&blob, 0, name)?);
    let required = count
        .checked_mul(4)
        .and_then(|bytes| bytes.checked_add(2))
        .ok_or_else(|| invalid(format!("{name} size overflow")))?;
    if blob.len() < required {
        return Err(invalid(format!("{name} is truncated")));
    }
    Ok((0..count)
        .map(|index| {
            let at = 2 + index * 4;
            u32::from_le_bytes(blob[at..at + 4].try_into().expect("four bytes"))
        })
        .collect())
}

fn count_prefixed_u16(raw: &Directory, tag: Tag, name: &str) -> Result<Vec<u16>, DecodeError> {
    let blob = required_bytes(raw, tag, name)?;
    let count = usize::from(read_u16(&blob, 0, name)?);
    let required = count
        .checked_mul(2)
        .and_then(|bytes| bytes.checked_add(2))
        .ok_or_else(|| invalid(format!("{name} size overflow")))?;
    if blob.len() < required {
        return Err(invalid(format!("{name} is truncated")));
    }
    Ok((0..count)
        .map(|index| {
            let at = 2 + index * 2;
            u16::from_le_bytes(blob[at..at + 2].try_into().expect("two bytes"))
        })
        .collect())
}

fn count_prefixed_pairs<T>(
    raw: &Directory,
    tag: Tag,
    name: &str,
) -> Result<Vec<(T, T)>, DecodeError>
where
    T: FromBlob,
{
    let blob = required_bytes(raw, tag, name)?;
    let count = usize::from(read_u16(&blob, 0, name)?);
    let pair_bytes = T::SIZE
        .checked_mul(2)
        .ok_or_else(|| invalid(format!("{name} pair size overflow")))?;
    let required = count
        .checked_mul(pair_bytes)
        .and_then(|bytes| bytes.checked_add(2))
        .ok_or_else(|| invalid(format!("{name} size overflow")))?;
    if blob.len() < required {
        return Err(invalid(format!("{name} is truncated")));
    }
    Ok((0..count)
        .map(|index| {
            let at = 2 + index * pair_bytes;
            (T::read(&blob, at), T::read(&blob, at + T::SIZE))
        })
        .collect())
}

fn count_prefixed_u8_u16(
    raw: &Directory,
    tag: Tag,
    name: &str,
) -> Result<Vec<(u8, u16)>, DecodeError> {
    let blob = required_bytes(raw, tag, name)?;
    let count = usize::from(read_u16(&blob, 0, name)?);
    let required = count
        .checked_mul(4)
        .and_then(|bytes| bytes.checked_add(2))
        .ok_or_else(|| invalid(format!("{name} size overflow")))?;
    if blob.len() < required {
        return Err(invalid(format!("{name} is truncated")));
    }
    Ok((0..count)
        .map(|index| {
            let at = 2 + index * 4;
            (blob[at], u16::from_le_bytes([blob[at + 2], blob[at + 3]]))
        })
        .collect())
}

trait FromBlob: Sized {
    const SIZE: usize;
    fn read(blob: &[u8], at: usize) -> Self;
}

impl FromBlob for u8 {
    const SIZE: usize = 1;
    fn read(blob: &[u8], at: usize) -> Self { blob[at] }
}

impl FromBlob for u16 {
    const SIZE: usize = 2;
    fn read(blob: &[u8], at: usize) -> Self { Self::from_le_bytes([blob[at], blob[at + 1]]) }
}

fn required_bytes(raw: &Directory, tag: Tag, name: &str) -> Result<Vec<u8>, DecodeError> {
    bytes_value(raw.get(&tag)).ok_or_else(|| invalid(format!("RW2 {name} is missing")))
}

pub fn bytes_value(value: Option<&Value>) -> Option<Vec<u8>> {
    let Value::List(values) = value? else {
        return None;
    };
    values
        .iter()
        .map(|value| match value {
            Value::Byte(value) => Some(*value),
            _ => None,
        })
        .collect()
}

fn read_u16(blob: &[u8], at: usize, name: &str) -> Result<u16, DecodeError> {
    let bytes = blob
        .get(at..at + 2)
        .ok_or_else(|| invalid(format!("{name} is truncated")))?;
    Ok(u16::from_le_bytes([bytes[0], bytes[1]]))
}

fn required_u16(raw: &Directory, tag: Tag, name: &str) -> Result<u16, DecodeError> {
    optional_u16(raw, tag, name)?.ok_or_else(|| invalid(format!("RW2 {name} is missing")))
}

pub fn optional_u16(raw: &Directory, tag: Tag, name: &str) -> Result<Option<u16>, DecodeError> {
    raw.get(&tag)
        .map(|value| {
            unsigned_scalar(Some(value))
                .and_then(|value| u16::try_from(value).ok())
                .ok_or_else(|| invalid(format!("RW2 {name} is invalid")))
        })
        .transpose()
}

fn codec_error(error: fornax_codec::CodecError) -> DecodeError {
    match error {
        fornax_codec::CodecError::InvalidInput(detail) => DecodeError::InvalidInput(detail),
        fornax_codec::CodecError::Unsupported(detail) => DecodeError::Unsupported(detail),
    }
}

fn tag_name(tag: Tag) -> String {
    match TiffTag::from(tag) {
        TiffTag::Unknown(value) => format!("tag 0x{value:04x}"),
        value => value.to_string(),
    }
}

pub fn invalid<T>(detail: T) -> DecodeError
where
    T: Into<Box<str>>,
{
    DecodeError::InvalidInput(detail.into())
}
