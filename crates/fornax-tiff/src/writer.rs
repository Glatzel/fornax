use std::io::{Seek, SeekFrom, Write};

use crate::{Tag, TiffError, Value};

/// A value convertible into a TIFF field.
pub trait IntoValue {
    /// Converts this value into a TIFF field.
    fn into_tiff_value(self) -> Value;
}

impl IntoValue for u8 {
    fn into_tiff_value(self) -> Value { Value::Byte(self) }
}

impl IntoValue for u16 {
    fn into_tiff_value(self) -> Value { Value::Short(self) }
}

impl IntoValue for u32 {
    fn into_tiff_value(self) -> Value { Value::Unsigned(self) }
}

impl IntoValue for f32 {
    fn into_tiff_value(self) -> Value { Value::Float(self) }
}

impl IntoValue for &str {
    fn into_tiff_value(self) -> Value { Value::Ascii(self.to_owned()) }
}

impl IntoValue for [u8; 4] {
    fn into_tiff_value(self) -> Value { Value::List(self.into_iter().map(Value::Byte).collect()) }
}

impl IntoValue for &[u16] {
    fn into_tiff_value(self) -> Value {
        Value::List(self.iter().copied().map(Value::Short).collect())
    }
}

impl IntoValue for &[u32] {
    fn into_tiff_value(self) -> Value {
        Value::List(self.iter().copied().map(Value::Unsigned).collect())
    }
}

impl IntoValue for Value {
    fn into_tiff_value(self) -> Value { self }
}

/// A classic little-endian TIFF writer.
pub struct TiffWriter<W> {
    writer: W,
}

impl<W> TiffWriter<W>
where
    W: Write + Seek,
{
    /// Creates a classic TIFF writer with a reserved header.
    pub fn new(mut writer: W) -> Result<Self, TiffError> {
        writer
            .write_all(&[0; 8])
            .map_err(|error| TiffError::new(format!("writing TIFF header failed: {error}")))?;
        Ok(Self { writer })
    }

    /// Appends data and returns its absolute file offset.
    pub fn write_data(&mut self, data: &[u8]) -> Result<u64, TiffError> {
        let offset = self
            .writer
            .stream_position()
            .map_err(|error| TiffError::new(format!("reading TIFF position failed: {error}")))?;
        self.writer
            .write_all(data)
            .map_err(|error| TiffError::new(format!("writing TIFF data failed: {error}")))?;
        Ok(offset)
    }

    /// Writes the root image directory and finalizes the header.
    pub fn finish<I>(&mut self, entries: I) -> Result<(), TiffError>
    where
        I: IntoIterator<Item = (Tag, Value)>,
    {
        let mut encoded = entries
            .into_iter()
            .map(|(tag, value)| EncodedEntry::new(tag, value))
            .collect::<Result<Vec<_>, _>>()?;
        encoded.sort_unstable_by_key(|entry| entry.tag);

        for entry in &mut encoded {
            if entry.bytes.len() <= 4 {
                continue;
            }
            entry.offset = Some(self.write_data(&entry.bytes)?);
        }

        let ifd_offset = self.write_data(&[])?;
        let count = u16::try_from(encoded.len())
            .map_err(|_| TiffError::new("TIFF directory has more than 65535 entries"))?;
        self.writer
            .write_all(&count.to_le_bytes())
            .map_err(write_error)?;
        for entry in &encoded {
            self.writer
                .write_all(&entry.tag.0.to_le_bytes())
                .map_err(write_error)?;
            self.writer
                .write_all(&entry.field_type.to_le_bytes())
                .map_err(write_error)?;
            self.writer
                .write_all(&entry.count.to_le_bytes())
                .map_err(write_error)?;
            if let Some(offset) = entry.offset {
                let offset = u32::try_from(offset)
                    .map_err(|_| TiffError::new("classic TIFF value offset exceeds u32"))?;
                self.writer
                    .write_all(&offset.to_le_bytes())
                    .map_err(write_error)?;
            } else {
                self.writer.write_all(&entry.bytes).map_err(write_error)?;
                for _ in entry.bytes.len()..4 {
                    self.writer.write_all(&[0]).map_err(write_error)?;
                }
            }
        }
        self.writer.write_all(&[0; 4]).map_err(write_error)?;
        self.writer.seek(SeekFrom::Start(0)).map_err(seek_error)?;
        self.writer
            .write_all(b"II")
            .and_then(|()| self.writer.write_all(&42_u16.to_le_bytes()))
            .and_then(|()| {
                let offset = u32::try_from(ifd_offset)
                    .map_err(|_| std::io::Error::other("IFD offset exceeds classic TIFF"))?;
                self.writer.write_all(&offset.to_le_bytes())
            })
            .map_err(write_error)?;
        self.writer.flush().map_err(write_error)
    }
}

