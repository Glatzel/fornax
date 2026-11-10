use std::collections::{BTreeMap, BTreeSet};
use std::fs;
use std::path::{Path, PathBuf};
use std::process::Command;

use crate::csvio::{read_manifest_csv, write_manifest_csv};
use crate::model::{ManifestRow, Result, TrackerError};

pub const VERSION: &str = "version https://git-lfs.github.com/spec/v1";

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum ManifestCheck {
    Current,
}

impl ManifestCheck {
    pub fn against(corpus: &Path, committed: &[u8]) -> Result<Self> {
        let generated = generate_manifest(corpus)?;
        let generated_bytes = write_manifest_csv(&generated)?;
        if generated_bytes != committed {
            let committed_rows = read_manifest_csv(committed)?;
            let committed_keys = committed_rows
                .iter()
                .map(|row| (row.path.as_str(), row.sha256.as_str()))
                .collect::<BTreeSet<_>>();
            let generated_keys = generated
                .iter()
                .map(|row| (row.path.as_str(), row.sha256.as_str()))
                .collect::<BTreeSet<_>>();
            let missing = committed_keys.difference(&generated_keys).count();
            let added = generated_keys.difference(&committed_keys).count();
            return Err(TrackerError::new(format!(
                "manifest snapshot drift: {missing} committed entries are gone, {added} entries are new; regenerate and review the manifest"
            )));
        }
        for row in &generated {
            validate_pointer(corpus, row)?;
        }
        Ok(Self::Current)
    }
}

pub fn normalize_repo_path(path: &str) -> String { path.replace('\\', "/") }

pub fn derive_identity(path: &str) -> Result<(String, String, String, String)> {
    let normalized = normalize_repo_path(path);
    let segments = normalized.split('/').collect::<Vec<_>>();
    if segments.len() < 3 || segments.iter().any(|segment| segment.is_empty()) {
        return Err(TrackerError::new(format!(
            "corpus path must contain vendor, camera, and filename segments: {path:?}"
        )));
    }
    let vendor = segments[0].to_owned();
    let model = segments[1].to_owned();
    let file = *segments
        .last()
        .ok_or_else(|| TrackerError::new("corpus path has no filename"))?;
    let (filename, _) = file
        .rsplit_once('.')
        .ok_or_else(|| TrackerError::new(format!("corpus filename has no extension: {file:?}")))?;
    let vendor_prefix = format!("{} ", vendor.to_lowercase());
    let model_lower = model.to_lowercase();
    let camera = if model_lower == vendor.to_lowercase() || model_lower.starts_with(&vendor_prefix)
    {
        model.clone()
    } else {
        format!("{vendor} {model}")
    };
    Ok((vendor, camera, model, filename.to_owned()))
}

pub fn generate_manifest(corpus: &Path) -> Result<Vec<ManifestRow>> {
    let filelist = fs::read_to_string(corpus.join("filelist.sha256"))?;
    let mut rows = Vec::new();
    for (line_number, line) in filelist.lines().enumerate() {
        if line.trim().is_empty() {
            continue;
        }
        let (sha256, raw_path) = parse_filelist_line(line).ok_or_else(|| {
            TrackerError::new(format!(
                "malformed filelist.sha256 line {}: {line:?}",
                line_number + 1
            ))
        })?;
        let path = normalize_repo_path(raw_path);
        let format = path_extension(&path).to_ascii_uppercase();
        let (vendor, camera, model, filename) = derive_identity(&path)?;
        let row = ManifestRow {
            sha256: sha256.to_owned(),
            size: 0,
            path,
            vendor,
            camera,
            model,
            format,
            filename,
        };
        validate_pointer(corpus, &row)?;
        rows.push(ManifestRow {
            size: pointer_size(corpus, &row)?,
            ..row
        });
    }
    validate_manifest_keys(&rows)?;
    rows.sort_by(|left, right| {
        left.sha256
            .cmp(&right.sha256)
            .then_with(|| left.path.cmp(&right.path))
    });
    Ok(rows)
}

