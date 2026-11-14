use decoder_tracker::golden::{
    GoldenCsv, GoldenRow, build_golden_coverage, check_golden_csv, decode_golden, read_golden_csv,
    write_golden_coverage_csv, write_golden_csv, write_golden_selection_csv,
};
use decoder_tracker::manifest::derive_identity;
use decoder_tracker::model::{BaselineRow, ManifestRow, status_for_stages};
use fornax_raw_core::metadata::{
    ColorCalibration, Levels, MetadataSource, ProcessMetadata, RawColorMeta,
};
use fornax_raw_core::{BayerPattern, MonoImage, RawMosaic};
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{Directory, Value};
use image::math::Rect;

fn manifest_row(path: &str, size: u64, sha_byte: char) -> ManifestRow {
    let (vendor, camera, model, filename) = derive_identity(path).unwrap();
    ManifestRow {
        sha256: sha_byte.to_string().repeat(64),
        size,
        path: path.into(),
        vendor,
        camera,
        model,
        format: path.rsplit_once('.').unwrap().1.to_ascii_uppercase(),
        filename,
    }
}

fn baseline_row(
    manifest: &ManifestRow,
    compression_name: &str,
    bits: u16,
    passing: bool,
) -> BaselineRow {
    BaselineRow {
        status: status_for_stages([u8::from(passing); 4]).into(),
        camera: manifest.camera.clone(),
        format: manifest.format.clone(),
        path: manifest.path.clone(),
        decode_pass: u8::from(passing),
        raw_mosaic_pass: u8::from(passing),
        raw_sha256: if passing {
            "11".repeat(32)
        } else {
            String::new()
        },
        process_metadata_pass: u8::from(passing),
        handoff_pass: u8::from(passing),
        error_kind: if passing {
            String::new()
        } else {
            "raw_mosaic:invalid_input".into()
        },
        width: 64,
        height: 32,
        cfa: "RGGB".into(),
        compression_id: Some(7),
        compression_name: compression_name.into(),
        compression_lossless: Some(1),
        storage: "tiles".into(),
        bits_per_sample: Some(bits),
        sample_format: "integer".into(),
        predictor: Some(1),
        rows_per_strip: None,
        tile_width: Some(256),
        tile_height: Some(256),
    }
}

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
        wb_source: MetadataSource::Default,
        black_source: MetadataSource::Default,
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
fn golden_coverage_groups_encoding_paths_and_chooses_smallest_representative() {
    let small = manifest_row("Canon/Cam/small.DNG", 10, 'a');
    let large = manifest_row("Canon/Cam/large.DNG", 20, 'b');
    let missing = manifest_row("Sony/Cam/missing.ARW", 30, 'c');
    let baseline = vec![
        baseline_row(&large, "lossless-jpeg", 14, true),
        baseline_row(&small, "lossless-jpeg", 14, true),
        baseline_row(&missing, "sony-arw6", 14, true),
    ];
    let manifest = vec![large, small, missing.clone()];
    let golden = vec![
        manifest[1].clone(),
        manifest[0].clone(),
        manifest_row("Sony/Cam/other.ARW", 40, 'd'),
    ];
    let other = manifest_row("Sony/Cam/other.ARW", 40, 'd');
    let mut baseline = baseline;
    baseline.push(baseline_row(&other, "sony-arw6", 14, true));
    let mut manifest = manifest;
    manifest.push(other);

    let report = build_golden_coverage(&baseline, &manifest, &golden).unwrap();

    assert_eq!(report.rows.len(), 2);
    assert_eq!(report.summary.missing, 0);
    assert_eq!(report.summary.covered, 1);
    assert_eq!(report.summary.redundant, 1);
    let dng = report.rows.iter().find(|row| row.format == "DNG").unwrap();
    assert_eq!(dng.golden_count, 2);
    assert_eq!(dng.coverage, "redundant");
    assert_eq!(dng.representative_path, "Canon/Cam/small.DNG");
    assert_eq!(dng.representative_size, 10);
}

#[test]
fn golden_coverage_reports_missing_groups() {
    let raw = manifest_row("Pentax/Cam/file.DNG", 10, 'a');
    let report = build_golden_coverage(
        &[baseline_row(&raw, "deflate", 16, true)],
        std::slice::from_ref(&raw),
        &[],
    )
    .unwrap();

    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0].coverage, "missing");
    assert_eq!(report.summary.missing, 1);
}

#[test]
fn golden_coverage_excludes_non_lossless_encodings() {
    let lossy = manifest_row("Sony/Cam/lossy.ARW", 10, 'a');
    let mut row = baseline_row(&lossy, "sony-arw2", 12, true);
    row.compression_lossless = Some(0);

    let report = build_golden_coverage(&[row], std::slice::from_ref(&lossy), &[]).unwrap();

    assert!(report.rows.is_empty());
    assert_eq!(report.summary.missing, 0);
}

