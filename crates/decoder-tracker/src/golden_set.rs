use std::collections::{BTreeMap, BTreeSet};
use std::path::Path;

use csv::{ReaderBuilder, StringRecord};
use serde::Deserialize;

use crate::csvio::{BaselineCsv, read_baseline_csv, read_manifest_csv};
use crate::golden::{GoldenCsv, decode_path, golden_encoding, read_golden_csv};
use crate::manifest::{derive_identity, validate_pointer};
use crate::model::{ManifestRow, Result, TrackerError};

pub const GOLDEN_MANIFEST_HEADER: [&str; 6] = [
    "path",
    "size",
    "sha256",
    "decoder_path",
    "reason",
    "overhead_only",
];
pub const GOLDEN_PERF_MANIFEST_HEADER: [&str; 7] = [
    "path",
    "size",
    "sha256",
    "decoder_path",
    "reason",
    "overhead_only",
    "group",
];

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum GoldenGroup {
    Core,
    Full,
    Overhead,
    Hdr,
}

impl GoldenGroup {
    pub const fn as_str(self) -> &'static str {
        match self {
            Self::Core => "core",
            Self::Full => "full",
            Self::Overhead => "overhead",
            Self::Hdr => "hdr",
        }
    }

    fn parse(value: &str) -> Result<Self> {
        match value.trim().to_ascii_lowercase().as_str() {
            "core" => Ok(Self::Core),
            "full" => Ok(Self::Full),
            "overhead" => Ok(Self::Overhead),
            "hdr" => Ok(Self::Hdr),
            _ => Err(TrackerError::new(format!(
                "invalid golden manifest group {value:?}; expected core, full, overhead, or hdr"
            ))),
        }
    }
}

