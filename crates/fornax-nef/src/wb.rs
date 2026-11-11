use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::MetadataSource;
use fornax_tiff::{Directory, Tag, Value, unsigned_scalar};

const WB0: Tag = Tag(0x000c);
const WB1: Tag = Tag(0x0097);
const SERIAL: Tag = Tag(0x001d);
const KEY: Tag = Tag(0x00a7);

pub(crate) fn neutral(
    directory: &Directory,
) -> Result<(Option<[f64; 3]>, MetadataSource), DecodeError> {
    if let Some(value) = directory.get(&WB0) {
        let gains = rationals(value)?;
        if gains.len() < 4 {
            return Err(invalid("Nikon WB0 must contain four rational gains"));
        }
        return Ok((
            neutral_from_gains([gains[0], gains[1], gains[2], gains[3]])?,
            MetadataSource::Tag {
                tag: WB0.0,
                version: None,
            },
        ));
    }
    if let Some(value) = directory.get(&WB1) {
        let data = bytes(value).ok_or_else(|| invalid("Nikon WB1 is not a byte string"))?;
        let version = wb1_version(&data)?;
        let source = MetadataSource::Tag {
            tag: WB1.0,
            version: Some(version),
        };
        let gains = match version {
            0x0100 => [
                be_u16(&data, 72)?,
                be_u16(&data, 76)?,
                be_u16(&data, 76)?,
                be_u16(&data, 74)?,
            ],
            0x0102 => [
                be_u16(&data, 10)?,
                be_u16(&data, 12)?,
                be_u16(&data, 12)?,
                be_u16(&data, 16)?,
            ],
            0x0103 => [
                be_u16(&data, 20)?,
                be_u16(&data, 22)?,
                be_u16(&data, 22)?,
                be_u16(&data, 24)?,
            ],
            0x0204 | 0x0205 | 0x0206 => {
                let Some((serial, key)) = decryption_inputs(directory)? else {
                    return Ok((
                        None,
                        MetadataSource::Unavailable {
                            tag: Some(WB1.0),
                            reason: "missing WB decryption key",
                        },
                    ));
                };
                let (start, wb_offset) = if version == 0x0205 { (4, 14) } else { (284, 6) };
                let encrypted = data
                    .get(start..start + 280)
                    .ok_or_else(|| invalid(format!("Nikon WB1 {version:04x} is truncated")))?;
                let plain = decrypt(encrypted, &serial, key);
                [
                    be_u16(&plain, wb_offset)?,
                    be_u16(&plain, wb_offset + 2)?,
                    be_u16(&plain, wb_offset + 4)?,
                    be_u16(&plain, wb_offset + 6)?,
                ]
            }
            _ => {
                return Ok((
                    None,
                    MetadataSource::Unavailable {
                        tag: Some(WB1.0),
                        reason: "unsupported WB1 version",
                    },
                ));
            }
        };
        return Ok((neutral_from_gains(gains.map(f64::from))?, source));
    }
    Ok((
        None,
        MetadataSource::Unavailable {
            tag: None,
            reason: "no Nikon white-balance tag",
        },
    ))
}

fn wb1_version(data: &[u8]) -> Result<u16, DecodeError> {
    let version = data
        .get(..4)
        .and_then(|bytes| std::str::from_utf8(bytes).ok())
        .ok_or_else(|| invalid("Nikon WB1 has no four-byte version"))?;
    u16::from_str_radix(version, 16).map_err(|_| invalid("Nikon WB1 version is not hexadecimal"))
}

fn decryption_inputs(directory: &Directory) -> Result<Option<(Vec<u8>, u32)>, DecodeError> {
    let Some(serial) = directory.get(&SERIAL) else {
        return Ok(None);
    };
    let serial = match serial {
        Value::Ascii(value) => value.as_bytes().to_vec(),
        value => bytes(value).ok_or_else(|| invalid("Nikon serial is not a byte string"))?,
    };
    let Some(key) = unsigned_scalar(directory.get(&KEY)) else {
        return Ok(None);
    };
    let key = u32::try_from(key).map_err(|_| invalid("Nikon decryption key exceeds u32"))?;
    if !serial.iter().any(u8::is_ascii_digit) {
        return Ok(None);
    }
    Ok(Some((serial, key)))
}

