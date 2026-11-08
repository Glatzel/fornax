use std::fs;

use fornax_core::metadata::{ColorCalibration, Levels, ProcessMetadata, RawColorMeta};
use fornax_core::{BayerPattern, MonoImage};
use fornax_dng::Directory;
use fornax_tiff::Value;
use image::math::Rect;
use rstest::rstest;
use tempfile::tempdir;

use crate::csvio::{BaselineCsv, read_baseline_csv, write_baseline_csv, write_manifest_csv};
use crate::duckdb::{DiffSummary, diff, merge, upsert};
use crate::manifest::{
    ManifestCheck, generate_manifest, normalize_repo_path, validate_pointer, write_pointer,
};
use crate::model::{BaselineKey, BaselineRow, ManifestRow};
use crate::selection::{Selection, Shard, runner_count_for, select_manifest};
use crate::stages::{
    StageRow, classify_decode_error, run_guarded, sanity_check_metadata, transition_row,
};

fn manifest_row(sha: &str, path: &str, format: &str) -> ManifestRow {
    let (vendor, camera, model, filename) =
        crate::manifest::derive_identity(path).expect("test path has identity");
    ManifestRow {
        sha256: sha.into(),
        size: 123,
        path: path.into(),
        vendor,
        camera,
        model,
        format: format.into(),
        filename,
    }
}

fn baseline_row(key: &BaselineKey, handoff: u8) -> BaselineRow {
    let mut row = BaselineRow {
        status: String::new(),
        camera: key.camera.clone(),
        format: key.format.clone(),
        path: key.path.clone(),
        decode_pass: 1,
        raw_mosaic_pass: 1,
        process_metadata_pass: 1,
        handoff_pass: handoff,
        error_kind: if handoff == 1 {
            String::new()
        } else {
            "handoff:unsupported".into()
        },
        width: 16,
        height: 8,
        cfa: "RGGB".into(),
    };
    row.status = crate::model::status_for_stages([
        row.decode_pass,
        row.raw_mosaic_pass,
        row.process_metadata_pass,
        row.handoff_pass,
    ])
    .to_owned();
    row
}

fn process_metadata(pattern: BayerPattern) -> ProcessMetadata {
    ProcessMetadata {
        active_area: Rect {
            x: 0,
            y: 0,
            width: 16,
            height: 8,
        },
        default_crop: Rect {
            x: 0,
            y: 0,
            width: 16,
            height: 8,
        },
        levels: Levels {
            black: MonoImage::from_raw(1, 1, vec![0.0]).expect("black image"),
            black_delta_h: Vec::new(),
            black_delta_v: Vec::new(),
            white: 4095.0,
        },
        bayer_pattern: pattern,
        color_calibration: ColorCalibration::new(RawColorMeta::default()),
    }
}

fn metadata_with_geometry(width: u32, height: u32) -> Directory {
    let mut metadata = Directory::default();
    metadata.insert(
        fornax_tiff::constants::TiffTag::ImageWidth.into(),
        Value::Unsigned(width),
    );
    metadata.insert(
        fornax_tiff::constants::TiffTag::ImageLength.into(),
        Value::Unsigned(height),
    );
    metadata
}

#[test]
fn normalizes_separators_without_changing_utf8_or_case() {
    assert_eq!(
        normalize_repo_path(r"Canon\EOS R5\Été_01.DNG"),
        "Canon/EOS R5/Été_01.DNG"
    );
}

#[test]
fn derives_vendor_camera_model_and_filename_from_path() {
    assert_eq!(
        crate::manifest::derive_identity("Sony/ILCE-7M5/full_compressed.ARW").unwrap(),
        (
            "Sony".into(),
            "Sony ILCE-7M5".into(),
            "ILCE-7M5".into(),
            "full_compressed".into()
        )
    );
    assert_eq!(
        crate::manifest::derive_identity("Canon/Canon EOS R50 V/IMG_0034.CR3").unwrap(),
        (
            "Canon".into(),
            "Canon EOS R50 V".into(),
            "Canon EOS R50 V".into(),
            "IMG_0034".into()
        )
    );
}

