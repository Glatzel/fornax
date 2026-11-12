use std::io::Cursor;

use fornax_dng::dng_decoder::DngImage;
use fornax_dng::writer::{DngCompression, DngWriter};
use fornax_raw_core::RawMosaic;
use fornax_raw_core::error::DecodeError;
use fornax_raw_phase_one::IiqImage;

#[test]
fn parses_private_offsets_and_maps_process_metadata() {
    let source = synthetic_iiq(
        "Leaf",
        "Credo 40",
        0,
        4,
        4,
        &[0, 1, 2, 3, 4, 5, 6, 7, 8, 9, 10, 11, 12, 13, 14, 15],
    );

    let decoder = IiqImage::parse(&source).unwrap();
    let raw = decoder.raw_mosaic().unwrap();
    let RawMosaic::U16(raw) = raw else {
        panic!("IIQ returned non-U16 samples");
    };
    assert_eq!(raw.dimensions(), (4, 4));
    assert_eq!(
        raw.as_raw(),
        &[0, 4, 8, 12, 16, 20, 24, 28, 32, 36, 40, 44, 48, 52, 56, 60]
    );

    let metadata = decoder.process_metadata().unwrap();
    assert_eq!(
        (
            metadata.active_area.x,
            metadata.active_area.y,
            metadata.active_area.width,
            metadata.active_area.height
        ),
        (1, 1, 2, 2)
    );
    assert_eq!(metadata.levels.black.as_raw(), &[0.0]);
    assert_eq!(metadata.levels.white, 65_535.0);
    assert_eq!(metadata.bayer_pattern, fornax_raw_core::BayerPattern::RGGB);

    let directory = decoder.dng_metadata().unwrap();
    let fornax_tiff::Value::List(neutral) = directory
        .get(&fornax_tiff::Tag::from(
            fornax_dng::metadata::DngTag::AsShotNeutral,
        ))
        .unwrap()
    else {
        panic!("AsShotNeutral is not a list");
    };
    let expected = [1.0 / 2.0, 1.0, 1.0 / 1.5];
    for (actual, expected) in neutral.iter().zip(expected) {
        let fornax_tiff::Value::Rational(numerator, denominator) = actual else {
            panic!("AsShotNeutral is not rational");
        };
        let actual = f64::from(*numerator) / f64::from(*denominator);
        assert!((actual - expected).abs() < 1.0e-6, "{actual} != {expected}");
    }
}

#[test]
fn legacy_and_unknown_modes_are_typed_unsupported() {
    for mode in [1, 2, 99] {
        let source = synthetic_iiq("Leaf", "Credo 40", mode, 2, 2, &[0; 4]);
        let error = IiqImage::parse(&source).unwrap_err();
        assert!(
            matches!(error, DecodeError::Unsupported(_)),
            "mode {mode}: {error:?}"
        );
    }
}

#[test]
fn unknown_camera_is_typed_unsupported() {
    let source = synthetic_iiq("Leaf", "Unknown Back", 0, 2, 2, &[0; 4]);

    let error = IiqImage::parse(&source).unwrap_err();

    assert!(
        matches!(error, DecodeError::Unsupported(ref message) if message.contains("Unknown Back")),
        "{error:?}"
    );
}

#[test]
fn truncated_private_ifd_is_invalid_input() {
    let mut source = synthetic_iiq("Leaf", "Credo 40", 0, 2, 2, &[0; 4]);
    let private_offset = 8 + u32::from_le_bytes(source[16..20].try_into().unwrap()) as usize;
    source[private_offset..private_offset + 4].copy_from_slice(&100_u32.to_le_bytes());

    let error = IiqImage::parse(&source).unwrap_err();

    assert!(matches!(error, DecodeError::InvalidInput(_)), "{error:?}");
}

#[test]
fn non_iiq_private_header_is_rejected() {
    let mut source = synthetic_iiq("Leaf", "Credo 40", 0, 2, 2, &[0; 4]);
    source[12..16].copy_from_slice(b"nope");

    let error = IiqImage::parse(&source).unwrap_err();

    assert!(error.to_string().contains("IIQ"), "{error}");
}

#[test]
fn applies_base_black_level_to_corrected_samples() {
    let mut source = synthetic_iiq("Leaf", "Credo 40", 0, 2, 2, &[4, 8, 12, 16]);
    set_maker_value(&mut source, 0x021d, 1);

    let decoder = IiqImage::parse(&source).unwrap();
    let RawMosaic::U16(raw) = decoder.raw_mosaic().unwrap() else {
        panic!("IIQ returned non-U16 samples");
    };

    assert_eq!(raw.as_raw(), &[15, 31, 47, 63]);
}

