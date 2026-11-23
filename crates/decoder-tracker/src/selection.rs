use std::ffi::OsString;
use std::fmt;
use std::path::{Path, PathBuf};
use std::process::{Command, Stdio};
use std::str::FromStr;

use crate::csvio::{read_manifest_csv, write_manifest_csv};
use crate::model::{Result, TrackerError};

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Shard {
    pub index: usize,
    pub count: usize,
}

impl FromStr for Shard {
    type Err = TrackerError;

    fn from_str(value: &str) -> Result<Self> {
        let (index, count) = value
            .split_once('/')
            .ok_or_else(|| TrackerError::new("shard must use zero-based INDEX/COUNT syntax"))?;
        let index = index
            .parse::<usize>()
            .map_err(|_| TrackerError::new(format!("invalid shard index: {index:?}")))?;
        let count = count
            .parse::<usize>()
            .map_err(|_| TrackerError::new(format!("invalid shard count: {count:?}")))?;
        if count == 0 || index >= count {
            return Err(TrackerError::new(format!(
                "shard {index}/{count} is outside the valid range"
            )));
        }
        Ok(Self { index, count })
    }
}

impl fmt::Display for Shard {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}", self.index, self.count)
    }
}

#[derive(Clone, Debug, Default)]
pub struct Selection {
    pub cameras: Vec<String>,
    pub filenames: Vec<String>,
    pub globs: Vec<String>,
    pub shard: Option<Shard>,
}

pub fn select_file(manifest: &Path, output: &Path, selection: &Selection) -> Result<usize> {
    if selection.cameras.is_empty() && selection.filenames.is_empty() && selection.globs.is_empty()
    {
        return shard_manifest_file(manifest, output, selection.shard);
    }
    let mut command = Command::new(python_executable());
    command
        .arg(tracker_data_script()?)
        .args(["select", "--manifest"])
        .arg(manifest)
        .arg("--out")
        .arg(output);
    for camera in &selection.cameras {
        command.args(["--camera", camera]);
    }
    for filename in &selection.filenames {
        command.args(["--filename", filename]);
    }
    for glob in &selection.globs {
        command.args(["--glob", glob]);
    }
    if let Some(shard) = selection.shard {
        command.args(["--shard", &shard.to_string()]);
    }
    let result = command
        .stdout(Stdio::null())
        .stderr(Stdio::piped())
        .output()
        .map_err(|error| {
            TrackerError::new(format!(
                "launching Python tracker-data failed: {error}; run through `pixi run -e dev`"
            ))
        })?;
    if !result.status.success() {
        return Err(TrackerError::new(format!(
            "tracker-data select failed: {}",
            String::from_utf8_lossy(&result.stderr).trim()
        )));
    }
    Ok(read_manifest_csv(std::fs::read(output)?)?.len())
}

fn shard_manifest_file(manifest: &Path, output: &Path, shard: Option<Shard>) -> Result<usize> {
    let mut rows = read_manifest_csv(std::fs::read(manifest)?)?;
    rows.sort_by(|left, right| {
        left.sha256
            .cmp(&right.sha256)
            .then_with(|| left.path.cmp(&right.path))
    });
    if let Some(shard) = shard {
        rows = rows
            .into_iter()
            .enumerate()
            .filter(|(position, _)| position % shard.count == shard.index)
            .map(|(_, row)| row)
            .collect();
    }
    if let Some(parent) = output.parent() {
        std::fs::create_dir_all(parent)?;
    }
    std::fs::write(output, write_manifest_csv(&rows)?)?;
    Ok(rows.len())
}

fn python_executable() -> OsString {
    if let Some(python) = std::env::var_os("DECODER_TRACKER_PYTHON") {
        return python;
    }
    for environment in ["tracker", "dev"] {
        let pixi_python = Path::new(env!("CARGO_MANIFEST_DIR"))
            .join(format!("../../.pixi/envs/{environment}/bin/python"));
        if pixi_python.is_file() {
            return pixi_python.into_os_string();
        }
    }
    "python3".into()
}

fn tracker_data_script() -> Result<PathBuf> {
    if let Some(script) = std::env::var_os("DECODER_TRACKER_DATA") {
        return Ok(script.into());
    }
    let script = Path::new(env!("CARGO_MANIFEST_DIR")).join("scripts/tracker_data.py");
    if script.is_file() {
        Ok(script)
    } else {
        Err(TrackerError::new(format!(
            "tracker-data script is missing at {}",
            script.display()
        )))
    }
}
