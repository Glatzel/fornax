use std::io::Cursor;

use fornax_tiff::constants::TiffTag;
use fornax_tiff::{Tag, TiffDirectory, TiffWriter, Value, parse};

#[test]
fn tiff_directory_round_trips_typed_and_dynamic_entries() {
    let mut output = Cursor::new(Vec::new());
    let mut writer = TiffWriter::new(&mut output).unwrap();
    let mut directory = TiffDirectory::new(&mut writer);
    let data_offset = directory.write_data(b"pixels").unwrap();
    let data_offset = u32::try_from(data_offset).unwrap();

    directory.write_tag(TiffTag::ImageWidth, 2_u32).unwrap();
    directory.write_tag(TiffTag::ImageLength, 2_u32).unwrap();
    directory
        .write_tag(TiffTag::StripOffsets, &[data_offset][..])
        .unwrap();
    directory
        .write_tag(Tag(60_001), Value::Ascii("private".into()))
        .unwrap();
    directory.finish().unwrap();

    let parsed = parse(output.get_ref()).unwrap();
    assert_eq!(
        parsed.root.get(&TiffTag::ImageWidth.into()),
        Some(&Value::Unsigned(2))
    );
    assert_eq!(
        parsed.root.get(&TiffTag::StripOffsets.into()),
        Some(&Value::Unsigned(data_offset))
    );
    assert_eq!(
        parsed.root.get(&Tag(60_001)),
        Some(&Value::Ascii("private".into()))
    );
}
