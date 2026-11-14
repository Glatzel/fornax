//! DNG compression and storage classification for the tracked RAW corpus.

use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::Path;

use fornax_dng::Directory;
use fornax_dng::dng_decoder::DngImage;
use fornax_tiff::constants::TiffTag;
use fornax_tiff::{Tag, unsigned_scalar};
use serde::{Deserialize, Serialize};

use crate::manifest::corpus_path;
use crate::model::{ManifestRow, Result, TrackerError};

pub const CORPUS_METADATA_HEADER: [&str; 14] = [
    "status",
    "path",
    "sha256",
    "format",
    "compression_id",
    "compression_name",
    "storage",
    "bits_per_sample",
    "sample_format",
    "predictor",
    "rows_per_strip",
    "tile_width",
    "tile_height",
    "error_kind",
];

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct CorpusMetadataRow {
    pub status: String,
    pub path: String,
    pub sha256: String,
    pub format: String,
    #[serde(default)]
    pub compression_id: Option<u16>,
    pub compression_name: String,
    pub storage: String,
    #[serde(default)]
    pub bits_per_sample: Option<u16>,
    #[serde(default)]
    pub sample_format: Option<u16>,
    #[serde(default)]
    pub predictor: Option<u16>,
    #[serde(default)]
    pub rows_per_strip: Option<u32>,
    #[serde(default)]
    pub tile_width: Option<u32>,
    #[serde(default)]
    pub tile_height: Option<u32>,
    pub error_kind: String,
}

pub fn classify_rows(rows: &[ManifestRow], corpus: &Path) -> Result<Vec<CorpusMetadataRow>> {
    let mut output = Vec::new();
    for row in rows.iter().filter(|row| row.format == "DNG") {
        let path = fs::canonicalize(corpus_path(corpus, &row.path)).map_err(|error| {
            TrackerError::new(format!(
                "resolving corpus file {} failed: {error}",
                row.path
            ))
        })?;
        let source = fs::read(&path).map_err(|error| {
            TrackerError::new(format!("reading corpus file {} failed: {error}", row.path))
        })?;
        let image = match DngImage::parse(&source) {
            Ok(image) => image,
            Err(error) => {
                output.push(CorpusMetadataRow::failed(
                    row,
                    decode_error_kind("metadata", &error),
                ));
                continue;
            }
        };
        let metadata = match image.dng_metadata() {
            Ok(metadata) => metadata,
            Err(error) => {
                output.push(CorpusMetadataRow::failed(
                    row,
                    decode_error_kind("metadata", &error),
                ));
                continue;
            }
        };
        match classify_dng(row, &metadata) {
            Ok(classified) => output.push(classified),
            Err(_) => output.push(CorpusMetadataRow::failed(
                row,
                "metadata:invalid_input".to_owned(),
            )),
        }
    }
    output.sort_by(|left, right| (&left.sha256, &left.path).cmp(&(&right.sha256, &right.path)));
    Ok(output)
}

pub fn classify_dng(row: &ManifestRow, metadata: &Directory) -> Result<CorpusMetadataRow> {
    let compression_id = required_u16(metadata, TiffTag::Compression)?;
    let bits_per_sample = required_u16(metadata, TiffTag::BitsPerSample)?;
    let sample_format = optional_u16(metadata, TiffTag::SampleFormat).unwrap_or(1);
    let predictor = optional_u16(metadata, TiffTag::Predictor).unwrap_or(1);
    let tiles = metadata.contains_key(&Tag::from(TiffTag::TileOffsets));
    Ok(CorpusMetadataRow {
        status: "✅".to_owned(),
        path: row.path.clone(),
        sha256: row.sha256.clone(),
        format: row.format.clone(),
        compression_id: Some(compression_id),
        compression_name: compression_name(compression_id).to_owned(),
        storage: (if tiles { "tiles" } else { "strips" }).to_owned(),
        bits_per_sample: Some(bits_per_sample),
        sample_format: Some(sample_format),
        predictor: Some(predictor),
        rows_per_strip: optional_u32(metadata, TiffTag::RowsPerStrip),
        tile_width: optional_u32(metadata, TiffTag::TileWidth),
        tile_height: optional_u32(metadata, TiffTag::TileLength),
        error_kind: String::new(),
    })
}

