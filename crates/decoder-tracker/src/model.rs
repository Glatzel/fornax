use std::fmt;

use serde::{Deserialize, Serialize};

pub const MANIFEST_HEADER: [&str; 6] = ["vendor", "camera", "format", "path", "size", "sha256"];
pub const BASELINE_HEADER: [&str; 13] = [
    "status",
    "vendor",
    "camera",
    "format",
    "path",
    "decode_pass",
    "raw_mosaic_pass",
    "process_metadata_pass",
    "handoff_pass",
    "error_kind",
    "width",
    "height",
    "cfa",
];

#[derive(Clone, Debug, Eq, PartialEq)]
pub struct TrackerError(pub String);

impl TrackerError {
    pub fn new(message: impl Into<String>) -> Self { Self(message.into()) }
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
    pub vendor: String,
    pub camera: String,
    pub format: String,
    pub path: String,
    pub size: u64,
    pub sha256: String,
}

impl ManifestRow {
    pub fn key(&self) -> BaselineKey {
        BaselineKey {
            vendor: self.vendor.clone(),
            camera: self.camera.clone(),
            format: self.format.clone(),
            path: self.path.clone(),
        }
    }
}

#[derive(Clone, Debug, Deserialize, Eq, Ord, PartialEq, PartialOrd, Serialize)]
pub struct BaselineKey {
    pub vendor: String,
    pub camera: String,
    pub format: String,
    pub path: String,
}

impl fmt::Display for BaselineKey {
    fn fmt(&self, formatter: &mut fmt::Formatter<'_>) -> fmt::Result {
        write!(
            formatter,
            "{}/{}/{}/{}",
            self.vendor, self.camera, self.format, self.path
        )
    }
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
pub struct BaselineRow {
    pub status: String,
    pub vendor: String,
    pub camera: String,
    pub format: String,
    pub path: String,
    pub decode_pass: u8,
    pub raw_mosaic_pass: u8,
    pub process_metadata_pass: u8,
    pub handoff_pass: u8,
    pub error_kind: String,
    pub width: u32,
    pub height: u32,
    pub cfa: String,
}

impl BaselineRow {
    pub fn key(&self) -> BaselineKey {
        BaselineKey {
            vendor: self.vendor.clone(),
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
