use std::io::Cursor;

use fornax_dng::DngTag;
use fornax_dng::dng_decoder::DngImage;
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{Tag, TiffWriter, Value};

#[test]
fn dng_metadata_returns_directory_with_tag_keys() {
    let mut output = Cursor::new(Vec::new());
    let mut writer = TiffWriter::new(&mut output).unwrap();
    writer
        .finish([
            (TiffTag::NewSubfileType.into(), Value::Unsigned(0)),
            (TiffTag::ImageWidth.into(), Value::Unsigned(4)),
            (TiffTag::ImageLength.into(), Value::Unsigned(4)),
            (TiffTag::BitsPerSample.into(), Value::Short(16)),
            (TiffTag::Compression.into(), Value::Short(1)),
            (
                TiffTag::PhotometricInterpretation.into(),
                Value::Short(32_803),
            ),
            (TiffTag::SamplesPerPixel.into(), Value::Short(1)),
            (TiffTag::TileWidth.into(), Value::Unsigned(4)),
            (TiffTag::TileLength.into(), Value::Unsigned(4)),
            (TiffTag::TileOffsets.into(), Value::Unsigned(0)),
            (TiffTag::TileByteCounts.into(), Value::Unsigned(32)),
            (
                DngTag::DngVersion.into(),
                Value::List(vec![
                    Value::Byte(1),
                    Value::Byte(7),
                    Value::Byte(1),
                    Value::Byte(0),
                ]),
            ),
            (
                DngTag::ActiveArea.into(),
                Value::List(vec![
                    Value::Unsigned(0),
                    Value::Unsigned(0),
                    Value::Unsigned(4),
                    Value::Unsigned(4),
                ]),
            ),
            (Tag(60_001), Value::Ascii("private metadata".into())),
        ])
        .unwrap();

    let metadata = DngImage::parse(output.get_ref())
        .unwrap()
        .dng_metadata()
        .unwrap();

    assert_eq!(
        metadata.get(&TiffTag::ImageWidth.into()),
        Some(&Value::Unsigned(4))
    );
    assert_eq!(
        metadata.get(&DngTag::ActiveArea.into()),
        Some(&Value::List(vec![
            Value::Unsigned(0),
            Value::Unsigned(0),
            Value::Unsigned(4),
            Value::Unsigned(4),
        ]))
    );
    assert_eq!(
        metadata.get(&Tag(60_001)),
        Some(&Value::Ascii("private metadata".into()))
    );
    assert_eq!(DngTag::from(Tag(50_706)), DngTag::DngVersion);
    assert_eq!(DngTag::from(Tag(60_001)), DngTag::Unknown(60_001));
}