fn parse_filelist_line(line: &str) -> Option<(&str, &str)> {
    let (sha256, path) = line.split_once(" *").or_else(|| line.split_once("  "))?;
    if sha256.len() != 64 || !sha256.bytes().all(|byte| byte.is_ascii_hexdigit()) {
        return None;
    }
    Some((sha256, path))
}

fn path_extension(path: &str) -> &str {
    path.rsplit_once('.').map_or("", |(_, extension)| extension)
}

pub fn validate_manifest_keys(rows: &[ManifestRow]) -> Result<()> {
    let mut keys = BTreeMap::new();
    for row in rows {
        let key = row.key();
        if let Some(previous) = keys.insert(key.clone(), row.path.clone()) {
            return Err(TrackerError::new(format!(
                "manifest has duplicate baseline key {key}: {previous:?} and {:?}",
                row.path
            )));
        }
    }
    Ok(())
}

pub fn validate_pointer(corpus: &Path, row: &ManifestRow) -> Result<()> {
    let pointer = read_pointer(corpus, &row.path).map_err(|error| {
        TrackerError::new(format!(
            "reading LFS pointer {} failed: {error}; initialize the pointer-only corpus submodule",
            row.path
        ))
    })?;
    if pointer.oid != row.sha256 {
        return Err(TrackerError::new(format!(
            "{} pointer oid {} does not match the listed SHA-256 {}",
            row.path, pointer.oid, row.sha256
        )));
    }
    Ok(())
}

fn pointer_size(corpus: &Path, row: &ManifestRow) -> Result<u64> {
    read_pointer(corpus, &row.path).map(|pointer| pointer.size)
}

fn read_pointer(corpus: &Path, repo_path: &str) -> Result<Pointer> {
    let working_path = corpus_path(corpus, repo_path);
    if let Ok(text) = fs::read_to_string(&working_path)
        && let Some(pointer) = parse_pointer(&text)
    {
        return Ok(pointer);
    }
    let output = Command::new("git")
        .arg("-C")
        .arg(corpus)
        .args(["cat-file", "blob"])
        .arg(format!("HEAD:{repo_path}"))
        .output()?;
    if !output.status.success() {
        return Err(TrackerError::new(format!(
            "{} is not an LFS pointer in the working tree or submodule HEAD: {}",
            working_path.display(),
            String::from_utf8_lossy(&output.stderr).trim()
        )));
    }
    let text = std::str::from_utf8(&output.stdout)
        .map_err(|error| TrackerError::new(format!("LFS pointer is not UTF-8: {error}")))?;
    parse_pointer(text).ok_or_else(|| {
        TrackerError::new(format!(
            "{} is not an LFS pointer in submodule HEAD",
            working_path.display()
        ))
    })
}

#[derive(Debug)]
struct Pointer {
    oid: String,
    size: u64,
}

fn parse_pointer(text: &str) -> Option<Pointer> {
    let mut version = None;
    let mut oid = None;
    let mut size = None;
    for line in text.lines() {
        if let Some(value) = line.strip_prefix("version ") {
            version = Some(value);
        } else if let Some(value) = line.strip_prefix("oid sha256:") {
            oid = Some(value.to_owned());
        } else if let Some(value) = line.strip_prefix("size ") {
            size = value.parse::<u64>().ok();
        }
    }
    if version? != "https://git-lfs.github.com/spec/v1" {
        return None;
    }
    Some(Pointer {
        oid: oid?,
        size: size?,
    })
}

pub fn corpus_path(corpus: &Path, path: &str) -> PathBuf {
    path.split('/')
        .fold(corpus.to_path_buf(), |path, segment| path.join(segment))
}

pub fn selected_formats(formats: &[String]) -> BTreeSet<String> {
    if formats.is_empty() {
        BTreeSet::from(["DNG".to_owned()])
    } else {
        formats
            .iter()
            .map(|format| format.to_ascii_uppercase())
            .collect()
    }
}

#[cfg(test)]
pub fn write_pointer(path: &Path, sha256: &str, size: u64) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(
        path,
        format!("{VERSION}\noid sha256:{sha256}\nsize {size}\n"),
    )?;
    Ok(())
}