#[test]
fn selected_fixtures_match_rawler_after_correction() {
    let Some(root) = fixture_root() else {
        return;
    };
    for relative in [
        "Phase One/P45+/CF125839.IIQ",
        "Phase One/P20+/CF051545.IIQ",
        "Phase One/H 25/H25_Outdoor_.IIQ",
        "Leaf/Credo 40/0001_20_32.IIQ",
        "Leaf/Credo 40/0002_20_32.IIQ",
        "Phase One/IQ3 100MP/Phase_One_IQ3_100MP_IIQ_L_16bit.IIQ",
    ] {
        let path = root.join(relative);
        let Ok(source) = std::fs::read(&path) else {
            continue;
        };
        if source.starts_with(b"version https://git-lfs.github.com/spec/v1") {
            continue;
        }
        let decoder = IiqImage::parse(&source)
            .unwrap_or_else(|error| panic!("{} failed to parse: {error}", path.display()));
        let raw = decoder
            .raw_mosaic()
            .unwrap_or_else(|error| panic!("{} raw mosaic failed: {error}", path.display()));
        let RawMosaic::U16(ours) = raw else {
            panic!("{} decoded as non-U16", path.display());
        };
        let reference = rawler::decode_file(&path)
            .unwrap_or_else(|error| panic!("Rawler failed on {}: {error}", path.display()));
        let dimensions = (reference.width as u32, reference.height as u32);
        let rawler::decoders::RawImageData::Integer(reference) = reference.data else {
            panic!("{} Rawler result is not integer", path.display());
        };
        assert_eq!(ours.dimensions(), dimensions, "{}", path.display());
        let mismatch = ours
            .as_raw()
            .iter()
            .zip(&reference)
            .position(|(ours, reference)| ours != reference);
        if let Some(index) = mismatch {
            panic!(
                "{} differs at sample {index}: {} != {}",
                path.display(),
                ours.as_raw()[index],
                reference[index]
            );
        }
    }
}

#[test]
fn p65_short_quadrant_linearization_is_typed_unsupported() {
    let Some(root) = fixture_root() else {
        return;
    };
    let path = root.join("Phase One/P65+/CF027312.IIQ");
    let Ok(source) = std::fs::read(&path) else {
        return;
    };
    if source.starts_with(b"version https://git-lfs.github.com/spec/v1") {
        return;
    }
    let decoder = IiqImage::parse(&source).unwrap();

    let error = decoder.raw_mosaic().unwrap_err();

    assert!(
        matches!(error, DecodeError::Unsupported(ref message) if message.contains("quadrant linearization")),
        "{error:?}"
    );
}

#[test]
fn corrected_samples_and_cfa_round_trip_through_dng() {
    let source = synthetic_iiq("Leaf", "Credo 40", 0, 4, 4, &[4, 8, 12, 16]);
    let decoder = IiqImage::parse(&source).unwrap();
    let raw = decoder.raw_mosaic().unwrap();
    let metadata = decoder.dng_metadata().unwrap();
    let mut output = Cursor::new(Vec::new());

    DngWriter::new(DngCompression::Uncompressed)
        .write_dng_with(&mut output, &raw, &metadata)
        .unwrap();

    let decoded = DngImage::parse(output.get_ref()).unwrap();
    assert_eq!(decoded.raw_mosaic().unwrap(), raw);
    let round_trip = decoded.dng_metadata().unwrap();
    assert_eq!(
        fornax_tiff::unsigned_vec::<_, u8>(
            &round_trip,
            fornax_tiff::constants::TiffTag::CfaPattern
        )
        .unwrap(),
        [0, 1, 1, 2]
    );
}

