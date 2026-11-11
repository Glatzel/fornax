use fornax_raw_core::error::DecodeError;
use fornax_raw_core::metadata::MetadataSource;
use fornax_raw_core::{BayerPattern, RawMosaic};
use fornax_tiff::{ByteOrder, Tag, Value};

use crate::NefImage;
use crate::makernote::MakerNote;

#[test]
fn parses_little_endian_type_two_maker_note_and_corrects_offsets() {
    let source = fixture(
        ByteOrder::Little,
        ByteOrder::Little,
        &[
            (0x0093, Value::Short(6)),
            (
                0x0045,
                Value::List(vec![
                    Value::Short(16),
                    Value::Short(8),
                    Value::Short(8256),
                    Value::Short(5504),
                ]),
            ),
        ],
    );
    let parsed = fornax_tiff::parse(&source).unwrap();
    let maker = MakerNote::parse(&parsed).unwrap();

    assert_eq!(maker.byte_order(), ByteOrder::Little);
    assert_eq!(
        maker.directory().get(&Tag(0x0045)),
        Some(&Value::List(vec![
            Value::Short(16),
            Value::Short(8),
            Value::Short(8256),
            Value::Short(5504),
        ]))
    );
    assert_eq!(maker.compression().unwrap(), Some(6));
}

#[test]
fn parses_big_endian_type_two_maker_note() {
    let source = fixture(ByteOrder::Big, ByteOrder::Big, &[(0x0093, Value::Short(9))]);
    let parsed = fornax_tiff::parse(&source).unwrap();
    let maker = MakerNote::parse(&parsed).unwrap();

    assert_eq!(maker.byte_order(), ByteOrder::Big);
    assert_eq!(maker.compression().unwrap(), Some(9));
}

#[test]
fn new_z_tag_fifty_one_supplies_compression_mode() {
    let mut bytes = vec![0_u8; 12];
    bytes[10..12].copy_from_slice(&10_u16.to_le_bytes());
    let source = fixture(
        ByteOrder::Little,
        ByteOrder::Little,
        &[(
            0x0051,
            Value::List(bytes.into_iter().map(Value::Byte).collect()),
        )],
    );
    let parsed = fornax_tiff::parse(&source).unwrap();

    assert_eq!(
        MakerNote::parse(&parsed).unwrap().compression().unwrap(),
        Some(10)
    );
}

#[test]
fn conflicting_compression_tags_are_unsupported() {
    let mut bytes = vec![0_u8; 12];
    bytes[10..12].copy_from_slice(&9_u16.to_le_bytes());
    let source = fixture(
        ByteOrder::Little,
        ByteOrder::Little,
        &[
            (0x0093, Value::Short(6)),
            (
                0x0051,
                Value::List(bytes.into_iter().map(Value::Byte).collect()),
            ),
        ],
    );
    let parsed = fornax_tiff::parse(&source).unwrap();
    let error = MakerNote::parse(&parsed)
        .unwrap()
        .compression()
        .unwrap_err();

    assert!(matches!(error, DecodeError::Unsupported(_)));
}

#[test]
fn tiff_and_maker_endianness_mismatch_is_unsupported() {
    let source = fixture(
        ByteOrder::Little,
        ByteOrder::Big,
        &[(0x0093, Value::Short(6))],
    );
    let parsed = fornax_tiff::parse(&source).unwrap();
    let error = MakerNote::parse(&parsed).unwrap_err();

    assert!(matches!(error, DecodeError::Unsupported(_)));
}

#[test]
fn headerless_maker_note_is_unsupported() {
    let source = fixture_with_maker_payload(ByteOrder::Little, vec![1, 2, 3, 4]);
    let parsed = fornax_tiff::parse(&source).unwrap();
    let error = MakerNote::parse(&parsed).unwrap_err();

    assert!(matches!(error, DecodeError::Unsupported(_)));
}