#[test]
fn filters_formats_first_then_selects_by_sha_before_sharding() {
    let rows = vec![
        manifest_row("ff", "Zed/Cam/c.ARW", "ARW"),
        manifest_row("cc", "Sony/Cam/c.DNG", "DNG"),
        manifest_row("aa", "Canon/Cam/a.DNG", "DNG"),
        manifest_row("bb", "Canon/Cam/b.dng", "DNG"),
    ];
    let selection = Selection {
        formats: vec!["dng".into()],
        cameras: Vec::new(),
        filenames: Vec::new(),
        globs: Vec::new(),
        shard: Some(Shard { index: 1, count: 2 }),
    };
    let selected = select_manifest(&rows, &selection).unwrap();
    assert_eq!(
        selected
            .iter()
            .map(|row| row.sha256.as_str())
            .collect::<Vec<_>>(),
        ["bb"]
    );
}

#[test]
fn filename_filter_is_exact_and_case_insensitive() {
    let rows = vec![
        manifest_row("aa", "Canon/Cam/IMAGE.DNG", "DNG"),
        manifest_row("bb", "Canon/Cam/IMAGE_2.DNG", "DNG"),
    ];
    let selection = Selection {
        formats: vec!["DNG".into()],
        cameras: Vec::new(),
        filenames: vec!["image".into()],
        globs: Vec::new(),
        shard: None,
    };
    assert_eq!(select_manifest(&rows, &selection).unwrap()[0].sha256, "aa");
}

#[test]
fn glob_filters_use_standard_segment_aware_matching() {
    let rows = vec![
        manifest_row("aa", "Canon/Cam/a.DNG", "DNG"),
        manifest_row("bb", "Canon/Nested/Cam/b.DNG", "DNG"),
        manifest_row("cc", "Canon/Cam/c.ARW", "ARW"),
    ];
    let selection = Selection {
        formats: Vec::new(),
        cameras: Vec::new(),
        filenames: Vec::new(),
        globs: vec!["Canon/*/a.DNG".into(), "Canon/**/b.DNG".into()],
        shard: None,
    };
    assert_eq!(
        select_manifest(&rows, &selection)
            .unwrap()
            .iter()
            .map(|row| row.sha256.as_str())
            .collect::<Vec<_>>(),
        ["aa", "bb"]
    );
}

#[test]
fn runner_count_targets_ten_files_and_caps_at_twenty() {
    assert_eq!(
        [5, 10, 11, 16, 210]
            .into_iter()
            .map(runner_count_for)
            .collect::<Vec<_>>(),
        [1, 1, 2, 2, 20]
    );
}

#[test]
fn balanced_modulo_shards_keep_a_210_file_selection_within_ten_or_eleven() {
    let rows = (0..210)
        .map(|index| {
            manifest_row(
                &format!("{index:064x}"),
                &format!("Canon/Cam/{index:03}.DNG"),
                "DNG",
            )
        })
        .collect::<Vec<_>>();
    let counts = (0..20)
        .map(|index| {
            select_manifest(
                &rows,
                &Selection {
                    formats: Vec::new(),
                    cameras: Vec::new(),
                    filenames: Vec::new(),
                    globs: Vec::new(),
                    shard: Some(Shard { index, count: 20 }),
                },
            )
            .unwrap()
            .len()
        })
        .collect::<Vec<_>>();
    assert_eq!(counts.iter().filter(|count| **count == 11).count(), 10);
    assert_eq!(counts.iter().filter(|count| **count == 10).count(), 10);
}

#[rstest]
#[case("1/0")]
#[case("2/2")]
#[case("-1/2")]
#[case("1/x")]
fn shard_rejects_out_of_bounds_or_malformed_values(#[case] value: &str) {
    assert!(value.parse::<Shard>().is_err());
}

