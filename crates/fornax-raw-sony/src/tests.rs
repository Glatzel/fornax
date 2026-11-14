use std::io::Cursor;
use std::path::{Path, PathBuf};

use fornax_dng::constants::DngPhotometricInterpretation;
use fornax_raw_core::protocol::IDecoder;
use fornax_raw_core::{EncodingSampleFormat, RawMosaic};
use fornax_tiff::constants::{PhotometricInterpretation, TiffTag};
use fornax_tiff::{Tag, TiffWriter, Value};
use rawler::decoders::RawImageData;
use sha2::{Digest, Sha256};

use super::ArwImage;
use crate::container::{CompressionFamily, RawLayout};

#[test]
fn malformed_tiff_is_invalid_input() {
    let error = ArwImage::parse(b"not a tiff").unwrap_err();
    assert!(matches!(
        error,
        fornax_raw_core::error::DecodeError::InvalidInput(_)
    ));
}

#[test]
fn arq_multi_sample_layout_is_unsupported() {
    let source = synthetic_tiff(1, u16::from(PhotometricInterpretation::Cfa), 16, 4, 8);
    let error = ArwImage::parse(&source).unwrap_err();
    assert!(matches!(
        error,
        fornax_raw_core::error::DecodeError::Unsupported(_)
    ));
}

#[test]
fn reduced_resolution_linear_raw_is_unsupported() {
    let source = synthetic_tiff(
        1,
        u16::from(DngPhotometricInterpretation::LinearRaw),
        16,
        1,
        8,
    );
    let error = ArwImage::parse(&source).unwrap_err();
    assert!(matches!(
        error,
        fornax_raw_core::error::DecodeError::Unsupported(_)
    ));
}

#[test]
fn unknown_sony_compression_is_unsupported() {
    let source = synthetic_tiff(12_345, u16::from(PhotometricInterpretation::Cfa), 16, 1, 8);
    let error = ArwImage::parse(&source).unwrap_err();
    assert!(matches!(
        error,
        fornax_raw_core::error::DecodeError::Unsupported(_)
    ));
}

#[test]
fn sony_private_compression_ids_remain_vendor_owned() {
    for (compression, bytes, expected) in [
        (1, 8, CompressionFamily::Uncompressed),
        (7, 8, CompressionFamily::SonyLosslessJpeg),
        (32_766, 8, CompressionFamily::Arw6),
        (32_767, 1, CompressionFamily::Arw2),
        (32_767, 8, CompressionFamily::Arw1),
    ] {
        let source = synthetic_tiff(
            compression,
            u16::from(PhotometricInterpretation::Cfa),
            16,
            1,
            bytes,
        );
        let parsed = fornax_tiff::parse(&source).unwrap();
        let (_, layout) = RawLayout::parse(&parsed).unwrap();

        assert_eq!(
            layout.compression, expected,
            "compression {compression} with {bytes} stored bytes"
        );
    }
}

#[test]
fn encoding_metadata_distinguishes_sony_compression_families() {
    for (compression, expected_id, expected_name) in [
        (CompressionFamily::Uncompressed, 1, "uncompressed"),
        (CompressionFamily::Arw1, 32_767, "sony-arw1"),
        (CompressionFamily::Arw2, 32_767, "sony-arw2"),
        (CompressionFamily::Arw6, 32_766, "sony-arw6"),
        (CompressionFamily::SonyLosslessJpeg, 7, "sony-lossless-jpeg"),
    ] {
        let layout = RawLayout {
            compression,
            width: 64,
            stored_height: 32,
            bits_per_sample: 14,
            offsets: vec![0],
            byte_counts: vec![128],
            tile_width: Some(16),
            tile_height: Some(8),
        };

        let encoding = crate::encoding_metadata_for_layout(&layout);

        assert_eq!(encoding.compression_id, Some(expected_id));
        assert_eq!(encoding.compression_name, expected_name);
        assert_eq!(encoding.storage, "tiles");
        assert_eq!(encoding.bits_per_sample, Some(14));
        assert_eq!(encoding.sample_format, Some(EncodingSampleFormat::Integer));
        assert_eq!(encoding.tile_width, Some(16));
        assert_eq!(encoding.tile_height, Some(8));
    }
}

