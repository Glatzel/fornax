use decoder_tracker::golden::{
    GoldenCsv, GoldenRow, check_golden_csv, decode_golden, read_golden_csv, write_golden_csv,
};
use fornax_core::metadata::{ColorCalibration, Levels, ProcessMetadata, RawColorMeta};
use fornax_core::{BayerPattern, MonoImage, RawMosaic};
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{Directory, Value};
use image::math::Rect;

fn process_metadata(pattern: BayerPattern) -> ProcessMetadata {
    ProcessMetadata {
        active_area: Rect {
            x: 1,
            y: 2,
            width: 3,
            height: 2,
        },
        default_crop: Rect {
            x: 0,
            y: 1,
            width: 2,
            height: 1,
        },
        levels: Levels {
            black: MonoImage::from_raw(2, 2, vec![0.0, 1.0, 2.0, 3.0]).unwrap(),
            black_delta_h: vec![0.25, 0.5],
            black_delta_v: vec![0.75],
            white: 4095.0,
        },
        bayer_pattern: pattern,
        color_calibration: ColorCalibration::new(RawColorMeta::default()),
    }
}

fn row(pattern: BayerPattern) -> GoldenRow {
    let raw = RawMosaic::U16(MonoImage::from_raw(2, 1, vec![10, 20]).unwrap());
    let cropped = MonoImage::from_raw(1, 2, vec![-0.0_f32, 0.5]).unwrap();
    let mut metadata = Directory::default();
    metadata.insert(
        TiffTag::ImageWidth.into(),
        Value::Unsigned(raw.dimensions().0),
    );
    metadata.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));

    GoldenRow::from_decoded(
        "Canon/Camera/sample.dng",
        &raw,
        &cropped,
        &process_metadata(pattern),
        &metadata,
    )
}

#[test]
fn raw_hash_changes_with_sample_type() {
    let u8_row = GoldenRow::from_decoded(
        "sample.dng",
        &RawMosaic::U8(MonoImage::from_raw(2, 1, vec![10, 20]).unwrap()),
        &MonoImage::from_raw(2, 1, vec![0.0, 1.0]).unwrap(),
        &process_metadata(BayerPattern::RGGB),
        &Directory::default(),
    );
    let u16_row = GoldenRow::from_decoded(
        "sample.dng",
        &RawMosaic::U16(MonoImage::from_raw(2, 1, vec![10, 20]).unwrap()),
        &MonoImage::from_raw(2, 1, vec![0.0, 1.0]).unwrap(),
        &process_metadata(BayerPattern::RGGB),
        &Directory::default(),
    );

    assert_ne!(u8_row.raw_sha256, u16_row.raw_sha256);
    assert_eq!(u8_row.sample_type, "u8");
    assert_eq!(u16_row.sample_type, "u16");
}

#[test]
fn cropped_hash_distinguishes_signed_zero() {
    let positive = GoldenRow::from_decoded(
        "sample.dng",
        &RawMosaic::U16(MonoImage::from_raw(1, 1, vec![1]).unwrap()),
        &MonoImage::from_raw(1, 1, vec![0.0]).unwrap(),
        &process_metadata(BayerPattern::RGGB),
        &Directory::default(),
    );
    let negative = GoldenRow::from_decoded(
        "sample.dng",
        &RawMosaic::U16(MonoImage::from_raw(1, 1, vec![1]).unwrap()),
        &MonoImage::from_raw(1, 1, vec![-0.0]).unwrap(),
        &process_metadata(BayerPattern::RGGB),
        &Directory::default(),
    );

    assert_ne!(positive.cropped_sha256, negative.cropped_sha256);
}

#[test]
fn golden_hashes_are_sensitive_to_metadata_changes() {
    let rggb = row(BayerPattern::RGGB);
    let bggr = row(BayerPattern::BGGR);
    assert_ne!(rggb.process_sha256, bggr.process_sha256);

    let mut changed_directory = Directory::default();
    changed_directory.insert(TiffTag::ImageWidth.into(), Value::Unsigned(3));
    changed_directory.insert(TiffTag::ImageLength.into(), Value::Unsigned(1));
    let changed = GoldenRow::from_decoded(
        "Canon/Camera/sample.dng",
        &RawMosaic::U16(MonoImage::from_raw(2, 1, vec![10, 20]).unwrap()),
        &MonoImage::from_raw(1, 2, vec![-0.0_f32, 0.5]).unwrap(),
        &process_metadata(BayerPattern::RGGB),
        &changed_directory,
    );
    assert_ne!(rggb.dng_sha256, changed.dng_sha256);
}

#[test]
fn golden_csv_is_sorted_and_round_trips() {
    let mut second = row(BayerPattern::RGGB);
    second.path = "Zebra/Camera/z.dng".into();
    let first = row(BayerPattern::BGGR);
    let bytes = write_golden_csv(&[second, first.clone()]).unwrap();
    let decoded = read_golden_csv(&bytes).unwrap();

    assert_eq!(decoded.rows.len(), 2);
    assert_eq!(decoded.rows[0], first);
    assert!(decoded.rows[0].path < decoded.rows[1].path);
}

#[test]
fn golden_check_reports_missing_extra_and_changed_rows() {
    let expected_row = row(BayerPattern::RGGB);
    let mut changed = expected_row.clone();
    changed.path = "Canon/Camera/changed.dng".into();
    changed.raw_sha256 = "11".repeat(32);
    let mut extra = expected_row.clone();
    extra.path = "Canon/Camera/extra.dng".into();
    let mut actual_changed = expected_row.clone();
    actual_changed.raw_sha256 = "22".repeat(32);
    let expected = GoldenCsv {
        rows: vec![expected_row, changed],
    };

    let error = check_golden_csv(&[actual_changed, extra], &expected).unwrap_err();
    assert_eq!(
        error.to_string(),
        "golden state differs: 1 missing, 1 extra, 1 changed; \
         missing=Canon/Camera/changed.dng; extra=Canon/Camera/extra.dng; \
         changed=Canon/Camera/sample.dng"
    );
}

#[test]
fn golden_check_rejects_duplicate_expected_rows() {
    let expected_row = row(BayerPattern::RGGB);
    let actual_row = expected_row.clone();
    let expected = GoldenCsv {
        rows: vec![expected_row.clone(), expected_row],
    };

    let error = check_golden_csv(&[actual_row], &expected).unwrap_err();
    assert!(error.to_string().contains("duplicate path"));
}

#[test]
fn fixture_decodes_to_a_complete_golden_row_when_available() {
    let path = std::path::Path::new(env!("CARGO_MANIFEST_DIR"))
        .join("../../external/raw-images/images/colorchart-5D2-6000K.dng");
    if !path.is_file() {
        return;
    }

    let row = decode_golden("colorchart-5D2-6000K.dng", &path).unwrap();
    assert_eq!((row.width, row.height), (5792, 3804));
    assert_eq!((row.cropped_width, row.cropped_height), (5634, 3753));
    assert_eq!(row.sample_type, "u16");
    assert_eq!(row.raw_sha256.len(), 64);
    assert_eq!(row.cropped_sha256.len(), 64);
    assert_eq!(row.process_sha256.len(), 64);
    assert_eq!(row.dng_sha256.len(), 64);
}
