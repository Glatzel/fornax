use std::fs;

use assert_cmd::Command;
use clap::Parser;
use fornax_cli::{CliArgs, ConversionOutcome, plan_inputs, resolve_config, run};
use fornax_core::{MonoImage, RawMosaic};
use fornax_dng::metadata::{DngMetadata, DngTag, DngValue};
use fornax_dng::{DngCompression, DngWriter};
use image::GenericImageView;
use tempfile::tempdir;

fn metadata(width: u32, height: u32) -> DngMetadata {
    let mut metadata = DngMetadata::default();
    metadata.insert(
        DngTag::DngVersion,
        DngValue::List(vec![
            DngValue::Byte(1),
            DngValue::Byte(4),
            DngValue::Byte(0),
            DngValue::Byte(0),
        ]),
    );
    metadata.insert(
        DngTag::ActiveArea,
        DngValue::List(vec![
            DngValue::Unsigned(0),
            DngValue::Unsigned(0),
            DngValue::Unsigned(height),
            DngValue::Unsigned(width),
        ]),
    );
    metadata.insert(
        DngTag::DefaultCropOrigin,
        DngValue::List(vec![DngValue::Unsigned(0), DngValue::Unsigned(0)]),
    );
    metadata.insert(
        DngTag::DefaultCropSize,
        DngValue::List(vec![DngValue::Unsigned(width), DngValue::Unsigned(height)]),
    );
    metadata.insert(
        DngTag::BlackLevelRepeatDim,
        DngValue::List(vec![DngValue::Short(2), DngValue::Short(2)]),
    );
    metadata.insert(
        DngTag::BlackLevel,
        DngValue::List(vec![
            DngValue::Short(0),
            DngValue::Short(0),
            DngValue::Short(0),
            DngValue::Short(0),
        ]),
    );
    metadata.insert(DngTag::WhiteLevel, DngValue::Unsigned(65_535));
    metadata.insert(
        DngTag::CfaRepeatPatternDim,
        DngValue::List(vec![DngValue::Short(2), DngValue::Short(2)]),
    );
    metadata.insert(
        DngTag::CfaPattern,
        DngValue::List(vec![
            DngValue::Byte(0),
            DngValue::Byte(1),
            DngValue::Byte(1),
            DngValue::Byte(2),
        ]),
    );
    metadata.insert(
        DngTag::CfaPlaneColor,
        DngValue::List(vec![
            DngValue::Byte(0),
            DngValue::Byte(1),
            DngValue::Byte(2),
        ]),
    );
    metadata.insert(DngTag::CfaLayout, DngValue::Short(1));
    metadata
}

fn synthetic_dng(path: &std::path::Path) {
    let samples = (0_u16..64).collect();
    let mosaic = RawMosaic::U16(MonoImage::from_raw(8, 8, samples).unwrap());
    DngWriter::new(DngCompression::Uncompressed)
        .write(path, &mosaic, &metadata(8, 8))
        .unwrap();
}

fn args(values: &[&std::ffi::OsStr]) -> CliArgs {
    let mut all = vec![std::ffi::OsStr::new("fornax")];
    all.extend_from_slice(values);
    CliArgs::try_parse_from(all).unwrap()
}

#[test]
fn binary_exposes_help() {
    Command::cargo_bin("fornax")
        .unwrap()
        .arg("--help")
        .assert()
        .success();
}

#[test]
fn recursive_inputs_mirror_files_and_reject_collisions() {
    let directory = tempdir().unwrap();
    let root = directory.path().join("raws");
    fs::create_dir_all(root.join("nested")).unwrap();
    fs::write(root.join("a.dng"), []).unwrap();
    fs::write(root.join("nested/b.arw"), []).unwrap();
    let output = directory.path().join("out");
    let cli = args(&["--output".as_ref(), output.as_os_str(), root.as_os_str()]);
    let config = resolve_config(&cli).unwrap();
    let jobs = plan_inputs(&config).unwrap();

    assert_eq!(jobs.len(), 2);
    assert_eq!(jobs[0].output, output.join("raws/a.dng"));
    assert_eq!(jobs[1].output, output.join("raws/nested/b.dng"));

    let first_dir = directory.path().join("first");
    let second_dir = directory.path().join("second");
    fs::create_dir_all(&first_dir).unwrap();
    fs::create_dir_all(&second_dir).unwrap();
    let first = first_dir.join("image.dng");
    let second = second_dir.join("image.dng");
    fs::write(&first, []).unwrap();
    fs::write(&second, []).unwrap();
    let colliding = args(&[
        "--output".as_ref(),
        output.as_os_str(),
        first.as_os_str(),
        second.as_os_str(),
    ]);
    let error = plan_inputs(&resolve_config(&colliding).unwrap()).unwrap_err();
    assert!(error.to_string().contains("same output"));
}

#[test]
fn dng_to_dng_skips_and_metadata_json_uses_the_input_stem() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("image.dng");
    synthetic_dng(&input);
    let output = directory.path().join("out");
    let cli = args(&["--output".as_ref(), output.as_os_str(), input.as_os_str()]);

    let outcomes = run(&cli).unwrap();
    assert!(matches!(
        outcomes.as_slice(),
        [ConversionOutcome::SkippedDngToDng { .. }]
    ));
    assert!(!output.join("image.dng").exists());

    let cli = args(&[
        "--output".as_ref(),
        output.as_os_str(),
        "--metadata-json".as_ref(),
        input.as_os_str(),
    ]);
    let outcomes = run(&cli).unwrap();
    assert!(matches!(
        outcomes.as_slice(),
        [ConversionOutcome::MetadataOnly { .. }]
    ));
    let json = fs::read_to_string(output.join("image.json")).unwrap();
    assert!(json.contains("\"ImageWidth\""));
    assert!(json.contains("\"DngVersion\""));
}

#[test]
fn renders_16bit_tiff_and_preview_jpeg() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("image.dng");
    synthetic_dng(&input);
    let output = directory.path().join("out");

    for format in ["tiff", "jpeg"] {
        let cli = args(&[
            "--output".as_ref(),
            output.as_os_str(),
            "--format".as_ref(),
            format.as_ref(),
            input.as_os_str(),
        ]);
        let outcomes = run(&cli).unwrap();
        assert!(matches!(
            outcomes.as_slice(),
            [ConversionOutcome::Converted { .. }]
        ));
    }

    let tiff = image::open(output.join("image.tiff")).unwrap();
    assert_eq!(tiff.dimensions(), (8, 8));
    let jpeg_bytes = fs::read(output.join("image.jpg")).unwrap();
    assert!(jpeg_bytes.starts_with(&[0xff, 0xd8]));
}

#[test]
fn non_dng_input_converts_to_raw_dng() {
    let directory = tempdir().unwrap();
    let input = directory.path().join("image.cr3");
    synthetic_dng(&input);
    let output = directory.path().join("out");
    let cli = args(&["--output".as_ref(), output.as_os_str(), input.as_os_str()]);

    let outcomes = run(&cli).unwrap();

    assert!(matches!(
        outcomes.as_slice(),
        [ConversionOutcome::Converted { .. }]
    ));
    let generated = output.join("image.dng");
    assert!(generated.is_file());
    assert!(fornax_dng::DngImage::parse(&fs::read(generated).unwrap()).is_ok());
}
