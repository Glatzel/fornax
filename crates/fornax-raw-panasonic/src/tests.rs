use std::io::Cursor;

use fornax_dng::metadata::DngTag;
use fornax_dng::writer::{DngCompression, DngWriter};
use fornax_raw_core::error::DecodeError;
use fornax_raw_core::{BayerPattern, EncodingSampleFormat, RawMosaic};
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{Tag, TiffDirectory, TiffWriter, Value};
use sha2::{Digest, Sha256};

use crate::Rw2Image;

const RAW_OFFSET: Tag = Tag(0x0118);
const STRIP_OFFSETS: Tag = Tag(0x0111);
const STRIP_BYTE_COUNTS: Tag = Tag(0x0117);

#[test]
fn parses_layout_from_rgb_samples_and_maps_crop_and_levels() {
    let source = synthetic_rw2(&Synthetic::default());

    let image = Rw2Image::parse(&source).unwrap();
    let metadata = image.process_metadata().unwrap();

    assert_eq!(image.layout_dimensions(), (14, 10));
    assert_eq!(metadata.active_area.x, 1);
    assert_eq!(metadata.active_area.y, 1);
    assert_eq!(metadata.active_area.width, 12);
    assert_eq!(metadata.active_area.height, 8);
    assert_eq!(
        (
            metadata.default_crop.x,
            metadata.default_crop.y,
            metadata.default_crop.width,
            metadata.default_crop.height,
        ),
        (2, 1, 9, 6)
    );
    assert_eq!(metadata.bayer_pattern, BayerPattern::BGGR);
    assert_eq!(image.black_pattern(), Some([135, 125, 125, 115]));
    assert_eq!(metadata.levels.white, 4095.0);
    let dng = image.dng_metadata().unwrap();
    let neutral = neutral(&dng);
    assert!((neutral[0] - 2.0).abs() < 1e-6);
    assert!((neutral[1] - 1.0).abs() < 1e-6);
    assert!((neutral[2] - 300.0 / 256.0).abs() < 1e-6);
}

#[test]
fn white_level_uses_the_maximum_present_linearity_limit() {
    let source = synthetic_rw2(&Synthetic {
        linearity: [Some(3900), None, Some(4000)],
        ..Synthetic::default()
    });
    let image = Rw2Image::parse(&source).unwrap();

    assert_eq!(image.process_metadata().unwrap().levels.white, 4000.0);
}

#[test]
fn crop_right_and_bottom_are_exclusive_coordinates() {
    let source = synthetic_rw2(&Synthetic {
        crop: (2, 3, 8, 13),
        ..Synthetic::default()
    });
    let image = Rw2Image::parse(&source).unwrap();
    let crop = image.process_metadata().unwrap().default_crop;

    assert_eq!((crop.x, crop.y, crop.width, crop.height), (2, 1, 10, 6));
}

#[test]
fn all_zero_crop_tags_mean_the_full_active_area() {
    let source = synthetic_rw2(&Synthetic {
        crop: (0, 0, 0, 0),
        ..Synthetic::default()
    });
    let crop = Rw2Image::parse(&source)
        .unwrap()
        .process_metadata()
        .unwrap()
        .default_crop;

    assert_eq!((crop.x, crop.y, crop.width, crop.height), (0, 0, 12, 8));
}

#[test]
fn strip_offset_layout_decodes_when_raw_data_offset_is_absent() {
    let source = synthetic_rw2(&Synthetic {
        use_raw_data_offset: false,
        ..Synthetic::default()
    });
    let image = Rw2Image::parse(&source).unwrap();

    assert!(matches!(image.raw_mosaic().unwrap(), RawMosaic::U16(_)));
}

#[test]
fn raw_data_offset_wins_when_both_layout_tags_are_present() {
    let source = synthetic_rw2(&Synthetic {
        include_strip_fallback: true,
        ..Synthetic::default()
    });
    let image = Rw2Image::parse(&source).unwrap();

    assert!(matches!(image.raw_mosaic().unwrap(), RawMosaic::U16(_)));
}

