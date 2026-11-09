use fornax_tiff::{ByteOrder, Tag, Value, parse_ifd_at};

#[test]
fn inner_ifd_offsets_can_be_corrected_for_an_extracted_buffer() {
    let mut source = [0_u8; 48];
    source[16..18].copy_from_slice(&1_u16.to_le_bytes());
    source[18..20].copy_from_slice(&0x1234_u16.to_le_bytes());
    source[20..22].copy_from_slice(&4_u16.to_le_bytes());
    source[22..26].copy_from_slice(&2_u32.to_le_bytes());
    source[26..30].copy_from_slice(&40_u32.to_le_bytes());
    source[30..32].copy_from_slice(&0_u16.to_le_bytes());
    source[40..44].copy_from_slice(&0x1122_3344_u32.to_le_bytes());
    source[44..48].copy_from_slice(&0x5566_7788_u32.to_le_bytes());

    let extracted = &source[16..];
    let metadata = parse_ifd_at(extracted, 0, ByteOrder::Little, -16).unwrap();

    assert_eq!(
        metadata.get(&Tag(0x1234)),
        Some(&Value::List(vec![
            Value::Unsigned(0x1122_3344),
            Value::Unsigned(0x5566_7788),
        ]))
    );
}

#[test]
fn corrected_inner_ifd_rejects_an_offset_before_the_extracted_buffer() {
    let mut source = [0_u8; 32];
    source[16..18].copy_from_slice(&1_u16.to_le_bytes());
    source[18..20].copy_from_slice(&0x1234_u16.to_le_bytes());
    source[20..22].copy_from_slice(&4_u16.to_le_bytes());
    source[22..26].copy_from_slice(&2_u32.to_le_bytes());
    source[26..30].copy_from_slice(&24_u32.to_le_bytes());

    let error = parse_ifd_at(&source[16..], 0, ByteOrder::Little, -25).unwrap_err();

    assert!(error.to_string().contains("offset correction"));
}
