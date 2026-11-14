use std::sync::Arc;

use fornax_raw_core::error::DecodeError;
use fornax_tiff::{ByteOrder, Directory, Tag, Value, unsigned_scalar};

use crate::profiles::{ModelProfile, profile};

const MAX_RAF_TAGS: u32 = 4000;
const PREVIEW_OFFSET: usize = 84;
const RAF_TAGS_OFFSET: usize = 92;
const RAW_TIFF_OFFSET: usize = 100;
const FUJI_IFD_TAG: Tag = Tag(0xf000);
const RAW_WIDTH: Tag = Tag(0xf001);
const RAW_HEIGHT: Tag = Tag(0xf002);
const RAW_BITS: Tag = Tag(0xf003);
const RAW_OFFSET: Tag = Tag(0xf007);
const RAW_BYTES: Tag = Tag(0xf008);

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Compression {
    Packed12,
    Words,
    FujiLossless,
}

#[derive(Clone, Debug)]
pub struct RawLayout {
    pub compression: Compression,
    pub width: usize,
    pub height: usize,
    pub bits: u8,
    pub offset: usize,
    pub byte_count: usize,
    pub byte_order: ByteOrder,
}

#[derive(Clone, Debug)]
pub struct Container {
    pub source: Arc<[u8]>,
    pub model: String,
    pub cfa: crate::Cfa,
    pub profile: Option<ModelProfile>,
    pub orientation: Option<u16>,
    pub tags: RafTags,
    pub fuji_ifd: Directory,
    pub layout: RawLayout,
}

#[derive(Clone, Debug, Default)]
pub struct RafTags {
    pub size: Option<[u16; 2]>,
    pub crop_origin: Option<[u16; 2]>,
    pub cropped_size: Option<[u16; 2]>,
    pub aspect_ratio: Option<[u16; 2]>,
    pub fuji_layout: Option<u8>,
    pub xtrans_layout: Option<[u8; 36]>,
    pub white_balance: Option<Vec<u16>>,
}

pub fn parse_container(source: &[u8]) -> Result<Container, DecodeError> {
    if !source.starts_with(b"FUJIFILM") {
        return Err(invalid("RAF header is missing FUJIFILM magic"));
    }
    let model = model_name(source)?;
    let orientation = preview_orientation(source);
    let tags_offset = usize::try_from(read_u32(source, RAF_TAGS_OFFSET, "RAF tags offset")?)
        .map_err(|_| invalid("RAF tags offset exceeds the address space"))?;
    let tags_length = usize::try_from(read_u32(source, RAF_TAGS_OFFSET + 4, "RAF tags length")?)
        .map_err(|_| invalid("RAF tags length exceeds the address space"))?;
    let tags_bytes = bounded(source, tags_offset, tags_length, "RAF tag block")?;
    let tags = parse_raf_tags(tags_bytes)?;
    let (cfa, profile) = if let Some(layout) = tags.xtrans_layout {
        if tags.fuji_layout.is_some_and(|layout| layout & 0x80 != 0) {
            return Err(DecodeError::Unsupported(
                "rotated SuperCCD RAF layouts are unsupported".into(),
            ));
        }
        (crate::Cfa::XTrans(xtrans_pattern(layout)?), None)
    } else {
        let profile = profile(&model).ok_or_else(|| {
            DecodeError::Unsupported(
                format!("RAF camera model {model:?} has no 2x2 Bayer profile").into(),
            )
        })?;
        (crate::Cfa::Bayer(profile.cfa), Some(profile))
    };

    let tiff_offset = usize::try_from(read_u32(source, RAW_TIFF_OFFSET, "RAF raw TIFF offset")?)
        .map_err(|_| invalid("RAF raw TIFF offset exceeds the address space"))?;
    let tiff_length = usize::try_from(read_u32(
        source,
        RAW_TIFF_OFFSET + 4,
        "RAF raw TIFF length",
    )?)
    .map_err(|_| invalid("RAF raw TIFF length exceeds the address space"))?;
    bounded(source, tiff_offset, tiff_length, "RAF raw TIFF block")?;
    let (fuji_ifd, byte_order) = parse_fuji_ifd(source, tiff_offset)?;

    let width = required_usize(&fuji_ifd, RAW_WIDTH, "FujiImageWidth")?;
    let height = required_usize(&fuji_ifd, RAW_HEIGHT, "FujiImageLength")?;
    if width == 0 || height == 0 {
        return Err(invalid("RAF raw dimensions must not be zero"));
    }
    let bits = required_usize(&fuji_ifd, RAW_BITS, "FujiBitsPerSample")?;
    let bits = u8::try_from(bits).map_err(|_| invalid("RAF bit depth exceeds u8"))?;
    let strip_offset = required_usize(&fuji_ifd, RAW_OFFSET, "FujiStripOffsets")?;
    let byte_count = required_usize(&fuji_ifd, RAW_BYTES, "FujiStripByteCounts")?;
    let raw_offset = tiff_offset
        .checked_add(strip_offset)
        .ok_or_else(|| invalid("RAF raw payload offset overflow"))?;
    let payload = bounded(source, raw_offset, byte_count, "RAF raw payload")?;

    validate_raf_size(tags.size, width, height)?;

    let compression = classify_compression(payload, width, height, bits)?;
    Ok(Container {
        source: Arc::from(source),
        model,
        cfa,
        profile,
        orientation,
        tags,
        fuji_ifd,
        layout: RawLayout {
            compression,
            width,
            height,
            bits,
            offset: raw_offset,
            byte_count,
            byte_order,
        },
    })
}