impl CorpusMetadataRow {
    fn failed(row: &ManifestRow, error_kind: String) -> Self {
        Self {
            status: "❌".to_owned(),
            path: row.path.clone(),
            sha256: row.sha256.clone(),
            format: row.format.clone(),
            compression_id: None,
            compression_name: "unknown".to_owned(),
            storage: "unknown".to_owned(),
            bits_per_sample: None,
            sample_format: None,
            predictor: None,
            rows_per_strip: None,
            tile_width: None,
            tile_height: None,
            error_kind,
        }
    }
}

fn decode_error_kind(stage: &str, error: &fornax_raw_core::error::DecodeError) -> String {
    use fornax_raw_core::error::DecodeError;

    let kind = match error {
        DecodeError::InvalidInput(_) => "invalid_input",
        DecodeError::Unsupported(_) => "unsupported",
        DecodeError::Backend(_) => "backend",
        DecodeError::InvalidMetadata(_) => "invalid_metadata",
        DecodeError::InvalidOutput(_) => "invalid_output",
        DecodeError::Other(_) => "other",
    };
    format!("{stage}:{kind}")
}

pub const fn compression_name(id: u16) -> &'static str {
    match id {
        1 => "uncompressed",
        5 => "lzw",
        7 => "lossless-jpeg",
        8 => "deflate",
        32773 => "packbits",
        32946 => "deflate",
        34892 => "lossy-jpeg",
        34925 => "lzma2",
        50000 => "zstandard",
        50001 => "webp",
        50002 | 52546 => "jpeg-xl",
        _ => "unknown",
    }
}

pub fn write_corpus_metadata_csv(rows: &[CorpusMetadataRow]) -> Result<Vec<u8>> {
    let mut rows = rows.to_vec();
    rows.sort_by(|left, right| (&left.sha256, &left.path).cmp(&(&right.sha256, &right.path)));
    for row in &rows {
        row.validate()?;
    }
    let mut output = Vec::new();
    {
        let mut writer = csv::WriterBuilder::new()
            .has_headers(false)
            .terminator(csv::Terminator::Any(b'\n'))
            .from_writer(&mut output);
        writer.write_record(CORPUS_METADATA_HEADER)?;
        for row in &rows {
            writer.serialize(row)?;
        }
        writer.flush()?;
    }
    Ok(output)
}

pub fn read_corpus_metadata_csv(bytes: &[u8]) -> Result<Vec<CorpusMetadataRow>> {
    let mut reader = csv::ReaderBuilder::new().from_reader(bytes);
    let headers = reader.headers()?;
    if headers.iter().collect::<Vec<_>>() != CORPUS_METADATA_HEADER {
        return Err(TrackerError::new(
            "corpus metadata header does not match the canonical schema",
        ));
    }
    let rows = reader
        .deserialize::<CorpusMetadataRow>()
        .collect::<std::result::Result<Vec<_>, _>>()?;
    let mut keys = BTreeSet::new();
    for row in &rows {
        row.validate()?;
        let key = (row.sha256.clone(), row.path.clone());
        if !keys.insert(key) {
            return Err(TrackerError::new(format!(
                "corpus metadata contains duplicate path {}",
                row.path
            )));
        }
    }
    Ok(rows)
}

impl CorpusMetadataRow {
    fn validate(&self) -> Result<()> {
        if self.sha256.len() != 64 || hex::decode(&self.sha256).is_err() {
            return Err(TrackerError::new(format!(
                "corpus metadata for {} has an invalid sha256",
                self.path
            )));
        }
        if self.format != "DNG" {
            return Err(TrackerError::new(format!(
                "corpus metadata for {} has unexpected format {:?}",
                self.path, self.format
            )));
        }
        match (self.status.as_str(), self.error_kind.is_empty()) {
            ("✅", true) => {
                let compression_id = self.compression_id.ok_or_else(|| {
                    TrackerError::new(format!(
                        "corpus metadata for {} is missing compression",
                        self.path
                    ))
                })?;
                if self.compression_name != compression_name(compression_id) {
                    return Err(TrackerError::new(format!(
                        "corpus metadata for {} has inconsistent compression name",
                        self.path
                    )));
                }
                if !matches!(self.storage.as_str(), "strips" | "tiles") {
                    return Err(TrackerError::new(format!(
                        "corpus metadata for {} has invalid storage {:?}",
                        self.path, self.storage
                    )));
                }
                if self.bits_per_sample.is_none()
                    || self.sample_format.is_none()
                    || self.predictor.is_none()
                {
                    return Err(TrackerError::new(format!(
                        "corpus metadata for {} is missing sample metadata",
                        self.path
                    )));
                }
            }
            ("❌", false)
                if matches!(
                    self.error_kind.as_str(),
                    "metadata:invalid_input"
                        | "metadata:unsupported"
                        | "metadata:backend"
                        | "metadata:invalid_metadata"
                        | "metadata:invalid_output"
                        | "metadata:other"
                ) =>
            {
                if self.compression_id.is_some()
                    || self.compression_name != "unknown"
                    || self.storage != "unknown"
                    || self.bits_per_sample.is_some()
                    || self.sample_format.is_some()
                    || self.predictor.is_some()
                    || self.rows_per_strip.is_some()
                    || self.tile_width.is_some()
                    || self.tile_height.is_some()
                {
                    return Err(TrackerError::new(format!(
                        "failed corpus metadata for {} contains classified fields",
                        self.path
                    )));
                }
            }
            _ => {
                return Err(TrackerError::new(format!(
                    "corpus metadata for {} has inconsistent status or error",
                    self.path
                )));
            }
        }
        Ok(())
    }
}