#[test]
fn short_maker_note_is_invalid_input() {
    let source = fixture_with_maker_payload(ByteOrder::Little, b"Nikon\0\x02".to_vec());
    let parsed = fornax_tiff::parse(&source).unwrap();
    let error = MakerNote::parse(&parsed).unwrap_err();

    assert!(matches!(error, DecodeError::InvalidInput(_)));
}

#[test]
fn excessive_maker_entry_count_is_invalid_input_not_a_panic() {
    let mut payload = maker_prefix(ByteOrder::Little);
    payload.extend_from_slice(&u16::MAX.to_le_bytes());
    let source = fixture_with_maker_payload(ByteOrder::Little, payload);
    let parsed = fornax_tiff::parse(&source).unwrap();
    let error = MakerNote::parse(&parsed).unwrap_err();

    assert!(matches!(error, DecodeError::InvalidInput(_)));
}

#[test]
fn mode_six_decodes_tightly_packed_twelve_bit_samples() {
    let source = raw_fixture(Some(6), 34_713, 12, 2, 1, &[0x21, 0x43, 0x65]);
    let image = NefImage::parse(&source).unwrap();
    let raw = image.raw_mosaic().unwrap();
    let RawMosaic::U16(raw) = raw else {
        panic!("NEF returned non-u16 samples");
    };

    assert_eq!(raw.dimensions(), (2, 1));
    assert_eq!(raw.as_raw(), &[0x321, 0x654]);
}

#[test]
fn mode_ten_selects_unpacked_fourteen_bit_words_by_byte_count() {
    let source = raw_fixture(Some(10), 34_713, 14, 2, 1, &[0x23, 0x01, 0x56, 0x04]);
    let image = NefImage::parse(&source).unwrap();
    let RawMosaic::U16(raw) = image.raw_mosaic().unwrap() else {
        panic!("NEF returned non-u16 samples");
    };

    assert_eq!(raw.as_raw(), &[0x123, 0x456]);
}

#[test]
fn tiff_uncompressed_fourteen_bit_words_are_accepted_without_maker_mode() {
    let source = raw_fixture(None, 1, 14, 2, 1, &[0x23, 0x01, 0x56, 0x04]);
    let image = NefImage::parse(&source).unwrap();
    let RawMosaic::U16(raw) = image.raw_mosaic().unwrap() else {
        panic!("NEF returned non-u16 samples");
    };

    assert_eq!(raw.as_raw(), &[0x123, 0x456]);
}

#[test]
fn mode_five_decodes_striped_twelve_bit_words() {
    let source = raw_fixture(
        Some(5),
        34_713,
        12,
        8,
        1,
        &[
            0x21, 0x43, 0x65, 0x87, 0xba, 0xdc, 0xfe, 0x10, 0x32, 0x54, 0x76, 0x98,
        ],
    );
    let image = NefImage::parse(&source).unwrap();
    let RawMosaic::U16(raw) = image.raw_mosaic().unwrap() else {
        panic!("NEF returned non-u16 samples");
    };

    assert_eq!(
        raw.as_raw(),
        &[
            0x0876, 0x0543, 0x0211, 0x00fe, 0x0dcb, 0x0a98, 0x0765, 0x0432,
        ]
    );
}

#[test]
fn deferred_huffman_and_he_modes_are_unsupported() {
    for mode in [1, 3, 4, 8, 13, 14, 99] {
        let source = raw_fixture(Some(mode), 34_713, 12, 2, 1, &[0; 3]);
        let error = NefImage::parse(&source).unwrap_err();
        assert!(
            matches!(error, DecodeError::Unsupported(_)),
            "mode {mode}: {error}"
        );
    }
}

#[test]
fn tiff_and_maker_mode_conflicts_are_unsupported() {
    let source = raw_fixture(Some(6), 1, 12, 2, 1, &[0; 4]);
    let error = NefImage::parse(&source).unwrap_err();

    assert!(matches!(error, DecodeError::Unsupported(_)));
}

