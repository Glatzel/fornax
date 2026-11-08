use std::collections::BTreeSet;
use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use decoder_tracker::csvio::{
    DiffReport, merge_baseline_rows, read_baseline_csv, read_manifest_csv, write_baseline_csv,
    write_manifest_csv,
};
use decoder_tracker::manifest::{
    ManifestCheck, corpus_path, generate_manifest, normalize_repo_path,
};
use decoder_tracker::model::{BaselineRow, ManifestRow, Result, TrackerError};
use decoder_tracker::runner::{materialize, run_isolation, timeout_from_env};
use decoder_tracker::selection::{Selection, Shard, select_manifest};
use decoder_tracker::stages::{StageRow, evaluate_one};

const DEFAULT_CORPUS: &str = "external/raw.pixls.us";
const DEFAULT_MANIFEST: &str = "crates/decoder-tracker/manifest.csv";
const DEFAULT_OUTPUT: &str = "target/decoder-tracker/corpus.csv";

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Track decoder results across the raw.pixls.us corpus"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate or verify the supported-format corpus manifest.
    Manifest(ManifestArgs),
    /// Materialize and run selected corpus files in isolated children.
    Run(RunArgs),
    /// Merge shard CSVs and reject missing or duplicate rows.
    Merge(MergeArgs),
    /// Compare two canonical baseline CSVs.
    Diff(DiffArgs),
    /// Evaluate one materialized file for the run parent process.
    #[command(hide = true)]
    RunOne(RunOneArgs),
}

#[derive(Debug, Args)]
struct ManifestArgs {
    #[arg(long, default_value = DEFAULT_CORPUS)]
    corpus: PathBuf,
    #[arg(long = "format", action = clap::ArgAction::Append)]
    formats: Vec<String>,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long)]
    check: bool,
}

#[derive(Clone, Debug, Args)]
struct FilterArgs {
    #[arg(long, default_value = DEFAULT_CORPUS)]
    corpus: PathBuf,
    #[arg(long = "format", action = clap::ArgAction::Append)]
    formats: Vec<String>,
    #[arg(long = "camera", action = clap::ArgAction::Append)]
    cameras: Vec<String>,
    #[arg(long = "filename", action = clap::ArgAction::Append)]
    filenames: Vec<String>,
}

impl FilterArgs {
    fn selection(&self, shard: Option<Shard>) -> Selection {
        Selection {
            formats: self.formats.clone(),
            cameras: self.cameras.clone(),
            filenames: self.filenames.clone(),
            shard,
        }
    }
}

#[derive(Debug, Args)]
struct RunArgs {
    #[command(flatten)]
    filter: FilterArgs,
    #[arg(long, default_value = DEFAULT_MANIFEST)]
    manifest: PathBuf,
    #[arg(long)]
    shard: Option<Shard>,
    #[arg(long, default_value = DEFAULT_OUTPUT)]
    out: PathBuf,
    #[arg(long, default_value_t = 2)]
    max_files: usize,
    #[arg(long)]
    allow_full: bool,
}

#[derive(Debug, Args)]
struct MergeArgs {
    #[command(flatten)]
    filter: FilterArgs,
    #[arg(long, default_value = DEFAULT_MANIFEST)]
    manifest: PathBuf,
    #[arg(long, default_value = DEFAULT_OUTPUT)]
    out: PathBuf,
    #[arg(required = true)]
    shards: Vec<PathBuf>,
}

#[derive(Debug, Args)]
struct DiffArgs {
    baseline: PathBuf,
    candidate: PathBuf,
}

#[derive(Debug, Args)]
struct RunOneArgs {
    #[arg(long)]
    input: PathBuf,
}