#[test]
fn selected_fixtures_match_rawler_samples() {
    for variable in [
        "FORNAX_SONY_ARW_FIXTURE",
        "FORNAX_SONY_ARW2_FIXTURE",
        "FORNAX_SONY_LJPEG_FIXTURE",
        "FORNAX_SONY_UNCOMPRESSED_FIXTURE",
    ] {
        let Some(path) = fixture(variable) else {
            continue;
        };
        let decoder = ArwImage::decode(&path).unwrap_or_else(|error| {
            panic!("{} failed to parse: {error}", path.display());
        });
        let ours = match decoder.raw_mosaic().unwrap() {
            RawMosaic::U16(image) => image,
            other => panic!("{} decoded as {other:?}", path.display()),
        };
        let rawler = rawler::decode_file(&path).unwrap_or_else(|error| {
            panic!("Rawler failed on {}: {error}", path.display());
        });
        let reference = match rawler.data {
            RawImageData::Integer(samples) => samples,
            RawImageData::Float(_) => panic!("Rawler returned floating-point samples"),
        };
        assert_eq!(
            ours.dimensions(),
            (rawler.width as u32, rawler.height as u32),
            "{}",
            path.display()
        );
        let mismatch = ours
            .as_raw()
            .iter()
            .zip(&reference)
            .position(|(ours, reference)| ours != reference);
        if let Some(index) = mismatch {
            let differences = ours
                .as_raw()
                .iter()
                .zip(&reference)
                .filter(|(ours, reference)| ours != reference)
                .count();
            panic!(
                "{} differs at sample {index}: {} != {} ({differences} differences)",
                path.display(),
                ours.as_raw()[index],
                reference[index]
            );
        }
        let process = decoder.process_metadata().unwrap_or_else(|error| {
            panic!("{} process metadata failed: {error}", path.display());
        });
        let active = rawler.active_area.map_or(
            (0, 0, rawler.width as u32, rawler.height as u32),
            |active| {
                (
                    active.x() as u32,
                    active.y() as u32,
                    active.width() as u32,
                    active.height() as u32,
                )
            },
        );
        assert_eq!(
            (
                process.active_area.x,
                process.active_area.y,
                process.active_area.width,
                process.active_area.height
            ),
            active,
            "{} active area",
            path.display()
        );
        assert_eq!(
            process.levels.white as u32,
            rawler.whitelevel.0[0],
            "{} white level",
            path.display()
        );
        if rawler.blacklevel.width == 2 && rawler.blacklevel.height == 2 {
            let ours_black = process
                .levels
                .black
                .as_raw()
                .iter()
                .map(|value| value.round() as u16)
                .collect::<Vec<_>>();
            let rawler_black = rawler
                .blacklevel
                .levels
                .iter()
                .map(|value| value.as_f32().round() as u16)
                .collect::<Vec<_>>();
            assert_eq!(ours_black, rawler_black, "{} black levels", path.display());
        }
        let cropped = decoder.cropped_mosaic::<f32>().unwrap_or_else(|error| {
            panic!("{} cropped mosaic failed: {error}", path.display());
        });
        assert_eq!(
            cropped.image().dimensions(),
            (process.active_area.width, process.active_area.height),
            "{} cropped dimensions",
            path.display()
        );
    }
}

#[test]
fn selected_arw6_fixture_decodes_to_the_active_area() {
    let Some(path) = fixture("FORNAX_SONY_ARW6_FIXTURE") else {
        return;
    };
    let decoder = ArwImage::decode(&path).unwrap_or_else(|error| {
        panic!("{} failed to parse: {error}", path.display());
    });
    let full = decoder.raw_mosaic().unwrap();
    assert!(full.dimensions().0 > 0);
    assert!(full.dimensions().1 > 0);
    let process = decoder.process_metadata().unwrap();
    let cropped = decoder.cropped_mosaic::<f32>().unwrap();
    assert_eq!(
        cropped.image().dimensions(),
        (process.active_area.width, process.active_area.height)
    );
}