#[test]
fn stage_transitions_stop_after_failure() {
    let row = transition_row(Ok(()), Err("raw_mosaic:bad_dimensions"), Ok(()), Ok(()));
    assert_eq!(
        (
            row.decode_pass,
            row.raw_mosaic_pass,
            row.process_metadata_pass,
            row.handoff_pass,
            row.error_kind.as_str()
        ),
        (1, 0, 0, 0, "raw_mosaic:bad_dimensions")
    );
}

#[test]
fn metadata_sanity_rejects_active_geometry_outside_raw_dimensions() {
    let mut process = process_metadata(BayerPattern::RGGB);
    process.active_area.width = 17;
    let error = sanity_check_metadata(16, 8, &metadata_with_geometry(16, 8), &process).unwrap_err();
    assert_eq!(error, "process_metadata:bad_geometry");
}

#[test]
fn metadata_sanity_rejects_non_finite_or_over_white_black_levels() {
    let mut process = process_metadata(BayerPattern::RGGB);
    process.levels.black = MonoImage::from_raw(1, 1, vec![f64::NAN]).unwrap();
    assert_eq!(
        sanity_check_metadata(16, 8, &metadata_with_geometry(16, 8), &process).unwrap_err(),
        "process_metadata:bad_levels"
    );
}

#[test]
fn panic_is_mapped_to_a_runner_row() {
    let row = run_guarded(|| -> Result<StageRow, String> { panic!("decoder exploded") });
    assert_eq!(row.error_kind, "runner:panic");
    assert_eq!(
        (
            row.decode_pass,
            row.raw_mosaic_pass,
            row.process_metadata_pass,
            row.handoff_pass
        ),
        (0, 0, 0, 0)
    );
}

#[test]
fn decode_errors_use_stable_error_kinds() {
    assert_eq!(
        classify_decode_error(&fornax_core::error::DecodeError::Unsupported(
            "layout".into()
        )),
        "unsupported"
    );
    assert_eq!(
        classify_decode_error(&fornax_core::error::DecodeError::InvalidInput(
            "bytes".into()
        )),
        "invalid_input"
    );
}

#[test]
fn baseline_csv_is_canonical_and_rfc4180_quoted() {
    let row = BaselineRow {
        status: "✅".into(),
        camera: "Canon, EOS \"R5\"".into(),
        format: "DNG".into(),
        path: "Canon/EOS R5/IMG,001.DNG".into(),
        decode_pass: 1,
        raw_mosaic_pass: 1,
        process_metadata_pass: 1,
        handoff_pass: 1,
        error_kind: String::new(),
        width: 10,
        height: 20,
        cfa: "RGGB".into(),
    };
    let bytes = write_baseline_csv(std::slice::from_ref(&row)).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.starts_with(
        "status,camera,format,path,decode_pass,raw_mosaic_pass,process_metadata_pass,handoff_pass,error_kind,width,height,cfa\n"
    ));
    assert!(text.contains("\n✅,"));
    assert!(text.contains("\"Canon, EOS \"\"R5\"\"\""));
    assert!(text.contains(",1,1,1,1,"));
    assert!(text.ends_with(",RGGB\n"));
    assert!(!text.contains('\r'));
}

#[test]
fn manifest_check_validates_pointer_against_filelist_sha() {
    let temp = tempdir().unwrap();
    let corpus = temp.path();
    fs::create_dir_all(corpus.join("Canon/Camera")).unwrap();
    let listed = "aa".repeat(32);
    fs::write(
        corpus.join("filelist.sha256"),
        format!("{listed} *Canon/Camera/IMG_1.DNG\n"),
    )
    .unwrap();
    write_pointer(&corpus.join("Canon/Camera/IMG_1.DNG"), &listed, 42).unwrap();
    let rows = generate_manifest(corpus).unwrap();
    assert_eq!(rows.len(), 1);
    assert_eq!(rows[0].size, 42);

    fs::write(
        corpus.join("Canon/Camera/IMG_1.DNG"),
        format!(
            "version https://git-lfs.github.com/spec/v1\noid sha256:{}\nsize 42\n",
            "bb".repeat(32)
        ),
    )
    .unwrap();
    assert!(matches!(
        validate_pointer(corpus, &rows[0]),
        Err(error) if error.to_string().contains("listed SHA-256")
    ));
}

