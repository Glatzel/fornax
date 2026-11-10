use std::io::Cursor;

use fornax_core::{MonoImage, RawMosaic};
use fornax_dng::writer::{DngCompression, DngWriter};
use fornax_dng::{Directory};
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{Value, parse};

#[test]
fn writer_emits_tiff_and_dng_owned_compression_ids() {
    let mosaic = RawMosaic::U16(MonoImage::from_raw(1, 1, vec![7]).unwrap());

    for (compression, expected) in [
        (DngCompression::Uncompressed, 1),
        (DngCompression::LosslessJpeg, 7),
        (DngCompression::Deflate, 8),
        (DngCompression::Zstd, 50_000),
    ] {
        let mut output = Cursor::new(Vec::new());
        DngWriter::new(compression)
            .write_dng_with(&mut output, &mosaic, &Directory::default())
            .unwrap();
        let parsed = parse(output.get_ref()).unwrap();
        let actual = parsed
            .root
            .get(&TiffTag::Compression.into())
            .and_then(|value| match value {
                Value::Short(value) => Some(*value),
                _ => None,
            });
        let photometric = parsed
            .root
            .get(&TiffTag::PhotometricInterpretation.into())
            .and_then(|value| match value {
                Value::Short(value) => Some(*value),
                _ => None,
            });

        assert_eq!(actual, Some(expected), "compression {compression:?}");
        assert_eq!(photometric, Some(32_803), "photometric for {compression:?}");
    }
}