fn synthetic_iiq(
    make: &str,
    model: &str,
    mode: u32,
    width: u32,
    height: u32,
    samples: &[u16],
) -> Vec<u8> {
    let root_ifd = 512_usize;
    let private_ifd = 64_usize;
    let private_base = 8_usize;
    let ifd_offset = private_base + private_ifd;
    let margin = u32::from(width > 2 && height > 2);
    let tags = [
        (0x0108_u16, width, 0_u32),
        (0x0109_u16, height, 0),
        (0x010a, margin, 0),
        (0x010b, margin, 0),
        (0x010c, width - 2 * margin, 0),
        (0x010d, height - 2 * margin, 0),
        (0x010e, mode, 0),
        (0x021d, 0, 0),
    ];
    let tag_count = tags.len() + 3;
    let mut source = vec![0_u8; 2_048];
    source[..4].copy_from_slice(b"II*\0");
    source[4..8].copy_from_slice(&(root_ifd as u32).to_le_bytes());
    source[8..12].copy_from_slice(b"IIII");
    source[12..16].copy_from_slice(b"CwaR");
    source[16..20].copy_from_slice(&(private_ifd as u32).to_le_bytes());
    source[ifd_offset..ifd_offset + 4].copy_from_slice(&(tag_count as u32).to_le_bytes());

    let data_offset = 768_usize;
    let data_bytes = width as usize * height as usize * 2;
    let wb_offset = data_offset + data_bytes;
    let serial_offset = wb_offset + 12;
    source[serial_offset..serial_offset + 8].copy_from_slice(b"LD001028");
    for (index, sample) in samples
        .iter()
        .copied()
        .take((width * height) as usize)
        .enumerate()
    {
        source[data_offset + index * 2..data_offset + index * 2 + 2]
            .copy_from_slice(&sample.to_le_bytes());
    }
    source[wb_offset..wb_offset + 4].copy_from_slice(&2.0_f32.to_le_bytes());
    source[wb_offset + 4..wb_offset + 8].copy_from_slice(&1.0_f32.to_le_bytes());
    source[wb_offset + 8..wb_offset + 12].copy_from_slice(&1.5_f32.to_le_bytes());

    let mut position = ifd_offset + 8;
    for (tag, value, _) in tags {
        put_maker_entry(&mut source, position, u32::from(tag), 4, 4, value);
        position += 16;
    }
    put_maker_entry(
        &mut source,
        position,
        0x010f,
        4,
        data_bytes as u32,
        (data_offset - private_base) as u32,
    );
    position += 16;
    put_maker_entry(
        &mut source,
        position,
        0x0107,
        4,
        12,
        (wb_offset - private_base) as u32,
    );
    position += 16;
    put_maker_entry(
        &mut source,
        position,
        0x0102,
        1,
        8,
        (serial_offset - private_base) as u32,
    );

    put_tiff_entry(
        &mut source,
        root_ifd + 2,
        0x010f,
        2,
        make.len() as u32 + 1,
        (root_ifd + 400) as u32,
    );
    put_tiff_entry(
        &mut source,
        root_ifd + 14,
        0x0110,
        2,
        model.len() as u32 + 1,
        (root_ifd + 420) as u32,
    );
    source[root_ifd..root_ifd + 2].copy_from_slice(&2_u16.to_le_bytes());
    source[root_ifd + 400..root_ifd + 400 + make.len()].copy_from_slice(make.as_bytes());
    source[root_ifd + 420..root_ifd + 420 + model.len()].copy_from_slice(model.as_bytes());
    source
}

fn set_maker_value(source: &mut [u8], tag: u32, value: u32) {
    let ifd = 8 + u32::from_le_bytes(source[16..20].try_into().unwrap()) as usize;
    let count = u32::from_le_bytes(source[ifd..ifd + 4].try_into().unwrap()) as usize;
    for index in 0..count {
        let position = ifd + 8 + index * 16;
        if u32::from_le_bytes(source[position..position + 4].try_into().unwrap()) == tag {
            source[position + 12..position + 16].copy_from_slice(&value.to_le_bytes());
            return;
        }
    }
    panic!("maker tag 0x{tag:04x} not found");
}

fn put_maker_entry(
    source: &mut [u8],
    position: usize,
    tag: u32,
    kind: u32,
    count: u32,
    value: u32,
) {
    source[position..position + 4].copy_from_slice(&tag.to_le_bytes());
    source[position + 4..position + 8].copy_from_slice(&kind.to_le_bytes());
    source[position + 8..position + 12].copy_from_slice(&count.to_le_bytes());
    source[position + 12..position + 16].copy_from_slice(&value.to_le_bytes());
}

fn put_tiff_entry(source: &mut [u8], position: usize, tag: u16, kind: u16, count: u32, value: u32) {
    source[position..position + 2].copy_from_slice(&tag.to_le_bytes());
    source[position + 2..position + 4].copy_from_slice(&kind.to_le_bytes());
    source[position + 4..position + 8].copy_from_slice(&count.to_le_bytes());
    source[position + 8..position + 12].copy_from_slice(&value.to_le_bytes());
}

fn fixture_root() -> Option<std::path::PathBuf> {
    let root = std::path::PathBuf::from(std::env::var_os("FORNAX_RAW_SAMPLES")?);
    assert!(root.is_dir(), "FORNAX_RAW_SAMPLES is not a directory");
    Some(root)
}
