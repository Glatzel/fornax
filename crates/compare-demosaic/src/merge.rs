use std::collections::BTreeMap;
use std::path::{Path, PathBuf};
use std::time::Duration;

use crate::error::{CompareError, Result};
use crate::results::{CanonicalResults, ComparisonResult, Coverage, RunMeta, RunMetadata};
use crate::timings::{self, RowTiming, Timings};

#[derive(Debug)]
pub struct MergeReport {
    pub path: PathBuf,
    pub rows: usize,
    pub complete: bool,
}

pub fn merge(
    shard_paths: &[PathBuf],
    output: &Path,
    allow_partial: bool,
    check_artifacts: bool,
) -> Result<MergeReport> {
    if shard_paths.is_empty() {
        return Err(CompareError::invalid("merge requires at least one shard"));
    }

    let shards = shard_paths
        .iter()
        .map(|path| {
            CanonicalResults::read(path)
                .map(|results| (path, results))
                .map_err(|error| CompareError::invalid(error.to_string()))
        })
        .collect::<Result<Vec<_>>>()?;

    let (_, first) = &shards[0];
    for (path, shard) in &shards {
        if shard.schema_version != first.schema_version {
            return Err(CompareError::invalid(format!(
                "{} has schema version {}, expected {}",
                path.display(),
                shard.schema_version,
                first.schema_version
            )));
        }
        if shard.meta.stable != first.meta.stable {
            return Err(CompareError::invalid(format!(
                "{} has different meta.stable from {}; shards must use the same input, build, and limits",
                path.display(),
                shard_paths[0].display()
            )));
        }
        if shard.meta.run.coverage != Coverage::Partial {
            return Err(CompareError::invalid(format!(
                "{} has coverage `{:?}`; merge inputs must be partial shards",
                path.display(),
                shard.meta.run.coverage
            )));
        }
    }

    let mut rows = BTreeMap::<String, ComparisonResult>::new();
    let mut shard_metadata = Vec::new();
    for (path, shard) in &shards {
        shard_metadata.extend(shard.meta.run.shards.iter().cloned());
        for row in &shard.results {
            let key = crate::validate::row_key(row)
                .map_err(|error| CompareError::invalid(format!("{}: {error}", path.display())))?;
            if let Some(existing) = rows.insert(key.clone(), row.clone()) {
                return Err(CompareError::invalid(format!(
                    "duplicate row {key} in {} and another shard ({})",
                    path.display(),
                    existing.algorithm
                )));
            }
        }
    }

    let expected = crate::registry::expected_rows()
        .into_iter()
        .map(|row| format!("{} {} {}", row.algorithm, row.pattern, row.reference))
        .collect::<std::collections::BTreeSet<_>>();
    let actual = rows
        .keys()
        .cloned()
        .collect::<std::collections::BTreeSet<_>>();
    let missing = expected.difference(&actual).cloned().collect::<Vec<_>>();
    let unexpected = actual.difference(&expected).cloned().collect::<Vec<_>>();
    if !missing.is_empty() && !allow_partial {
        return Err(CompareError::invalid(format!(
            "shards do not cover the full matrix; missing [{}]",
            missing.join(", ")
        )));
    }
    if !unexpected.is_empty() {
        return Err(CompareError::invalid(format!(
            "shards contain rows outside the expected matrix: [{}]",
            unexpected.join(", ")
        )));
    }

    let mut results = rows.into_values().collect::<Vec<_>>();
    results.sort_unstable_by(crate::results::comparison_order);
    let complete = missing.is_empty();
    let merged = CanonicalResults {
        schema_version: first.schema_version,
        meta: RunMeta {
            stable: first.meta.stable.clone(),
            run: RunMetadata {
                coverage: if complete {
                    Coverage::Full
                } else {
                    Coverage::Partial
                },
                shards: shard_metadata,
                generated_by: format!("{} merge", crate::results::generated_by()),
            },
        },
        results,
    };
    crate::validate::validate_results(&merged, !complete, check_artifacts, output.parent())?;
    merged
        .write(output)
        .map_err(|error| CompareError::usage(error.to_string()))?;

    let mut timings_rows = Vec::<RowTiming>::new();
    let mut timing_total = Duration::ZERO;
    for shard_path in shard_paths {
        let timing_path = timings::shard_timings_path(shard_path);
        if !timing_path.is_file() {
            continue;
        }
        let shard_timings = std::fs::read(&timing_path)
            .ok()
            .and_then(|bytes| serde_json::from_slice::<Timings>(&bytes).ok());
        if let Some(timings) = shard_timings {
            timing_total += Duration::from_millis(timings.total_ms);
            timings_rows.extend(timings.rows);
        }
    }
    let merged_timings = Timings::merge(timing_total, timings_rows);
    let timings_path = output.with_file_name("timings.json");
    merged_timings.write(&timings_path)?;

    Ok(MergeReport {
        path: output.to_path_buf(),
        rows: merged.results.len(),
        complete,
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn merge_requires_an_input() {
        let error = merge(&[], Path::new("canonical.json"), false, false).unwrap_err();
        assert_eq!(error.class().exit_code(), 2);
    }
}