#[test]
fn encoding_metadata_distinguishes_panasonic_compression_families() {
    let v8 = crate::container::Family::V8(
        crate::PanasonicV8Params::new(
            vec![0; 6],
            vec![0; 6],
            1,
            [0; 4],
            vec![(1, 0)],
            vec![0],
            vec![crate::PanasonicV8Strip::new(0, 1, 0, 0, 2, 2).unwrap()],
        )
        .unwrap(),
    );
    for (family, expected_name) in [
        (
            crate::container::Family::V4 {
                split_blocks: false,
            },
            "panasonic-v4",
        ),
        (crate::container::Family::V5, "panasonic-v5"),
        (crate::container::Family::V6, "panasonic-v6"),
        (v8, "panasonic-v8"),
        (crate::container::Family::Unpacked12, "panasonic-unpacked12"),
        (
            crate::container::Family::Packed12Control,
            "panasonic-packed12-control",
        ),
    ] {
        let layout = crate::container::RawLayout {
            width: 64,
            height: 32,
            bits: 14,
            chunk: fornax_codec::Chunk {
                offset: 0,
                byte_count: 128,
            },
            family,
        };

        let encoding = crate::encoding_metadata_for_layout(&layout);

        assert_eq!(encoding.compression_id, Some(34_316));
        assert_eq!(encoding.compression_name, expected_name);
        assert_eq!(encoding.storage, "single-chunk");
        assert_eq!(encoding.bits_per_sample, Some(14));
        assert_eq!(encoding.sample_format, Some(EncodingSampleFormat::Integer));
    }
}

#[test]
fn v7_unknown_versions_multishot_and_unknown_cfa_are_unsupported() {
    for source in [
        synthetic_rw2(&Synthetic {
            version: 7,
            ..Synthetic::default()
        }),
        synthetic_rw2(&Synthetic {
            version: 9,
            ..Synthetic::default()
        }),
        synthetic_rw2(&Synthetic {
            multishot: 65_536,
            ..Synthetic::default()
        }),
        synthetic_rw2(&Synthetic {
            cfa: 9,
            ..Synthetic::default()
        }),
    ] {
        let error = Rw2Image::parse(&source).unwrap_err();
        assert!(matches!(error, DecodeError::Unsupported(_)), "{error}");
    }
}

#[test]
fn non_rectangular_active_area_is_invalid() {
    let source = synthetic_rw2(&Synthetic {
        borders: (1, 1, 1, 13),
        ..Synthetic::default()
    });

    let error = Rw2Image::parse(&source).unwrap_err();

    assert!(matches!(error, DecodeError::InvalidInput(_)), "{error}");
}

#[test]
fn malformed_camera_ifd_is_ignored_for_single_shot_files() {
    let source = synthetic_rw2(&Synthetic {
        camera_ifd: Some(vec![0x55; 2048]),
        ..Synthetic::default()
    });

    let image = Rw2Image::parse(&source).unwrap();

    assert!(matches!(image.raw_mosaic().unwrap(), RawMosaic::U16(_)));
}

#[test]
fn high_resolution_composites_are_typed_unsupported() {
    let source = synthetic_rw2(&Synthetic {
        width: 10_000,
        height: 7_000,
        ..Synthetic::default()
    });

    let error = Rw2Image::parse(&source).unwrap_err();

    assert!(matches!(error, DecodeError::Unsupported(_)), "{error}");
}

#[test]
fn dng_round_trip_preserves_samples_geometry_cfa_levels_neutral_and_orientation() {
    let source = synthetic_rw2(&Synthetic::default());
    let image = Rw2Image::parse(&source).unwrap();
    let mosaic = image.raw_mosaic().unwrap();
    let metadata = image.dng_metadata().unwrap();
    let mut output = Cursor::new(Vec::new());
    DngWriter::new(DngCompression::Uncompressed)
        .write_dng_with(&mut output, &mosaic, &metadata)
        .unwrap();

    let dng = fornax_dng::dng_decoder::DngImage::parse(output.get_ref()).unwrap();
    let round_trip = dng.dng_metadata().unwrap();
    for tag in [
        u16::from(DngTag::ActiveArea),
        u16::from(DngTag::DefaultCropOrigin),
        u16::from(DngTag::DefaultCropSize),
        u16::from(TiffTag::CfaPattern),
        u16::from(DngTag::BlackLevel),
        u16::from(DngTag::WhiteLevel),
        u16::from(DngTag::AsShotNeutral),
        u16::from(TiffTag::Orientation),
    ] {
        assert_eq!(round_trip.get(&Tag(tag)), metadata.get(&Tag(tag)), "{tag}");
    }
    assert_eq!(
        dng.raw_mosaic().unwrap().dimensions(),
        image.raw_mosaic().unwrap().dimensions()
    );
}