fn decrypt(encrypted: &[u8], serial: &[u8], key: u32) -> Vec<u8> {
    let serial_number =
        serial
            .iter()
            .take_while(|byte| **byte != 0)
            .fold(0_usize, |value, byte| {
                value
                    .wrapping_mul(10)
                    .wrapping_add(usize::from(if byte.is_ascii_digit() {
                        byte - b'0'
                    } else {
                        byte % 10
                    }))
            });
    let key_bytes = key.to_le_bytes();
    let key_number = key_bytes.into_iter().fold(0_u8, |value, byte| value ^ byte) as usize;
    let ci = u32::from(WB_SERIALMAP[serial_number & 0xff]);
    let mut cj = u32::from(WB_KEYMAP[key_number & 0xff]);
    let mut ck = 0x60_u32;
    encrypted
        .iter()
        .map(|byte| {
            cj = cj.wrapping_add(ci.wrapping_mul(ck));
            ck = ck.wrapping_add(1);
            byte ^ cj as u8
        })
        .collect()
}

fn neutral_from_gains(gains: [f64; 4]) -> Result<Option<[f64; 3]>, DecodeError> {
    let [red, green1, green2, blue] = gains;
    if red <= 0.0 || green1 <= 0.0 || green2 <= 0.0 || blue <= 0.0 {
        return Err(invalid("Nikon white-balance gains must be positive"));
    }
    let green = f64::midpoint(green1, green2);
    Ok(Some([green / red, 1.0, green / blue]))
}

fn rationals(value: &Value) -> Result<Vec<f64>, DecodeError> {
    fn one(value: &Value) -> Result<f64, DecodeError> {
        match value {
            Value::Rational(numerator, denominator) => {
                if *denominator == 0 {
                    return Err(invalid("Nikon white-balance denominator is zero"));
                }
                Ok(f64::from(*numerator) / f64::from(*denominator))
            }
            Value::SRational(numerator, denominator) => {
                if *denominator == 0 {
                    return Err(invalid("Nikon white-balance denominator is zero"));
                }
                Ok(f64::from(*numerator) / f64::from(*denominator))
            }
            _ => Err(invalid("Nikon WB0 is not a rational list")),
        }
    }
    match value {
        Value::List(values) => values.iter().map(one).collect(),
        value => Ok(vec![one(value)?]),
    }
}

