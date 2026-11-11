use std::sync::Arc;

use fornax_raw_core::error::DecodeError;
use fornax_tiff::{ByteOrder, Directory, Tag};

use crate::makernote::bytes;

const META_2: Tag = Tag(0x0096);
const META_1: Tag = Tag(0x008C);
const LOSSLESS_VERSION: [u8; 2] = [0x46, 0x30];

pub struct NikonCurve {
    pub init: [[u16; 2]; 2],
    pub curve: Arc<[u16]>,
}

pub fn parse(directory: &Directory, bits: u8, order: ByteOrder) -> Result<NikonCurve, DecodeError> {
    let value = directory
        .get(&META_2)
        .or_else(|| directory.get(&META_1))
        .ok_or_else(|| invalid("Nikon mode 3 is missing metadata tag 0x0096/0x008c"))?;
    let metadata =
        bytes(value).ok_or_else(|| invalid("Nikon mode-3 metadata is not a byte string"))?;
    if metadata.len() < 12 {
        return Err(invalid("Nikon mode-3 metadata is shorter than 12 bytes"));
    }
    if metadata[..2] != LOSSLESS_VERSION {
        return Err(unsupported(format!(
            "Nikon mode-3 metadata version {:02x}{:02x} is unsupported",
            metadata[0], metadata[1]
        )));
    }

    let init = [
        [
            read_u16(&metadata, 2, order)?,
            read_u16(&metadata, 4, order)?,
        ],
        [
            read_u16(&metadata, 6, order)?,
            read_u16(&metadata, 8, order)?,
        ],
    ];
    let curve_length = usize::from(read_u16(&metadata, 10, order)?);
    let curve_limit = 1_usize << bits;
    if curve_length == 0 || curve_length > curve_limit {
        return Err(invalid(format!(
            "Nikon mode-3 curve length {curve_length} is inconsistent with {bits} bits"
        )));
    }

    Ok(NikonCurve {
        init,
        curve: Arc::from(
            (0..curve_limit)
                .map(|value| value as u16)
                .collect::<Vec<_>>(),
        ),
    })
}

fn read_u16(bytes: &[u8], offset: usize, order: ByteOrder) -> Result<u16, DecodeError> {
    let value: [u8; 2] = bytes
        .get(offset..offset + 2)
        .ok_or_else(|| invalid("Nikon mode-3 metadata is truncated"))?
        .try_into()
        .expect("the metadata slice has exactly two bytes");
    Ok(match order {
        ByteOrder::Little => u16::from_le_bytes(value),
        ByteOrder::Big => u16::from_be_bytes(value),
    })
}

fn invalid(detail: impl Into<Box<str>>) -> DecodeError { DecodeError::InvalidInput(detail.into()) }

fn unsupported(detail: impl Into<Box<str>>) -> DecodeError {
    DecodeError::Unsupported(detail.into())
}
