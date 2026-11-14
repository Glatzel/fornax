use fornax_tiff::{TiffSource, parse, parse_source};

#[test]
fn parse_source_matches_borrowed_parse_and_borrows_root_from_directory_zero() {
    let source = [
        b'I', b'I', 0x2a, 0x00, // Classic little-endian TIFF.
        0x08, 0x00, 0x00, 0x00, // First IFD.
        0x01, 0x00, // One entry.
        0x00, 0x01, 0x04, 0x00, 0x01, 0x00, 0x00, 0x00, 0x09, 0x00, 0x00, 0x00, 0x00, 0x00, 0x00,
        0x00, // No next IFD.
    ];

    let borrowed = parse(&source).unwrap();
    let owned = parse_source(TiffSource::from_vec(source.to_vec())).unwrap();

    assert_eq!(borrowed.directories, owned.directories);
    assert_eq!(borrowed.source.as_ref(), source.as_slice());
    assert_eq!(owned.source.as_ref(), source.as_slice());
    assert!(std::ptr::eq(
        std::ptr::from_ref(owned.root()),
        std::ptr::from_ref(&owned.directories[0]),
    ));
}