/// Builder for the root directory of a [`TiffWriter`].
pub struct TiffDirectory<'a, W> {
    writer: &'a mut TiffWriter<W>,
    entries: Vec<(Tag, Value)>,
}

impl<'a, W> TiffDirectory<'a, W>
where
    W: Write + Seek,
{
    /// Creates a directory builder for `writer`.
    pub const fn new(writer: &'a mut TiffWriter<W>) -> Self {
        Self {
            writer,
            entries: Vec::new(),
        }
    }

    /// Appends image data and returns its absolute file offset.
    pub fn write_data(&mut self, data: &[u8]) -> Result<u64, TiffError> {
        self.writer.write_data(data)
    }

    /// Adds a directory entry.
    pub fn write_tag<T, V>(&mut self, tag: T, value: V) -> Result<(), TiffError>
    where
        T: Into<Tag>,
        V: IntoValue,
    {
        self.entries.push((tag.into(), value.into_tiff_value()));
        Ok(())
    }

    /// Writes the accumulated entries and finalizes the TIFF header.
    pub fn finish(self) -> Result<(), TiffError> { self.writer.finish(self.entries) }
}

struct EncodedEntry {
    tag: Tag,
    field_type: u16,
    count: u32,
    bytes: Vec<u8>,
    offset: Option<u64>,
}

impl EncodedEntry {
    fn new(tag: Tag, value: Value) -> Result<Self, TiffError> {
        let (field_type, count, bytes) = encode_value(&value)?;
        Ok(Self {
            tag,
            field_type,
            count,
            bytes,
            offset: None,
        })
    }
}

fn encode_value(value: &Value) -> Result<(u16, u32, Vec<u8>), TiffError> {
    if let Value::Bytes(bytes) = value {
        let count = u32::try_from(bytes.len())
            .map_err(|_| TiffError::new("TIFF byte count exceeds u32"))?;
        return Ok((1, count, bytes.clone()));
    }
    if let Value::List(values) = value {
        if values.is_empty() {
            return Err(TiffError::new("empty TIFF lists cannot be serialized"));
        }
        let mut encoded = Vec::new();
        let mut field_type = None;
        for value in values {
            let (current_type, current_count, current_bytes) = encode_scalar(value)?;
            if current_count != 1 || field_type.is_some_and(|kind| kind != current_type) {
                return Err(TiffError::new("TIFF lists must contain one scalar type"));
            }
            field_type.get_or_insert(current_type);
            encoded.extend_from_slice(&current_bytes);
        }
        let count = u32::try_from(values.len())
            .map_err(|_| TiffError::new("TIFF list value count exceeds u32"))?;
        return Ok((field_type.expect("the list is non-empty"), count, encoded));
    }
    encode_scalar(value)
}

