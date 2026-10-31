use std::path::{Path, PathBuf};
use std::time::Duration;

use serde::{Deserialize, Serialize};

use crate::error::Result;

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Timings {
    pub total_ms: u64,
    pub rows: Vec<RowTiming>,
}

#[derive(Clone, Debug, Deserialize, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RowTiming {
    pub algorithm: String,
    pub pattern: String,
    pub reference: String,
    pub fornax_ms: u64,
    pub reference_ms: u64,
    pub metrics_ms: u64,
    pub artifacts_ms: u64,
    pub peak_rss_mb: u64,
}

impl Timings {
    pub fn write(&self, path: &Path) -> Result<()> {
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let mut bytes = serde_json::to_vec_pretty(self)?;
        bytes.push(b'\n');
        std::fs::write(path, bytes)?;
        Ok(())
    }

    pub fn merge(total: Duration, mut rows: Vec<RowTiming>) -> Self {
        rows.sort_by(|left, right| {
            (
                left.algorithm.to_ascii_lowercase(),
                left.reference.to_ascii_lowercase(),
                left.pattern.as_str(),
            )
                .cmp(&(
                    right.algorithm.to_ascii_lowercase(),
                    right.reference.to_ascii_lowercase(),
                    right.pattern.as_str(),
                ))
        });
        Self {
            total_ms: millis(total),
            rows,
        }
    }
}

pub fn millis(duration: Duration) -> u64 {
    duration.as_millis().try_into().unwrap_or(u64::MAX)
}

pub fn shard_timings_path(shard_path: &Path) -> PathBuf {
    let name = shard_path
        .file_name()
        .and_then(|name| name.to_str())
        .unwrap_or("shard.json")
        .trim_end_matches(".shard.json");
    shard_path.with_file_name(format!("{name}.timings.json"))
}

pub fn peak_rss_mb() -> u64 {
    #[cfg(target_os = "linux")]
    {
        let status = std::fs::read_to_string("/proc/self/status").unwrap_or_default();
        let kilobytes = status
            .lines()
            .find_map(|line| line.strip_prefix("VmHWM:"))
            .and_then(|value| value.split_whitespace().next())
            .and_then(|value| value.parse::<u64>().ok())
            .unwrap_or(0);
        kilobytes / 1024
    }
    #[cfg(not(target_os = "linux"))]
    {
        0
    }
}
