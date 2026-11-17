use std::collections::BTreeSet;
use std::fs::{self, File};
use std::io::Read;
use std::path::{Path, PathBuf};

use csv::ReaderBuilder;

const LFS_POINTER_PREFIX: &str = "version https://git-lfs.github.com/spec/v1";
#[allow(
    dead_code,
    reason = "used by the decoder benchmark and equality targets but not parser unit tests"
)]
const DEFAULT_TEST_MANIFEST: &str = "crates/decoder-tracker/golden/test-manifest.csv";
const DEFAULT_PERF_MANIFEST: &str = "crates/decoder-tracker/golden/perf-manifest.csv";
#[allow(
    dead_code,
    reason = "used by the decoder benchmark and equality targets but not parser unit tests"
)]
const DEFAULT_CORPUS: &str = "external/raw.pixls.us";

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoldenInput {
    pub path: String,
    pub size: u64,
    pub sha256: String,
    pub group: Option<BenchmarkGroup>,
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
pub enum BenchmarkGroup {
    Core,
    Full,
    Overhead,
    Hdr,
}

impl BenchmarkGroup {
    fn parse(value: &str) -> Result<Self, String> {
        match value.trim().to_ascii_lowercase().as_str() {
            "core" => Ok(Self::Core),
            "full" => Ok(Self::Full),
            "overhead" => Ok(Self::Overhead),
            "hdr" => Ok(Self::Hdr),
            _ => Err(format!(
                "invalid benchmark group {value:?}; expected core, full, overhead, hdr, or all"
            )),
        }
    }
}

impl GoldenInput {
    pub fn format(&self) -> String {
        self.path
            .rsplit_once('.')
            .map_or_else(String::new, |(_, extension)| extension.to_ascii_uppercase())
    }

    pub fn case_label(&self) -> String {
        let mut path = String::with_capacity(self.path.len());
        for character in self.path.chars() {
            match character {
                '/' => path.push_str("--"),
                ' ' => path.push('_'),
                character
                    if character.is_ascii_alphanumeric()
                        || matches!(character, '.' | '-' | '_') =>
                {
                    path.push(character);
                }
                _ => path.push('-'),
            }
        }
        format!("{}__{path}--{}", self.format(), &self.sha256[..8])
    }
}

pub fn parse_golden_manifest(bytes: &[u8]) -> Result<Vec<GoldenInput>, String> {
    let mut reader = ReaderBuilder::new().flexible(false).from_reader(bytes);
    let headers = reader
        .headers()
        .map_err(|error| format!("reading golden manifest header failed: {error}"))?;
    let header = headers.iter().collect::<Vec<_>>();
    let has_group = matches!(
        header.as_slice(),
        [
            "path",
            "size",
            "sha256",
            "decoder_path",
            "reason",
            "overhead_only",
            "group",
        ]
    );
    let extended = has_group
        || matches!(
            header.as_slice(),
            [
                "path",
                "size",
                "sha256",
                "decoder_path",
                "reason",
                "overhead_only",
            ]
        );
    let basic = matches!(header.as_slice(), ["path", "size", "sha256"]);
    if !basic && !extended {
        return Err(
            "golden manifest header must be path,size,sha256 or include \
             decoder_path,reason,overhead_only and optional group"
                .to_owned(),
        );
    }

    let mut paths = BTreeSet::new();
    let mut rows = Vec::new();
    for (index, record) in reader.records().enumerate() {
        let record =
            record.map_err(|error| format!("reading golden manifest row failed: {error}"))?;
        let line = index + 2;
        let path = normalized_repo_path(record.get(0).unwrap_or_default())
            .map_err(|error| format!("golden manifest line {line}: {error}"))?;
        if !paths.insert(path.clone()) {
            return Err(format!("golden manifest contains duplicate path {path}"));
        }
        let size = record
            .get(1)
            .unwrap_or_default()
            .parse::<u64>()
            .map_err(|error| format!("golden manifest line {line} has invalid size: {error}"))?;
        let sha256 = record.get(2).unwrap_or_default().to_ascii_lowercase();
        if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
            return Err(format!(
                "golden manifest line {line} has invalid SHA-256 {sha256:?}"
            ));
        }
        let group = if has_group {
            Some(
                BenchmarkGroup::parse(record.get(6).unwrap_or_default())
                    .map_err(|error| format!("golden manifest line {line}: {error}"))?,
            )
        } else {
            None
        };
        rows.push(GoldenInput {
            path,
            size,
            sha256,
            group,
        });
    }
    if rows.is_empty() {
        return Err("golden manifest contains no inputs".to_owned());
    }
    Ok(rows)
}

pub fn select_golden_inputs(
    rows: &[GoldenInput],
    explicit: Option<&Path>,
    corpus: &Path,
) -> Result<Vec<(GoldenInput, PathBuf)>, String> {
    let corpus = fs::canonicalize(corpus).map_err(|error| {
        format!(
            "resolving benchmark corpus {} failed: {error}; initialize and materialize the golden corpus",
            corpus.display()
        )
    })?;
    let Some(explicit) = explicit else {
        return rows
            .iter()
            .map(|row| {
                let path = corpus.join(&row.path);
                let path = fs::canonicalize(&path).map_err(|error| {
                    format!(
                        "golden input {} is not materialized: {error}; materialize the golden corpus",
                        row.path
                    )
                })?;
                Ok((row.clone(), path))
            })
            .collect();
    };
    let explicit = fs::canonicalize(explicit).map_err(|error| {
        format!(
            "resolving requested benchmark input {} failed: {error}",
            explicit.display()
        )
    })?;
    let selected = rows
        .iter()
        .filter_map(|row| {
            let expected = corpus.join(&row.path);
            let matches = expected == explicit
                || fs::canonicalize(&expected).is_ok_and(|path| path == explicit);
            matches.then(|| (row.clone(), explicit.clone()))
        })
        .collect::<Vec<_>>();
    if selected.len() != 1 {
        return Err(format!(
            "requested benchmark input {} is not present in the golden manifest",
            explicit.display()
        ));
    }
    Ok(selected)
}

