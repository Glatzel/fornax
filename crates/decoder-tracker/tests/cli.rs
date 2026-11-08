use std::fs;

use assert_cmd::Command;
use decoder_tracker::csvio::{read_manifest_csv, write_manifest_csv};
use decoder_tracker::manifest::{derive_identity, generate_manifest};
use decoder_tracker::model::ManifestRow;
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
        write_manifest_csv(&generate_manifest(&corpus, &["DNG".into()]).unwrap()).unwrap(),
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
