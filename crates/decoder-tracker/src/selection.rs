use std::fmt;
use std::str::FromStr;

use globset::{GlobBuilder, GlobSet, GlobSetBuilder};

use crate::manifest::{selected_formats, validate_manifest_keys};
use crate::model::{ManifestRow, Result, TrackerError};

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
    pub formats: Vec<String>,
    pub cameras: Vec<String>,
    pub filenames: Vec<String>,
    pub globs: Vec<String>,
    pub shard: Option<Shard>,
}

pub fn select_manifest(rows: &[ManifestRow], selection: &Selection) -> Result<Vec<ManifestRow>> {
    validate_manifest_keys(rows)?;
    let formats = selected_formats(&selection.formats);
    let cameras = selection
        .cameras
        .iter()
        .map(|value| value.to_lowercase())
        .collect::<Vec<_>>();
    let globs = build_globs(&selection.globs)?;
    let mut selected = rows
        .iter()
        .filter(|row| formats.contains(&row.format.to_ascii_uppercase()))
        .filter(|row| {
            cameras.is_empty()
                || cameras
                    .iter()
                    .any(|camera| row.camera.to_lowercase().contains(camera))
        })
        .filter(|row| {
            selection.filenames.is_empty()
                || selection
                    .filenames
                    .iter()
                    .any(|filename| row.filename.eq_ignore_ascii_case(filename))
        })
        .filter(|row| globs.as_ref().is_none_or(|globs| globs.is_match(&row.path)))
        .cloned()
        .collect::<Vec<_>>();
    selected.sort_by(|left, right| {
        left.sha256
            .cmp(&right.sha256)
            .then_with(|| left.path.cmp(&right.path))
    });
    if let Some(shard) = selection.shard {
        selected = selected
            .into_iter()
            .enumerate()
            .filter(|(position, _)| position % shard.count == shard.index)
            .map(|(_, row)| row)
            .collect();
    }
    Ok(selected)
}

pub const fn runner_count_for(file_count: usize) -> usize {
    let count = file_count.div_ceil(10);
    if count < 1 {
        1
    } else if count > 20 {
        20
    } else {
        count
    }
}

fn build_globs(patterns: &[String]) -> Result<Option<GlobSet>> {
    if patterns.is_empty() {
        return Ok(None);
    }
    let mut builder = GlobSetBuilder::new();
    for pattern in patterns {
        let glob = GlobBuilder::new(pattern)
            .literal_separator(true)
            .backslash_escape(true)
            .build()
            .map_err(|error| {
                TrackerError::new(format!("invalid path glob {pattern:?}: {error}"))
            })?;
        builder.add(glob);
    }
    builder
        .build()
        .map(Some)
        .map_err(|error| TrackerError::new(format!("building path globs failed: {error}")))
}
