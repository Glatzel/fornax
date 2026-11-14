use std::io::Cursor;

use fornax_tiff::{Tag, TiffWriter, Value, parse};

#[test]
fn unsupported_field_types_are_skipped_and_later_tags_still_parse() {
    let source = one_ifd_tiff();

    let parsed = parse(&source).unwrap();

    assert!(!parsed.root().contains_key(&Tag(259)));
    assert_eq!(parsed.root().get(&Tag(256)), Some(&Value::Unsigned(4)));
    assert_eq!(
        parsed.root().get(&Tag(60_001)),
        Some(&Value::Ascii("abc".into()))
    );
}

#[test]
fn sub_ifd_tag_keeps_following_tiff_tree_children() {
    let source = tiff_with_child_ifd();

    let parsed = parse(&source).unwrap();

    assert_eq!(parsed.directories.len(), 2);
    assert_eq!(
        parsed.directories[1].get(&Tag(256)),
        Some(&Value::Unsigned(9))
    );
}

#[test]
fn large_byte_values_switch_to_compact_storage_without_changing_access() {
    let source = tiff_with_byte_payloads(64, 65);
    let parsed = parse(&source).unwrap();
    let small = parsed.root().get(&Tag(100)).unwrap();
    let large = parsed.root().get(&Tag(101)).unwrap();

    assert!(matches!(small, Value::List(_)));
    assert!(matches!(large, Value::Bytes(_)));
    assert_eq!((small.count(), large.count()), (64, 65));
    for index in 0..small.count() {
        assert_eq!(small.force_u32(index), index as u32);
    }
    for index in 0..large.count() {
        assert_eq!(large.force_u32(index), index as u32);
    }
    assert_eq!(
        large.clone().into_u64_vec().unwrap(),
        (0_u64..65).collect::<Vec<_>>()
    );

    let payload = (0_u8..65).collect::<Vec<_>>();
    let mut output = Cursor::new(Vec::new());
    let mut writer = TiffWriter::new(&mut output).unwrap();
    writer
        .finish([(Tag(101), Value::Bytes(payload.clone()))])
        .unwrap();
    let round_trip = parse(output.get_ref()).unwrap();
    assert_eq!(
        round_trip.root().get(&Tag(101)),
        Some(&Value::Bytes(payload))
    );
}

fn one_ifd_tiff() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"II");
    bytes.extend_from_slice(&42_u16.to_le_bytes());
    bytes.extend_from_slice(&8_u32.to_le_bytes());
    bytes.extend_from_slice(&3_u16.to_le_bytes());

    push_entry(&mut bytes, 259, 99, 1, [0; 4]);
    push_entry(&mut bytes, 256, 4, 1, 4_u32.to_le_bytes());
    push_entry(&mut bytes, 60_001, 2, 4, *b"abc\0");

    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes
}

fn tiff_with_byte_payloads(small_len: usize, large_len: usize) -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"II");
    bytes.extend_from_slice(&42_u16.to_le_bytes());
    bytes.extend_from_slice(&8_u32.to_le_bytes());
    bytes.extend_from_slice(&2_u16.to_le_bytes());

    let data_offset = 8 + 2 + 2 * 12 + 4;
    push_entry(
        &mut bytes,
        100,
        7,
        u32::try_from(small_len).unwrap(),
        u32::try_from(data_offset).unwrap().to_le_bytes(),
    );
    push_entry(
        &mut bytes,
        101,
        7,
        u32::try_from(large_len).unwrap(),
        u32::try_from(data_offset + small_len)
            .unwrap()
            .to_le_bytes(),
    );
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes.extend((0..small_len).map(|value| value as u8));
    bytes.extend((0..large_len).map(|value| value as u8));
    bytes
}

fn tiff_with_child_ifd() -> Vec<u8> {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(b"II");
    bytes.extend_from_slice(&42_u16.to_le_bytes());
    bytes.extend_from_slice(&8_u32.to_le_bytes());
    bytes.extend_from_slice(&1_u16.to_le_bytes());
    push_entry(&mut bytes, 330, 4, 1, 26_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());

    bytes.extend_from_slice(&1_u16.to_le_bytes());
    push_entry(&mut bytes, 256, 4, 1, 9_u32.to_le_bytes());
    bytes.extend_from_slice(&0_u32.to_le_bytes());
    bytes
}

fn push_entry(bytes: &mut Vec<u8>, tag: u16, field_type: u16, count: u32, value: [u8; 4]) {
    bytes.extend_from_slice(&tag.to_le_bytes());
    bytes.extend_from_slice(&field_type.to_le_bytes());
    bytes.extend_from_slice(&count.to_le_bytes());
    bytes.extend_from_slice(&value);
}