#[test]
fn golden_coverage_rejects_absent_or_failed_golden_rows() {
    let absent = manifest_row("Canon/Cam/absent.DNG", 10, 'a');
    let failed = manifest_row("Canon/Cam/failed.DNG", 20, 'b');
    let manifest = vec![absent.clone(), failed.clone()];
    let baseline = vec![baseline_row(&failed, "lossless-jpeg", 14, false)];

    assert!(
        build_golden_coverage(&baseline, &manifest, std::slice::from_ref(&absent))
            .unwrap_err()
            .to_string()
            .contains("absent")
    );
    let error = build_golden_coverage(&baseline, &manifest, std::slice::from_ref(&failed))
        .unwrap_err()
        .to_string();
    assert!(error.contains("failed") || error.contains("not passing"));
}

#[test]
fn golden_coverage_rejects_golden_corpus_identity_drift() {
    let raw = manifest_row("Canon/Cam/file.DNG", 10, 'a');
    let mut drifted = raw.clone();
    drifted.sha256 = "ff".repeat(32);

    let error = build_golden_coverage(
        &[baseline_row(&raw, "lossless-jpeg", 14, true)],
        std::slice::from_ref(&raw),
        std::slice::from_ref(&drifted),
    )
    .unwrap_err()
    .to_string();

    assert!(error.contains("does not match the corpus manifest"));
}

#[test]
fn golden_coverage_csv_is_sorted_and_includes_debug_fields() {
    let raw = manifest_row("Canon/Cam/file.DNG", 10, 'a');
    let report = build_golden_coverage(
        &[baseline_row(&raw, "lossless-jpeg", 14, true)],
        std::slice::from_ref(&raw),
        std::slice::from_ref(&raw),
    )
    .unwrap();
    let bytes = write_golden_coverage_csv(&report.rows).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();

    assert!(text.starts_with(
        "format,compression_name,compression_lossless,compression_id,bits_per_sample,sample_format,storage,predictor,cfa,coverage,passing_count,golden_count,golden_paths,representative_path,representative_camera,representative_size,representative_sha256,representative_width,representative_height\n"
    ));
    assert!(text.contains(
        "DNG,lossless-jpeg,1,7,14,integer,tiles,1,RGGB,covered,1,1,Canon/Cam/file.DNG,Canon/Cam/file.DNG,Canon Cam,10,aaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaaa,64,32"
    ));
}

#[test]
fn golden_coverage_groups_by_encoding_and_uses_lowest_pixel_count() {
    let large_pixels = manifest_row("Canon/Cam/large-pixels.DNG", 10, 'a');
    let small_pixels = manifest_row("Canon/Cam/small-pixels.DNG", 20, 'b');
    let other_cfa = manifest_row("Canon/Cam/other-cfa.DNG", 5, 'c');
    let mut large = baseline_row(&large_pixels, "lossless-jpeg", 14, true);
    large.width = 4000;
    large.height = 3000;
    let mut small = baseline_row(&small_pixels, "lossless-jpeg", 14, true);
    small.width = 1200;
    small.height = 800;
    let mut other = baseline_row(&other_cfa, "lossless-jpeg", 14, true);
    other.width = 1600;
    other.height = 1000;
    other.cfa = "BGGR".into();
    let baseline = vec![large, small, other];
    let manifest = vec![large_pixels.clone(), small_pixels, other_cfa];

    let report =
        build_golden_coverage(&baseline, &manifest, std::slice::from_ref(&large_pixels)).unwrap();

    assert_eq!(report.rows.len(), 1);
    assert_eq!(report.rows[0].passing_count, 3);
    assert_eq!(
        report.rows[0].representative_path,
        "Canon/Cam/small-pixels.DNG"
    );
    assert_eq!(report.rows[0].representative_width, 1200);
    assert_eq!(report.rows[0].representative_height, 800);
}

#[test]
fn golden_selection_manifest_writes_one_lowest_pixel_representative_per_encoding() {
    let first = manifest_row("Canon/Cam/first.DNG", 10, 'a');
    let second = manifest_row("Canon/Cam/second.DNG", 20, 'b');
    let mut first_row = baseline_row(&first, "lossless-jpeg", 14, true);
    first_row.width = 2000;
    first_row.height = 1000;
    let mut second_row = baseline_row(&second, "lossless-jpeg", 14, true);
    second_row.width = 1000;
    second_row.height = 500;
    second_row.cfa = "BGGR".into();
    let report = build_golden_coverage(
        &[first_row, second_row],
        &[first.clone(), second.clone()],
        &[],
    )
    .unwrap();

    let bytes = write_golden_selection_csv(&report.rows).unwrap();

    assert_eq!(
        std::str::from_utf8(&bytes).unwrap(),
        format!(
            "path,size,sha256\nCanon/Cam/second.DNG,20,{}\n",
            "b".repeat(64)
        )
    );
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