fn be_u16(data: &[u8], offset: usize) -> Result<u16, DecodeError> {
    let bytes = data
        .get(offset..offset + 2)
        .ok_or_else(|| invalid("Nikon white-balance payload is truncated"))?;
    Ok(u16::from_be_bytes([bytes[0], bytes[1]]))
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

pub(crate) const WB_SERIALMAP: [u8; 256] = [
    0xc1, 0xbf, 0x6d, 0x0d, 0x59, 0xc5, 0x13, 0x9d, 0x83, 0x61, 0x6b, 0x4f, 0xc7, 0x7f, 0x3d, 0x3d,
    0x53, 0x59, 0xe3, 0xc7, 0xe9, 0x2f, 0x95, 0xa7, 0x95, 0x1f, 0xdf, 0x7f, 0x2b, 0x29, 0xc7, 0x0d,
    0xdf, 0x07, 0xef, 0x71, 0x89, 0x3d, 0x13, 0x3d, 0x3b, 0x13, 0xfb, 0x0d, 0x89, 0xc1, 0x65, 0x1f,
    0xb3, 0x0d, 0x6b, 0x29, 0xe3, 0xfb, 0xef, 0xa3, 0x6b, 0x47, 0x7f, 0x95, 0x35, 0xa7, 0x47, 0x4f,
    0xc7, 0xf1, 0x59, 0x95, 0x35, 0x11, 0x29, 0x61, 0xf1, 0x3d, 0xb3, 0x2b, 0x0d, 0x43, 0x89, 0xc1,
    0x9d, 0x9d, 0x89, 0x65, 0xf1, 0xe9, 0xdf, 0xbf, 0x3d, 0x7f, 0x53, 0x97, 0xe5, 0xe9, 0x95, 0x17,
    0x1d, 0x3d, 0x8b, 0xfb, 0xc7, 0xe3, 0x67, 0xa7, 0x07, 0xf1, 0x71, 0xa7, 0x53, 0xb5, 0x29, 0x89,
    0xe5, 0x2b, 0xa7, 0x17, 0x29, 0xe9, 0x4f, 0xc5, 0x65, 0x6d, 0x6b, 0xef, 0x0d, 0x89, 0x49, 0x2f,
    0xb3, 0x43, 0x53, 0x65, 0x1d, 0x49, 0xa3, 0x13, 0x89, 0x59, 0xef, 0x6b, 0xef, 0x65, 0x1d, 0x0b,
    0x59, 0x13, 0xe3, 0x4f, 0x9d, 0xb3, 0x29, 0x43, 0x2b, 0x07, 0x1d, 0x95, 0x59, 0x59, 0x47, 0xfb,
    0xe5, 0xe9, 0x61, 0x47, 0x2f, 0x35, 0x7f, 0x17, 0x7f, 0xef, 0x7f, 0x95, 0x95, 0x71, 0xd3, 0xa3,
    0x0b, 0x71, 0xa3, 0xad, 0x0b, 0x3b, 0xb5, 0xfb, 0xa3, 0xbf, 0x4f, 0x83, 0x1d, 0xad, 0xe9, 0x2f,
    0x71, 0x65, 0xa3, 0xe5, 0x07, 0x35, 0x3d, 0x0d, 0xb5, 0xe9, 0xe5, 0x47, 0x3b, 0x9d, 0xef, 0x35,
    0xa3, 0xbf, 0xb3, 0xdf, 0x53, 0xd3, 0x97, 0x53, 0x49, 0x71, 0x07, 0x35, 0x61, 0x71, 0x2f, 0x43,
    0x2f, 0x11, 0xdf, 0x17, 0x97, 0xfb, 0x95, 0x3b, 0x7f, 0x6b, 0xd3, 0x25, 0xbf, 0xad, 0xc7, 0xc5,
    0xc5, 0xb5, 0x8b, 0xef, 0x2f, 0xd3, 0x07, 0x6b, 0x25, 0x49, 0x95, 0x25, 0x49, 0x6d, 0x71, 0xc7,
];

pub(crate) const WB_KEYMAP: [u8; 256] = [
    0xa7, 0xbc, 0xc9, 0xad, 0x91, 0xdf, 0x85, 0xe5, 0xd4, 0x78, 0xd5, 0x17, 0x46, 0x7c, 0x29, 0x4c,
    0x4d, 0x03, 0xe9, 0x25, 0x68, 0x11, 0x86, 0xb3, 0xbd, 0xf7, 0x6f, 0x61, 0x22, 0xa2, 0x26, 0x34,
    0x2a, 0xbe, 0x1e, 0x46, 0x14, 0x68, 0x9d, 0x44, 0x18, 0xc2, 0x40, 0xf4, 0x7e, 0x5f, 0x1b, 0xad,
    0x0b, 0x94, 0xb6, 0x67, 0xb4, 0x0b, 0xe1, 0xea, 0x95, 0x9c, 0x66, 0xdc, 0xe7, 0x5d, 0x6c, 0x05,
    0xda, 0xd5, 0xdf, 0x7a, 0xef, 0xf6, 0xdb, 0x1f, 0x82, 0x4c, 0xc0, 0x68, 0x47, 0xa1, 0xbd, 0xee,
    0x39, 0x50, 0x56, 0x4a, 0xdd, 0xdf, 0xa5, 0xf8, 0xc6, 0xda, 0xca, 0x90, 0xca, 0x01, 0x42, 0x9d,
    0x8b, 0x0c, 0x73, 0x43, 0x75, 0x05, 0x94, 0xde, 0x24, 0xb3, 0x80, 0x34, 0xe5, 0x2c, 0xdc, 0x9b,
    0x3f, 0xca, 0x33, 0x45, 0xd0, 0xdb, 0x5f, 0xf5, 0x52, 0xc3, 0x21, 0xda, 0xe2, 0x22, 0x72, 0x6b,
    0x3e, 0xd0, 0x5b, 0xa8, 0x87, 0x8c, 0x06, 0x5d, 0x0f, 0xdd, 0x09, 0x19, 0x93, 0xd0, 0xb9, 0xfc,
    0x8b, 0x0f, 0x84, 0x60, 0x33, 0x1c, 0x9b, 0x45, 0xf1, 0xf0, 0xa3, 0x94, 0x3a, 0x12, 0x77, 0x33,
    0x4d, 0x44, 0x78, 0x28, 0x3c, 0x9e, 0xfd, 0x65, 0x57, 0x16, 0x94, 0x6b, 0xfb, 0x59, 0xd0, 0xc8,
    0x22, 0x36, 0xdb, 0xd2, 0x63, 0x98, 0x43, 0xa1, 0x04, 0x87, 0x86, 0xf7, 0xa6, 0x26, 0xbb, 0xd6,
    0x59, 0x4d, 0xbf, 0x6a, 0x2e, 0xaa, 0x2b, 0xef, 0xe6, 0x78, 0xb6, 0x4e, 0xe0, 0x2f, 0xdc, 0x7c,
    0xbe, 0x57, 0x19, 0x32, 0x7e, 0x2a, 0xd0, 0xb8, 0xba, 0x29, 0x00, 0x3c, 0x52, 0x7d, 0xa8, 0x49,
    0x3b, 0x2d, 0xeb, 0x25, 0x49, 0xfa, 0xa3, 0xaa, 0x39, 0xa7, 0xc5, 0xa7, 0x50, 0x11, 0x36, 0xfb,
    0xc6, 0x67, 0x4a, 0xf5, 0xa5, 0x12, 0x65, 0x7e, 0xb0, 0xdf, 0xaf, 0x4e, 0xb3, 0x61, 0x7f, 0x2f,
];

#[cfg(test)]
mod tests {
    use fornax_raw_core::metadata::MetadataSource;
    use fornax_tiff::{Tag, Value};

    use super::neutral;

    fn directory(entries: &[(u16, Value)]) -> fornax_tiff::Directory {
        entries
            .iter()
            .cloned()
            .map(|(tag, value)| (Tag(tag), value))
            .collect()
    }

    #[test]
    fn wb0_uses_four_rational_gains() {
        let directory = directory(&[(
            0x000c,
            Value::List(vec![
                Value::Rational(2, 1),
                Value::Rational(3, 1),
                Value::Rational(4, 1),
                Value::Rational(5, 1),
            ]),
        )]);
        let (neutral, source) = neutral(&directory).unwrap();

        assert_eq!(neutral, Some([1.75, 1.0, 0.7]));
        assert_eq!(
            source,
            MetadataSource::Tag {
                tag: 0x000c,
                version: None
            }
        );
    }

    #[test]
    fn unencrypted_wb1_versions_use_asymmetric_channels() {
        for (version, offsets) in [
            (0x0100, [72, 76, 76, 74]),
            (0x0102, [10, 12, 12, 16]),
            (0x0103, [20, 22, 22, 24]),
        ] {
            let mut data = vec![0_u8; 100];
            data[..4].copy_from_slice(format!("{version:04x}").as_bytes());
            for (value, offset) in [2000_u16, 1100, 1300, 900].into_iter().zip(offsets) {
                data[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
            }
            let directory = directory(&[(
                0x0097,
                Value::List(data.into_iter().map(Value::Byte).collect()),
            )]);
            let (neutral, source) = neutral(&directory).unwrap();

            assert_eq!(neutral, Some([0.65, 1.0, 1.444_444_444_444_444_4]));
            assert_eq!(
                source,
                MetadataSource::Tag {
                    tag: 0x0097,
                    version: Some(version)
                }
            );
        }
    }

    #[test]
    fn encrypted_wb1_versions_use_serial_and_key() {
        for (version, prefix, wb_offset) in [(0x0204, 284, 6), (0x0205, 4, 14), (0x0206, 284, 6)] {
            let mut data = vec![0_u8; prefix + 280];
            data[..4].copy_from_slice(format!("{version:04x}").as_bytes());
            let gains = [2200_u16, 1300, 1500, 700];
            let mut plain = vec![0_u8; 280];
            for (value, offset) in
                gains
                    .into_iter()
                    .zip([wb_offset, wb_offset + 2, wb_offset + 4, wb_offset + 6])
            {
                plain[offset..offset + 2].copy_from_slice(&value.to_be_bytes());
            }
            encrypt(&mut plain, "1234", 0x1020_3040);
            data[prefix..].copy_from_slice(&plain);
            let directory = directory(&[
                (
                    0x0097,
                    Value::List(data.into_iter().map(Value::Byte).collect()),
                ),
                (0x001d, Value::Ascii("1234".into())),
                (0x00a7, Value::Unsigned(0x1020_3040)),
            ]);
            let (neutral, source) = neutral(&directory).unwrap();

            assert_eq!(neutral, Some([0.636_363_636_363_636_4, 1.0, 2.0]));
            assert_eq!(
                source,
                MetadataSource::Tag {
                    tag: 0x0097,
                    version: Some(version)
                }
            );
        }
    }

    #[test]
    fn unknown_wb_version_and_missing_key_are_reported_without_neutral() {
        let mut unknown = vec![0_u8; 8];
        unknown[..4].copy_from_slice(b"0802");
        let value = Value::List(unknown.into_iter().map(Value::Byte).collect());
        let (value_neutral, source) = neutral(&directory(&[(0x0097, value)])).unwrap();
        assert_eq!(value_neutral, None);
        assert!(matches!(
            source,
            MetadataSource::Unavailable {
                reason: "unsupported WB1 version",
                ..
            }
        ));

        let mut encrypted = vec![0_u8; 4 + 280];
        encrypted[..4].copy_from_slice(b"0205");
        let value = Value::List(encrypted.into_iter().map(Value::Byte).collect());
        let (value_neutral, source) = neutral(&directory(&[(0x0097, value)])).unwrap();
        assert_eq!(value_neutral, None);
        assert!(matches!(
            source,
            MetadataSource::Unavailable {
                reason: "missing WB decryption key",
                ..
            }
        ));
    }

    #[test]
    fn zero_wb_denominator_is_invalid_input() {
        let directory = directory(&[(0x000c, Value::Rational(1, 0))]);
        let error = neutral(&directory).unwrap_err();

        assert!(matches!(
            error,
            fornax_raw_core::error::DecodeError::InvalidInput(_)
        ));
    }

    fn encrypt(data: &mut [u8], serial: &str, key: u32) {
        let serialmap = serial
            .bytes()
            .fold(0_usize, |value, byte| value * 10 + usize::from(byte - b'0'));
        let key_bytes = key.to_le_bytes();
        let key_number = key_bytes.into_iter().fold(0_u8, |value, byte| value ^ byte) as usize;
        let ci = crate::wb::WB_SERIALMAP[serialmap & 0xff] as u32;
        let mut cj = crate::wb::WB_KEYMAP[key_number & 0xff] as u32;
        let mut ck = 0x60_u32;
        for byte in data {
            cj += ci * ck;
            ck += 1;
            *byte ^= cj as u8;
        }
    }
}
