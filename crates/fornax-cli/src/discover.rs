use std::collections::HashSet;
use std::fs;
use std::path::{Path, PathBuf};

use fornax_raw::RawFormat;
use thiserror::Error;

use crate::{OutputFormat, ResolvedConfig};

/// One input/output conversion target.
#[derive(Clone, Debug, Eq, PartialEq)]
pub struct InputJob {
    pub input: PathBuf,
    pub output: PathBuf,
    pub format: RawFormat,
}

/// Input discovery or output-planning failure.
#[derive(Debug, Error)]
pub enum DiscoveryError {
    #[error("input does not exist: {0}")]
    MissingInput(PathBuf),
    #[error("reading directory {path} failed: {source}")]
    ReadDirectory {
        path: PathBuf,
        #[source]
        source: std::io::Error,
    },
    #[error("input has no supported RAW extension: {0}")]
    UnsupportedInput(PathBuf),
    #[error("multiple inputs map to the same output: {0}")]
    DuplicateOutput(PathBuf),
}

/// Resolves files and directories into a deterministic output plan.
pub fn plan_inputs(config: &ResolvedConfig) -> Result<Vec<InputJob>, DiscoveryError> {
    let mut jobs = Vec::new();
    for input in &config.inputs {
        let metadata =
            fs::metadata(input).map_err(|_| DiscoveryError::MissingInput(input.clone()))?;
        if metadata.is_file() {
            jobs.push(plan_file(input, &config.output, config.format)?);
        } else if metadata.is_dir() {
            let root_name = input
                .file_name()
                .ok_or_else(|| DiscoveryError::MissingInput(input.clone()))?;
            let mut files = Vec::new();
            collect_files(input, config.recursive, &mut files)?;
            files.sort();
            for file in files {
                let relative = file
                    .strip_prefix(input)
                    .expect("discovered files belong to the input root");
                let format = raw_format(&file)?;
                let parent = relative.parent().unwrap_or_else(|| Path::new(""));
                let output = config.output.join(root_name).join(parent).join(output_name(
                    relative.file_stem().expect("RAW files have a stem"),
                    config.format,
                ));
                jobs.push(InputJob {
                    input: file,
                    output,
                    format,
                });
            }
        } else {
            return Err(DiscoveryError::MissingInput(input.clone()));
        }
    }
    let mut seen = HashSet::new();
    for job in &jobs {
        if !seen.insert(job.output.clone()) {
            return Err(DiscoveryError::DuplicateOutput(job.output.clone()));
        }
    }
    Ok(jobs)
}

fn plan_file(
    input: &Path,
    output_root: &Path,
    format: OutputFormat,
) -> Result<InputJob, DiscoveryError> {
    Ok(InputJob {
        input: input.to_owned(),
        output: output_root.join(output_name(
            input.file_stem().expect("RAW files have a stem"),
            format,
        )),
        format: raw_format(input)?,
    })
}

fn raw_format(path: &Path) -> Result<RawFormat, DiscoveryError> {
    path.extension()
        .and_then(|extension| extension.to_str())
        .and_then(|extension| RawFormat::try_from(extension).ok())
        .ok_or_else(|| DiscoveryError::UnsupportedInput(path.to_owned()))
}

fn output_name(stem: &std::ffi::OsStr, format: OutputFormat) -> PathBuf {
    let mut name = stem.to_os_string();
    name.push(".");
    name.push(format.extension());
    PathBuf::from(name)
}

fn collect_files(
    directory: &Path,
    recursive: bool,
    output: &mut Vec<PathBuf>,
) -> Result<(), DiscoveryError> {
    let entries = fs::read_dir(directory).map_err(|source| DiscoveryError::ReadDirectory {
        path: directory.to_owned(),
        source,
    })?;
    for entry in entries {
        let entry = entry.map_err(|source| DiscoveryError::ReadDirectory {
            path: directory.to_owned(),
            source,
        })?;
        let path = entry.path();
        if path.is_dir() {
            if recursive {
                collect_files(&path, recursive, output)?;
            }
        } else if path.is_file() && raw_format(&path).is_ok() {
            output.push(path);
        }
    }
    Ok(())
}
