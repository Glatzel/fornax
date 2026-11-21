use std::path::{Path, PathBuf};
use std::{env, fs};

use decoder_tracker::csvio::read_manifest_csv;
use decoder_tracker::golden::{check_golden_csv_subset, decode_golden, read_golden_csv};
use sha2::{Digest, Sha256};

fn repository_root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..") }

#[test]
fn committed_golden_hashes_match_materialized_corpus() {
    if env::var_os("FORNAX_GOLDEN_CORPUS_REQUIRED").as_deref() != Some(std::ffi::OsStr::new("1")) {
        return;
    }

    let root = repository_root();
    let corpus = root.join("external/raw.pixls.us");
    let manifest_path = root.join("crates/decoder-tracker/golden/test-manifest.csv");
    let hashes_path = root.join("crates/decoder-tracker/golden/decode-hashes.csv");
    let rows = read_manifest_csv(fs::read(manifest_path).unwrap()).unwrap();
    let expected = read_golden_csv(fs::read(hashes_path).unwrap()).unwrap();
    let actual = rows
        .iter()
        .map(|row| {
            let path = corpus.join(&row.path);
            let bytes = fs::read(&path)
                .unwrap_or_else(|error| panic!("reading {} failed: {error}", path.display()));
            assert_eq!(
                bytes.len() as u64,
                row.size,
                "{} has the wrong byte length",
                row.path
            );
            assert_eq!(
                hex::encode(Sha256::digest(&bytes)),
                row.sha256,
                "{} has the wrong source SHA-256",
                row.path
            );
            decode_golden(&row.path, &path).unwrap()
        })
        .collect::<Vec<_>>();

    check_golden_csv_subset(&actual, &expected).unwrap();
}