fn preview_orientation(source: &[u8]) -> Option<u16> {
    let offset =
        usize::try_from(read_u32(source, PREVIEW_OFFSET, "RAF preview offset").ok()?).ok()?;
    let length =
        usize::try_from(read_u32(source, PREVIEW_OFFSET + 4, "RAF preview length").ok()?).ok()?;
    let jpeg = bounded(source, offset, length, "RAF preview").ok()?;
    if !jpeg.starts_with(&[0xff, 0xd8]) {
        return None;
    }
    let mut position = 2_usize;
    while position.checked_add(4)? <= jpeg.len() {
        if jpeg[position] != 0xff {
            return None;
        }
        while jpeg.get(position) == Some(&0xff) {
            position += 1;
        }
        let marker = *jpeg.get(position)?;
        position += 1;
        if matches!(marker, 0xd9 | 0xda) {
            return None;
        }
        if matches!(marker, 0x01 | 0xd0..=0xd7) {
            continue;
        }
        let length_end = position.checked_add(2)?;
        let segment_length =
            u16::from_be_bytes(jpeg.get(position..length_end)?.try_into().ok()?) as usize;
        if segment_length < 2 {
            return None;
        }
        let segment_end = position.checked_add(segment_length)?;
        let payload = jpeg.get(length_end..segment_end)?;
        if marker == 0xe1
            && payload.starts_with(b"Exif\0\0")
            && let Some(orientation) = exif_orientation(&payload[6..])
        {
            return Some(orientation);
        }
        position = segment_end;
    }
    None
}

fn exif_orientation(tiff: &[u8]) -> Option<u16> {
    let parsed = fornax_tiff::parse(tiff).ok()?;
    let orientation = unsigned_scalar(
        parsed
            .root()
            .get(&Tag::from(fornax_tiff::constants::TiffTag::Orientation)),
    )?;
    let orientation = u16::try_from(orientation).ok()?;
    (1..=8).contains(&orientation).then_some(orientation)
}