pub fn merge_corpus_metadata(
    selected: &[ManifestRow],
    shards: &[std::path::PathBuf],
) -> Result<Vec<CorpusMetadataRow>> {
    let dng = selected
        .iter()
        .filter(|row| row.format == "DNG")
        .map(|row| (row.sha256.clone(), row.path.clone()))
        .collect::<BTreeSet<_>>();
    let mut merged = BTreeMap::new();
    for shard in shards {
        let rows = read_corpus_metadata_csv(&fs::read(shard).map_err(|error| {
            TrackerError::new(format!(
                "reading corpus metadata shard {} failed: {error}",
                shard.display()
            ))
        })?)?;
        for row in rows {
            let key = (row.sha256.clone(), row.path.clone());
            if !dng.contains(&key) {
                return Err(TrackerError::new(format!(
                    "corpus metadata shard contains unexpected path {}",
                    row.path
                )));
            }
            if merged.insert(key.clone(), row).is_some() {
                return Err(TrackerError::new(format!(
                    "corpus metadata shards contain duplicate path {}",
                    key.1
                )));
            }
        }
    }
    if merged.len() != dng.len() {
        return Err(TrackerError::new(format!(
            "corpus metadata is missing {} selected DNG rows",
            dng.len().saturating_sub(merged.len())
        )));
    }
    Ok(merged.into_values().collect())
}

fn required_u16(metadata: &Directory, tag: TiffTag) -> Result<u16> {
    optional_u16(metadata, tag).ok_or_else(|| {
        TrackerError::new(format!("DNG is missing valid {} metadata", Tag::from(tag)))
    })
}

fn optional_u16(metadata: &Directory, tag: TiffTag) -> Option<u16> {
    unsigned_scalar(metadata.get(&Tag::from(tag))).and_then(|value| u16::try_from(value).ok())
}

fn optional_u32(metadata: &Directory, tag: TiffTag) -> Option<u32> {
    unsigned_scalar(metadata.get(&Tag::from(tag))).and_then(|value| u32::try_from(value).ok())
}

#[cfg(test)]
mod tests {
    use std::fs;

    use fornax_tiff::{Directory, Tag, Value};
    use tempfile::tempdir;

    use super::*;

    fn row() -> ManifestRow {
        ManifestRow {
            sha256: "a".repeat(64),
            size: 1,
            path: "Canon/camera/file.dng".to_owned(),
            vendor: "Canon".to_owned(),
            camera: "camera".to_owned(),
            model: "file.dng".to_owned(),
            format: "DNG".to_owned(),
            filename: "file.dng".to_owned(),
        }
    }

    #[test]
    fn classifies_dng_compression_and_storage() {
        let mut metadata = Directory::default();
        metadata.insert(Tag::from(TiffTag::Compression), Value::Short(8));
        metadata.insert(Tag::from(TiffTag::BitsPerSample), Value::Short(32));
        metadata.insert(Tag::from(TiffTag::SampleFormat), Value::Short(3));
        metadata.insert(Tag::from(TiffTag::Predictor), Value::Short(34894));
        metadata.insert(Tag::from(TiffTag::TileOffsets), Value::Unsigned(0));
        metadata.insert(Tag::from(TiffTag::TileWidth), Value::Unsigned(496));
        metadata.insert(Tag::from(TiffTag::TileLength), Value::Unsigned(496));

        let row = classify_dng(&row(), &metadata).unwrap();
        assert_eq!(row.status, "✅");
        assert_eq!(row.compression_id, Some(8));
        assert_eq!(row.compression_name, "deflate");
        assert_eq!(row.storage, "tiles");
        assert_eq!(row.bits_per_sample, Some(32));
        assert_eq!(row.sample_format, Some(3));
        assert_eq!(row.predictor, Some(34894));
        assert_eq!(row.tile_width, Some(496));
        assert!(row.error_kind.is_empty());
    }

