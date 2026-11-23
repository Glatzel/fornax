use std::path::{Path, PathBuf};
use std::sync::OnceLock;
use std::{env, fs};

use decoder_tracker::csvio::read_manifest_csv;
use decoder_tracker::golden::{GoldenCsv, check_golden_csv_subset, decode_golden, read_golden_csv};
use decoder_tracker::model::ManifestRow;
use decoder_tracker::runner::materialize;
use rstest::rstest;
use sha2::{Digest, Sha256};

const CORPUS_PARTITIONS: usize = 4;

fn repository_root() -> PathBuf { Path::new(env!("CARGO_MANIFEST_DIR")).join("../..") }

struct CorpusFixture {
    corpus: PathBuf,
    rows: Vec<ManifestRow>,
    expected: GoldenCsv,
}

fn corpus_fixture() -> &'static CorpusFixture {
    static FIXTURE: OnceLock<Result<CorpusFixture, String>> = OnceLock::new();
    let fixture = FIXTURE.get_or_init(|| {
        let root = repository_root();
        let corpus = root.join("external/raw.pixls.us");
        let manifest_path = root.join("crates/decoder-tracker/golden/test-manifest.csv");
        let hashes_path = root.join("crates/decoder-tracker/golden/decode-hashes.csv");
        let rows = read_manifest_csv(fs::read(manifest_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
        materialize(&corpus, &rows).map_err(|error| error.to_string())?;
        let expected = read_golden_csv(fs::read(hashes_path).map_err(|error| error.to_string())?)
            .map_err(|error| error.to_string())?;
        Ok(CorpusFixture {
            corpus,
            rows,
            expected,
        })
    });
    match fixture {
        Ok(fixture) => fixture,
        Err(error) => panic!("preparing golden corpus fixture failed: {error}"),
    }
}

#[rstest]
#[case(0)]
#[case(1)]
#[case(2)]
#[case(3)]
fn committed_golden_hashes_match_materialized_corpus(#[case] partition: usize) {
    if env::var_os("FORNAX_GOLDEN_CORPUS_REQUIRED").as_deref() != Some(std::ffi::OsStr::new("1")) {
        return;
    }

    let fixture = corpus_fixture();
    let actual = fixture
        .rows
        .iter()
        .enumerate()
        .filter(|(index, _)| index % CORPUS_PARTITIONS == partition)
        .map(|(_, row)| row)
        .map(|row| {
            let path = fixture.corpus.join(&row.path);
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

    check_golden_csv_subset(&actual, &fixture.expected).unwrap();
}
