use fornax_raw_core::error::DecodeError;
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{ByteOrder, Directory, ParsedTiff, Tag, Value, unsigned_scalar};

const NEF_COMPRESSION_OLD: Tag = Tag(0x0093);
const NEF_COMPRESSION_NEW: Tag = Tag(0x0051);

#[derive(Clone, Debug)]
pub(crate) struct MakerNote {
    directory: Directory,
    byte_order: ByteOrder,
}

impl MakerNote {
    pub(crate) fn parse(parsed: &ParsedTiff) -> Result<Self, DecodeError> {
        let byte_order = file_byte_order(&parsed.source)?;
        let exif_offset = unsigned_scalar(parsed.root.get(&Tag::from(TiffTag::ExifIfdPointer)))
            .ok_or_else(|| invalid("NEF root IFD has no valid ExifIFDPointer"))?;
        let exif = fornax_tiff::parse_ifd_at(&parsed.source, exif_offset, byte_order, 0)
            .map_err(|error| invalid(format!("parsing NEF Exif IFD failed: {error}")))?;
        let maker = exif
            .get(&Tag::from(TiffTag::MakerNote))
            .ok_or_else(|| unsupported("NEF Exif IFD has no MakerNote"))?;
        let bytes = bytes(maker).ok_or_else(|| invalid("NEF MakerNote is not a byte string"))?;
        if !bytes.starts_with(b"Nikon\0\x02") {
            return Err(unsupported(
                "NEF MakerNote is not a Nikon type-2 maker note",
            ));
        }
        if bytes.len() < 18 {
            return Err(invalid("NEF MakerNote is shorter than the type-2 header"));
        }
        let maker_order = match &bytes[10..12] {
            b"II" => ByteOrder::Little,
            b"MM" => ByteOrder::Big,
            _ => return Err(invalid("NEF MakerNote has no valid byte-order marker")),
        };
        if maker_order != byte_order {
            return Err(unsupported("NEF TIFF and maker-note byte order disagree"));
        }
        let directory = fornax_tiff::parse_ifd_at(&bytes, 18, maker_order, 10)
            .map_err(|error| invalid(format!("parsing NEF maker-note IFD failed: {error}")))?;
        Ok(Self {
            directory,
            byte_order: maker_order,
        })
    }

    pub(crate) const fn byte_order(&self) -> ByteOrder { self.byte_order }

    pub(crate) const fn directory(&self) -> &Directory { &self.directory }

    pub(crate) fn compression(&self) -> Result<Option<u16>, DecodeError> {
        let old = unsigned_scalar(self.directory.get(&NEF_COMPRESSION_OLD))
            .map(u16::try_from)
            .transpose()
            .map_err(|_| invalid("Nikon tag 0x0093 exceeds u16"))?;
        let new = self
            .directory
            .get(&NEF_COMPRESSION_NEW)
            .and_then(bytes)
            .map(|bytes| -> Result<u16, DecodeError> {
                let value = bytes
                    .get(10..12)
                    .ok_or_else(|| invalid("Nikon tag 0x0051 is shorter than 12 bytes"))?;
                Ok(u16::from_le_bytes([value[0], value[1]]))
            })
            .transpose()?;
        match (old, new) {
            (Some(old), Some(new)) if old != new => Err(unsupported(format!(
                "Nikon compression tags disagree: 0x0093={old}, 0x0051={new}"
            ))),
            (Some(value), _) | (None, Some(value)) => Ok(Some(value)),
            (None, None) => Ok(None),
        }
    }
}

fn file_byte_order(source: &[u8]) -> Result<ByteOrder, DecodeError> {
    match source.get(..2) {
        Some(b"II") => Ok(ByteOrder::Little),
        Some(b"MM") => Ok(ByteOrder::Big),
        _ => Err(invalid("NEF has no valid TIFF byte-order marker")),
    }
}

fn bytes(value: &Value) -> Option<Vec<u8>> {
    match value {
        Value::Byte(value) => Some(vec![*value]),
        Value::List(values) => values.iter().try_fold(Vec::new(), |mut output, value| {
            output.extend(bytes(value)?);
            Some(output)
        }),
        _ => None,
    }
}

fn invalid(detail: impl Into<Box<str>>) -> DecodeError { DecodeError::InvalidInput(detail.into()) }

fn unsupported(detail: impl Into<Box<str>>) -> DecodeError {
    DecodeError::Unsupported(detail.into())
}
