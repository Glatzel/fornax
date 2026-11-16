use std::io::Cursor;

use fornax_dng::Directory;
use fornax_dng::writer::{DngWriter, DngWriterCompression};
use fornax_raw_core::{MonoImage, RawMosaic};
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{Value, parse};

#[test]
fn writer_emits_tiff_and_dng_owned_compression_ids() {
    let mosaic = RawMosaic::U16(MonoImage::from_raw(1, 1, vec![7]).unwrap());

    let mut cases = vec![
        (DngWriterCompression::Uncompressed, 1),
        (DngWriterCompression::Lossless, 7),
        (DngWriterCompression::Deflate, 8),
    ];
    #[cfg(feature = "zstd")]
    cases.push((DngWriterCompression::Zstd { level: 3 }, 50_000));

    for (compression, expected) in cases {
        let mut output = Cursor::new(Vec::new());
        DngWriter::new(compression)
            .write_dng_with(&mut output, mosaic.clone(), &Directory::default())
            .unwrap();
        let parsed = parse(output.get_ref()).unwrap();
        let actual =
            parsed
                .root()
                .get(&TiffTag::Compression.into())
                .and_then(|value| match value {
                    Value::Short(value) => Some(*value),
                    _ => None,
                });
        let photometric = parsed
            .root()
            .get(&TiffTag::PhotometricInterpretation.into())
            .and_then(|value| match value {
                Value::Short(value) => Some(*value),
                _ => None,
            });

        assert_eq!(actual, Some(expected), "compression {compression:?}");
        assert_eq!(photometric, Some(32_803), "photometric for {compression:?}");
    }
}
