use std::io::Cursor;
use std::path::PathBuf;

use fornax_dng::dng_decoder::DngImage;
use fornax_dng::writer::{DngCompression, DngWriter};
use fornax_raw_core::RawMosaic;
use fornax_raw_core::error::DecodeError;

use super::container::{parse_raf_tags, xtrans_pattern};
use super::profiles::profile;

#[test]
fn non_raf_signature_is_rejected() {
    let error = super::RafImage::parse(b"NOT-A-RAF").unwrap_err();

    assert!(error.to_string().contains("FUJIFILM"));
}

#[test]
fn proprietary_tag_parser_uses_length_instead_of_tiff_types() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&2_u32.to_be_bytes());
    bytes.extend_from_slice(&0x0100_u16.to_be_bytes());
    bytes.extend_from_slice(&4_u16.to_be_bytes());
    bytes.extend_from_slice(&5196_u16.to_be_bytes());
    bytes.extend_from_slice(&7872_u16.to_be_bytes());
    bytes.extend_from_slice(&0x0110_u16.to_be_bytes());
    bytes.extend_from_slice(&4_u16.to_be_bytes());
    bytes.extend_from_slice(&21_u16.to_be_bytes());
    bytes.extend_from_slice(&12_u16.to_be_bytes());

    let tags = parse_raf_tags(&bytes).unwrap();

    assert_eq!(tags.size, Some([5196, 7872]));
    assert_eq!(tags.crop_origin, Some([21, 12]));
}

#[test]
fn proprietary_tag_parser_rejects_oversized_entry_counts() {
    let bytes = [0_u8, 0, 0x10, 0];

    let error = parse_raf_tags(&bytes).unwrap_err();

    assert!(error.to_string().contains("entry count"));
}

#[test]
fn proprietary_tag_parser_rejects_truncated_values() {
    let mut bytes = Vec::new();
    bytes.extend_from_slice(&1_u32.to_be_bytes());
    bytes.extend_from_slice(&0x0131_u16.to_be_bytes());
    bytes.extend_from_slice(&36_u16.to_be_bytes());
    bytes.extend_from_slice(&[1, 2, 3]);

    let error = parse_raf_tags(&bytes).unwrap_err();

    assert!(error.to_string().contains("truncated"));
}

#[test]
fn xtrans_layout_is_reversed_into_dng_plane_order() {
    let mut file_order = [0_u8; 36];
    for (index, value) in file_order.iter_mut().enumerate() {
        *value = (index % 3) as u8;
    }

    let pattern = xtrans_pattern(file_order).unwrap();

    assert_eq!(pattern[0], 2);
    assert_eq!(pattern[35], 0);
}

#[test]
fn gfx_bayer_profiles_cover_all_supported_models() {
    for model in [
        "GFX 50S",
        "GFX 50R",
        "GFX50S II",
        "GFX 100",
        "GFX100S",
        "GFX100 II",
        "GFX100S II",
        "GFX100RF",
    ] {
        assert!(profile(model).is_some(), "missing profile for {model}");
    }
}

#[test]
fn unknown_bayer_profile_is_rejected() {
    assert!(profile("GFX unknown").is_none());
}

#[test]
fn unknown_non_xtrans_model_is_typed_unsupported_before_modern_ifd_parsing() {
    let mut source = vec![0_u8; 120];
    source[..8].copy_from_slice(b"FUJIFILM");
    source[28..41].copy_from_slice(b"FinePix S5000");
    source[92..96].copy_from_slice(&104_u32.to_be_bytes());
    source[96..100].copy_from_slice(&4_u32.to_be_bytes());

    let error = super::RafImage::parse(&source).unwrap_err();

    assert!(
        matches!(
            &error,
            DecodeError::Unsupported(message) if message.contains("FinePix S5000")
        ),
        "unexpected error: {error:?}"
    );
}

#[test]
fn gfx_lossless_fixture_matches_rawler_samples() {
    let Some(root) = fixture_root() else {
        return;
    };
    let path = root.join("Fujifilm/GFX 50S/20170525_0037TEST.RAF");
    let Ok(source) = std::fs::read(&path) else {
        return;
    };
    if source.starts_with(b"version https://git-lfs.github.com/spec/v1") {
        return;
    }
    let decoder = super::RafImage::parse(&source)
        .unwrap_or_else(|error| panic!("{} failed to parse: {error}", path.display()));
    let ours = match decoder.raw_mosaic().unwrap() {
        RawMosaic::U16(image) => image,
        other => panic!("RAF decoded as {other:?}"),
    };
    let reference = rawler::decode_file(&path).unwrap();
    let rawler::decoders::RawImageData::Integer(reference_samples) = reference.data else {
        panic!("Rawler returned floating-point RAF samples");
    };
    assert_eq!(
        ours.dimensions(),
        (reference.width as u32, reference.height as u32)
    );
    assert_eq!(ours.as_raw().len(), reference_samples.len());
    let mismatch = ours
        .as_raw()
        .iter()
        .zip(&reference_samples)
        .position(|(ours, reference)| ours != reference);
    if let Some(index) = mismatch {
        panic!(
            "RAF sample {index} differs: {} != {}",
            ours.as_raw()[index],
            reference_samples[index]
        );
    }

    let metadata = decoder.process_metadata().unwrap();
    assert_eq!(metadata.active_area.width, 8280);
    assert_eq!(metadata.active_area.height, 6208);
    assert_eq!(metadata.default_crop.width, 8256);
    assert_eq!(metadata.default_crop.height, 6192);
    let cropped = decoder.cropped_mosaic::<f32>().unwrap();
    assert_eq!(
        cropped.image().dimensions(),
        (metadata.active_area.width, metadata.active_area.height)
    );
}

