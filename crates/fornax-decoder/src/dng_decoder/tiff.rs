use std::collections::{HashSet, VecDeque};

use ::tiff::decoder::ifd::Value;
use fornax_core::error::DecodeError;
use fornax_core::metadata::dng::{DngMetadata, DngTag};

const TAG_SUB_IFDS: u16 = 330;

pub(super) struct ParsedTiff {
    pub(super) root: DngMetadata,
    pub(super) directories: Vec<DngMetadata>,
}

#[derive(Clone, Copy)]
enum ByteOrder {
    Little,
    Big,
}

struct Reader<'a> {
    bytes: &'a [u8],
    byte_order: ByteOrder,
    big_tiff: bool,
}

impl<'a> Reader<'a> {
    fn new(bytes: &'a [u8]) -> Result<Self, DecodeError> {
        let byte_order = match bytes.get(..2) {
            Some(b"II") => ByteOrder::Little,
            Some(b"MM") => ByteOrder::Big,
            _ => return Err(corrupt("missing TIFF byte-order marker")),
        };
        let reader = Self {
            bytes,
            byte_order,
            big_tiff: false,
        };
        let magic = reader.u16(2)?;
        let big_tiff = match magic {
            42 => false,
            43 => {
                if reader.u16(4)? != 8 || reader.u16(6)? != 0 {
                    return Err(corrupt("invalid BigTIFF header"));
                }
                true
            }
            _ => return Err(corrupt(format!("unknown TIFF magic {magic}"))),
        };
        Ok(Self {
            bytes,
            byte_order,
            big_tiff,
        })
    }

    fn first_ifd(&self) -> Result<u64, DecodeError> {
        if self.big_tiff {
            self.u64(8)
        } else {
            self.u32(4).map(u64::from)
        }
    }

    fn parse_ifd(&self, offset: u64) -> Result<(DngMetadata, Option<u64>, Vec<u64>), DecodeError> {
        let mut position = usize::try_from(offset)
            .map_err(|_| corrupt("IFD offset exceeds the platform address space"))?;
        let entry_count = if self.big_tiff {
            self.u64(position)?
        } else {
            u64::from(self.u16(position)?)
        };
        position = position
            .checked_add(if self.big_tiff { 8 } else { 2 })
            .ok_or_else(|| corrupt("IFD offset overflow"))?;
        let entry_size = if self.big_tiff { 20 } else { 12 };
        let inline_size = if self.big_tiff { 8 } else { 4 };
        let entries_bytes = usize::try_from(entry_count)
            .ok()
            .and_then(|count| count.checked_mul(entry_size))
            .ok_or_else(|| corrupt("IFD entry count overflow"))?;
        self.bytes
            .get(position..position.saturating_add(entries_bytes))
            .ok_or_else(|| corrupt("IFD entries are truncated"))?;

        let mut metadata = DngMetadata::new();
        let mut child_pointers = Vec::new();
        for _ in 0..entry_count {
            let tag = self.u16(position)?;
            let field_type = self.u16(position + 2)?;
            let count = if self.big_tiff {
                self.u64(position + 4)?
            } else {
                u64::from(self.u32(position + 4)?)
            };
            let value_field = position + if self.big_tiff { 12 } else { 8 };
            let Some(unit_size) = field_type_size(field_type) else {
                position += entry_size;
                continue;
            };
            let byte_count = count
                .checked_mul(unit_size)
                .ok_or_else(|| corrupt(format!("TIFF tag 0x{tag:04x} byte count overflow")))?;
            let value_offset = if byte_count <= inline_size {
                value_field
            } else {
                let offset = if self.big_tiff {
                    self.u64(value_field)?
                } else {
                    u64::from(self.u32(value_field)?)
                };
                usize::try_from(offset)
                    .map_err(|_| corrupt("TIFF value offset exceeds the platform address space"))?
            };
            let value_bytes = usize::try_from(byte_count)
                .ok()
                .and_then(|count| value_offset.checked_add(count))
                .and_then(|end| self.bytes.get(value_offset..end))
                .ok_or_else(|| corrupt(format!("TIFF tag 0x{tag:04x} data is out of bounds")))?;
            if let Some(value) = self.parse_value(field_type, count, value_bytes)? {
                if tag == TAG_SUB_IFDS {
                    child_pointers.extend(pointers(&value));
                }
                metadata.insert(DngTag::from(tag), value);
            }
            position += entry_size;
        }

        let next = if self.big_tiff {
            self.u64(position)?
        } else {
            u64::from(self.u32(position)?)
        };
        Ok((metadata, (next != 0).then_some(next), child_pointers))
    }