#[test]
fn manifest_includes_formats_without_decoder_support() {
    let temp = tempdir().unwrap();
    let corpus = temp.path();
    fs::create_dir_all(corpus.join("Canon/Camera")).unwrap();
    fs::create_dir_all(corpus.join("Sony/ILCE-7M5")).unwrap();
    let dng_sha = "aa".repeat(32);
    let arw_sha = "bb".repeat(32);
    fs::write(
        corpus.join("filelist.sha256"),
        format!("{dng_sha} *Canon/Camera/IMG_1.DNG\n{arw_sha} *Sony/ILCE-7M5/IMG_2.ARW\n"),
    )
    .unwrap();
    write_pointer(&corpus.join("Canon/Camera/IMG_1.DNG"), &dng_sha, 42).unwrap();
    write_pointer(&corpus.join("Sony/ILCE-7M5/IMG_2.ARW"), &arw_sha, 84).unwrap();

    let rows = generate_manifest(corpus).unwrap();
    assert_eq!(
        rows.iter().map(|row| row.path.as_str()).collect::<Vec<_>>(),
        ["Canon/Camera/IMG_1.DNG", "Sony/ILCE-7M5/IMG_2.ARW"]
    );
}

#[test]
fn manifest_csv_is_sorted_by_sha_and_round_trips() {
    let rows = vec![
        manifest_row("bb", "Canon/Cam/b.DNG", "DNG"),
        manifest_row("aa", "Canon/Cam/a.DNG", "DNG"),
    ];
    let bytes = write_manifest_csv(&rows).unwrap();
    assert!(
        std::str::from_utf8(&bytes)
            .unwrap()
            .starts_with("path,size,sha256\n")
    );
    let parsed = crate::csvio::read_manifest_csv(bytes.as_slice()).unwrap();
    assert_eq!(
        parsed
            .iter()
            .map(|row| row.sha256.as_str())
            .collect::<Vec<_>>(),
        ["aa", "bb"]
    );
}

#[test]
fn manifest_check_reports_snapshot_drift() {
    let temp = tempdir().unwrap();
    let corpus = temp.path();
    fs::create_dir_all(corpus.join("Canon/Camera")).unwrap();
    let listed = "aa".repeat(32);
    fs::write(
        corpus.join("filelist.sha256"),
        format!("{listed} *Canon/Camera/IMG_1.DNG\n"),
    )
    .unwrap();
    write_pointer(&corpus.join("Canon/Camera/IMG_1.DNG"), &listed, 42).unwrap();
    let manifest = write_manifest_csv(&generate_manifest(corpus).unwrap()).unwrap();
    assert_eq!(
        ManifestCheck::against(corpus, manifest.as_slice()).unwrap(),
        ManifestCheck::Current
    );

    fs::write(
        corpus.join("filelist.sha256"),
        format!("{} *Canon/Camera/IMG_1.DNG\n", "cc".repeat(32)),
    )
    .unwrap();
    write_pointer(&corpus.join("Canon/Camera/IMG_1.DNG"), &"cc".repeat(32), 42).unwrap();
    assert!(ManifestCheck::against(corpus, manifest.as_slice()).is_err());
}

#[test]
fn baseline_reader_rejects_non_boolean_stage_values() {
    let csv = "status,camera,format,path,decode_pass,raw_mosaic_pass,process_metadata_pass,handoff_pass,error_kind,width,height,cfa\n❌,Canon Cam,DNG,Canon/Cam/IMG.DNG,2,0,0,0,decode:bad,0,0,\n";
    assert!(read_baseline_csv(csv.as_bytes()).is_err());
}