#[derive(Debug, Deserialize)]
struct GoldenManifestCsvRow {
    path: String,
    size: u64,
    sha256: String,
    decoder_path: String,
    reason: String,
    overhead_only: String,
    group: Option<String>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoldenManifestEntry {
    pub manifest: ManifestRow,
    pub decoder_path: String,
    pub reason: String,
    pub overhead_only: bool,
    pub group: Option<GoldenGroup>,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct GoldenCheckOptions {
    pub allow_uncovered: bool,
}

#[derive(Clone, Debug, Default, Eq, PartialEq)]
pub struct GoldenCheckReport {
    pub test_count: usize,
    pub perf_count: usize,
    pub union_count: usize,
    pub path_count: usize,
    pub perf_overhead_count: usize,
    pub large_perf_count: usize,
    pub largest_perf_bytes: u64,
    pub core_count: usize,
    pub core_bytes: u64,
    pub core_estimated_probe_seconds: u64,
    pub warnings: Vec<String>,
}

pub struct GoldenInputs {
    pub baseline: BaselineCsv,
    pub corpus_manifest: Vec<ManifestRow>,
    pub test: Vec<GoldenManifestEntry>,
    pub perf: Vec<GoldenManifestEntry>,
    pub hashes: GoldenCsv,
}

pub fn read_golden_manifest_csv(input: &[u8]) -> Result<Vec<GoldenManifestEntry>> {
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(input);
    let has_group = validate_header(reader.headers()?)?;
    let rows = reader
        .deserialize::<GoldenManifestCsvRow>()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut paths = BTreeSet::new();
    rows.into_iter()
        .map(|row| {
            if !paths.insert(row.path.clone()) {
                return Err(TrackerError::new(format!(
                    "golden manifest contains duplicate path {}",
                    row.path
                )));
            }
            if row.reason.trim().is_empty() {
                return Err(TrackerError::new(format!(
                    "golden path {} has an empty reason",
                    row.path
                )));
            }
            let group = if has_group {
                Some(GoldenGroup::parse(row.group.as_deref().ok_or_else(
                    || TrackerError::new("golden group is missing"),
                )?)?)
            } else {
                None
            };
            let (vendor, camera, model, filename) = derive_identity(&row.path)?;
            let format = row
                .path
                .rsplit_once('.')
                .map_or_else(String::new, |(_, extension)| extension.to_ascii_uppercase());
            Ok(GoldenManifestEntry {
                manifest: ManifestRow {
                    sha256: row.sha256,
                    size: row.size,
                    path: row.path,
                    vendor,
                    camera,
                    model,
                    format,
                    filename,
                },
                decoder_path: row.decoder_path,
                reason: row.reason,
                overhead_only: parse_bool(&row.overhead_only)?,
                group,
            })
        })
        .collect()
}

pub fn check_golden_sets(
    corpus: &Path,
    baseline: &BaselineCsv,
    corpus_manifest: &[ManifestRow],
    test: &[GoldenManifestEntry],
    perf: &[GoldenManifestEntry],
    hashes: &GoldenCsv,
    options: GoldenCheckOptions,
) -> Result<GoldenCheckReport> {
    let baseline_by_path = baseline
        .rows
        .iter()
        .map(|row| (row.path.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let manifest_by_path = corpus_manifest
        .iter()
        .map(|row| (row.path.as_str(), row))
        .collect::<BTreeMap<_, _>>();
    let hash_paths = hashes
        .rows
        .iter()
        .map(|row| row.path.as_str())
        .collect::<BTreeSet<_>>();
    let mut failures = Vec::new();
    let mut union = BTreeSet::new();

    for (entries, is_perf) in [(test, false), (perf, true)] {
        for entry in entries {
            let path = entry.manifest.path.as_str();
            union.insert(path);
            let Some(corpus_row) = manifest_by_path.get(path) else {
                failures.push(format!("{path} is absent from corpus manifest"));
                continue;
            };
            if corpus_row.size != entry.manifest.size || corpus_row.sha256 != entry.manifest.sha256
            {
                failures.push(format!("{path} does not match corpus pointer metadata"));
            }
            let Some(baseline_row) = baseline_by_path.get(path) else {
                failures.push(format!("{path} is absent from baseline"));
                continue;
            };
            if entry.decoder_path != decode_path(baseline_row) {
                failures.push(format!(
                    "{path} decoder path {} does not match {}",
                    entry.decoder_path,
                    decode_path(baseline_row)
                ));
            }
            if !golden_encoding(baseline_row) {
                failures.push(format!("{path} is not a fully passing lossless file"));
            }
            if !hash_paths.contains(path) {
                failures.push(format!("{path} has no decode hash"));
            }
            if is_perf {
                let Some(group) = entry.group else {
                    failures.push(format!("{path} has no performance group"));
                    continue;
                };
                if entry.overhead_only != (group == GoldenGroup::Overhead) {
                    failures.push(format!(
                        "{path} overhead_only={} disagrees with group {}",
                        entry.overhead_only,
                        group.as_str()
                    ));
                }
            }
            if let Err(error) = validate_pointer(corpus, &entry.manifest) {
                failures.push(format!("{path}: {error}"));
            }
        }
    }

    let mut path_counts = BTreeMap::<String, usize>::new();
    for row in baseline.rows.iter().filter(|row| golden_encoding(row)) {
        *path_counts.entry(decode_path(row)).or_default() += 1;
    }
    let test_paths = test
        .iter()
        .map(|entry| entry.decoder_path.as_str())
        .collect::<BTreeSet<_>>();
    let perf_paths = perf
        .iter()
        .filter(|entry| !entry.overhead_only)
        .map(|entry| entry.decoder_path.as_str())
        .collect::<BTreeSet<_>>();
    let core_paths = perf
        .iter()
        .filter(|entry| entry.group == Some(GoldenGroup::Core))
        .map(|entry| entry.decoder_path.as_str())
        .collect::<BTreeSet<_>>();
    let special_paths = perf
        .iter()
        .filter(|entry| {
            entry.overhead_only
                || matches!(entry.group, Some(GoldenGroup::Overhead | GoldenGroup::Hdr))
        })
        .map(|entry| entry.decoder_path.as_str())
        .collect::<BTreeSet<_>>();
    let mut warnings = Vec::new();
    for path in path_counts.keys() {
        if !test_paths.contains(path.as_str()) {
            warnings.push(format!("test manifest does not cover decoder path {path}"));
        }
        if !perf_paths.contains(path.as_str()) {
            warnings.push(format!(
                "perf manifest has no non-overhead file for decoder path {path}"
            ));
        } else if !core_paths.contains(path.as_str()) && !special_paths.contains(path.as_str()) {
            warnings.push(format!(
                "perf manifest has no core, hdr, or overhead file for decoder path {path}"
            ));
        }
    }
    if !warnings.is_empty() && !options.allow_uncovered {
        failures.extend(warnings.iter().cloned());
    }
    if !failures.is_empty() {
        return Err(TrackerError::new(format!(
            "golden check failed:\n{}",
            failures
                .iter()
                .map(|failure| format!("- {failure}"))
                .collect::<Vec<_>>()
                .join("\n")
        )));
    }

    let largest_perf_bytes = perf
        .iter()
        .map(|entry| entry.manifest.size)
        .max()
        .unwrap_or(0);
    let core_count = perf
        .iter()
        .filter(|entry| entry.group == Some(GoldenGroup::Core))
        .count();
    let core_bytes: u64 = perf
        .iter()
        .filter(|entry| entry.group == Some(GoldenGroup::Core))
        .map(|entry| entry.manifest.size)
        .sum();
    const ESTIMATED_PROBE_BYTES_PER_SECOND: u64 = 64 * 1024 * 1024;
    const PROBE_DECODES_PER_FILE: u64 = 21;
    let core_estimated_probe_seconds =
        core_bytes.saturating_mul(PROBE_DECODES_PER_FILE) / ESTIMATED_PROBE_BYTES_PER_SECOND;
    Ok(GoldenCheckReport {
        test_count: test.len(),
        perf_count: perf.len(),
        union_count: union.len(),
        path_count: path_counts.len(),
        perf_overhead_count: perf.iter().filter(|entry| entry.overhead_only).count(),
        large_perf_count: perf
            .iter()
            .filter(|entry| !entry.overhead_only && entry.manifest.size >= 20_000_000)
            .count(),
        largest_perf_bytes,
        core_count,
        core_bytes,
        core_estimated_probe_seconds,
        warnings: if options.allow_uncovered {
            warnings
        } else {
            Vec::new()
        },
    })
}

pub fn read_golden_inputs(
    baseline_path: &Path,
    manifest_path: &Path,
    test_path: &Path,
    perf_path: &Path,
    hash_path: &Path,
) -> Result<GoldenInputs> {
    Ok(GoldenInputs {
        baseline: read_baseline_csv(std::fs::read(baseline_path)?)?,
        corpus_manifest: read_manifest_csv(std::fs::read(manifest_path)?)?,
        test: read_golden_manifest_csv(&std::fs::read(test_path)?)?,
        perf: read_golden_manifest_csv(&std::fs::read(perf_path)?)?,
        hashes: read_golden_csv(std::fs::read(hash_path)?)?,
    })
}

fn parse_bool(value: &str) -> Result<bool> {
    match value.trim().to_ascii_lowercase().as_str() {
        "" | "0" | "false" | "no" => Ok(false),
        "1" | "true" | "yes" => Ok(true),
        _ => Err(TrackerError::new(format!(
            "invalid overhead_only value {value:?}"
        ))),
    }
}

fn validate_header(actual: &StringRecord) -> Result<bool> {
    let matches = |expected: &[&str]| {
        actual.len() == expected.len()
            && actual
                .iter()
                .zip(expected)
                .all(|(actual, expected)| actual == *expected)
    };
    if matches(&GOLDEN_MANIFEST_HEADER) {
        return Ok(false);
    }
    if matches(&GOLDEN_PERF_MANIFEST_HEADER) {
        return Ok(true);
    }
    Err(TrackerError::new(
        "golden manifest header must be path,size,sha256,decoder_path,reason,overhead_only \
         or append group",
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_extended_manifest_metadata() {
        let csv = b"path,size,sha256,decoder_path,reason,overhead_only\n\
Canon/Cam/a.dng,42,aa,dng.test,representative,0\n";
        let rows = read_golden_manifest_csv(csv).unwrap();
        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].decoder_path, "dng.test");
        assert!(!rows[0].overhead_only);
        assert_eq!(rows[0].group, None);
    }

    #[test]
    fn parses_performance_group() {
        let csv = b"path,size,sha256,decoder_path,reason,overhead_only,group\n\
Canon/Cam/a.dng,42,aa,dng.test,representative,0,core\n";
        let rows = read_golden_manifest_csv(csv).unwrap();
        assert_eq!(rows[0].group, Some(GoldenGroup::Core));
    }

    #[test]
    fn rejects_duplicate_paths() {
        let csv = b"path,size,sha256,decoder_path,reason,overhead_only\n\
Canon/Cam/a.dng,42,aa,dng.test,first,0\n\
Canon/Cam/a.dng,42,aa,dng.test,second,1\n";
        assert!(read_golden_manifest_csv(csv).is_err());
    }
}