    fn parse_value(
        &self,
        field_type: u16,
        count: u64,
        bytes: &[u8],
    ) -> Result<Option<Value>, DecodeError> {
        if field_type == 2 {
            return Ok(Some(Value::Ascii(
                String::from_utf8_lossy(bytes)
                    .trim_end_matches('\0')
                    .to_owned(),
            )));
        }
        let count = usize::try_from(count)
            .map_err(|_| corrupt("TIFF tag value count exceeds the platform address space"))?;
        let values = match field_type {
            1 | 7 => bytes
                .iter()
                .map(|value| Value::Byte(*value))
                .collect::<Vec<_>>(),
            3 => (0..count)
                .map(|index| {
                    read_u16(bytes, index * 2, self.byte_order)
                        .map(Value::Short)
                        .ok_or_else(|| corrupt("SHORT tag is truncated"))
                })
                .collect::<Result<Vec<_>, _>>()?,
            4 => (0..count)
                .map(|index| {
                    read_u32(bytes, index * 4, self.byte_order)
                        .map(Value::Unsigned)
                        .ok_or_else(|| corrupt("LONG tag is truncated"))
                })
                .collect::<Result<Vec<_>, _>>()?,
            5 => (0..count)
                .map(|index| {
                    let offset = index * 8;
                    let numerator = read_u32(bytes, offset, self.byte_order)
                        .ok_or_else(|| corrupt("RATIONAL numerator is truncated"))?;
                    let denominator = read_u32(bytes, offset + 4, self.byte_order)
                        .ok_or_else(|| corrupt("RATIONAL denominator is truncated"))?;
                    Ok(Value::Rational(numerator, denominator))
                })
                .collect::<Result<Vec<_>, DecodeError>>()?,
            6 => bytes
                .iter()
                .map(|value| Value::SignedByte(*value as i8))
                .collect(),
            8 => (0..count)
                .map(|index| {
                    read_u16(bytes, index * 2, self.byte_order)
                        .map(|value| Value::SignedShort(value as i16))
                        .ok_or_else(|| corrupt("SSHORT tag is truncated"))
                })
                .collect::<Result<Vec<_>, _>>()?,
            9 => (0..count)
                .map(|index| {
                    read_u32(bytes, index * 4, self.byte_order)
                        .map(|value| Value::Signed(value as i32))
                        .ok_or_else(|| corrupt("SLONG tag is truncated"))
                })
                .collect::<Result<Vec<_>, _>>()?,
            10 => (0..count)
                .map(|index| {
                    let offset = index * 8;
                    let numerator = read_u32(bytes, offset, self.byte_order)
                        .ok_or_else(|| corrupt("SRATIONAL numerator is truncated"))?;
                    let denominator = read_u32(bytes, offset + 4, self.byte_order)
                        .ok_or_else(|| corrupt("SRATIONAL denominator is truncated"))?;
                    Ok(Value::SRational(numerator as i32, denominator as i32))
                })
                .collect::<Result<Vec<_>, DecodeError>>()?,
            11 => (0..count)
                .map(|index| {
                    read_u32(bytes, index * 4, self.byte_order)
                        .map(|value| Value::Float(f32::from_bits(value)))
                        .ok_or_else(|| corrupt("FLOAT tag is truncated"))
                })
                .collect::<Result<Vec<_>, _>>()?,
            12 => (0..count)
                .map(|index| {
                    read_u64(bytes, index * 8, self.byte_order)
                        .map(|value| Value::Double(f64::from_bits(value)))
                        .ok_or_else(|| corrupt("DOUBLE tag is truncated"))
                })
                .collect::<Result<Vec<_>, _>>()?,
            13 => (0..count)
                .map(|index| {
                    read_u32(bytes, index * 4, self.byte_order)
                        .map(Value::Ifd)
                        .ok_or_else(|| corrupt("IFD tag is truncated"))
                })
                .collect::<Result<Vec<_>, _>>()?,
            16 => (0..count)
                .map(|index| {
                    read_u64(bytes, index * 8, self.byte_order)
                        .map(Value::UnsignedBig)
                        .ok_or_else(|| corrupt("LONG8 tag is truncated"))
                })
                .collect::<Result<Vec<_>, _>>()?,
            17 => (0..count)
                .map(|index| {
                    read_u64(bytes, index * 8, self.byte_order)
                        .map(|value| Value::SignedBig(value as i64))
                        .ok_or_else(|| corrupt("SLONG8 tag is truncated"))
                })
                .collect::<Result<Vec<_>, _>>()?,
            18 => (0..count)
                .map(|index| {
                    read_u64(bytes, index * 8, self.byte_order)
                        .map(Value::IfdBig)
                        .ok_or_else(|| corrupt("IFD8 tag is truncated"))
                })
                .collect::<Result<Vec<_>, _>>()?,
            _ => return Ok(None),
        };
        if values.is_empty() {
            return Ok(Some(Value::List(Vec::new())));
        }
        if values.len() == 1 {
            return Ok(values.into_iter().next());
        }
        Ok(Some(Value::List(values)))
    }