#[test]
fn baseline_rejects_status_that_disagrees_with_stages() {
    let key = BaselineKey {
        camera: "Canon Cam".into(),
        format: "DNG".into(),
        path: "Canon/Cam/IMG.DNG".into(),
    };
    let mut row = baseline_row(&key, 1);
    row.status = "❌".into();
    assert!(
        write_baseline_csv(&[row])
            .unwrap_err()
            .to_string()
            .contains("status")
    );
}

#[test]
fn manifest_rows_have_unique_baseline_keys() {
    let rows = vec![
        manifest_row("aa", "Canon/Cam/IMG.DNG", "DNG"),
        manifest_row("bb", "Canon/Cam/IMG.DNG", "DNG"),
    ];
    assert!(crate::manifest::validate_manifest_keys(&rows).is_err());
}

#[test]
fn process_metadata_failure_retains_descriptive_cfa() {
    let mut metadata = metadata_with_geometry(16, 8);
    metadata.insert(
        fornax_tiff::constants::TiffTag::CfaRepeatPatternDim.into(),
        Value::List(vec![Value::Short(6), Value::Short(6)]),
    );
    metadata.insert(
        fornax_tiff::constants::TiffTag::CfaPattern.into(),
        Value::List(vec![Value::Byte(0); 36]),
    );
    assert_eq!(crate::stages::describe_cfa(&metadata), "XTrans-6x6");
}

#[test]
fn child_timeout_kills_the_process_and_returns_timeout() {
    #[cfg(unix)]
    {
        let mut command = std::process::Command::new("sh");
        command.args(["-c", "sleep 5"]);
        let outcome =
            crate::runner::wait_with_timeout(&mut command, std::time::Duration::from_millis(30))
                .unwrap();
        assert!(matches!(
            outcome,
            crate::runner::ChildOutcome::TimedOut { .. }
        ));
    }
}

#[test]
fn lfs_paths_escape_gitignore_glob_metacharacters() {
    assert_eq!(
        crate::runner::escape_lfs_glob("Canon/Cam/IMG_[1].DNG"),
        "Canon/Cam/IMG_\\[1\\].DNG"
    );
}

#[test]
fn baseline_roundtrip_keeps_models_and_rows_stable() {
    let key = BaselineKey {
        camera: "Canon EOS".into(),
        format: "DNG".into(),
        path: "Canon/Cam/IMG_1.DNG".into(),
    };
    let rows = vec![baseline_row(&key, 1)];
    let bytes = write_baseline_csv(&rows).unwrap();
    let parsed = read_baseline_csv(bytes.as_slice()).unwrap();
    assert_eq!(parsed, BaselineCsv { rows });
}

fn write_manifest(path: &std::path::Path, rows: &[ManifestRow]) {
    fs::write(path, write_manifest_csv(rows).unwrap()).unwrap();
}

fn write_baseline(path: &std::path::Path, rows: &[BaselineRow]) {
    fs::write(path, write_baseline_csv(rows).unwrap()).unwrap();
}

fn duckdb_test_guard() -> bool {
    std::process::Command::new("duckdb")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
}

#[test]
fn duckdb_merge_rejects_duplicate_shard_rows() {
    if !duckdb_test_guard() {
        return;
    }
    let temp = tempdir().unwrap();
    let key = BaselineKey {
        camera: "Canon EOS".into(),
        format: "DNG".into(),
        path: "Canon/Cam/IMG_1.DNG".into(),
    };
    let manifest = temp.path().join("selected.csv");
    let shard_a = temp.path().join("a.csv");
    let shard_b = temp.path().join("b.csv");
    let output = temp.path().join("merged.csv");
    write_manifest(
        &manifest,
        &[manifest_row("aa", "Canon/Cam/IMG_1.DNG", "DNG")],
    );
    write_baseline(&shard_a, &[baseline_row(&key, 1)]);
    write_baseline(&shard_b, &[baseline_row(&key, 1)]);
    assert!(
        merge(&manifest, &[shard_a, shard_b], &output)
            .unwrap_err()
            .to_string()
            .contains("duplicate")
    );
}

