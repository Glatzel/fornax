use std::fs;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};

use serde::Deserialize;

use crate::csvio::{read_baseline_csv, read_manifest_csv};
use crate::model::{Result, TrackerError};

const MERGE_SQL: &str = include_str!("../sql/merge.sql");
const DIFF_SQL: &str = include_str!("../sql/diff.sql");
const UPSERT_SQL: &str = include_str!("../sql/upsert.sql");

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct DiffSummary {
    pub missing: usize,
    pub extra: usize,
    pub changed: usize,
}

impl DiffSummary {
    pub const fn is_empty(self) -> bool {
        self.missing == 0 && self.extra == 0 && self.changed == 0
    }
}

#[derive(Deserialize)]
struct MergeStats {
    duplicates: usize,
    missing: usize,
    unexpected: usize,
    rows_merged: usize,
}

#[derive(Deserialize)]
struct RawDiffSummary {
    missing: i64,
    extra: i64,
    changed: i64,
}

#[derive(Deserialize)]
struct UpsertStats {
    rows_merged: usize,
}

pub fn merge(selected: &Path, shards: &[PathBuf], output: &Path) -> Result<usize> {
    ensure_parent(output)?;
    read_manifest_csv(fs::read(selected)?)?;
    for shard in shards {
        read_baseline_csv(fs::read(shard)?)?;
    }
    let sql = MERGE_SQL
        .replace("@MANIFEST@", &sql_string(selected))
        .replace("@SHARDS@", &sql_list(shards))
        .replace("@OUTPUT@", &sql_string(output));
    let stats = run_json::<MergeStats>(&sql)?;
    if stats.duplicates > 0 {
        return Err(TrackerError::new(format!(
            "merged shards contain {} duplicate rows",
            stats.duplicates
        )));
    }
    if stats.missing > 0 || stats.unexpected > 0 {
        return Err(TrackerError::new(format!(
            "merged shards are incomplete: {} missing and {} unexpected rows",
            stats.missing, stats.unexpected
        )));
    }
    read_baseline_csv(fs::read(output)?)?;
    Ok(stats.rows_merged)
}

pub fn diff(
    baseline: &Path,
    candidate: &Path,
    selected: &Path,
    output: &Path,
) -> Result<DiffSummary> {
    ensure_parent(output)?;
    read_baseline_csv(fs::read(baseline)?)?;
    read_baseline_csv(fs::read(candidate)?)?;
    read_manifest_csv(fs::read(selected)?)?;
    let sql = DIFF_SQL
        .replace("@BASELINE@", &sql_string(baseline))
        .replace("@CANDIDATE@", &sql_string(candidate))
        .replace("@SELECTED@", &sql_string(selected))
        .replace("@OUTPUT@", &sql_string(output));
    let raw = run_json::<RawDiffSummary>(&sql)?;
    Ok(DiffSummary {
        missing: usize::try_from(raw.missing)
            .map_err(|_| TrackerError::new("diff returned a negative missing count"))?,
        extra: usize::try_from(raw.extra)
            .map_err(|_| TrackerError::new("diff returned a negative extra count"))?,
        changed: usize::try_from(raw.changed)
            .map_err(|_| TrackerError::new("diff returned a negative changed count"))?,
    })
}

pub fn upsert(baseline: Option<&Path>, candidate: &Path, output: &Path) -> Result<usize> {
    ensure_parent(output)?;
    read_baseline_csv(fs::read(candidate)?)?;
    let Some(baseline) = baseline else {
        fs::copy(candidate, output)?;
        return read_baseline_csv(fs::read(candidate)?).map(|csv| csv.rows.len());
    };
    read_baseline_csv(fs::read(baseline)?)?;
    let sql = UPSERT_SQL
        .replace("@BASELINE@", &sql_string(baseline))
        .replace("@CANDIDATE@", &sql_string(candidate))
        .replace("@OUTPUT@", &sql_string(output));
    let stats = run_json::<UpsertStats>(&sql)?;
    read_baseline_csv(fs::read(output)?)?;
    Ok(stats.rows_merged)
}

fn run_json<T>(sql: &str) -> Result<T>
where
    T: for<'de> Deserialize<'de>,
{
    let executable = std::env::var_os("DUCKDB").unwrap_or_else(|| "duckdb".into());
    let mut child = Command::new(executable)
        .args(["-no-agent", "-json", ":memory:"])
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(|error| {
            TrackerError::new(format!(
                "launching DuckDB failed: {error}; run through `pixi run -e dev`"
            ))
        })?;
    child
        .stdin
        .take()
        .ok_or_else(|| TrackerError::new("DuckDB stdin was unavailable"))?
        .write_all(sql.as_bytes())?;
    let output = child.wait_with_output()?;
    if !output.status.success() {
        return Err(TrackerError::new(format!(
            "DuckDB failed: {}",
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let values = serde_json::from_slice::<Vec<T>>(&output.stdout)?;
    values
        .into_iter()
        .next()
        .ok_or_else(|| TrackerError::new("DuckDB returned no result row"))
}

fn ensure_parent(path: &Path) -> Result<()> {
    if let Some(parent) = path.parent()
        && !parent.as_os_str().is_empty()
    {
        fs::create_dir_all(parent)?;
    }
    Ok(())
}

fn sql_string(path: &Path) -> String {
    path.to_string_lossy()
        .replace('\\', "/")
        .replace('\'', "''")
}

fn sql_list(paths: &[PathBuf]) -> String {
    paths
        .iter()
        .map(|path| format!("'{}'", sql_string(path)))
        .collect::<Vec<_>>()
        .join(", ")
}
