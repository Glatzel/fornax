use fornax_raw_core::error::DecodeError;
use fornax_tiff::{Tag, Value};

use crate::invalid;

const COLOR_DATA_TAG: u16 = 0x4001;
const MAKERNOTE_BLOCK_TAG: u16 = 0x927c;

#[derive(Clone, Debug)]
pub struct ColorData {
    pub version: i16,
    pub white_balance: [u16; 4],
    pub black_level: [u16; 4],
    pub specular_white_level: u16,
}

#[derive(Clone, Debug)]
pub enum ColorDataStatus {
    Parsed(ColorData),
    Missing,
    Unknown(i16),
}

pub struct Ctmd {
    pub color_data: Option<ColorDataStatus>,
}

pub fn parse_ctmd(data: &[u8]) -> Result<Ctmd, DecodeError> {
    let mut position = 0_usize;
    let mut color_data = None;
    while position + 12 <= data.len() {
        let size = usize::try_from(read_u32_le(data, position)?)
            .map_err(|_| invalid("Canon CTMD record size exceeds usize"))?;
        if size < 12 || position + size > data.len() {
            break;
        }
        let record_type = read_u16_le(data, position + 4)?;
        let payload = &data[position + 12..position + size];
        if record_type == 8 {
            color_data = color_data_from_blocks(payload)?;
        }
        position += size;
    }
    Ok(Ctmd { color_data })
}

fn color_data_from_blocks(data: &[u8]) -> Result<Option<ColorDataStatus>, DecodeError> {
    let mut position = 0_usize;
    while position + 8 <= data.len() {
        let size = usize::try_from(read_u32_le(data, position)?)
            .map_err(|_| invalid("Canon CTMD block size exceeds usize"))?;
        if size < 8 || position + size > data.len() {
            return Err(invalid("Canon CTMD block is malformed"));
        }
        let tag = read_u16_le(data, position + 4)?;
        if tag == MAKERNOTE_BLOCK_TAG {
            let block = &data[position + 8..position + size];
            let root = parse_single_tiff_ifd(block)?;
            let Some(value) = root.get(&Tag(COLOR_DATA_TAG)) else {
                return Ok(None);
            };
            return parse_color_data(value).map(Some);
        }
        position += size;
    }
    Ok(None)
}

fn parse_single_tiff_ifd(block: &[u8]) -> Result<fornax_tiff::Directory, DecodeError> {
    let byte_order = match block.get(..2) {
        Some(b"II") => fornax_tiff::ByteOrder::Little,
        Some(b"MM") => fornax_tiff::ByteOrder::Big,
        _ => return Err(invalid("Canon CTMD TIFF header is missing")),
    };
    let ifd_offset = match byte_order {
        fornax_tiff::ByteOrder::Little => block
            .get(4..8)
            .map(|bytes| u32::from_le_bytes(bytes.try_into().unwrap_or([0; 4])))
            .ok_or_else(|| invalid("Canon CTMD TIFF IFD offset is truncated"))?,
        fornax_tiff::ByteOrder::Big => block
            .get(4..8)
            .map(|bytes| u32::from_be_bytes(bytes.try_into().unwrap_or([0; 4])))
            .ok_or_else(|| invalid("Canon CTMD TIFF IFD offset is truncated"))?,
    };
    fornax_tiff::parse_ifd_at(block, u64::from(ifd_offset), byte_order, 0)
        .map_err(|error| invalid(format!("parsing CTMD maker note failed: {error}")))
}

fn parse_color_data(value: &Value) -> Result<ColorDataStatus, DecodeError> {
    let data: Vec<u16> = value
        .clone()
        .into_u64_vec()
        .map_err(|_| invalid("Canon ColorData is not an integer array"))?
        .into_iter()
        .map(|value| u16::try_from(value).map_err(|_| invalid("Canon ColorData exceeds u16")))
        .collect::<Result<_, _>>()?;
    let raw_version = *data
        .first()
        .ok_or_else(|| invalid("Canon ColorData is empty"))?;
    let version = raw_version as i16;
    let offsets = match version {
        -4 => (0x47, 0x14d, 0x0569, 0x056a),
        -3 => (0x47, 0x108, 0, 0),
        1 => (63, 196, 0, 0),
        2 | 3 => (0x3f, 0xe7, 0, 0),
        4 | 5 => (0x3f, 0x2b4, 0x2b8, 0x2b9),
        6 | 7 => (0x3f, 0x2cb, 0x2cf, 0x2d0),
        9 => (0x3f, 0x2cf, 0x2d3, 0x2d4),
        10 if data.len() == 1273 || data.len() == 1275 => (0x3f, 0x1df, 0x1e3, 0x1e4),
        10 => (0x3f, 0x1f8, 0x1fc, 0x1fd),
        11 => (0x3f, 0x2d8, 0x2dc, 0x2dd),
        12 | 13 | 15 => (0x3f, 0x30a, 0x30e, 0x30f),
        14 => (0x3f, 0x22c, 0x230, 0x231),
        16..=19 => (0x47, 0x149, 0x31c, 0x31d),
        32 | 33 => (0x55, 0x157, 0x32a, 0x32b),
        34 | 48 => (0x69, 0x16b, 0x280, 0x281),
        64..=66 => (0x69, 0x17f, 0x294, 0x295),
        _ => return Ok(ColorDataStatus::Unknown(version)),
    };
    let white_balance = four(&data, offsets.0)?;
    let black_level = four(&data, offsets.1)?;
    let normal_white_level = optional_scalar(&data, offsets.2)?;
    let specular_white_level = optional_scalar(&data, offsets.3)?;
    let white_level = if specular_white_level != 0 {
        specular_white_level
    } else {
        normal_white_level
    };
    Ok(ColorDataStatus::Parsed(ColorData {
        version,
        white_balance,
        black_level,
        specular_white_level: white_level,
    }))
}

