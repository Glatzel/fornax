use std::fs;

use assert_cmd::Command;
use decoder_tracker::csvio::{read_manifest_csv, write_baseline_csv, write_manifest_csv};
use decoder_tracker::manifest::{derive_identity, generate_manifest};
use decoder_tracker::model::{BaselineRow, ManifestRow, status_for_stages};
use tempfile::tempdir;

fn manifest_row(sha: &str, path: &str) -> ManifestRow {
    let (vendor, camera, model, filename) = derive_identity(path).unwrap();
    ManifestRow {
        sha256: sha.into(),
        size: 32,
        path: path.into(),
        vendor,
        camera,
        model,
        format: "DNG".into(),
        filename,
    }
}

#[test]
fn run_rejects_more_than_two_files_without_ci_or_allow_full() {
    let temp = tempdir().unwrap();
    let rows = [
        manifest_row(&"aa".repeat(32), "Canon/Cam/a.DNG"),
        manifest_row(&"bb".repeat(32), "Canon/Cam/b.DNG"),
        manifest_row(&"cc".repeat(32), "Canon/Cam/c.DNG"),
    ];
    let manifest = temp.path().join("manifest.csv");
    fs::write(&manifest, write_manifest_csv(&rows).unwrap()).unwrap();

    Command::cargo_bin("decoder-tracker")
        .unwrap()
        .args([
            "run",
            "--manifest",
            manifest.to_str().unwrap(),
            "--out",
            temp.path().join("out.csv").to_str().unwrap(),
        ])
        .env_remove("CI")
        .assert()
        .failure()
        .stderr(predicates::str::contains("selection has 3 files"));
}

