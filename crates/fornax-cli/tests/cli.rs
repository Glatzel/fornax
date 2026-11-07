use std::fs;

use assert_cmd::Command;
use clap::Parser;
use fornax::dng::metadata::{DngTag, Tag, TiffTag, Value};
use fornax::dng::{Directory, DngCompression, DngImage, DngWriter};
use fornax::{MonoImage, RawMosaic};
use fornax_cli::{CliArgs, ConversionOutcome, plan_inputs, resolve_config, run};
use image::GenericImageView;
use tempfile::tempdir;

fn metadata(width: u32, height: u32) -> Directory {
    let mut metadata = Directory::default();
    metadata.insert(
        Tag::from(DngTag::DngVersion),
        Value::List(vec![
            Value::Byte(1),
            Value::Byte(4),
            Value::Byte(0),
            Value::Byte(0),
        ]),
    );
    metadata.insert(
        Tag::from(DngTag::ActiveArea),
        Value::List(vec![
            Value::Unsigned(0),
            Value::Unsigned(0),
            Value::Unsigned(height),
            Value::Unsigned(width),
        ]),
    );
    metadata.insert(
        Tag::from(DngTag::DefaultCropOrigin),
        Value::List(vec![Value::Unsigned(0), Value::Unsigned(0)]),
    );
    metadata.insert(
        Tag::from(DngTag::DefaultCropSize),
        Value::List(vec![Value::Unsigned(width), Value::Unsigned(height)]),
    );
    metadata.insert(
        Tag::from(DngTag::BlackLevelRepeatDim),
        Value::List(vec![Value::Short(2), Value::Short(2)]),
    );
    metadata.insert(
        Tag::from(DngTag::BlackLevel),
        Value::List(vec![
            Value::Short(0),
            Value::Short(0),
            Value::Short(0),
            Value::Short(0),
        ]),
    );
    metadata.insert(Tag::from(DngTag::WhiteLevel), Value::Unsigned(65_535));
    metadata.insert(
        Tag::from(TiffTag::CfaRepeatPatternDim),
        Value::List(vec![Value::Short(2), Value::Short(2)]),
    );
    metadata.insert(
        Tag::from(TiffTag::CfaPattern),
        Value::List(vec![
            Value::Byte(0),
            Value::Byte(1),
            Value::Byte(1),
            Value::Byte(2),
        ]),
    );
    metadata.insert(
        Tag::from(DngTag::CfaPlaneColor),
        Value::List(vec![Value::Byte(0), Value::Byte(1), Value::Byte(2)]),
    );
    metadata.insert(Tag::from(DngTag::CfaLayout), Value::Short(1));
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
fn binary_help_shows_default_values() {
    let output = Command::cargo_bin("fornax")
        .unwrap()
        .arg("--help")
        .output()
        .unwrap();
    assert!(output.status.success());

    let help = String::from_utf8(output.stdout).unwrap();
    for expected in [
        "Output directory [default: converted]",
        "Output image format [default: dng]",
        "Maximum simultaneous file conversions [default: 1]",
        "Recurse into directory inputs [default: true]",
        "Overwrite an existing output file [default: false]",
        "Write `<output-stem>.json` beside each output [default: false]",
        "Demosaic algorithm with its default parameters [default: rcd]",
        "Apply the derived camera-to-linear-sRGB matrix [default: true]",
        "JPEG preview quality [default: 90]",
        "DNG compression strategy [default: lossless-jpeg]",
        "Experimental Zstandard compression level [default: 3]",
    ] {
        assert!(
            help.contains(expected),
            "missing {expected:?} in help:\n{help}"
        );
    }
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