fn four(data: &[u16], offset: usize) -> Result<[u16; 4], DecodeError> {
    let values = data
        .get(offset..offset + 4)
        .ok_or_else(|| invalid(format!("Canon ColorData offset 0x{offset:x} is truncated")))?;
    values
        .try_into()
        .map_err(|_| invalid("Canon ColorData is truncated"))
}

fn optional_scalar(data: &[u16], offset: usize) -> Result<u16, DecodeError> {
    if offset == 0 {
        return Ok(0);
    }
    data.get(offset)
        .copied()
        .ok_or_else(|| invalid(format!("Canon ColorData offset 0x{offset:x} is truncated")))
}

fn read_u16_le(data: &[u8], offset: usize) -> Result<u16, DecodeError> {
    let bytes: [u8; 2] = data
        .get(offset..offset + 2)
        .ok_or_else(|| invalid("Canon CTMD u16 is truncated"))?
        .try_into()
        .map_err(|_| invalid("Canon CTMD u16 is truncated"))?;
    Ok(u16::from_le_bytes(bytes))
}

fn read_u32_le(data: &[u8], offset: usize) -> Result<u32, DecodeError> {
    let bytes: [u8; 4] = data
        .get(offset..offset + 4)
        .ok_or_else(|| invalid("Canon CTMD u32 is truncated"))?
        .try_into()
        .map_err(|_| invalid("Canon CTMD u32 is truncated"))?;
    Ok(u32::from_le_bytes(bytes))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn unknown_color_data_version_is_preserved_as_typed_status() {
        let status = parse_color_data(&Value::List(vec![Value::Short(999)])).unwrap();
        assert!(matches!(status, ColorDataStatus::Unknown(999)));
    }

    #[test]
    fn ctmd_stops_before_a_truncated_record() {
        let data = [100_u8, 0, 0, 0, 8, 0, 0, 0, 0, 0, 0, 0];
        assert!(parse_ctmd(&data).unwrap().color_data.is_none());
    }

    #[test]
    fn parses_every_audited_color_data_version() {
        let cases = [
            (1, 63, 196, 0, 0),
            (2, 63, 0xe7, 0, 0),
            (3, 63, 0xe7, 0, 0),
            (4, 63, 0x2b4, 0x2b8, 0x2b9),
            (5, 63, 0x2b4, 0x2b8, 0x2b9),
            (6, 63, 0x2cb, 0x2cf, 0x2d0),
            (7, 63, 0x2cb, 0x2cf, 0x2d0),
            (9, 63, 0x2cf, 0x2d3, 0x2d4),
            (10, 63, 0x1f8, 0x1fc, 0x1fd),
            (11, 63, 0x2d8, 0x2dc, 0x2dd),
            (12, 63, 0x30a, 0x30e, 0x30f),
            (13, 63, 0x30a, 0x30e, 0x30f),
            (14, 63, 0x22c, 0x230, 0x231),
            (15, 63, 0x30a, 0x30e, 0x30f),
            (16, 0x47, 0x149, 0x31c, 0x31d),
            (17, 0x47, 0x149, 0x31c, 0x31d),
            (18, 0x47, 0x149, 0x31c, 0x31d),
            (19, 0x47, 0x149, 0x31c, 0x31d),
            (32, 0x55, 0x157, 0x32a, 0x32b),
            (33, 0x55, 0x157, 0x32a, 0x32b),
            (34, 0x69, 0x16b, 0x280, 0x281),
            (48, 0x69, 0x16b, 0x280, 0x281),
            (64, 0x69, 0x17f, 0x294, 0x295),
            (65, 0x69, 0x17f, 0x294, 0x295),
            (66, 0x69, 0x17f, 0x294, 0x295),
        ];
        for (version, wb, black, normal, specular) in cases {
            let length = [wb, black, normal, specular, 3].into_iter().max().unwrap() + 4;
            let mut data = vec![Value::Short(0); length];
            data[0] = Value::Short(version as u16);
            data[wb..wb + 4].fill(Value::Short(100));
            data[black..black + 4].fill(Value::Short(512));
            if normal != 0 {
                data[normal] = Value::Short(15_000);
            }
            if specular != 0 {
                data[specular] = Value::Short(16_383);
            }
            let ColorDataStatus::Parsed(parsed) = parse_color_data(&Value::List(data)).unwrap()
            else {
                panic!("ColorData version {version} parsed as unknown");
            };
            assert_eq!(parsed.version, version);
            assert_eq!(parsed.black_level, [512; 4]);
        }
    }
}