fn encode_scalar(value: &Value) -> Result<(u16, u32, Vec<u8>), TiffError> {
    let encoded = match value {
        Value::Byte(value) => (1, vec![*value]),
        Value::Ascii(value) => {
            let count = u32::try_from(value.len() + 1)
                .map_err(|_| TiffError::new("ASCII tag exceeds u32 count"))?;
            let mut bytes = value.as_bytes().to_vec();
            bytes.push(0);
            return Ok((2, count, bytes));
        }
        Value::Short(value) => (3, value.to_le_bytes().to_vec()),
        Value::Unsigned(value) => (4, value.to_le_bytes().to_vec()),
        Value::Rational(numerator, denominator) => {
            let mut bytes = Vec::with_capacity(8);
            bytes.extend_from_slice(&numerator.to_le_bytes());
            bytes.extend_from_slice(&denominator.to_le_bytes());
            (5, bytes)
        }
        Value::SignedByte(value) => (6, vec![*value as u8]),
        Value::SignedShort(value) => (8, value.to_le_bytes().to_vec()),
        Value::Signed(value) => (9, value.to_le_bytes().to_vec()),
        Value::SRational(numerator, denominator) => {
            let mut bytes = Vec::with_capacity(8);
            bytes.extend_from_slice(&numerator.to_le_bytes());
            bytes.extend_from_slice(&denominator.to_le_bytes());
            (10, bytes)
        }
        Value::Float(value) => (11, value.to_le_bytes().to_vec()),
        Value::Double(value) => (12, value.to_le_bytes().to_vec()),
        Value::Ifd(value) => (13, value.to_le_bytes().to_vec()),
        Value::UnsignedBig(value) => (16, value.to_le_bytes().to_vec()),
        Value::SignedBig(value) => (17, value.to_le_bytes().to_vec()),
        Value::IfdBig(value) => (18, value.to_le_bytes().to_vec()),
        Value::RationalBig(numerator, denominator) => {
            let mut bytes = Vec::with_capacity(16);
            bytes.extend_from_slice(&numerator.to_le_bytes());
            bytes.extend_from_slice(&denominator.to_le_bytes());
            (5, bytes)
        }
        Value::SRationalBig(numerator, denominator) => {
            let mut bytes = Vec::with_capacity(16);
            bytes.extend_from_slice(&numerator.to_le_bytes());
            bytes.extend_from_slice(&denominator.to_le_bytes());
            (10, bytes)
        }
        Value::Bytes(_) => {
            return Err(TiffError::new("nested TIFF byte arrays are unsupported"));
        }
        Value::List(_) => return Err(TiffError::new("nested TIFF lists are unsupported")),
    };
    Ok((encoded.0, 1, encoded.1))
}

fn write_error(error: std::io::Error) -> TiffError {
    TiffError::new(format!("writing TIFF failed: {error}"))
}

fn seek_error(error: std::io::Error) -> TiffError {
    TiffError::new(format!("seeking TIFF failed: {error}"))
}

#[cfg(test)]
mod tests {
    use std::io::Cursor;

    use crate::{Tag, TiffWriter, Value, parse};

    #[test]
    fn writer_round_trips_inline_and_external_values() {
        let mut output = Cursor::new(Vec::new());
        let mut writer = TiffWriter::new(&mut output).unwrap();
        let data_offset = writer.write_data(b"pixels").unwrap();
        writer
            .finish([
                (Tag(256), Value::Unsigned(2)),
                (
                    Tag(273),
                    Value::List(vec![
                        Value::Unsigned(u32::try_from(data_offset).unwrap()),
                        Value::Unsigned(u32::try_from(data_offset).unwrap()),
                    ]),
                ),
                (Tag(270), Value::Ascii("Fornax".into())),
            ])
            .unwrap();

        let parsed = parse(output.get_ref()).unwrap();
        assert_eq!(parsed.root().get(&Tag(256)), Some(&Value::Unsigned(2)));
        assert_eq!(
            parsed.root().get(&Tag(270)),
            Some(&Value::Ascii("Fornax".into()))
        );
        let Value::List(offsets) = parsed.root().get(&Tag(273)).unwrap() else {
            panic!("strip offsets should remain a list");
        };
        assert_eq!(offsets.len(), 2);
    }
}