#[test]
fn manifest_check_cli_accepts_current_snapshot_and_rejects_drift() {
    let temp = tempdir().unwrap();
    let corpus = temp.path().join("corpus");
    let path = corpus.join("Canon/Camera/IMG_1.DNG");
    fs::create_dir_all(path.parent().unwrap()).unwrap();
    let listed = "aa".repeat(32);
    fs::write(
        corpus.join("filelist.sha256"),
        format!("{listed} *Canon/Camera/IMG_1.DNG\n"),
    )
    .unwrap();
    fs::write(
        &path,
        format!("version https://git-lfs.github.com/spec/v1\noid sha256:{listed}\nsize 32\n"),
    )
    .unwrap();
    let manifest = temp.path().join("manifest.csv");
    fs::write(
        &manifest,
        write_manifest_csv(&generate_manifest(&corpus).unwrap()).unwrap(),
    )
    .unwrap();

    Command::cargo_bin("decoder-tracker")
        .unwrap()
        .args([
            "manifest",
            "--check",
            "--corpus",
            corpus.to_str().unwrap(),
            "--out",
            manifest.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("manifest: current (1 rows)"));

    fs::write(
        corpus.join("filelist.sha256"),
        format!("{} *Canon/Camera/IMG_2.DNG\n", listed),
    )
    .unwrap();
    let new_path = corpus.join("Canon/Camera/IMG_2.DNG");
    fs::write(
        new_path,
        format!("version https://git-lfs.github.com/spec/v1\noid sha256:{listed}\nsize 32\n"),
    )
    .unwrap();
    Command::cargo_bin("decoder-tracker")
        .unwrap()
        .args([
            "manifest",
            "--check",
            "--corpus",
            corpus.to_str().unwrap(),
            "--out",
            manifest.to_str().unwrap(),
        ])
        .assert()
        .failure()
        .stderr(predicates::str::contains("manifest snapshot drift"));
}

#[test]
fn select_writes_only_glob_matches_and_prints_the_count() {
    let temp = tempdir().unwrap();
    let rows = [
        manifest_row(&"aa".repeat(32), "Canon/Cam/A.DNG"),
        manifest_row(&"bb".repeat(32), "Canon/Nested/B.DNG"),
        manifest_row(&"cc".repeat(32), "Sony/Cam/C.DNG"),
    ];
    let manifest = temp.path().join("manifest.csv");
    let selected = temp.path().join("selected.csv");
    fs::write(&manifest, write_manifest_csv(&rows).unwrap()).unwrap();

    Command::cargo_bin("decoder-tracker")
        .unwrap()
        .args([
            "select",
            "--manifest",
            manifest.to_str().unwrap(),
            "--glob",
            "Canon/**/*.DNG",
            "--out",
            selected.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stdout(predicates::str::contains("selected=2"));
    assert_eq!(
        read_manifest_csv(fs::read(selected).unwrap())
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn select_emits_clerk_info_logs() {
    let temp = tempdir().unwrap();
    let rows = [
        manifest_row(&"aa".repeat(32), "Canon/Cam/A.DNG"),
        manifest_row(&"bb".repeat(32), "Canon/Nested/B.DNG"),
        manifest_row(&"cc".repeat(32), "Sony/Cam/C.DNG"),
    ];
    let manifest = temp.path().join("manifest.csv");
    let selected = temp.path().join("selected.csv");
    fs::write(&manifest, write_manifest_csv(&rows).unwrap()).unwrap();

    Command::cargo_bin("decoder-tracker")
        .unwrap()
        .args([
            "select",
            "-v",
            "--manifest",
            manifest.to_str().unwrap(),
            "--glob",
            "Canon/**/*.DNG",
            "--out",
            selected.to_str().unwrap(),
        ])
        .assert()
        .success()
        .stderr(predicates::str::contains("selected 2 corpus files"));
}

#[test]
fn duckdb_select_script_filters_without_the_rust_binary() {
    if !std::process::Command::new("duckdb")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        return;
    }
    let temp = tempdir().unwrap();
    let rows = [
        manifest_row(&"aa".repeat(32), "Canon/Cam/A.DNG"),
        manifest_row(&"bb".repeat(32), "Canon/Nested/B.DNG"),
        manifest_row(&"cc".repeat(32), "Sony/Cam/C.DNG"),
    ];
    let manifest = temp.path().join("manifest.csv");
    let selected = temp.path().join("selected.csv");
    fs::write(&manifest, write_manifest_csv(&rows).unwrap()).unwrap();
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/select.sh");

    let output = std::process::Command::new("bash")
        .arg(&script)
        .args([
            "--manifest",
            manifest.to_str().unwrap(),
            "--filter",
            "Canon/**/*.DNG",
            "--out",
            selected.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert_eq!(String::from_utf8_lossy(&output.stdout).trim(), "selected=2");
    assert_eq!(
        read_manifest_csv(fs::read(selected).unwrap())
            .unwrap()
            .len(),
        2
    );
}

#[test]
fn duckdb_aggregate_script_merges_and_diffs_without_rust() {
    if !std::process::Command::new("duckdb")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        return;
    }
    let temp = tempdir().unwrap();
    let rows = [
        manifest_row(&"aa".repeat(32), "Canon/Cam/A.DNG"),
        manifest_row(&"bb".repeat(32), "Canon/Cam/B.DNG"),
    ];
    let manifest = temp.path().join("selected.csv");
    fs::write(&manifest, write_manifest_csv(&rows).unwrap()).unwrap();
    let baseline_rows = rows
        .iter()
        .map(|row| BaselineRow {
            status: status_for_stages([1, 1, 1, 1]).into(),
            camera: row.camera.clone(),
            format: row.format.clone(),
            path: row.path.clone(),
            decode_pass: 1,
            raw_mosaic_pass: 1,
            process_metadata_pass: 1,
            handoff_pass: 1,
            error_kind: String::new(),
            width: 16,
            height: 8,
            cfa: "RGGB".into(),
        })
        .collect::<Vec<_>>();
    let shard_a = temp.path().join("a.csv");
    let shard_b = temp.path().join("b.csv");
    fs::write(&shard_a, write_baseline_csv(&baseline_rows[..1]).unwrap()).unwrap();
    fs::write(&shard_b, write_baseline_csv(&baseline_rows[1..]).unwrap()).unwrap();
    let baseline = temp.path().join("baseline.csv");
    fs::write(&baseline, write_baseline_csv(&baseline_rows).unwrap()).unwrap();
    let output_file = temp.path().join("github-output");
    fs::write(&output_file, "").unwrap();
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/aggregate.sh");

    let output = std::process::Command::new("bash")
        .arg(&script)
        .args([
            "--selected",
            manifest.to_str().unwrap(),
            "--baseline",
            baseline.to_str().unwrap(),
            "--work-dir",
            temp.path().join("update").to_str().unwrap(),
            "--github-output",
            output_file.to_str().unwrap(),
            "--",
            shard_a.to_str().unwrap(),
            shard_b.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    assert!(String::from_utf8_lossy(&output.stdout).contains("outcome=unchanged"));

    let changed_a = temp.path().join("changed-a.csv");
    let mut changed_row = baseline_rows[0].clone();
    changed_row.status = "❌".into();
    changed_row.decode_pass = 0;
    changed_row.raw_mosaic_pass = 0;
    changed_row.process_metadata_pass = 0;
    changed_row.handoff_pass = 0;
    changed_row.error_kind = "decode:invalid_input".into();
    fs::write(
        &changed_a,
        write_baseline_csv(std::slice::from_ref(&changed_row)).unwrap(),
    )
    .unwrap();
    let changed_output = temp.path().join("github-output-changed");
    fs::write(&changed_output, "").unwrap();
    let changed = std::process::Command::new("bash")
        .arg(script)
        .args([
            "--selected",
            manifest.to_str().unwrap(),
            "--baseline",
            baseline.to_str().unwrap(),
            "--work-dir",
            temp.path().join("update-changed").to_str().unwrap(),
            "--github-output",
            changed_output.to_str().unwrap(),
            "--",
            changed_a.to_str().unwrap(),
            shard_b.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        changed.status.success(),
        "{}",
        String::from_utf8_lossy(&changed.stderr)
    );
    assert!(String::from_utf8_lossy(&changed.stdout).contains("outcome=changed"));
    assert!(
        fs::read_to_string(temp.path().join("update-changed/baseline.csv"))
            .unwrap()
            .contains("decode:invalid_input")
    );
}

#[test]
fn duckdb_badge_script_counts_formats_missing_from_the_baseline() {
    if !std::process::Command::new("duckdb")
        .arg("--version")
        .output()
        .is_ok_and(|output| output.status.success())
    {
        return;
    }
    let temp = tempdir().unwrap();
    let baseline = temp.path().join("baseline.csv");
    let manifest = temp.path().join("manifest.csv");
    let readme = temp.path().join("README.md");
    let rows = vec![
        BaselineRow {
            status: status_for_stages([1, 1, 1, 1]).into(),
            camera: "Canon Cam".into(),
            format: "DNG".into(),
            path: "Canon/Cam/A.DNG".into(),
            decode_pass: 1,
            raw_mosaic_pass: 1,
            process_metadata_pass: 1,
            handoff_pass: 1,
            error_kind: String::new(),
            width: 16,
            height: 8,
            cfa: "RGGB".into(),
        },
        BaselineRow {
            status: status_for_stages([0, 0, 0, 0]).into(),
            camera: "Canon Cam".into(),
            format: "DNG".into(),
            path: "Canon/Cam/B.DNG".into(),
            decode_pass: 0,
            raw_mosaic_pass: 0,
            process_metadata_pass: 0,
            handoff_pass: 0,
            error_kind: "decode:invalid_input".into(),
            width: 0,
            height: 0,
            cfa: "unknown".into(),
        },
    ];
    fs::write(&baseline, write_baseline_csv(&rows).unwrap()).unwrap();
    let manifest_rows = vec![
        manifest_row(&"aa".repeat(32), "Canon/Cam/A.DNG"),
        manifest_row(&"bb".repeat(32), "Canon/Cam/B.DNG"),
        manifest_row(&"cc".repeat(32), "Sony/Cam/C.ARW"),
    ];
    fs::write(&manifest, write_manifest_csv(&manifest_rows).unwrap()).unwrap();
    fs::write(
        &readme,
        "# decoder-tracker\n\n<!-- decoder-tracker-badge:start -->\nold\n<!-- decoder-tracker-badge:end -->\n",
    )
    .unwrap();
    let script = std::path::Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/update-badge.sh");

    let output = std::process::Command::new("bash")
        .arg(script)
        .args([
            "--baseline",
            baseline.to_str().unwrap(),
            "--manifest",
            manifest.to_str().unwrap(),
            "--readme",
            readme.to_str().unwrap(),
        ])
        .output()
        .unwrap();
    assert!(
        output.status.success(),
        "{}",
        String::from_utf8_lossy(&output.stderr)
    );
    let text = fs::read_to_string(readme).unwrap();
    assert!(text.contains("raw.pixls.us_corpus_pass-1%2F3"));
    assert!(text.contains("| DNG | 1/2 |"));
    assert!(text.contains("| ARW | 0/1 |"));
    assert!(!text.contains("\nold\n"));
}