#[test]
fn preview_only_or_missing_cfa_contains_no_raw_ifd() {
    let source = raw_fixture_without_cfa(Some(6), 34_713, 12, 2, 1, &[0x21, 0x43, 0x65]);
    let error = NefImage::parse(&source).unwrap_err();

    assert!(matches!(error, DecodeError::Unsupported(_)));
}

#[test]
fn maps_crop_black_cfa_white_and_odd_origin_phase() {
    let source = raw_fixture_with_maker_entries(
        Some(6),
        34_713,
        12,
        8,
        4,
        &vec![0_u8; 48],
        &[
            (
                0x0045,
                Value::List(vec![
                    Value::Short(2),
                    Value::Short(1),
                    Value::Short(6),
                    Value::Short(3),
                ]),
            ),
            (
                0x003d,
                Value::List(vec![
                    Value::Short(100),
                    Value::Short(200),
                    Value::Short(300),
                    Value::Short(400),
                ]),
            ),
        ],
    );
    let metadata = NefImage::parse(&source)
        .unwrap()
        .process_metadata()
        .unwrap();

    assert_eq!(
        (
            metadata.active_area.x,
            metadata.active_area.y,
            metadata.active_area.width,
            metadata.active_area.height
        ),
        (2, 1, 6, 3)
    );
    assert_eq!(metadata.levels.black.as_raw(), &[25.0, 50.0, 75.0, 100.0]);
    assert_eq!(metadata.levels.white, 4095.0);
    assert_eq!(metadata.bayer_pattern, BayerPattern::RGGB);
    assert_eq!(
        metadata.black_source,
        MetadataSource::Tag {
            tag: 0x003d,
            version: None
        }
    );
}

#[test]
fn process_metadata_exposes_wb0_source() {
    let source = raw_fixture_with_maker_entries(
        Some(6),
        34_713,
        12,
        2,
        1,
        &[0x21, 0x43, 0x65],
        &[(
            0x000c,
            Value::List(vec![
                Value::Rational(2, 1),
                Value::Rational(3, 1),
                Value::Rational(4, 1),
                Value::Rational(5, 1),
            ]),
        )],
    );
    let metadata = NefImage::parse(&source)
        .unwrap()
        .process_metadata()
        .unwrap();

    assert_eq!(
        metadata.wb_source,
        MetadataSource::Tag {
            tag: 0x000c,
            version: None
        }
    );
}

fn fixture(file_order: ByteOrder, maker_order: ByteOrder, entries: &[(u16, Value)]) -> Vec<u8> {
    fixture_with_maker_payload(file_order, maker_note(maker_order, entries))
}

fn fixture_with_maker_payload(file_order: ByteOrder, maker_payload: Vec<u8>) -> Vec<u8> {
    let mut builder = TiffFixture::new(file_order);
    let exif = builder.ifd(&[(
        0x927c,
        Value::List(maker_payload.into_iter().map(Value::Byte).collect()),
    )]);
    builder.root(&[
        (0x010f, Value::Ascii("NIKON CORPORATION".into())),
        (0x8769, Value::Unsigned(exif)),
    ])
}

