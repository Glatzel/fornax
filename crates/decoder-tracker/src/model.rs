use std::fmt;

use serde::{Deserialize, Serialize};

pub const MANIFEST_HEADER: [&str; 3] = ["path", "size", "sha256"];
pub const BASELINE_HEADER: [&str; 23] = [
    "status",
    "camera",
    "format",
    "path",
    "decode_pass",
    "raw_mosaic_pass",
    "raw_sha256",
    "process_metadata_pass",
    "handoff_pass",
    "error_kind",
    "width",
    "height",
    "cfa",
    "compression_id",
    "compression_name",
    "compression_lossless",
    "storage",
    "bits_per_sample",
    "sample_format",
    "predictor",
    "rows_per_strip",
    "tile_width",
    "tile_height",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackerError(pub String);

impl TrackerError {
    pub fn new<S>(message: S) -> Self
    where
        S: Into<String>,
    {
        Self(message.into())
    }
}

impl fmt::Display for TrackerError {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        formatter.write_str(&self.0)
    }
}

impl std::error::Error for TrackerError {}

impl From<std::io::Error> for TrackerError {
    fn from(error: std::io::Error) -> Self { Self(error.to_string()) }
}

impl From<csv::Error> for TrackerError {
    fn from(error: csv::Error) -> Self { Self(error.to_string()) }
}

impl From<serde_json::Error> for TrackerError {
    fn from(error: serde_json::Error) -> Self { Self(error.to_string()) }
}

pub type Result<T> = std::result::Result<T, TrackerError>;

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct ManifestRow {
    pub sha256: String,
    pub size: u64,
    pub path: String,
    pub vendor: String,
    pub camera: String,
    pub model: String,
    pub format: String,
    pub filename: String,
}

#[derive(Debug, Deserialize, Serialize)]
pub(crate) struct ManifestCsvRecord {
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

impl ManifestRow {
    pub fn key(&self) -> BaselineKey {
        BaselineKey {
            camera: self.camera.clone(),
            format: self.format.clone(),
            path: self.path.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct BaselineKey {
    pub camera: String,
    pub format: String,
    pub path: String,
}

impl fmt::Display for BaselineKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(formatter, "{}/{}/{}", self.camera, self.format, self.path)
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BaselineRow {
    pub status: String,
    pub camera: String,
    pub format: String,
    pub path: String,
    pub decode_pass: u8,
    pub raw_mosaic_pass: u8,
    pub raw_sha256: String,
    pub process_metadata_pass: u8,
    pub handoff_pass: u8,
    pub error_kind: String,
    pub width: u32,
    pub height: u32,
    pub cfa: String,
    pub compression_id: Option<u32>,
    pub compression_name: String,
    pub compression_lossless: Option<u8>,
    pub storage: String,
    pub bits_per_sample: Option<u16>,
    pub sample_format: String,
    pub predictor: Option<u16>,
    pub rows_per_strip: Option<u32>,
    pub tile_width: Option<u32>,
    pub tile_height: Option<u32>,
}

impl BaselineRow {
    pub fn key(&self) -> BaselineKey {
        BaselineKey {
            camera: self.camera.clone(),
            format: self.format.clone(),
            path: self.path.clone(),
        }
    }

    pub fn validate(&self) -> Result<()> {
        let passes = [
            self.decode_pass,
            self.raw_mosaic_pass,
            self.process_metadata_pass,
            self.handoff_pass,
        ];
        if passes.iter().any(|value| !matches!(value, 0 | 1)) {
            return Err(TrackerError::new(format!(
                "{} contains a stage value other than 0 or 1",
                self.key()
            )));
        }
        if (passes[0] == 0 && passes[1..].contains(&1))
            || (passes[1] == 0 && passes[2..].contains(&1))
            || (passes[2] == 0 && passes[3] == 1)
        {
            return Err(TrackerError::new(format!(
                "{} continues a failed stage",
                self.key()
            )));
        }
        if !self.raw_sha256.is_empty()
            && (self.raw_sha256.len() != 64 || hex::decode(&self.raw_sha256).is_err())
        {
            return Err(TrackerError::new(format!(
                "{} has an invalid raw_sha256",
                self.key()
            )));
        }
        let all_passed = passes.iter().all(|value| *value == 1);
        if all_passed != self.error_kind.is_empty() {
            return Err(TrackerError::new(format!(
                "{} has inconsistent error_kind",
                self.key()
            )));
        }
        if self.format != self.format.to_ascii_uppercase() {
            return Err(TrackerError::new(format!(
                "{} has a non-uppercase format",
                self.key()
            )));
        }
        let expected_status = if all_passed { "✅" } else { "❌" };
        if self.status != expected_status {
            return Err(TrackerError::new(format!(
                "{} has status {:?}, expected {expected_status:?}",
                self.key(),
                self.status
            )));
        }
        if self.raw_mosaic_pass == 1
            && (self.compression_name.is_empty()
                || self.compression_name == "unknown"
                || !self
                    .compression_lossless
                    .is_some_and(|lossless| matches!(lossless, 0 | 1))
                || self.storage.is_empty()
                || self.storage == "unknown"
                || self.bits_per_sample.is_none_or(|bits| bits == 0)
                || !matches!(self.sample_format.as_str(), "integer" | "float"))
        {
            return Err(TrackerError::new(format!(
                "{} is missing source encoding metadata after a successful raw decode",
                self.key()
            )));
        }
        Ok(())
    }
}

pub fn status_for_stages(passes: [u8; 4]) -> &'static str {
    if passes.iter().all(|pass| *pass == 1) {
        "✅"
    } else {
        "❌"
    }
}
