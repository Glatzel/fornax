use fornax_tiff::{Tag, Value, parse};

#[test]
fn panasonic_magic_parses_a_classic_ifd() {
    let source = [
        b'I', b'I', 0x55, 0x00, // Panasonic classic-TIFF magic.
        0x08, 0x00, 0x00, 0x00, // First IFD.
        0x01, 0x00, // One entry.
        0x00, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00,
        0x00, 0x00, 0x00, 0x00, // No next IFD.
    ];

    let parsed = parse(&source).unwrap();

    assert_eq!(parsed.root.get(&Tag(0x0100)), Some(&Value::Unsigned(9)));
}

#[test]
fn panasonic_magic_does_not_enable_bigtiff_offsets() {
    let source = [
        b'I', b'I', 0x55, 0x00, // Panasonic classic-TIFF magic.
        0x08, 0x00, 0x00, 0x00, // Classic first IFD; BigTIFF version would be 8.
        0x00, 0x00, 0x00, 0x00, // Empty IFD count and first four bytes of offset.
        0x00, 0x00, 0x00, 0x00, // Remainder of the BigTIFF-looking first offset.
    ];

    let parsed = parse(&source).unwrap();

    assert_eq!(parsed.directories.len(), 1);
    assert!(parsed.root.is_empty());
}