#[test]
fn selected_fixtures_match_expected_raw_sample_hashes() {
    for (variable, expected) in [
        (
            "FORNAX_SONY_ARW_FIXTURE",
            (
                (4288, 2856),
                "41d68e2824b9ec3d6a590cbc0c83897467069fb988c4060aa28615493fc5fe58",
            ),
        ),
        (
            "FORNAX_SONY_ARW2_FIXTURE",
            (
                (6048, 4024),
                "72ca99b65c342ce267104ed153ce66f52d2b30653dd3071e0fbeac0b05266a5f",
            ),
        ),
        (
            "FORNAX_SONY_ARW6_FIXTURE",
            (
                (7040, 4688),
                "3a6d2903c8a442796135eb1482cbc8809e058da5fb889a6e1f341ce592a7221f",
            ),
        ),
        (
            "FORNAX_SONY_LJPEG_FIXTURE",
            (
                (8704, 6144),
                "6653f2429954b8e29c03a863efedf5fcabb7d980cc21a7f1f8b2072b3e551acd",
            ),
        ),
        (
            "FORNAX_SONY_UNCOMPRESSED_FIXTURE",
            (
                (6048, 4024),
                "946cd70f2bcb6a653cec8f3f4f05fd5b999d9d35f6c57c703ba76e823099092c",
            ),
        ),
    ] {
        let Some(path) = fixture(variable) else {
            continue;
        };
        let decoder = ArwImage::decode(&path).unwrap();
        let mosaic = decoder.raw_mosaic().unwrap();
        assert_eq!(mosaic.dimensions(), expected.0, "{}", path.display());
        let digest = match mosaic {
            RawMosaic::U8(image) => Sha256::digest(image.as_raw()),
            RawMosaic::U16(image) => Sha256::digest(bytemuck::cast_slice(image.as_raw())),
            RawMosaic::U32(image) => Sha256::digest(bytemuck::cast_slice(image.as_raw())),
            RawMosaic::F32(image) => Sha256::digest(bytemuck::cast_slice(image.as_raw())),
        };
        let actual = digest
            .iter()
            .map(|byte| format!("{byte:02x}"))
            .collect::<String>();
        assert_eq!(actual, expected.1, "{}", path.display());
    }
}

fn fixture(variable: &str) -> Option<PathBuf> {
    let path = PathBuf::from(std::env::var_os(variable)?);
    assert!(
        Path::new(&path).is_file(),
        "{variable} does not point to a file: {}",
        path.display()
    );
    Some(path)
}

fn synthetic_tiff(
    compression: u16,
    photometric: u16,
    bits: u16,
    samples_per_pixel: u16,
    bytes: usize,
) -> Vec<u8> {
    let mut output = Cursor::new(Vec::new());
    {
        let mut writer = TiffWriter::new(&mut output).unwrap();
        let data = writer.write_data(&vec![0_u8; bytes]).unwrap();
        let data = u32::try_from(data).unwrap();
        let bytes = u32::try_from(bytes).unwrap();
        writer
            .finish([
                (Tag::from(TiffTag::Make), Value::Ascii("SONY".into())),
                (Tag::from(TiffTag::ImageWidth), Value::Unsigned(1)),
                (Tag::from(TiffTag::ImageLength), Value::Unsigned(1)),
                (Tag::from(TiffTag::BitsPerSample), Value::Short(bits)),
                (Tag::from(TiffTag::Compression), Value::Short(compression)),
                (
                    Tag::from(TiffTag::PhotometricInterpretation),
                    Value::Short(photometric),
                ),
                (Tag::from(TiffTag::StripOffsets), Value::Unsigned(data)),
                (Tag::from(TiffTag::StripByteCounts), Value::Unsigned(bytes)),
                (
                    Tag::from(TiffTag::SamplesPerPixel),
                    Value::Short(samples_per_pixel),
                ),
            ])
            .unwrap();
    };
    output.into_inner()
}