pub fn parse_raf_tags(bytes: &[u8]) -> Result<RafTags, DecodeError> {
    let count = read_u32(bytes, 0, "RAF tag entry count")?;
    if count > MAX_RAF_TAGS {
        return Err(DecodeError::InvalidInput(
            format!("RAF tag entry count {count} exceeds {MAX_RAF_TAGS}").into(),
        ));
    }
    let mut position = 4_usize;
    let mut tags = RafTags::default();
    for _ in 0..count {
        let tag = read_u16(bytes, position, "RAF tag id")?;
        let length = usize::from(read_u16(bytes, position + 2, "RAF tag length")?);
        position = position
            .checked_add(4)
            .ok_or_else(|| invalid("RAF tag header offset overflow"))?;
        let end = position
            .checked_add(length)
            .ok_or_else(|| invalid("RAF tag value length overflow"))?;
        let value = bytes
            .get(position..end)
            .ok_or_else(|| invalid("RAF tag value is truncated"))?;
        match tag {
            0x0100 => tags.size = Some(pair_u16(value)?),
            0x0110 => tags.crop_origin = Some(pair_u16(value)?),
            0x0111 => tags.cropped_size = Some(pair_u16(value)?),
            0x0115 => tags.aspect_ratio = Some(pair_u16(value)?),
            0x0130 => {
                tags.fuji_layout = Some(
                    value
                        .first()
                        .copied()
                        .ok_or_else(|| invalid("RAF FujiLayout is empty"))?,
                );
            }
            0x0131 => {
                let layout: [u8; 36] = value
                    .try_into()
                    .map_err(|_| invalid("RAF XTransLayout must contain exactly 36 bytes"))?;
                tags.xtrans_layout = Some(layout);
            }
            0x2ff0 => tags.white_balance = Some(shorts_u16(value)?),
            _ => {}
        }
        position = end;
    }
    Ok(tags)
}

pub fn xtrans_pattern(file_order: [u8; 36]) -> Result<[u8; 36], DecodeError> {
    let mut pattern = [0_u8; 36];
    for (destination, source) in pattern.iter_mut().zip(file_order.into_iter().rev()) {
        if source > 2 {
            return Err(invalid(format!("RAF X-Trans plane {source} is invalid")));
        }
        *destination = source;
    }
    Ok(pattern)
}

fn parse_fuji_ifd(source: &[u8], offset: usize) -> Result<(Directory, ByteOrder), DecodeError> {
    let header = source
        .get(offset..offset.saturating_add(8))
        .ok_or_else(|| invalid("RAF raw TIFF header is truncated"))?;
    let order = match &header[..2] {
        b"II" => ByteOrder::Little,
        b"MM" => ByteOrder::Big,
        _ => return Err(invalid("RAF raw TIFF byte-order marker is invalid")),
    };
    let magic = match order {
        ByteOrder::Little => u16::from_le_bytes([header[2], header[3]]),
        ByteOrder::Big => u16::from_be_bytes([header[2], header[3]]),
    };
    if magic != 42 {
        return Err(invalid("RAF raw TIFF magic is invalid"));
    }
    let first_ifd = match order {
        ByteOrder::Little => u32::from_le_bytes(header[4..8].try_into().expect("four bytes")),
        ByteOrder::Big => u32::from_be_bytes(header[4..8].try_into().expect("four bytes")),
    };
    let correction = i64::try_from(offset).map_err(|_| invalid("RAF TIFF offset exceeds i64"))?;
    let root = fornax_tiff::parse_ifd_at(
        source,
        u64::try_from(offset)
            .ok()
            .and_then(|base| base.checked_add(u64::from(first_ifd)))
            .ok_or_else(|| invalid("RAF first IFD offset overflow"))?,
        order,
        correction,
    )
    .map_err(|error| invalid(format!("parsing RAF Fuji IFD failed: {error}")))?;
    let Some(child) = root.get(&FUJI_IFD_TAG) else {
        return Ok((root, order));
    };
    let child_offset = ifd_pointer(child)?;
    let fuji = fornax_tiff::parse_ifd_at(
        source,
        u64::try_from(offset)
            .ok()
            .and_then(|base| base.checked_add(child_offset))
            .ok_or_else(|| invalid("RAF Fuji IFD child offset overflow"))?,
        order,
        correction,
    )
    .map_err(|error| invalid(format!("parsing RAF Fuji IFD child failed: {error}")))?;
    Ok((fuji, order))
}

fn ifd_pointer(value: &Value) -> Result<u64, DecodeError> {
    match value {
        Value::Ifd(value) | Value::Unsigned(value) => Ok(u64::from(*value)),
        Value::IfdBig(value) | Value::UnsignedBig(value) => Ok(*value),
        _ => Err(invalid("RAF Fuji IFD pointer has an invalid value type")),
    }
}

