#![cfg(all(feature = "mmap", unix))]

use fornax_dng::DngTag;
use fornax_dng::writer::{DngCompression, DngWriter};
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_core::{MonoImage, RawMosaic};
use fornax_raw_decoder::Decoder;
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{Directory, Tag, Value};

#[test]
fn mapped_dng_remains_decodable_after_the_file_is_removed() {
    let temp = tempfile::tempdir().unwrap();
    let path = temp.path().join("mapped.dng");
    let samples = vec![0_u16, 1, 2, 3, 4, 5, 6, 7];
    let mosaic = RawMosaic::U16(MonoImage::from_raw(4, 2, samples.clone()).unwrap());
    DngWriter::new(DngCompression::Uncompressed)
        .write_dng_with(std::fs::File::create(&path).unwrap(), &mosaic, &metadata())
        .unwrap();

    let decoder = Decoder::decode(&path).unwrap();
    std::fs::remove_file(&path).unwrap();
    let RawMosaic::U16(decoded) = decoder.raw_mosaic().unwrap() else {
        panic!("written U16 samples must decode as U16");
    };

    assert_eq!(decoded.as_raw().as_slice(), samples);
}

fn metadata() -> Directory {
    let (width, height) = (4_u32, 2_u32);
    Directory::from([
        (
            DngTag::ActiveArea.into(),
            Value::List(vec![
                Value::Unsigned(0),
                Value::Unsigned(0),
                Value::Unsigned(height),
                Value::Unsigned(width),
            ]),
        ),
        (
            DngTag::DefaultCropOrigin.into(),
            Value::List(vec![Value::Unsigned(0), Value::Unsigned(0)]),
        ),
        (
            DngTag::DefaultCropSize.into(),
            Value::List(vec![Value::Unsigned(width), Value::Unsigned(height)]),
        ),
        (
            DngTag::BlackLevelRepeatDim.into(),
            Value::List(vec![Value::Short(2), Value::Short(2)]),
        ),
        (
            DngTag::BlackLevel.into(),
            Value::List(vec![
                Value::Short(0),
                Value::Short(0),
                Value::Short(0),
                Value::Short(0),
            ]),
        ),
        (DngTag::WhiteLevel.into(), Value::Unsigned(65_535)),
        (
            TiffTag::CfaRepeatPatternDim.into(),
            Value::List(vec![Value::Short(2), Value::Short(2)]),
        ),
        (
            TiffTag::CfaPattern.into(),
            Value::List(vec![
                Value::Byte(0),
                Value::Byte(1),
                Value::Byte(1),
                Value::Byte(2),
            ]),
        ),
        (
            DngTag::CfaPlaneColor.into(),
            Value::List(vec![Value::Byte(0), Value::Byte(1), Value::Byte(2)]),
        ),
        (Tag::from(DngTag::CfaLayout), Value::Short(1)),
    ])
}
