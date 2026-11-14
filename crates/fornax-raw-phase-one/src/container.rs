use std::collections::BTreeMap;

use fornax_codec::Chunk;
use fornax_raw_core::error::DecodeError;
use fornax_tiff::TiffSource;

use crate::invalid;

pub const PRIVATE_BASE: usize = 8;
const MAX_MAKER_ENTRIES: u32 = 8192;
const MARGIN_LEFT: u32 = 0x010a;
const MARGIN_TOP: u32 = 0x010b;
const IMAGE_WIDTH: u32 = 0x010c;
const IMAGE_HEIGHT: u32 = 0x010d;
const FORMAT: u32 = 0x010e;
const DATA_OFFSET: u32 = 0x010f;
const WHITE_BALANCE: u32 = 0x0107;
const BLACK_LEVEL: u32 = 0x021d;
const STRIP_OFFSETS: u32 = 0x021c;
pub const SENSOR_CORRECTION: u32 = 0x0110;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum Compression {
    Uncompressed,
    Lossless14,
    Lossless16,
    Nonlinear,
    Sv2,
}

impl Compression {
    pub const fn codec(self) -> crate::codec::IiqCodec {
        match self {
            Self::Uncompressed => crate::codec::IiqCodec::Uncompressed,
            Self::Lossless14 | Self::Lossless16 => crate::codec::IiqCodec::Lossless,
            Self::Nonlinear => crate::codec::IiqCodec::Nonlinear,
            Self::Sv2 => crate::codec::IiqCodec::Sv2,
        }
    }

    pub const fn bits(self) -> u16 {
        match self {
            Self::Lossless16 => 16,
            Self::Uncompressed | Self::Lossless14 | Self::Nonlinear | Self::Sv2 => 14,
        }
    }
}

#[derive(Clone, Debug)]
pub struct RawLayout {
    pub width: usize,
    pub height: usize,
    pub compression: Compression,
    pub data_offset: usize,
    pub data_size: usize,
    pub strip_offset: Option<usize>,
    pub strip_size: Option<usize>,
}

impl RawLayout {
    pub fn parse(source: &[u8], maker: &MakerNote) -> Result<Self, DecodeError> {
        let width = usize::try_from(maker.required_u32(0x0108, "raw width")?)
            .map_err(|_| invalid("IIQ width exceeds the address space"))?;
        let height = usize::try_from(maker.required_u32(0x0109, "raw height")?)
            .map_err(|_| invalid("IIQ height exceeds the address space"))?;
        if width == 0 || height == 0 {
            return Err(invalid("IIQ raw dimensions must not be zero"));
        }
        width
            .checked_mul(height)
            .and_then(|samples| samples.checked_mul(2))
            .ok_or_else(|| invalid("IIQ raw byte count overflow"))?;
        let compression = match maker.required_u32(FORMAT, "compression mode")? {
            0 => Compression::Uncompressed,
            3 => Compression::Lossless14,
            5 => Compression::Nonlinear,
            6 => Compression::Sv2,
            8 => Compression::Lossless16,
            mode @ (1 | 2) => {
                return Err(DecodeError::Unsupported(
                    format!("legacy IIQ Raw{mode} compression is unsupported").into(),
                ));
            }
            mode => {
                return Err(DecodeError::Unsupported(
                    format!("IIQ compression mode {mode} is unsupported").into(),
                ));
            }
        };
        let (data_rel, data_size) = maker.offset_and_count(DATA_OFFSET, "raw data")?;
        let data_offset = PRIVATE_BASE
            .checked_add(data_rel)
            .ok_or_else(|| invalid("IIQ raw data offset overflow"))?;
        bounded(source, data_offset, data_size, "IIQ raw data")?;
        let (strip_offset, strip_size) = if compression == Compression::Uncompressed {
            (None, None)
        } else {
            let (offset, count) = maker.offset_and_count(STRIP_OFFSETS, "row offsets")?;
            let offset = PRIVATE_BASE
                .checked_add(offset)
                .ok_or_else(|| invalid("IIQ row-offset table overflow"))?;
            bounded(source, offset, count, "IIQ row-offset table")?;
            let expected = height
                .checked_mul(4)
                .ok_or_else(|| invalid("IIQ row-offset table size overflow"))?;
            if count != expected {
                return Err(invalid(format!(
                    "IIQ row-offset table contains {count} bytes, expected {expected}"
                )));
            }
            (Some(offset), Some(count))
        };
        Ok(Self {
            width,
            height,
            compression,
            data_offset,
            data_size,
            strip_offset,
            strip_size,
        })
    }

    pub const fn chunk_height(&self) -> usize {
        if matches!(self.compression, Compression::Uncompressed) {
            self.height
        } else {
            1
        }
    }