#[test]
fn supported_corpus_fixtures_parse_and_decode() {
    let Some(root) = std::env::var_os("FORNAX_RAW_CORPUS") else {
        return;
    };
    for (relative, dimensions, digest) in [
        (
            "Panasonic/DMC-G85/_1030982.RW2",
            (4816, 3464),
            "6300522432fe22addf150cad43fb2d4f1a43dcb8a6e349533ea872e1f2b1c19a",
        ),
        (
            "Panasonic/DC-GH5S/P1022085.RW2",
            (3744, 2776),
            "9140b72797ff0ecd3e16476457cc1d9bd3b96ded8b0b39eddfe9ba07c625218c",
        ),
        (
            "Panasonic/DC-S1/P1033119.RW2",
            (6072, 4016),
            "35479403cc54a997da32e57a857f3af303ef596a2108cf7064fbce8d0252e9e8",
        ),
        (
            "Panasonic/DC-GH6/4-3.RW2",
            (5792, 4352),
            "6a6e6455b0006d3ee03ca88f429ed2bf94659b00d99217b70418eb69d6a30296",
        ),
    ] {
        let source = std::fs::read(std::path::Path::new(&root).join(relative)).unwrap();
        let image = Rw2Image::parse(&source).unwrap();
        assert_eq!(image.layout_dimensions(), dimensions, "{relative}");
        let RawMosaic::U16(mosaic) = image
            .raw_mosaic()
            .unwrap_or_else(|error| panic!("{relative}: {error}"))
        else {
            panic!("{relative}: non-u16 output");
        };
        assert_eq!(sample_digest(mosaic.as_raw()), digest, "{relative}");
        image.process_metadata().unwrap();
    }
}

fn sample_digest(samples: &[u16]) -> String {
    let mut hasher = Sha256::new();
    for sample in samples {
        hasher.update(sample.to_le_bytes());
    }
    hasher
        .finalize()
        .iter()
        .map(|byte| format!("{byte:02x}"))
        .collect()
}

#[test]
fn expected_unsupported_corpus_fixtures_return_unsupported() {
    let Some(root) = std::env::var_os("FORNAX_RAW_CORPUS") else {
        return;
    };
    for relative in [
        "Panasonic/DC-G9/P1000475.RW2",
        "Panasonic/DC-S5/dc-s5_hr_96MP.RW2",
        "Panasonic/DC-S1RM2/S1Rii_multiexposure.RW2",
    ] {
        let source = std::fs::read(std::path::Path::new(&root).join(relative)).unwrap();
        match Rw2Image::parse(&source) {
            Err(DecodeError::Unsupported(_)) => {}
            Err(error) => panic!("{relative}: {error}"),
            Ok(_) => panic!("{relative}: unexpectedly decoded"),
        }
    }
}

#[test]
fn corpus_metadata_preserves_exif_serial_lens_and_orientation() {
    let Some(root) = std::env::var_os("FORNAX_RAW_CORPUS") else {
        return;
    };
    for (relative, serial, lens) in [
        ("Panasonic/DC-S1/P1033119.RW2", Some("WJ-PP001183"), None),
        ("Panasonic/DC-GH6/4-3.RW2", Some("WG2DA002569"), None),
    ] {
        let source = std::fs::read(std::path::Path::new(&root).join(relative)).unwrap();
        let image = Rw2Image::parse(&source).unwrap();
        let metadata = image.dng_metadata().unwrap();
        assert_eq!(
            metadata
                .get(&Tag::from(DngTag::CameraSerialNumber))
                .and_then(|value| match value {
                    Value::Ascii(value) => Some(value.as_str()),
                    _ => None,
                }),
            serial,
            "{relative}"
        );
        assert_eq!(
            metadata.get(&Tag(0xa434)).and_then(|value| match value {
                Value::Ascii(value) => Some(value.trim()),
                _ => None,
            }),
            lens,
            "{relative}"
        );
        assert_eq!(
            metadata.get(&Tag::from(TiffTag::Orientation)),
            Some(&Value::Short(1)),
            "{relative}"
        );
    }
}

#[derive(Clone)]
struct Synthetic {
    version: u16,
    compression: Option<u16>,
    bits: u16,
    width: u16,
    height: u16,
    borders: (u16, u16, u16, u16),
    crop: (u16, u16, u16, u16),
    cfa: u16,
    linearity: [Option<u16>; 3],
    black: [u16; 3],
    neutral: (u16, u16, u16),
    multishot: u32,
    camera_ifd: Option<Vec<u8>>,
    use_raw_data_offset: bool,
    include_strip_fallback: bool,
}

