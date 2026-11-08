use std::fmt;
use std::str::FromStr;

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