#[test]
fn duckdb_merge_rejects_missing_selected_rows() {
    if !duckdb_test_guard() {
        return;
    }
    let temp = tempdir().unwrap();
    let key_a = BaselineKey {
        camera: "Canon Cam".into(),
        format: "DNG".into(),
        path: "Canon/Cam/A.DNG".into(),
    };
    let manifest = temp.path().join("selected.csv");
    let shard = temp.path().join("shard.csv");
    let output = temp.path().join("merged.csv");
    write_manifest(
        &manifest,
        &[
            manifest_row("aa", "Canon/Cam/A.DNG", "DNG"),
            manifest_row("bb", "Canon/Cam/B.DNG", "DNG"),
        ],
    );
    write_baseline(&shard, &[baseline_row(&key_a, 1)]);
    assert!(
        merge(&manifest, &[shard], &output)
            .unwrap_err()
            .to_string()
            .contains("missing")
    );
}

#[test]
fn duckdb_diff_classifies_selected_missing_extra_and_changed_rows() {
    if !duckdb_test_guard() {
        return;
    }
    let temp = tempdir().unwrap();
    let key_a = BaselineKey {
        camera: "Canon Cam".into(),
        format: "DNG".into(),
        path: "Canon/Cam/A.DNG".into(),
    };
    let key_b = BaselineKey {
        path: "Canon/Cam/B.DNG".into(),
        ..key_a.clone()
    };
    let key_c = BaselineKey {
        path: "Canon/Cam/C.DNG".into(),
        ..key_a.clone()
    };
    let baseline = temp.path().join("baseline.csv");
    let candidate = temp.path().join("candidate.csv");
    let selected = temp.path().join("selected.csv");
    let report = temp.path().join("diff.csv");
    write_baseline(
        &baseline,
        &[baseline_row(&key_a, 1), baseline_row(&key_b, 1)],
    );
    let mut changed_a = baseline_row(&key_a, 0);
    changed_a.width = 99;
    write_baseline(&candidate, &[changed_a, baseline_row(&key_c, 1)]);
    write_manifest(
        &selected,
        &[
            manifest_row("aa", "Canon/Cam/A.DNG", "DNG"),
            manifest_row("bb", "Canon/Cam/B.DNG", "DNG"),
            manifest_row("cc", "Canon/Cam/C.DNG", "DNG"),
        ],
    );
    assert_eq!(
        diff(&baseline, &candidate, &selected, &report).unwrap(),
        DiffSummary {
            missing: 1,
            extra: 1,
            changed: 1,
        }
    );
    assert!(fs::read_to_string(report).unwrap().contains("changed"));
}

#[test]
fn duckdb_upsert_preserves_unselected_rows_and_uses_canonical_order() {
    if !duckdb_test_guard() {
        return;
    }
    let temp = tempdir().unwrap();
    let key_a = BaselineKey {
        camera: "Canon EOS".into(),
        format: "DNG".into(),
        path: "Canon/Cam/A.DNG".into(),
    };
    let key_b = BaselineKey {
        path: "Canon/Cam/B.DNG".into(),
        ..key_a.clone()
    };
    let baseline = temp.path().join("baseline.csv");
    let candidate = temp.path().join("candidate.csv");
    let output = temp.path().join("updated.csv");
    write_baseline(
        &baseline,
        &[baseline_row(&key_b, 1), baseline_row(&key_a, 1)],
    );
    let mut changed_a = baseline_row(&key_a, 0);
    changed_a.error_kind = "handoff:unsupported".into();
    write_baseline(&candidate, &[changed_a]);

    upsert(Some(&baseline), &candidate, &output).unwrap();
    let text = fs::read_to_string(output).unwrap();
    assert!(text.starts_with(&format!("{}\n", crate::model::BASELINE_HEADER.join(","))));
    assert_eq!(text.matches("Canon/Cam/A.DNG").count(), 1);
    assert_eq!(text.matches("Canon/Cam/B.DNG").count(), 1);
    assert!(!text.contains('\r'));
    assert!(text.find("Canon/Cam/A.DNG").unwrap() < text.find("Canon/Cam/B.DNG").unwrap());
}
