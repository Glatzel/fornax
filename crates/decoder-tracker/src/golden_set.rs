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

#[derive(Debug, Deserialize)]
struct GoldenManifestCsvRow {
    path: String,
    size: u64,
    sha256: String,
    decoder_path: String,
    reason: String,
    overhead_only: String,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct GoldenManifestEntry {
    pub manifest: ManifestRow,
    pub decoder_path: String,
    pub reason: String,
    pub overhead_only: bool,
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
    validate_header(reader.headers()?)?;
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

    for entries in [test, perf] {
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
    let mut warnings = Vec::new();
    for path in path_counts.keys() {
        if !test_paths.contains(path.as_str()) {
            warnings.push(format!("test manifest does not cover decoder path {path}"));
        }
        if !perf_paths.contains(path.as_str()) {
            warnings.push(format!(
                "perf manifest has no non-overhead file for decoder path {path}"
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

fn validate_header(actual: &StringRecord) -> Result<()> {
    if actual.len() != GOLDEN_MANIFEST_HEADER.len()
        || !actual
            .iter()
            .zip(GOLDEN_MANIFEST_HEADER)
            .all(|(actual, expected)| actual == expected)
    {
        return Err(TrackerError::new(
            "golden manifest header does not match path,size,sha256,decoder_path,reason,overhead_only",
        ));
    }
    Ok(())
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
    }

    #[test]
    fn rejects_duplicate_paths() {
        let csv = b"path,size,sha256,decoder_path,reason,overhead_only\n\
Canon/Cam/a.dng,42,aa,dng.test,first,0\n\
Canon/Cam/a.dng,42,aa,dng.test,second,1\n";
        assert!(read_golden_manifest_csv(csv).is_err());
    }
}