fn raw_fixture(
    mode: Option<u16>,
    compression: u16,
    bits: u16,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Vec<u8> {
    raw_fixture_inner(mode, compression, bits, width, height, pixels, true)
}

fn raw_fixture_without_cfa(
    mode: Option<u16>,
    compression: u16,
    bits: u16,
    width: u32,
    height: u32,
    pixels: &[u8],
) -> Vec<u8> {
    raw_fixture_inner(mode, compression, bits, width, height, pixels, false)
}

fn raw_fixture_inner(
    mode: Option<u16>,
    compression: u16,
    bits: u16,
    width: u32,
    height: u32,
    pixels: &[u8],
    with_cfa: bool,
) -> Vec<u8> {
    let maker_entries = mode
        .map(|mode| vec![(0x0093, Value::Short(mode))])
        .unwrap_or_default();
    raw_fixture_with_entries_inner(
        maker_entries,
        compression,
        bits,
        width,
        height,
        pixels,
        with_cfa,
    )
}

fn raw_fixture_with_maker_entries(
    mode: Option<u16>,
    compression: u16,
    bits: u16,
    width: u32,
    height: u32,
    pixels: &[u8],
    extra: &[(u16, Value)],
) -> Vec<u8> {
    let mut maker_entries = mode
        .map(|mode| vec![(0x0093, Value::Short(mode))])
        .unwrap_or_default();
    maker_entries.extend_from_slice(extra);
    raw_fixture_with_entries_inner(
        maker_entries,
        compression,
        bits,
        width,
        height,
        pixels,
        true,
    )
}

fn raw_fixture_with_entries_inner(
    maker_entries: Vec<(u16, Value)>,
    compression: u16,
    bits: u16,
    width: u32,
    height: u32,
    pixels: &[u8],
    with_cfa: bool,
) -> Vec<u8> {
    let maker_payload = maker_note(ByteOrder::Little, &maker_entries);
    let mut builder = TiffFixture::new(ByteOrder::Little);
    let pixels_offset = builder.data(pixels);
    let mut raw_entries = vec![
        (0x00fe, Value::Unsigned(0)),
        (0x0100, Value::Unsigned(width)),
        (0x0101, Value::Unsigned(height)),
        (0x0102, Value::Short(bits)),
        (0x0103, Value::Short(compression)),
        (0x0106, Value::Short(32_803)),
        (0x0111, Value::Unsigned(pixels_offset)),
        (0x0115, Value::Short(1)),
        (0x0116, Value::Unsigned(height)),
        (0x0117, Value::Unsigned(pixels.len() as u32)),
    ];
    if with_cfa {
        raw_entries.extend([
            (33_421, Value::List(vec![Value::Short(2), Value::Short(2)])),
            (
                33_422,
                Value::List(vec![
                    Value::Byte(0),
                    Value::Byte(1),
                    Value::Byte(1),
                    Value::Byte(2),
                ]),
            ),
        ]);
    }
    let raw_ifd = builder.ifd(&raw_entries);
    let exif = builder.ifd(&[(
        0x927c,
        Value::List(maker_payload.into_iter().map(Value::Byte).collect()),
    )]);
    builder.root(&[
        (0x010f, Value::Ascii("NIKON CORPORATION".into())),
        (0x0110, Value::Ascii("NIKON D850".into())),
        (0x014a, Value::Unsigned(raw_ifd)),
        (0x8769, Value::Unsigned(exif)),
    ])
}

fn maker_prefix(order: ByteOrder) -> Vec<u8> {
    let mut bytes = b"Nikon\0\x02\x00\x00\x00".to_vec();
    bytes.extend_from_slice(match order {
        ByteOrder::Little => b"II*\0\x08\0\0\0",
        ByteOrder::Big => b"MM\0*\0\0\0\x08",
    });
    bytes
}

fn maker_note(order: ByteOrder, entries: &[(u16, Value)]) -> Vec<u8> {
    let mut payload = maker_prefix(order);
    let ifd_start = payload.len();
    payload.extend_from_slice(&(entries.len() as u16).to_bytes(order));
    payload.resize(ifd_start + 2 + entries.len() * 12 + 4, 0);

    for (index, (tag, value)) in entries.iter().enumerate() {
        let (field_type, count, bytes) = encode_value(order, value);
        let entry = ifd_start + 2 + index * 12;
        payload[entry..entry + 2].copy_from_slice(&tag.to_bytes(order));
        payload[entry + 2..entry + 4].copy_from_slice(&field_type.to_bytes(order));
        payload[entry + 4..entry + 8].copy_from_slice(&count.to_bytes(order));
        if bytes.len() <= 4 {
            payload[entry + 8..entry + 8 + bytes.len()].copy_from_slice(&bytes);
        } else {
            let value_offset = payload.len() - 10;
            payload[entry + 8..entry + 12].copy_from_slice(&(value_offset as u32).to_bytes(order));
            payload.extend_from_slice(&bytes);
        }
    }
    payload
}

fn encode_value(order: ByteOrder, value: &Value) -> (u16, u32, Vec<u8>) {
    match value {
        Value::Byte(value) => (1, 1, vec![*value]),
        Value::Ascii(value) => {
            let mut bytes = value.as_bytes().to_vec();
            bytes.push(0);
            (2, bytes.len() as u32, bytes)
        }
        Value::Short(value) => (3, 1, value.to_bytes(order)),
        Value::Unsigned(value) => (4, 1, value.to_bytes(order)),
        Value::Rational(numerator, denominator) => {
            let mut bytes = numerator.to_bytes(order);
            bytes.extend_from_slice(&denominator.to_bytes(order));
            (5, 1, bytes)
        }
        Value::List(values) => {
            let first = values.first().expect("fixture lists are non-empty");
            let field_type = encode_value(order, first).0;
            let bytes = values
                .iter()
                .flat_map(|value| encode_value(order, value).2)
                .collect::<Vec<_>>();
            (field_type, values.len() as u32, bytes)
        }
        _ => panic!("unsupported fixture value"),
    }
}

struct TiffFixture {
    bytes: Vec<u8>,
    order: ByteOrder,
}

impl TiffFixture {
    fn new(order: ByteOrder) -> Self {
        let mut bytes = match order {
            ByteOrder::Little => b"II*\0".to_vec(),
            ByteOrder::Big => b"MM\0*".to_vec(),
        };
        bytes.extend_from_slice(&8_u32.to_bytes(order));
        Self { bytes, order }
    }

    fn data(&mut self, data: &[u8]) -> u32 {
        let offset = self.bytes.len() as u32;
        self.bytes.extend_from_slice(data);
        if self.bytes.len() % 2 != 0 {
            self.bytes.push(0);
        }
        offset
    }

    fn ifd(&mut self, entries: &[(u16, Value)]) -> u32 {
        let ifd_offset = self.bytes.len() as u32;
        self.bytes
            .extend_from_slice(&(entries.len() as u16).to_bytes(self.order));
        self.bytes
            .resize(self.bytes.len() + entries.len() * 12 + 4, 0);

        for (index, (tag, value)) in entries.iter().enumerate() {
            let (field_type, count, bytes) = encode_value(self.order, value);
            let entry = ifd_offset as usize + 2 + index * 12;
            self.bytes[entry..entry + 2].copy_from_slice(&tag.to_bytes(self.order));
            self.bytes[entry + 2..entry + 4].copy_from_slice(&field_type.to_bytes(self.order));
            self.bytes[entry + 4..entry + 8].copy_from_slice(&count.to_bytes(self.order));
            if bytes.len() <= 4 {
                self.bytes[entry + 8..entry + 8 + bytes.len()].copy_from_slice(&bytes);
            } else {
                let offset = self.data(&bytes);
                self.bytes[entry + 8..entry + 12].copy_from_slice(&offset.to_bytes(self.order));
            }
        }
        ifd_offset
    }

    fn root(mut self, entries: &[(u16, Value)]) -> Vec<u8> {
        let root = self.ifd(entries);
        self.bytes[4..8].copy_from_slice(&root.to_bytes(self.order));
        self.bytes
    }
}

trait ToBytes {
    fn to_bytes(self, order: ByteOrder) -> Vec<u8>;
}

impl ToBytes for u16 {
    fn to_bytes(self, order: ByteOrder) -> Vec<u8> {
        match order {
            ByteOrder::Little => self.to_le_bytes().to_vec(),
            ByteOrder::Big => self.to_be_bytes().to_vec(),
        }
    }
}

impl ToBytes for u32 {
    fn to_bytes(self, order: ByteOrder) -> Vec<u8> {
        match order {
            ByteOrder::Little => self.to_le_bytes().to_vec(),
            ByteOrder::Big => self.to_be_bytes().to_vec(),
        }
    }
}