    pub fn chunks(&self, source: &[u8]) -> Result<Vec<Chunk>, DecodeError> {
        if self.compression == Compression::Uncompressed {
            let byte_count = self
                .width
                .checked_mul(self.height)
                .and_then(|samples| samples.checked_mul(2))
                .ok_or_else(|| invalid("IIQ uncompressed byte count overflow"))?;
            if self.data_size < byte_count {
                return Err(invalid("IIQ uncompressed payload is truncated"));
            }
            return Ok(vec![Chunk {
                offset: self.data_offset,
                byte_count,
            }]);
        }
        let strip_offset = self
            .strip_offset
            .ok_or_else(|| invalid("IIQ row-offset table is missing"))?;
        let strip_size = self
            .strip_size
            .ok_or_else(|| invalid("IIQ row-offset table size is missing"))?;
        let table = bounded(source, strip_offset, strip_size, "IIQ row-offset table")?;
        let mut rows = table
            .as_chunks::<4>()
            .0
            .iter()
            .enumerate()
            .map(|(row, bytes)| {
                Ok((
                    row,
                    usize::try_from(u32::from_le_bytes(*bytes))
                        .map_err(|_| invalid("IIQ row offset exceeds the address space"))?,
                ))
            })
            .collect::<Result<Vec<_>, DecodeError>>()?;
        rows.sort_unstable_by_key(|(_, offset)| *offset);
        for pair in rows.windows(2) {
            if pair[0].1 == pair[1].1 {
                return Err(invalid("IIQ row offsets contain a duplicate"));
            }
        }
        let mut chunks = vec![
            Chunk {
                offset: 0,
                byte_count: 0,
            };
            self.height
        ];
        for (index, (row, relative)) in rows.iter().copied().enumerate() {
            if relative >= self.data_size {
                return Err(invalid("IIQ row offset exceeds the raw data size"));
            }
            let end = rows
                .get(index + 1)
                .map_or(self.data_size, |(_, offset)| *offset);
            let byte_count = end
                .checked_sub(relative)
                .filter(|count| *count > 0)
                .ok_or_else(|| invalid("IIQ row range is empty"))?;
            chunks[row] = Chunk {
                offset: self
                    .data_offset
                    .checked_add(relative)
                    .ok_or_else(|| invalid("IIQ row data offset overflow"))?,
                byte_count,
            };
        }
        Ok(chunks)
    }

    pub fn active_area(&self, maker: &MakerNote) -> Result<[u32; 4], DecodeError> {
        let left = maker.required_u32(MARGIN_LEFT, "left margin")?;
        let top = maker.required_u32(MARGIN_TOP, "top margin")?;
        let width = maker.required_u32(IMAGE_WIDTH, "image width")?;
        let height = maker.required_u32(IMAGE_HEIGHT, "image height")?;
        let full_width = u32::try_from(self.width).map_err(|_| invalid("IIQ width exceeds u32"))?;
        let full_height =
            u32::try_from(self.height).map_err(|_| invalid("IIQ height exceeds u32"))?;
        let right = left
            .checked_add(width)
            .ok_or_else(|| invalid("IIQ ActiveArea right edge overflow"))?;
        let bottom = top
            .checked_add(height)
            .ok_or_else(|| invalid("IIQ ActiveArea bottom edge overflow"))?;
        if width == 0 || height == 0 || right > full_width || bottom > full_height {
            return Err(invalid("IIQ ActiveArea is outside the raw image"));
        }
        Ok([top, left, bottom, right])
    }

    pub fn white_balance(&self, maker: &MakerNote) -> Result<[f32; 3], DecodeError> {
        maker.f32x3(WHITE_BALANCE, "white balance")
    }

    pub fn sensor_margins(&self, maker: &MakerNote) -> Result<(usize, usize), DecodeError> {
        let left = usize::try_from(maker.required_u32(MARGIN_LEFT, "left margin")?)
            .map_err(|_| invalid("IIQ left margin exceeds the address space"))?;
        let top = usize::try_from(maker.required_u32(MARGIN_TOP, "top margin")?)
            .map_err(|_| invalid("IIQ top margin exceeds the address space"))?;
        Ok((left, top))
    }

    pub fn black_level(&self, maker: &MakerNote) -> Result<u16, DecodeError> {
        maker.optional_u32(BLACK_LEVEL)?.map_or(Ok(0), |value| {
            u16::try_from(value).map_err(|_| invalid("IIQ black level exceeds u16"))
        })
    }
}

#[derive(Clone, Debug)]
pub struct MakerNote {
    source: TiffSource,
    entries: BTreeMap<u32, Entry>,
}

#[derive(Clone, Copy, Debug)]
struct Entry {
    kind: u32,
    count: usize,
    value: u32,
}

