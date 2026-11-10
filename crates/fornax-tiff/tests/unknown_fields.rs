use fornax_tiff::{Tag, Value, parse};

#[test]
fn unsupported_field_types_are_skipped_and_later_tags_still_parse() {
    let source = one_ifd_tiff();

    let parsed = parse(&source).unwrap();

    assert!(!parsed.root.contains_key(&Tag(259)));
    assert_eq!(parsed.root.get(&Tag(256)), Some(&Value::Unsigned(4)));
    assert_eq!(
        parsed.root.get(&Tag(60_001)),
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