#[test]
fn xtrans_lossless_fixture_matches_rawler_and_round_trips_through_dng() {
    let Some(root) = fixture_root() else {
        return;
    };
    let path = root.join("Fujifilm/X-H2/Fuji-X-H2-Lossless-Compressed-Raw.RAF");
    let Ok(source) = std::fs::read(&path) else {
        return;
    };
    if source.starts_with(b"version https://git-lfs.github.com/spec/v1") {
        return;
    }
    let decoder = super::RafImage::parse(&source)
        .unwrap_or_else(|error| panic!("{} failed to parse: {error}", path.display()));
    let ours = match decoder.raw_mosaic().unwrap() {
        RawMosaic::U16(image) => image,
        other => panic!("RAF decoded as {other:?}"),
    };
    let reference = rawler::decode_file(&path).unwrap();
    let rawler::decoders::RawImageData::Integer(reference_samples) = reference.data else {
        panic!("Rawler returned floating-point RAF samples");
    };
    assert_eq!(
        ours.dimensions(),
        (reference.width as u32, reference.height as u32)
    );
    assert_eq!(ours.as_raw().as_slice(), reference_samples.as_slice());

    let metadata = decoder.dng_metadata().unwrap();
    let repeat = fornax_tiff::unsigned_vec::<_, u32>(
        &metadata,
        fornax_tiff::constants::TiffTag::CfaRepeatPatternDim,
    )
    .unwrap();
    let pattern =
        fornax_tiff::unsigned_vec::<_, u8>(&metadata, fornax_tiff::constants::TiffTag::CfaPattern)
            .unwrap();
    assert_eq!(repeat, [6, 6]);
    assert_eq!(pattern.len(), 36);
    assert_eq!(
        fornax_tiff::unsigned_scalar(metadata.get(&fornax_tiff::Tag::from(
            fornax_tiff::constants::TiffTag::Orientation
        ))),
        Some(6)
    );
    assert!(matches!(
        decoder.process_metadata(),
        Err(DecodeError::Unsupported(message)) if message.contains("X-Trans")
    ));
    assert!(matches!(
        decoder.cropped_mosaic::<f32>(),
        Err(DecodeError::Unsupported(message)) if message.contains("X-Trans")
    ));

    let mut output = Cursor::new(Vec::new());
    DngWriter::new(DngCompression::Uncompressed)
        .write_dng_with(&mut output, &RawMosaic::U16(ours.clone()), &metadata)
        .unwrap();
    let decoded = DngImage::parse(output.get_ref()).unwrap();
    let RawMosaic::U16(round_trip) = decoded.raw_mosaic().unwrap() else {
        panic!("DNG round trip returned non-u16 samples");
    };
    assert_eq!(round_trip, ours);
    let round_trip_metadata = decoded.dng_metadata().unwrap();
    assert_eq!(
        fornax_tiff::unsigned_vec::<_, u32>(
            &round_trip_metadata,
            fornax_tiff::constants::TiffTag::CfaRepeatPatternDim
        )
        .unwrap(),
        [6, 6]
    );
    assert_eq!(
        fornax_tiff::unsigned_vec::<_, u8>(
            &round_trip_metadata,
            fornax_tiff::constants::TiffTag::CfaPattern
        )
        .unwrap(),
        pattern
    );
    assert_eq!(
        fornax_tiff::unsigned_scalar(round_trip_metadata.get(&fornax_tiff::Tag::from(
            fornax_tiff::constants::TiffTag::Orientation
        ))),
        Some(6)
    );
}

#[test]
fn uncompressed_word_fixtures_match_rawler() {
    let Some(root) = fixture_root() else {
        return;
    };
    for relative in [
        "Fujifilm/GFX100S/Fujifilm-GFX100S-14bits-uncompress-4_3.RAF",
        "Fujifilm/GFX100S/Fujifilm-GFX100S-16bits-uncompress-4_3.RAF",
        "Fujifilm/X-H2/Fuji-X-H2-Uncompressed-Raw.RAF",
    ] {
        let path = root.join(relative);
        let Ok(source) = std::fs::read(&path) else {
            continue;
        };
        if source.starts_with(b"version https://git-lfs.github.com/spec/v1") {
            continue;
        }
        let decoder = super::RafImage::parse(&source)
            .unwrap_or_else(|error| panic!("{} failed to parse: {error}", path.display()));
        let RawMosaic::U16(ours) = decoder.raw_mosaic().unwrap() else {
            panic!("{} decoded as non-u16", path.display());
        };
        let reference = rawler::decode_file(&path).unwrap();
        let rawler::decoders::RawImageData::Integer(reference_samples) = reference.data else {
            panic!("{} Rawler result is not integer", path.display());
        };
        assert_eq!(
            ours.as_raw().as_slice(),
            reference_samples.as_slice(),
            "{}",
            path.display()
        );
    }
}

#[test]
fn lossy_fixture_returns_typed_unsupported() {
    let Some(root) = fixture_root() else {
        return;
    };
    let path = root.join("Fujifilm/X-H2/Fuji-X-H2-Compressed-Raw.RAF");
    let Ok(source) = std::fs::read(&path) else {
        return;
    };
    if source.starts_with(b"version https://git-lfs.github.com/spec/v1") {
        return;
    }
    let decoder = super::RafImage::parse(&source).unwrap();
    let error = decoder.raw_mosaic().unwrap_err();
    assert!(matches!(
        error,
        DecodeError::Unsupported(message) if message.contains("lossy")
    ));
}

fn fixture_root() -> Option<PathBuf> {
    let root = PathBuf::from(std::env::var_os("FORNAX_RAW_SAMPLES")?);
    assert!(root.is_dir(), "FORNAX_RAW_SAMPLES is not a directory");
    Some(root)
}
