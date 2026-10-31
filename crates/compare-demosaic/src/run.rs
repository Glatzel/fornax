use std::path::{Path, PathBuf};
use std::time::Instant;

use crate::case::ReferenceBackend;
use crate::error::{CompareError, Result};
use crate::registry;
use crate::results::{self, CanonicalResults, Coverage, RunMetadata, Selection, ShardMetadata};
use crate::timings::Timings;

#[derive(Clone, Debug)]
pub struct RunRequest {
    pub algorithms: Vec<String>,
    pub references: Vec<String>,
    pub patterns: Vec<String>,
    pub input: Option<PathBuf>,
    pub output_dir: PathBuf,
    pub shard: Option<String>,
    pub artifacts: bool,
    pub border_px: u32,
    pub jobs: usize,
}

#[derive(Debug)]
pub struct RunOutput {
    pub path: PathBuf,
    pub results: CanonicalResults,
    pub timings: Timings,
}

pub fn execute(request: &RunRequest) -> Result<RunOutput> {
    if request.jobs == 0 {
        return Err(CompareError::usage("--jobs must be at least 1"));
    }
    if request.border_px < 2 {
        return Err(CompareError::usage("--border-px must be at least 2"));
    }
    let algorithms = normalize_algorithms(&request.algorithms)?;
    let references = normalize_references(&request.references)?;
    let patterns = normalize_patterns(&request.patterns)?;
    let selection = Selection::new(
        &algorithms,
        &references
            .iter()
            .map(|reference| reference.library().to_ascii_lowercase())
            .collect::<Vec<_>>(),
        &patterns.iter().map(ToString::to_string).collect::<Vec<_>>(),
    );
    let selected_rows = registry::expected_rows_for_selection(&algorithms, &references, &patterns);
    if selected_rows.is_empty() {
        return Err(CompareError::usage(
            "the selected algorithm/reference/pattern intersection contains no rows",
        ));
    }
    let complete = selected_rows == registry::expected_rows();
    if !complete && request.shard.is_none() {
        return Err(CompareError::usage(
            "partial selections must pass `--shard <name>`; refusing to write canonical evidence",
        ));
    }
    if let Some(name) = &request.shard {
        validate_shard_name(name)?;
    }

    let started_at = jiff::Timestamp::now().to_string();
    let total_started = Instant::now();
    let input_path = request
        .input
        .clone()
        .unwrap_or_else(fornax_devtool::raw_file);
    let input = crate::data::active_area_from(&input_path)
        .map_err(|error| CompareError::usage(error.to_string()))?;
    let stable = results::stable_metadata(&input, request.border_px);
    let (rows, row_timings) = crate::report::comparison_rows(
        &input,
        &crate::report::ComparisonOptions {
            algorithms: &algorithms,
            references: &references,
            patterns: &patterns,
            artifacts: request.artifacts,
            output_dir: Some(&request.output_dir),
            jobs: request.jobs,
            border_px: request.border_px,
        },
    )
    .map_err(|error| CompareError::usage(error.to_string()))?;
    let finished_at = jiff::Timestamp::now().to_string();
    let shard = ShardMetadata {
        name: request
            .shard
            .clone()
            .unwrap_or_else(|| "canonical".to_owned()),
        selection,
        started: started_at,
        finished: finished_at,
        runner: runner_name(),
    };
    let run = RunMetadata {
        coverage: if complete && request.shard.is_none() {
            Coverage::Full
        } else {
            Coverage::Partial
        },
        shards: vec![shard],
        generated_by: results::generated_by(),
    };
    let canonical = results::from_rows(&input, rows, stable, run);
    let path = output_path(&request.output_dir, request.shard.as_deref());
    canonical
        .write(&path)
        .map_err(|error| CompareError::usage(error.to_string()))?;
    let timings = Timings::merge(total_started.elapsed(), row_timings);
    let timings_path = timings_path(&request.output_dir, request.shard.as_deref());
    timings.write(&timings_path)?;
    Ok(RunOutput {
        path,
        results: canonical,
        timings,
    })
}

fn output_path(output_dir: &Path, shard: Option<&str>) -> PathBuf {
    output_dir.join(shard.map_or_else(
        || "canonical.json".to_owned(),
        |name| format!("{name}.shard.json"),
    ))
}

fn timings_path(output_dir: &Path, shard: Option<&str>) -> PathBuf {
    output_dir.join(shard.map_or_else(
        || "timings.json".to_owned(),
        |name| format!("{name}.timings.json"),
    ))
}

fn normalize_algorithms(values: &[String]) -> Result<Vec<String>> {
    values
        .iter()
        .map(|value| {
            registry::canonical_algorithm_name(value)
                .map(str::to_owned)
                .ok_or_else(|| CompareError::usage(format!("unknown algorithm `{value}`")))
        })
        .collect()
}

fn normalize_references(values: &[String]) -> Result<Vec<ReferenceBackend>> {
    values
        .iter()
        .map(|value| {
            registry::parse_reference(value)
                .ok_or_else(|| CompareError::usage(format!("unknown reference `{value}`")))
        })
        .collect()
}

fn normalize_patterns(values: &[String]) -> Result<Vec<fornax_core::BayerPattern>> {
    values
        .iter()
        .map(|value| {
            registry::parse_pattern(value)
                .ok_or_else(|| CompareError::usage(format!("unknown Bayer pattern `{value}`")))
        })
        .collect()
}

fn validate_shard_name(name: &str) -> Result<()> {
    if name.is_empty() || name == "." || name == ".." || name.contains('/') || name.contains('\\') {
        return Err(CompareError::usage(
            "--shard must be a single file-name component",
        ));
    }
    Ok(())
}

fn runner_name() -> String {
    std::env::var("RUNNER_NAME")
        .or_else(|_| std::env::var("HOSTNAME"))
        .unwrap_or_else(|_| "unknown".to_owned())
}

#[cfg(test)]
mod tests {
    use super::*;

    fn request() -> RunRequest {
        RunRequest {
            algorithms: vec!["AHD".to_owned()],
            references: Vec::new(),
            patterns: Vec::new(),
            input: None,
            output_dir: PathBuf::from("unused"),
            shard: None,
            artifacts: false,
            border_px: 16,
            jobs: 1,
        }
    }

    #[test]
    fn partial_selection_without_a_shard_is_a_usage_error() {
        let error = execute(&request()).unwrap_err();
        assert_eq!(error.class().exit_code(), 3);
        assert!(error.to_string().contains("--shard"));
    }

    #[test]
    fn worker_count_must_be_positive() {
        let mut request = request();
        request.jobs = 0;
        let error = execute(&request).unwrap_err();
        assert_eq!(error.class().exit_code(), 3);
    }
}