    #[test]
    fn metadata_csv_round_trips() {
        let row = CorpusMetadataRow {
            status: "✅".to_owned(),
            path: "Zed/camera/file.dng".to_owned(),
            sha256: "a".repeat(64),
            format: "DNG".to_owned(),
            compression_id: Some(7),
            compression_name: "lossless-jpeg".to_owned(),
            storage: "strips".to_owned(),
            bits_per_sample: Some(14),
            sample_format: Some(1),
            predictor: Some(1),
            rows_per_strip: Some(3284),
            tile_width: None,
            tile_height: None,
            error_kind: String::new(),
        };
        let mut failed_row = row.clone();
        failed_row.status = "❌".to_owned();
        failed_row.path = "Canon/camera/file.dng".to_owned();
        failed_row.compression_id = None;
        failed_row.compression_name = "unknown".to_owned();
        failed_row.storage = "unknown".to_owned();
        failed_row.bits_per_sample = None;
        failed_row.sample_format = None;
        failed_row.predictor = None;
        failed_row.rows_per_strip = None;
        failed_row.error_kind = "metadata:invalid_input".to_owned();
        let bytes = write_corpus_metadata_csv(&[row.clone(), failed_row.clone()]).unwrap();
        assert_eq!(
            read_corpus_metadata_csv(&bytes).unwrap(),
            vec![failed_row, row]
        );
    }

    #[test]
    fn unparseable_dng_is_recorded_as_a_failed_result() {
        let temp = tempdir().unwrap();
        let corpus = temp.path();
        let row = row();
        let path = corpus.join(&row.path);
        fs::create_dir_all(path.parent().unwrap()).unwrap();
        fs::write(&path, b"not a TIFF").unwrap();

        let rows = classify_rows(std::slice::from_ref(&row), corpus).unwrap();

        assert_eq!(rows.len(), 1);
        assert_eq!(rows[0].path, row.path);
        assert_eq!(rows[0].status, "❌");
        assert_eq!(rows[0].error_kind, "metadata:invalid_input");
        assert_eq!(rows[0].compression_id, None);
        assert_eq!(rows[0].compression_name, "unknown");
        assert_eq!(rows[0].storage, "unknown");
    }

    #[test]
    fn merge_requires_one_metadata_row_per_selected_dng() {
        let temp = tempdir().unwrap();
        let first = row();
        let mut second = row();
        second.path = "Canon/camera/other.dng".to_owned();
        second.sha256 = "b".repeat(64);
        let selected = vec![first.clone(), second.clone()];
        let first_csv = CorpusMetadataRow {
            status: "✅".to_owned(),
            path: first.path,
            sha256: first.sha256,
            format: "DNG".to_owned(),
            compression_id: Some(7),
            compression_name: "lossless-jpeg".to_owned(),
            storage: "strips".to_owned(),
            bits_per_sample: Some(14),
            sample_format: Some(1),
            predictor: Some(1),
            rows_per_strip: Some(128),
            tile_width: None,
            tile_height: None,
            error_kind: String::new(),
        };
        let second_csv = CorpusMetadataRow {
            status: "✅".to_owned(),
            path: second.path,
            sha256: second.sha256,
            format: "DNG".to_owned(),
            compression_id: Some(1),
            compression_name: "uncompressed".to_owned(),
            storage: "tiles".to_owned(),
            bits_per_sample: Some(16),
            sample_format: Some(1),
            predictor: Some(1),
            rows_per_strip: None,
            tile_width: Some(256),
            tile_height: Some(256),
            error_kind: String::new(),
        };
        let first_shard = temp.path().join("first.csv");
        let second_shard = temp.path().join("second.csv");
        fs::write(
            &first_shard,
            write_corpus_metadata_csv(std::slice::from_ref(&first_csv)).unwrap(),
        )
        .unwrap();
        fs::write(
            &second_shard,
            write_corpus_metadata_csv(std::slice::from_ref(&second_csv)).unwrap(),
        )
        .unwrap();

        assert_eq!(
            merge_corpus_metadata(&selected, &[first_shard.clone(), second_shard.clone()]).unwrap(),
            vec![first_csv.clone(), second_csv]
        );
        assert!(merge_corpus_metadata(&selected, &[first_shard]).is_err());
    }
}
