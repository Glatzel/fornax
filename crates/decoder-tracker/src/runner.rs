use std::process::{Child, Command, Output, Stdio};
use std::time::{Duration, Instant};
use std::{fs, thread};

use crate::manifest::corpus_path;
use crate::model::{ManifestRow, Result, TrackerError};
use crate::stages::{StageRow, runner_row};

pub const DEFAULT_TIMEOUT: Duration = Duration::from_secs(300);
const FETCH_ATTEMPTS: usize = 3;

#[derive(Debug)]
pub enum ChildOutcome {
    Exited(Output),
    TimedOut { output: Output },
}

pub fn wait_with_timeout(command: &mut Command, timeout: Duration) -> Result<ChildOutcome> {
    command.stdout(Stdio::piped()).stderr(Stdio::piped());
    let mut child = command
        .spawn()
        .map_err(|error| TrackerError::new(format!("launching child failed: {error}")))?;
    let start = Instant::now();
    loop {
        if child.try_wait()?.is_some() {
            return Ok(ChildOutcome::Exited(child.wait_with_output()?));
        }
        if start.elapsed() >= timeout {
            terminate(&mut child)?;
            let output = child.wait_with_output()?;
            return Ok(ChildOutcome::TimedOut { output });
        }
        thread::sleep(Duration::from_millis(100));
    }
}

fn terminate(child: &mut Child) -> Result<()> {
    child
        .kill()
        .map_err(|error| TrackerError::new(format!("killing timed-out child failed: {error}")))
}

pub fn run_isolation(
    executable: &std::path::Path,
    input: &std::path::Path,
    timeout: Duration,
) -> Result<StageRow> {
    let mut command = Command::new(executable);
    command.arg("run-one").arg("--input").arg(input);
    match wait_with_timeout(&mut command, timeout)? {
        ChildOutcome::TimedOut { output } => {
            let _ = output;
            Ok(runner_row("runner:timeout"))
        }
        ChildOutcome::Exited(output) if output.status.success() => {
            if !output.stderr.is_empty() {
                eprint!("{}", String::from_utf8_lossy(&output.stderr));
            }
            serde_json::from_slice::<StageRow>(&output.stdout).map_err(|error| {
                TrackerError::new(format!(
                    "run-one produced malformed JSON for {}: {error}; stderr: {}",
                    input.display(),
                    String::from_utf8_lossy(&output.stderr).trim()
                ))
            })
        }
        ChildOutcome::Exited(output) => {
            eprintln!(
                "run-one crashed for {} ({}): {}",
                input.display(),
                output.status,
                String::from_utf8_lossy(&output.stderr).trim()
            );
            Ok(runner_row("runner:crash"))
        }
    }
}

pub fn materialize(corpus: &std::path::Path, rows: &[ManifestRow]) -> Result<()> {
    if rows.is_empty() {
        return Ok(());
    }
    clerk::info!("materializing {} selected LFS objects", rows.len());
    let install = Command::new("git")
        .arg("-C")
        .arg(corpus)
        .args(["lfs", "install", "--local"])
        .env("GIT_TERMINAL_PROMPT", "0")
        .output()
        .map_err(|error| {
            TrackerError::new(format!(
                "running git-lfs install failed: {error}; install the Pixi dev environment"
            ))
        })?;
    if !install.status.success() {
        return Err(TrackerError::new(format!(
            "git-lfs install failed: {}",
            String::from_utf8_lossy(&install.stderr).trim()
        )));
    }
    let paths = rows.iter().map(|row| row.path.as_str()).collect::<Vec<_>>();
    run_lfs(corpus, "fetch", &paths)?;
    run_lfs(corpus, "checkout", &paths)?;
    for row in rows {
        let path = corpus_path(corpus, &row.path);
        if !path.is_file() {
            return Err(TrackerError::new(format!(
                "LFS materialization did not create {}",
                path.display()
            )));
        }
        let prefix = fs::read(&path)?
            .into_iter()
            .take(crate::manifest::VERSION.len())
            .collect::<Vec<_>>();
        if prefix == crate::manifest::VERSION.as_bytes() {
            return Err(TrackerError::new(format!(
                "LFS materialization left pointer metadata at {}",
                path.display()
            )));
        }
    }
    clerk::info!("materialized {} selected LFS objects", rows.len());
    Ok(())
}

fn run_lfs(corpus: &std::path::Path, action: &str, paths: &[&str]) -> Result<()> {
    let mut last_error = String::new();
    for attempt in 1..=FETCH_ATTEMPTS {
        let mut command = Command::new("git");
        command
            .arg("-C")
            .arg(corpus)
            .args(["-c", "lfs.concurrenttransfers=4"])
            .args(["lfs", action]);
        if action == "fetch" {
            let include = paths
                .iter()
                .map(|path| lfs_include_glob(path))
                .collect::<Vec<_>>()
                .join(",");
            command.arg("origin").arg("--include").arg(include);
        } else {
            command.args(paths.iter().map(|path| escape_lfs_glob(path)));
        }
        command.env("GIT_TERMINAL_PROMPT", "0");
        let output = command.output().map_err(|error| {
            TrackerError::new(format!(
                "running git-lfs {action} failed: {error}; install the Pixi dev environment"
            ))
        })?;
        if output.status.success() {
            return Ok(());
        }
        let stderr = String::from_utf8_lossy(&output.stderr);
        last_error = stderr.trim().to_owned();
        let lower = stderr.to_lowercase();
        if lower.contains("429")
            || lower.contains("rate limit")
            || lower.contains("could not resolve")
            || lower.contains("connection")
            || lower.contains("timed out")
            || lower.contains("network")
        {
            eprintln!(
                "git-lfs {action} attempt {attempt}/{FETCH_ATTEMPTS} hit a network or rate-limit error: {last_error}"
            );
        }
        if attempt < FETCH_ATTEMPTS {
            thread::sleep(Duration::from_millis(500 * attempt as u64));
        }
    }
    Err(TrackerError::new(format!(
        "git-lfs {action} failed after {FETCH_ATTEMPTS} attempts: {last_error}"
    )))
}

pub(crate) fn escape_lfs_glob(path: &str) -> String {
    let mut escaped = String::with_capacity(path.len());
    for character in path.chars() {
        if matches!(character, '\\' | '*' | '?' | '[' | ']') {
            escaped.push('\\');
        }
        escaped.push(character);
    }
    escaped
}

pub(crate) fn lfs_include_glob(path: &str) -> String { escape_lfs_glob(path).replace(',', "?") }

pub fn timeout_from_env() -> Duration {
    std::env::var("DECODER_TRACKER_TIMEOUT_SECS")
        .ok()
        .and_then(|value| value.parse::<u64>().ok())
        .map_or(DEFAULT_TIMEOUT, Duration::from_secs)
}