impl MakerNote {
    pub fn parse(source: TiffSource) -> Result<Self, DecodeError> {
        if source.get(PRIVATE_BASE..PRIVATE_BASE + 4) != Some(b"IIII") {
            return Err(invalid("IIQ private header marker is missing"));
        }
        let magic = read_u32(&source, PRIVATE_BASE + 4, "IIQ private magic")?;
        if magic >> 8 != 0x0052_6177 {
            return Err(invalid("IIQ private header Raw signature is missing"));
        }
        let ifd_relative = read_u32(&source, PRIVATE_BASE + 8, "IIQ private IFD offset")?;
        let ifd = PRIVATE_BASE
            .checked_add(
                usize::try_from(ifd_relative)
                    .map_err(|_| invalid("IIQ private IFD offset exceeds the address space"))?,
            )
            .ok_or_else(|| invalid("IIQ private IFD offset overflow"))?;
        let count = read_u32(&source, ifd, "IIQ private IFD entry count")?;
        if count > MAX_MAKER_ENTRIES {
            return Err(invalid(format!(
                "IIQ private IFD entry count {count} exceeds {MAX_MAKER_ENTRIES}"
            )));
        }
        let table_bytes = usize::try_from(count)
            .ok()
            .and_then(|count| count.checked_mul(16))
            .ok_or_else(|| invalid("IIQ private IFD size overflow"))?;
        let table_start = ifd
            .checked_add(8)
            .ok_or_else(|| invalid("IIQ private IFD offset overflow"))?;
        let table = bounded(&source, table_start, table_bytes, "IIQ private IFD")?;
        let mut entries = BTreeMap::new();
        for entry in table.as_chunks::<16>().0 {
            let tag = u32::from_le_bytes(entry[0..4].try_into().expect("four bytes"));
            let kind = u32::from_le_bytes(entry[4..8].try_into().expect("four bytes"));
            let count = usize::try_from(u32::from_le_bytes(
                entry[8..12].try_into().expect("four bytes"),
            ))
            .map_err(|_| invalid("IIQ maker-note count exceeds the address space"))?;
            let value = u32::from_le_bytes(entry[12..16].try_into().expect("four bytes"));
            if entries.insert(tag, Entry { kind, count, value }).is_some() {
                return Err(invalid(format!(
                    "IIQ maker-note tag 0x{tag:04x} is duplicated"
                )));
            }
        }
        Ok(Self { source, entries })
    }

    pub fn required_u32(&self, tag: u32, name: &str) -> Result<u32, DecodeError> {
        self.optional_u32(tag)?
            .ok_or_else(|| invalid(format!("IIQ {name} tag 0x{tag:04x} is missing")))
    }

    pub fn optional_u32(&self, tag: u32) -> Result<Option<u32>, DecodeError> {
        let Some(entry) = self.entries.get(&tag) else {
            return Ok(None);
        };
        match entry.kind {
            2 | 4 => Ok(Some(entry.value)),
            kind => Err(invalid(format!(
                "IIQ maker tag 0x{tag:04x} uses non-numeric type {kind}"
            ))),
        }
    }

    pub fn offset_and_count(&self, tag: u32, name: &str) -> Result<(usize, usize), DecodeError> {
        let entry = self
            .entries
            .get(&tag)
            .ok_or_else(|| invalid(format!("IIQ {name} tag 0x{tag:04x} is missing")))?;
        Ok((
            usize::try_from(entry.value)
                .map_err(|_| invalid(format!("IIQ {name} offset exceeds the address space")))?,
            entry.count,
        ))
    }

    pub fn bytes(&self, tag: u32) -> Result<&[u8], DecodeError> {
        let entry = self
            .entries
            .get(&tag)
            .ok_or_else(|| invalid(format!("IIQ maker tag 0x{tag:04x} is missing")))?;
        let offset = PRIVATE_BASE
            .checked_add(
                usize::try_from(entry.value)
                    .map_err(|_| invalid("IIQ maker value offset exceeds the address space"))?,
            )
            .ok_or_else(|| invalid("IIQ maker value offset overflow"))?;
        bounded(
            &self.source,
            offset,
            entry.count,
            &format!("IIQ maker tag 0x{tag:04x}"),
        )
    }

    pub fn f32x3(&self, tag: u32, name: &str) -> Result<[f32; 3], DecodeError> {
        let bytes = self.bytes(tag)?;
        let bytes = bytes
            .get(..12)
            .ok_or_else(|| invalid(format!("IIQ {name} is truncated")))?;
        Ok([
            f32::from_le_bytes(bytes[0..4].try_into().expect("four bytes")),
            f32::from_le_bytes(bytes[4..8].try_into().expect("four bytes")),
            f32::from_le_bytes(bytes[8..12].try_into().expect("four bytes")),
        ])
    }
}

fn read_u32(source: &[u8], offset: usize, name: &str) -> Result<u32, DecodeError> {
    let end = offset
        .checked_add(4)
        .ok_or_else(|| invalid(format!("{name} offset overflow")))?;
    let bytes = source
        .get(offset..end)
        .ok_or_else(|| invalid(format!("{name} is truncated")))?;
    Ok(u32::from_le_bytes(bytes.try_into().expect("four bytes")))
}

pub fn bounded<'a>(
    source: &'a [u8],
    offset: usize,
    count: usize,
    name: &str,
) -> Result<&'a [u8], DecodeError> {
    let end = offset
        .checked_add(count)
        .ok_or_else(|| invalid(format!("{name} range overflow")))?;
    source
        .get(offset..end)
        .ok_or_else(|| invalid(format!("{name} is out of bounds")))
}