    fn u16(&self, offset: usize) -> Result<u16, DecodeError> {
        read_u16(self.bytes, offset, self.byte_order)
            .ok_or_else(|| corrupt("TIFF read is out of bounds"))
    }

    fn u32(&self, offset: usize) -> Result<u32, DecodeError> {
        read_u32(self.bytes, offset, self.byte_order)
            .ok_or_else(|| corrupt("TIFF read is out of bounds"))
    }

    fn u64(&self, offset: usize) -> Result<u64, DecodeError> {
        read_u64(self.bytes, offset, self.byte_order)
            .ok_or_else(|| corrupt("TIFF read is out of bounds"))
    }
}

pub(super) fn parse(source: &[u8]) -> Result<ParsedTiff, DecodeError> {
    let reader = Reader::new(source)?;
    let first_ifd = reader.first_ifd()?;
    if first_ifd == 0 {
        return Err(corrupt("TIFF contains no image file directories"));
    }

    let mut pending = VecDeque::from([first_ifd]);
    let mut visited = HashSet::new();
    let mut directories = Vec::new();
    while let Some(offset) = pending.pop_front() {
        if !visited.insert(offset) {
            continue;
        }
        let (metadata, next, children) = reader.parse_ifd(offset)?;
        pending.extend(children);
        if let Some(next) = next {
            pending.push_back(next);
        }
        directories.push(metadata);
    }
    let root = directories
        .first()
        .cloned()
        .ok_or_else(|| corrupt("TIFF contains no readable image file directory"))?;
    Ok(ParsedTiff { root, directories })
}

fn pointers(value: &Value) -> Vec<u64> {
    match value {
        Value::Unsigned(value) | Value::Ifd(value) => vec![u64::from(*value)],
        Value::UnsignedBig(value) | Value::IfdBig(value) => vec![*value],
        Value::List(values) => values.iter().flat_map(pointers).collect(),
        _ => Vec::new(),
    }
}

const fn field_type_size(field_type: u16) -> Option<u64> {
    Some(match field_type {
        1 | 2 | 6 | 7 => 1,
        3 | 8 => 2,
        4 | 9 | 11 | 13 => 4,
        5 | 10 | 12 | 16 | 17 | 18 => 8,
        _ => return None,
    })
}

fn read_u16(bytes: &[u8], offset: usize, byte_order: ByteOrder) -> Option<u16> {
    let end = offset.checked_add(2)?;
    let bytes: [u8; 2] = bytes.get(offset..end)?.try_into().ok()?;
    Some(match byte_order {
        ByteOrder::Little => u16::from_le_bytes(bytes),
        ByteOrder::Big => u16::from_be_bytes(bytes),
    })
}

fn read_u32(bytes: &[u8], offset: usize, byte_order: ByteOrder) -> Option<u32> {
    let end = offset.checked_add(4)?;
    let bytes: [u8; 4] = bytes.get(offset..end)?.try_into().ok()?;
    Some(match byte_order {
        ByteOrder::Little => u32::from_le_bytes(bytes),
        ByteOrder::Big => u32::from_be_bytes(bytes),
    })
}

fn read_u64(bytes: &[u8], offset: usize, byte_order: ByteOrder) -> Option<u64> {
    let end = offset.checked_add(8)?;
    let bytes: [u8; 8] = bytes.get(offset..end)?.try_into().ok()?;
    Some(match byte_order {
        ByteOrder::Little => u64::from_le_bytes(bytes),
        ByteOrder::Big => u64::from_be_bytes(bytes),
    })
}

fn corrupt(detail: impl Into<Box<str>>) -> DecodeError { DecodeError::InvalidInput(detail.into()) }

#[cfg(test)]
mod tests {
    use super::{ByteOrder, read_u16, read_u32, read_u64};

    #[test]
    fn read_helpers_reject_overflowing_offsets() {
        assert_eq!(read_u16(&[], usize::MAX, ByteOrder::Little), None);
        assert_eq!(read_u32(&[], usize::MAX, ByteOrder::Little), None);
        assert_eq!(read_u64(&[], usize::MAX, ByteOrder::Little), None);
    }
}
