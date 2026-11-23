use std::fs;
use std::path::Path;

use fornax_raw::dng::Directory;
use fornax_raw::dng::metadata::IDecoderDng;
use fornax_raw::error::DecodeError;
use fornax_raw::metadata::{
    ColorCalibration, EncodingMetadata, EncodingSampleFormat, Levels, MetadataSource,
    ProcessMetadata, RawColorMeta,
};
use fornax_raw::protocol::{IDecoder, IDecoderEncoding};
use fornax_raw::{BayerPattern, CroppedMosaic, MonoImage, RawFloat, RawMosaic};
use fornax_tiff::Value;
use image::math::Rect;
use rstest::rstest;
use tempfile::tempdir;

use crate::csvio::{
    BaselineCsv, read_baseline_csv, read_manifest_csv, write_baseline_csv, write_manifest_csv,
};
use crate::manifest::{
    ManifestCheck, generate_manifest, normalize_repo_path, validate_pointer, write_pointer,
};
use crate::model::{BaselineKey, BaselineRow, ManifestRow};
use crate::selection::{Selection, Shard, select_file};
use crate::stages::{
    StageRow, classify_decode_error, evaluate_decoder, run_guarded, sanity_check_metadata,
    transition_row,
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
        raw_sha256: "aa".repeat(32),
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
        compression_id: Some(7),
        compression_name: "lossless-jpeg".into(),
        compression_lossless: Some(1),
        storage: "tiles".into(),
        bits_per_sample: Some(14),
        sample_format: "integer".into(),
        predictor: Some(1),
        rows_per_strip: None,
        tile_width: Some(256),
        tile_height: Some(256),
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
        wb_source: MetadataSource::Default,
        black_source: MetadataSource::Default,
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

#[rstest]
#[case("1/0")]
#[case("2/2")]
#[case("-1/2")]
#[case("1/x")]
fn shard_rejects_out_of_bounds_or_malformed_values(#[case] value: &str) {
    assert!(value.parse::<Shard>().is_err());
}

#[test]
fn selected_manifest_sharding_does_not_require_python() {
    let temp = tempdir().unwrap();
    let manifest = temp.path().join("selected.csv");
    let output = temp.path().join("shard.csv");
    let rows = [
        manifest_row("dd", "Canon/Cam/d.DNG", "DNG"),
        manifest_row("bb", "Canon/Cam/b.DNG", "DNG"),
        manifest_row("aa", "Canon/Cam/a.DNG", "DNG"),
        manifest_row("cc", "Canon/Cam/c.DNG", "DNG"),
    ];
    fs::write(&manifest, write_manifest_csv(&rows).unwrap()).unwrap();

    let count = select_file(
        &manifest,
        &output,
        &Selection {
            shard: Some(Shard { index: 0, count: 2 }),
            ..Selection::default()
        },
    )
    .unwrap();

    assert_eq!(count, 2);
    assert_eq!(
        read_manifest_csv(fs::read(output).unwrap())
            .unwrap()
            .iter()
            .map(|row| row.sha256.as_str())
            .collect::<Vec<_>>(),
        ["aa", "cc"]
    );
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
    assert_eq!(row.compression_name, "unknown");
    assert_eq!(row.storage, "unknown");
}

#[test]
fn decode_errors_use_stable_error_kinds() {
    assert_eq!(
        classify_decode_error(&fornax_raw::error::DecodeError::Unsupported(
            "layout".into()
        )),
        "unsupported"
    );
    assert_eq!(
        classify_decode_error(&fornax_raw::error::DecodeError::InvalidInput(
            "bytes".into()
        )),
        "invalid_input"
    );
}

struct RawFailureDecoder;

impl IDecoder for RawFailureDecoder {
    fn decode(_: &Path) -> Result<Self, DecodeError> { Ok(Self) }

    fn raw_mosaic(&self) -> Result<RawMosaic, DecodeError> {
        Err(DecodeError::Unsupported("raw layout".into()))
    }

    fn cropped_mosaic<T>(&self) -> Result<CroppedMosaic<T>, DecodeError>
    where
        T: RawFloat,
    {
        Err(DecodeError::Unsupported("raw layout".into()))
    }

    fn process_metadata(&self) -> Result<ProcessMetadata, DecodeError> {
        Err(DecodeError::Unsupported("raw layout".into()))
    }
}

impl IDecoderDng for RawFailureDecoder {
    fn dng_metadata(&self) -> Result<Directory, DecodeError> {
        Err(DecodeError::Unsupported("raw layout".into()))
    }
}

impl IDecoderEncoding for RawFailureDecoder {
    fn encoding_metadata(&self) -> EncodingMetadata {
        EncodingMetadata {
            compression_id: Some(7),
            compression_name: "lossless-jpeg",
            lossless: true,
            storage: "tiles",
            bits_per_sample: Some(14),
            sample_format: Some(EncodingSampleFormat::Integer),
            predictor: Some(1),
            rows_per_strip: Some(171),
            tile_width: Some(256),
            tile_height: Some(256),
        }
    }
}

#[test]
fn raw_decode_failure_retains_constructed_decoder_metadata() {
    let row = evaluate_decoder::<RawFailureDecoder>(Path::new("ignored.dng")).unwrap();

    assert_eq!(row.decode_pass, 0);
    assert_eq!(row.error_kind, "raw_mosaic:unsupported");
    assert_eq!(row.compression_name, "lossless-jpeg");
    assert_eq!(row.compression_lossless, Some(1));
    assert_eq!(row.storage, "tiles");
    assert_eq!(row.bits_per_sample, Some(14));
    assert_eq!(row.rows_per_strip, Some(171));
}

#[test]
fn stage_row_preserves_encoding_metadata_after_construction() {
    let row = StageRow::default().with_encoding(EncodingMetadata {
        compression_id: Some(7),
        compression_name: "sony-lossless-jpeg",
        lossless: true,
        storage: "tiles",
        bits_per_sample: Some(14),
        sample_format: Some(EncodingSampleFormat::Integer),
        predictor: None,
        rows_per_strip: None,
        tile_width: Some(256),
        tile_height: Some(256),
    });

    assert_eq!(row.compression_id, Some(7));
    assert_eq!(row.compression_name, "sony-lossless-jpeg");
    assert_eq!(row.compression_lossless, Some(1));
    assert_eq!(row.storage, "tiles");
    assert_eq!(row.bits_per_sample, Some(14));
    assert_eq!(row.sample_format, "integer");
    assert_eq!(row.tile_width, Some(256));
    assert_eq!(row.tile_height, Some(256));
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
        raw_sha256: "11".repeat(32),
        process_metadata_pass: 1,
        handoff_pass: 1,
        error_kind: String::new(),
        width: 10,
        height: 20,
        cfa: "RGGB".into(),
        compression_id: Some(7),
        compression_name: "lossless-jpeg".into(),
        compression_lossless: Some(1),
        storage: "tiles".into(),
        bits_per_sample: Some(14),
        sample_format: "integer".into(),
        predictor: Some(1),
        rows_per_strip: None,
        tile_width: Some(256),
        tile_height: Some(256),
    };
    let bytes = write_baseline_csv(std::slice::from_ref(&row)).unwrap();
    let text = std::str::from_utf8(&bytes).unwrap();
    assert!(text.starts_with(
        "status,camera,format,path,decode_pass,raw_mosaic_pass,raw_sha256,process_metadata_pass,handoff_pass,error_kind,width,height,cfa,compression_id,compression_name,compression_lossless,storage,bits_per_sample,sample_format,predictor,rows_per_strip,tile_width,tile_height\n"
    ));
    assert!(text.contains("\n✅,"));
    assert!(text.contains("\"Canon, EOS \"\"R5\"\"\""));
    assert!(text.contains(&format!(",1,1,{},1,1,", "11".repeat(32))));
    assert!(text.ends_with(",RGGB,7,lossless-jpeg,1,tiles,14,integer,1,,256,256\n"));
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
fn materialized_payload_is_not_parsed_as_an_unbounded_lfs_pointer() {
    let temp = tempdir().unwrap();
    let path = temp.path().join("payload.RAW");
    let mut bytes = format!(
        "version https://git-lfs.github.com/spec/v1\noid sha256:{}\nsize 42\n",
        "aa".repeat(32)
    )
    .into_bytes();
    bytes.resize(2048, 0);
    fs::write(&path, bytes).unwrap();
    assert!(crate::manifest::read_working_pointer(&path).is_none());
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
    let csv = "status,camera,format,path,decode_pass,raw_mosaic_pass,raw_sha256,process_metadata_pass,handoff_pass,error_kind,width,height,cfa,compression_id,compression_name,compression_lossless,storage,bits_per_sample,sample_format,predictor,rows_per_strip,tile_width,tile_height\n❌,Canon Cam,DNG,Canon/Cam/IMG.DNG,2,0,,0,0,decode:bad,0,0,,,,,,,,,,\n";
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
fn baseline_rejects_successful_raw_decode_without_encoding_metadata() {
    let key = BaselineKey {
        camera: "Canon Cam".into(),
        format: "DNG".into(),
        path: "Canon/Cam/IMG.DNG".into(),
    };
    let mut row = baseline_row(&key, 1);
    row.compression_name = "unknown".into();
    row.storage = "unknown".into();
    row.bits_per_sample = None;
    row.sample_format.clear();
    assert!(
        write_baseline_csv(&[row])
            .unwrap_err()
            .to_string()
            .contains("encoding metadata")
    );
}

#[test]
fn baseline_rejects_successful_raw_decode_without_lossless_flag() {
    let key = BaselineKey {
        camera: "Canon Cam".into(),
        format: "DNG".into(),
        path: "Canon/Cam/IMG.DNG".into(),
    };
    let mut row = baseline_row(&key, 1);
    row.compression_lossless = None;
    assert!(
        write_baseline_csv(&[row])
            .unwrap_err()
            .to_string()
            .contains("encoding metadata")
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
    assert_eq!(
        crate::runner::escape_lfs_glob("FUJIFILM/X-M5/DSCF0623_-_crop_1,25.RAF"),
        "FUJIFILM/X-M5/DSCF0623_-_crop_1,25.RAF"
    );
    assert_eq!(
        crate::runner::lfs_include_glob("FUJIFILM/X-M5/DSCF0623_-_crop_1,25.RAF"),
        "FUJIFILM/X-M5/DSCF0623_-_crop_1?25.RAF"
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