fn classify_compression(
    payload: &[u8],
    width: usize,
    height: usize,
    bits: u8,
) -> Result<Compression, DecodeError> {
    if payload.starts_with(&0x4953_u16.to_be_bytes()) {
        return Ok(Compression::FujiLossless);
    }
    let pixels = width
        .checked_mul(height)
        .ok_or_else(|| invalid("RAF sample count overflow"))?;
    match bits {
        12 if payload.len()
            == pixels
                .checked_mul(3)
                .ok_or_else(|| invalid("RAF packed size overflow"))?
                .div_ceil(2) =>
        {
            Ok(Compression::Packed12)
        }
        14 | 16
            if payload.len()
                == pixels
                    .checked_mul(2)
                    .ok_or_else(|| invalid("RAF word size overflow"))? =>
        {
            Ok(Compression::Words)
        }
        12 | 14 | 16 => Err(DecodeError::Unsupported(
            format!(
                "RAF {bits}-bit payload length {} does not match packed or word storage",
                payload.len()
            )
            .into(),
        )),
        _ => Err(DecodeError::Unsupported(
            format!("RAF bit depth {bits} is unsupported").into(),
        )),
    }
}

fn validate_raf_size(
    declared: Option<[u16; 2]>,
    width: usize,
    height: usize,
) -> Result<(), DecodeError> {
    let Some([declared_height, declared_width]) = declared else {
        return Ok(());
    };
    if usize::from(declared_width) != width || usize::from(declared_height) != height {
        return Err(invalid(format!(
            "RAF size tag {declared_width}x{declared_height} disagrees with Fuji IFD {width}x{height}"
        )));
    }
    Ok(())
}

fn model_name(source: &[u8]) -> Result<String, DecodeError> {
    let bytes = source
        .get(28..60)
        .ok_or_else(|| invalid("RAF model name is truncated"))?;
    let end = bytes
        .iter()
        .position(|value| *value == 0)
        .unwrap_or(bytes.len());
    let model = std::str::from_utf8(&bytes[..end])
        .map_err(|_| invalid("RAF model name is not UTF-8"))?
        .trim();
    if model.is_empty() {
        return Err(invalid("RAF model name is empty"));
    }
    Ok(model.to_owned())
}

fn required_usize(directory: &Directory, tag: Tag, name: &str) -> Result<usize, DecodeError> {
    unsigned_scalar(directory.get(&tag))
        .and_then(|value| usize::try_from(value).ok())
        .ok_or_else(|| invalid(format!("RAF {name} is missing or invalid")))
}

fn bounded<'a>(
    source: &'a [u8],
    offset: usize,
    length: usize,
    what: &str,
) -> Result<&'a [u8], DecodeError> {
    let end = offset
        .checked_add(length)
        .ok_or_else(|| invalid(format!("{what} range overflow")))?;
    source
        .get(offset..end)
        .ok_or_else(|| invalid(format!("{what} is out of bounds")))
}

fn pair_u16(bytes: &[u8]) -> Result<[u16; 2], DecodeError> {
    let values = shorts_u16(bytes)?;
    values
        .try_into()
        .map_err(|_| invalid("RAF tag must contain exactly two values"))
}

fn shorts_u16(bytes: &[u8]) -> Result<Vec<u16>, DecodeError> {
    if !bytes.len().is_multiple_of(2) {
        return Err(invalid("RAF short tag has an odd byte length"));
    }
    bytes
        .as_chunks::<2>()
        .0
        .iter()
        .map(|value| Ok(u16::from_be_bytes([value[0], value[1]])))
        .collect()
}

fn read_u16(bytes: &[u8], offset: usize, what: &str) -> Result<u16, DecodeError> {
    let end = offset
        .checked_add(2)
        .ok_or_else(|| invalid(format!("{what} offset overflow")))?;
    let value = bytes
        .get(offset..end)
        .ok_or_else(|| invalid(format!("{what} is truncated")))?;
    Ok(u16::from_be_bytes([value[0], value[1]]))
}

fn read_u32(bytes: &[u8], offset: usize, what: &str) -> Result<u32, DecodeError> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| invalid(format!("{what} offset overflow")))?;
    let value = bytes
        .get(offset..end)
        .ok_or_else(|| invalid(format!("{what} is truncated")))?;
    Ok(u32::from_be_bytes([value[0], value[1], value[2], value[3]]))
}

fn invalid(detail: impl Into<Box<str>>) -> DecodeError { DecodeError::InvalidInput(detail.into()) }
