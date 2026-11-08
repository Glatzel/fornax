use std::collections::{BTreeMap, BTreeSet};

use csv::{ReaderBuilder, StringRecord, Terminator, WriterBuilder};

use crate::model::{
    BASELINE_HEADER, BaselineKey, BaselineRow, MANIFEST_HEADER, ManifestCsvRecord, ManifestRow,
    Result, TrackerError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BaselineCsv {
    pub rows: Vec<BaselineRow>,
}

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct DiffReport {
    pub missing: Vec<BaselineKey>,
    pub extra: Vec<BaselineKey>,
    pub changed: Vec<(BaselineRow, BaselineRow)>,
}

impl DiffReport {
    pub fn between(baseline: &[BaselineRow], candidate: &[BaselineRow]) -> Result<Self> {
        let baseline = index_rows(baseline)?;
        let candidate = index_rows(candidate)?;
        let missing = baseline
            .keys()
            .filter(|key| !candidate.contains_key(*key))
            .cloned()
            .collect();
        let extra = candidate
            .keys()
            .filter(|key| !baseline.contains_key(*key))
            .cloned()
            .collect();
        let changed = baseline
            .iter()
            .filter_map(|(key, before)| {
                candidate
                    .get(key)
                    .filter(|after| *after != before)
                    .map(|after| (before.clone(), after.clone()))
            })
            .collect();
        Ok(Self {
            missing,
            extra,
            changed,
        })
    }

    pub fn is_empty(&self) -> bool {
        self.missing.is_empty() && self.extra.is_empty() && self.changed.is_empty()
    }
}

pub fn write_manifest_csv(rows: &[ManifestRow]) -> Result<Vec<u8>> {
    crate::manifest::validate_manifest_keys(rows)?;
    let mut rows = rows.to_vec();
    rows.sort_by(|left, right| {
        left.sha256
            .cmp(&right.sha256)
            .then_with(|| left.path.cmp(&right.path))
    });
    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .terminator(Terminator::Any(b'\n'))
        .from_writer(Vec::new());
    writer.write_record(MANIFEST_HEADER)?;
    for row in rows {
        writer.serialize(ManifestCsvRecord {
            vendor: row.vendor.clone(),
            camera: row.camera.clone(),
            format: row.format.clone(),
            path: row.path.clone(),
            size: row.size,
            sha256: row.sha256.clone(),
        })?;
    }
    Ok(writer.into_inner().map_err(|error| error.into_error())?)
}

pub fn read_manifest_csv(input: impl AsRef<[u8]>) -> Result<Vec<ManifestRow>> {
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(input.as_ref());
    validate_header(reader.headers()?, &MANIFEST_HEADER, "manifest")?;
    let rows = reader
        .deserialize::<ManifestCsvRecord>()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let rows = rows
        .into_iter()
        .map(|row| {
            let (vendor, camera, model, filename) = crate::manifest::derive_identity(&row.path)?;
            if (row.vendor.as_str(), row.camera.as_str()) != (vendor.as_str(), camera.as_str()) {
                return Err(TrackerError::new(format!(
                    "manifest identity columns disagree with path {:?}",
                    row.path
                )));
            }
            Ok(ManifestRow {
                sha256: row.sha256,
                size: row.size,
                path: row.path,
                vendor,
                camera,
                model,
                format: row.format,
                filename,
            })
        })
        .collect::<Result<Vec<_>>>()?;
    crate::manifest::validate_manifest_keys(&rows)?;
    Ok(rows)
}

pub fn write_baseline_csv(rows: &[BaselineRow]) -> Result<Vec<u8>> {
    index_rows(rows)?;
    let mut rows = rows.to_vec();
    rows.sort_by_key(BaselineRow::key);
    let mut writer = WriterBuilder::new()
        .has_headers(false)
        .terminator(Terminator::Any(b'\n'))
        .from_writer(Vec::new());
    writer.write_record(BASELINE_HEADER)?;
    for row in rows {
        writer.serialize(row)?;
    }
    Ok(writer.into_inner().map_err(|error| error.into_error())?)
}

pub fn read_baseline_csv(input: impl AsRef<[u8]>) -> Result<BaselineCsv> {
    let mut reader = ReaderBuilder::new()
        .has_headers(true)
        .flexible(false)
        .from_reader(input.as_ref());
    validate_header(reader.headers()?, &BASELINE_HEADER, "baseline")?;
    let rows = reader
        .deserialize::<BaselineRow>()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let index = index_rows(&rows)?;
    if let Some(key) = index.keys().next() {
        let _ = key;
    }
    Ok(BaselineCsv { rows })
}

pub fn merge_baseline_rows<I, J>(
    shards: I,
    expected: &BTreeSet<BaselineKey>,
) -> Result<Vec<BaselineRow>>
where
    I: IntoIterator<Item = J>,
    J: IntoIterator<Item = BaselineRow>,
{
    let mut merged = BTreeMap::new();
    for shard in shards {
        for row in shard {
            row.validate()?;
            let key = row.key();
            if !expected.contains(&key) {
                return Err(TrackerError::new(format!(
                    "merged baseline contains unexpected row {key}"
                )));
            }
            if merged.insert(key.clone(), row).is_some() {
                return Err(TrackerError::new(format!(
                    "merged baseline contains duplicate row {key}"
                )));
            }
        }
    }
    let actual = merged.keys().cloned().collect::<BTreeSet<_>>();
    if &actual != expected {
        let missing = expected
            .difference(&actual)
            .map(ToString::to_string)
            .collect::<Vec<_>>();
        return Err(TrackerError::new(format!(
            "merged baseline is missing {} rows: {}",
            missing.len(),
            missing.join(", ")
        )));
    }
    Ok(merged.into_values().collect())
}

fn index_rows(rows: &[BaselineRow]) -> Result<BTreeMap<BaselineKey, BaselineRow>> {
    let mut index = BTreeMap::new();
    for row in rows {
        row.validate()?;
        let key = row.key();
        if index.insert(key.clone(), row.clone()).is_some() {
            return Err(TrackerError::new(format!(
                "baseline contains duplicate row {key}"
            )));
        }
    }
    Ok(index)
}

fn validate_header(actual: &StringRecord, expected: &[&str], description: &str) -> Result<()> {
    if actual.len() != expected.len()
        || !actual
            .iter()
            .zip(expected)
            .all(|(actual, expected)| actual == *expected)
    {
        return Err(TrackerError::new(format!(
            "{description} CSV header does not match the canonical schema"
        )));
    }
    Ok(())
}