#[allow(
    dead_code,
    reason = "shared by the decoder benchmark and equivalence test; parser unit tests include only lower-level helpers"
)]
pub fn benchmark_inputs() -> Vec<(GoldenInput, PathBuf)> {
    golden_inputs(DEFAULT_PERF_MANIFEST, true)
}

#[allow(
    dead_code,
    reason = "used by decoder equivalence tests, not the benchmark binary"
)]
pub fn test_inputs() -> Vec<(GoldenInput, PathBuf)> { golden_inputs(DEFAULT_TEST_MANIFEST, false) }

fn golden_inputs(default_manifest: &str, grouped: bool) -> Vec<(GoldenInput, PathBuf)> {
    let repo_root = Path::new(env!("CARGO_MANIFEST_DIR")).join("../..");
    let manifest = std::env::var_os("FORNAX_BENCH_MANIFEST")
        .map_or_else(|| PathBuf::from(default_manifest), PathBuf::from);
    let corpus = std::env::var_os("FORNAX_BENCH_CORPUS")
        .map_or_else(|| PathBuf::from(DEFAULT_CORPUS), PathBuf::from);
    let manifest = resolve_repo_relative(&repo_root, &manifest);
    let corpus = resolve_repo_relative(&repo_root, &corpus);
    let bytes = fs::read(&manifest).unwrap_or_else(|error| {
        panic!(
            "reading decoder benchmark golden manifest {} failed: {error}",
            manifest.display()
        )
    });
    let mut rows = parse_golden_manifest(&bytes).unwrap_or_else(|error| {
        panic!(
            "parsing decoder benchmark golden manifest {} failed: {error}",
            manifest.display()
        )
    });
    if grouped {
        rows = select_benchmark_group(rows)
            .unwrap_or_else(|error| panic!("selecting decoder benchmark group failed: {error}"));
    }
    let explicit = std::env::var_os("FORNAX_BENCH_RAW")
        .map(PathBuf::from)
        .map(|path| resolve_repo_relative(&repo_root, &path));
    let inputs = select_golden_inputs(&rows, explicit.as_deref(), &corpus)
        .unwrap_or_else(|error| panic!("configuring decoder benchmark inputs failed: {error}"));
    ensure_materialized(&inputs)
        .unwrap_or_else(|error| panic!("decoder benchmark input check failed: {error}"));
    inputs
}

fn select_benchmark_group(rows: Vec<GoldenInput>) -> Result<Vec<GoldenInput>, String> {
    let value = std::env::var("FORNAX_BENCH_GROUP").unwrap_or_else(|_| "core".to_owned());
    if value.trim().eq_ignore_ascii_case("all") {
        return Ok(rows);
    }
    let groups = value
        .split(',')
        .map(BenchmarkGroup::parse)
        .collect::<Result<BTreeSet<_>, _>>()?;
    if groups.is_empty() {
        return Err("benchmark group selection is empty".to_owned());
    }
    let selected = rows
        .into_iter()
        .filter(|row| row.group.is_some_and(|group| groups.contains(&group)))
        .collect::<Vec<_>>();
    if selected.is_empty() {
        return Err(format!(
            "benchmark group {value:?} selected no golden inputs"
        ));
    }
    Ok(selected)
}

pub fn ensure_materialized(inputs: &[(GoldenInput, PathBuf)]) -> Result<(), String> {
    for (row, path) in inputs {
        let metadata = fs::metadata(path).map_err(|error| {
            format!("reading benchmark input {} failed: {error}", path.display())
        })?;
        if !metadata.is_file() {
            return Err(format!(
                "benchmark input {} is not a regular file",
                path.display()
            ));
        }
        let mut prefix = vec![0; LFS_POINTER_PREFIX.len()];
        let read = File::open(path)
            .and_then(|mut file| file.read_exact(&mut prefix))
            .map(|()| prefix == LFS_POINTER_PREFIX.as_bytes());
        if read.unwrap_or(false) {
            return Err(format!(
                "benchmark input {} is an unmaterialized LFS pointer",
                path.display()
            ));
        }
        if metadata.len() != row.size {
            return Err(format!(
                "benchmark input {} has {} bytes; golden manifest records {}",
                path.display(),
                metadata.len(),
                row.size
            ));
        }
    }
    Ok(())
}

pub fn resolve_repo_relative(repo_root: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        repo_root.join(path)
    }
}

fn normalized_repo_path(path: &str) -> Result<String, String> {
    let path = path.replace('\\', "/");
    if path.is_empty()
        || path.starts_with('/')
        || path
            .split('/')
            .any(|segment| segment.is_empty() || matches!(segment, "." | ".."))
    {
        return Err(format!("invalid repository path {path:?}"));
    }
    Ok(path)
}