impl Default for Synthetic {
    fn default() -> Self {
        Self {
            version: 4,
            compression: Some(34316),
            bits: 12,
            width: 14,
            height: 10,
            borders: (1, 1, 9, 13),
            crop: (2, 3, 8, 12),
            cfa: 1,
            linearity: [Some(4095), Some(4095), Some(4095)],
            black: [100, 110, 120],
            neutral: (512, 256, 300),
            multishot: 0,
            camera_ifd: None,
            use_raw_data_offset: true,
            include_strip_fallback: false,
        }
    }
}

fn synthetic_rw2(spec: &Synthetic) -> Vec<u8> {
    let payload = vec![0_u8; 0x4000];
    let mut output = Cursor::new(Vec::new());
    let mut writer = TiffWriter::new(&mut output).unwrap();
    let payload_offset = u32::try_from(writer.write_data(&payload).unwrap()).unwrap();
    let mut directory = TiffDirectory::new(&mut writer);
    directory.write_tag(Tag(0x0002), spec.width).unwrap();
    directory.write_tag(Tag(0x0003), spec.height).unwrap();
    directory.write_tag(Tag(0x0004), spec.borders.0).unwrap();
    directory.write_tag(Tag(0x0005), spec.borders.1).unwrap();
    directory.write_tag(Tag(0x0006), spec.borders.2).unwrap();
    directory.write_tag(Tag(0x0007), spec.borders.3).unwrap();
    directory.write_tag(Tag(0x0008), 1_u16).unwrap();
    directory.write_tag(Tag(0x0009), spec.cfa).unwrap();
    directory.write_tag(Tag(0x000a), spec.bits).unwrap();
    if let Some(compression) = spec.compression {
        directory.write_tag(Tag(0x000b), compression).unwrap();
    }
    for (tag, value) in [0x000e_u16, 0x000f, 0x0010].into_iter().zip(spec.linearity) {
        if let Some(value) = value {
            directory.write_tag(Tag(tag), value).unwrap();
        }
    }
    directory.write_tag(Tag(0x001c), spec.black[0]).unwrap();
    directory.write_tag(Tag(0x001d), spec.black[1]).unwrap();
    directory.write_tag(Tag(0x001e), spec.black[2]).unwrap();
    directory.write_tag(Tag(0x0011), spec.neutral.0).unwrap();
    directory.write_tag(Tag(0x0012), spec.neutral.2).unwrap();
    directory.write_tag(Tag(0x0024), spec.neutral.0).unwrap();
    directory.write_tag(Tag(0x0025), spec.neutral.1).unwrap();
    directory.write_tag(Tag(0x0026), spec.neutral.2).unwrap();
    directory.write_tag(Tag(0x002d), spec.version).unwrap();
    directory.write_tag(Tag(0x002f), spec.crop.0).unwrap();
    directory.write_tag(Tag(0x0030), spec.crop.1).unwrap();
    directory.write_tag(Tag(0x0031), spec.crop.2).unwrap();
    directory.write_tag(Tag(0x0032), spec.crop.3).unwrap();
    directory.write_tag(TiffTag::Make, "Panasonic").unwrap();
    directory.write_tag(TiffTag::Model, "Synthetic").unwrap();
    directory.write_tag(TiffTag::Orientation, 6_u16).unwrap();
    if spec.multishot != 0 {
        directory.write_tag(Tag(0x0121), spec.multishot).unwrap();
    }
    if spec.use_raw_data_offset {
        directory.write_tag(RAW_OFFSET, payload_offset).unwrap();
    }
    if !spec.use_raw_data_offset || spec.include_strip_fallback {
        directory.write_tag(STRIP_OFFSETS, payload_offset).unwrap();
        directory
            .write_tag(STRIP_BYTE_COUNTS, u32::try_from(payload.len()).unwrap())
            .unwrap();
    }
    if let Some(camera_ifd) = &spec.camera_ifd {
        directory
            .write_tag(
                Tag(0x0120),
                Value::List(camera_ifd.iter().copied().map(Value::Byte).collect()),
            )
            .unwrap();
    }
    directory.finish().unwrap();

    let mut bytes = output.into_inner();
    bytes[2..4].copy_from_slice(&85_u16.to_le_bytes());
    bytes
}

fn neutral(directory: &fornax_tiff::Directory) -> Vec<f64> {
    match directory.get(&Tag::from(DngTag::AsShotNeutral)) {
        Some(Value::List(values)) => values
            .iter()
            .map(|value| match value {
                Value::Rational(numerator, denominator) => {
                    f64::from(*numerator) / f64::from(*denominator)
                }
                value => panic!("unexpected neutral value: {value:?}"),
            })
            .collect(),
        value => panic!("missing AsShotNeutral: {value:?}"),
    }
}
