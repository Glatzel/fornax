use std::collections::BTreeMap;

use csv::{ReaderBuilder, StringRecord, Terminator, WriterBuilder};

use crate::model::{
    BASELINE_HEADER, BaselineKey, BaselineRow, MANIFEST_HEADER, ManifestCsvRecord, ManifestRow,
    Result, TrackerError,
};

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct BaselineCsv {
    pub rows: Vec<BaselineRow>,
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
    index_rows(&rows)?;
    Ok(BaselineCsv { rows })
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