fn main() -> ExitCode {
    match execute(Cli::parse()) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn execute(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Command::Manifest(args) => manifest(args),
        Command::Run(args) => run(args),
        Command::Merge(args) => merge(args),
        Command::Diff(args) => diff(args),
        Command::RunOne(args) => {
            println!("{}", serde_json::to_string(&evaluate_one(&args.input))?);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn manifest(args: ManifestArgs) -> Result<ExitCode> {
    let rows = generate_manifest(&args.corpus, &args.formats)?;
    let bytes = write_manifest_csv(&rows)?;
    let out = args.out.unwrap_or_else(|| PathBuf::from(DEFAULT_MANIFEST));
    if args.check {
        let committed = fs::read(&out).map_err(|error| {
            TrackerError::new(format!(
                "reading committed manifest {} failed: {error}",
                out.display()
            ))
        })?;
        ManifestCheck::against(&args.corpus, &args.formats, &committed)?;
        println!("manifest: current ({} rows)", rows.len());
    } else {
        write_file(&out, &bytes)?;
        println!("manifest: wrote {} rows to {}", rows.len(), out.display());
    }
    Ok(ExitCode::SUCCESS)
}

fn run(args: RunArgs) -> Result<ExitCode> {
    let rows = read_manifest_csv(fs::read(&args.manifest).map_err(|error| {
        TrackerError::new(format!(
            "reading manifest {} failed: {error}",
            args.manifest.display()
        ))
    })?)?;
    let selected = select_manifest(&rows, &args.filter.selection(args.shard))?;
    let ci = std::env::var("CI").is_ok_and(|value| value == "true");
    if !args.allow_full && !ci && selected.len() > args.max_files {
        return Err(TrackerError::new(format!(
            "selection has {} files but --max-files is {}; use filters, a shard, CI=true, or --allow-full",
            selected.len(),
            args.max_files
        )));
    }

    materialize(&args.filter.corpus, &selected)?;
    let executable = std::env::current_exe().map_err(|error| {
        TrackerError::new(format!(
            "resolving decoder-tracker executable failed: {error}"
        ))
    })?;
    let timeout = timeout_from_env();
    let mut output = Vec::with_capacity(selected.len());
    for row in &selected {
        let path =
            fs::canonicalize(corpus_path(&args.filter.corpus, &row.path)).map_err(|error| {
                TrackerError::new(format!(
                    "resolving corpus file {} failed: {error}",
                    row.path
                ))
            })?;
        let stage = run_isolation(&executable, &path, timeout)?;
        output.push(baseline_row(row, stage));
    }
    let bytes = write_baseline_csv(&output)?;
    write_file(&args.out, &bytes)?;
    println!(
        "run: recorded {} rows in {}",
        output.len(),
        args.out.display()
    );
    Ok(ExitCode::SUCCESS)
}

fn merge(args: MergeArgs) -> Result<ExitCode> {
    let manifest = read_manifest_csv(fs::read(&args.manifest).map_err(|error| {
        TrackerError::new(format!(
            "reading manifest {} failed: {error}",
            args.manifest.display()
        ))
    })?)?;
    let expected_rows = select_manifest(&manifest, &args.filter.selection(None))?;
    let expected = expected_rows
        .iter()
        .map(ManifestRow::key)
        .collect::<BTreeSet<_>>();
    let shards = args
        .shards
        .iter()
        .map(|path| {
            fs::read(path)
                .map_err(|error| {
                    TrackerError::new(format!("reading shard {} failed: {error}", path.display()))
                })
                .and_then(read_baseline_csv)
                .map(|csv| csv.rows)
        })
        .collect::<Result<Vec<_>>>()?;
    let rows = merge_baseline_rows(shards, &expected)?;
    let bytes = write_baseline_csv(&rows)?;
    write_file(&args.out, &bytes)?;
    println!(
        "merge: wrote {} complete rows to {}",
        rows.len(),
        args.out.display()
    );
    Ok(ExitCode::SUCCESS)
}

fn diff(args: DiffArgs) -> Result<ExitCode> {
    let baseline = read_baseline_csv(read_file(&args.baseline)?)?;
    let candidate = read_baseline_csv(read_file(&args.candidate)?)?;
    let report = DiffReport::between(&baseline.rows, &candidate.rows)?;
    for key in &report.missing {
        println!("missing {key}");
    }
    for key in &report.extra {
        println!("extra {key}");
    }
    for (before, after) in &report.changed {
        println!("changed {}", before.key());
        println!(
            "  before: passes={}{}{}{} error={:?} size={}x{} cfa={:?}",
            before.decode_pass,
            before.raw_mosaic_pass,
            before.process_metadata_pass,
            before.handoff_pass,
            before.error_kind,
            before.width,
            before.height,
            before.cfa
        );
        println!(
            "  after:  passes={}{}{}{} error={:?} size={}x{} cfa={:?}",
            after.decode_pass,
            after.raw_mosaic_pass,
            after.process_metadata_pass,
            after.handoff_pass,
            after.error_kind,
            after.width,
            after.height,
            after.cfa
        );
    }
    println!(
        "diff: {} missing, {} extra, {} changed",
        report.missing.len(),
        report.extra.len(),
        report.changed.len()
    );
    Ok(if report.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn baseline_row(manifest: &ManifestRow, stage: StageRow) -> BaselineRow {
    BaselineRow {
        vendor: manifest.vendor.clone(),
        camera: manifest.camera.clone(),
        format: normalize_repo_path(&manifest.format),
        path: manifest.path.clone(),
        decode_pass: stage.decode_pass,
        raw_mosaic_pass: stage.raw_mosaic_pass,
        process_metadata_pass: stage.process_metadata_pass,
        handoff_pass: stage.handoff_pass,
        error_kind: stage.error_kind,
        width: stage.width,
        height: stage.height,
        cfa: stage.cfa,
    }
}

fn read_file(path: &Path) -> Result<Vec<u8>> {
    fs::read(path)
        .map_err(|error| TrackerError::new(format!("reading {} failed: {error}", path.display())))
}

fn write_file(path: &Path, bytes: &[u8]) -> Result<()> {
    if let Some(parent) = path.parent() {
        fs::create_dir_all(parent)?;
    }
    fs::write(path, bytes)
        .map_err(|error| TrackerError::new(format!("writing {} failed: {error}", path.display())))
}
