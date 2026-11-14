use std::fs;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args, Parser, Subcommand};
use clap_verbosity_flag::InfoLevel;
use clerk::tracing_subscriber::layer::SubscriberExt;
use clerk::tracing_subscriber::util::SubscriberInitExt;
use clerk::tracing_subscriber::{EnvFilter, Layer, Registry};
use decoder_tracker::csvio::{
    read_baseline_csv, read_manifest_csv, write_baseline_csv, write_manifest_csv,
};
use decoder_tracker::duckdb::{diff as duckdb_diff, merge as duckdb_merge};
use decoder_tracker::golden::{
    build_golden_coverage, check_golden_csv, decode_golden, read_golden_csv,
    write_golden_coverage_csv, write_golden_csv, write_golden_selection_csv,
};
use decoder_tracker::manifest::{
    ManifestCheck, corpus_path, generate_manifest, normalize_repo_path,
};
use decoder_tracker::model::{BaselineRow, ManifestRow, Result, TrackerError, status_for_stages};
use decoder_tracker::runner::{materialize, run_isolation, timeout_from_env};
use decoder_tracker::selection::{Selection, Shard, select_manifest};
use decoder_tracker::stages::{StageRow, evaluate_one};

const DEFAULT_CORPUS: &str = "external/raw.pixls.us";
const DEFAULT_MANIFEST: &str = "crates/decoder-tracker/data/manifest.csv";
const DEFAULT_OUTPUT: &str = "target/decoder-tracker/corpus.csv";
const DEFAULT_BASELINE: &str = "crates/decoder-tracker/data/baseline.csv";
const DEFAULT_GOLDEN: &str = "crates/decoder-tracker/golden/selected-manifest.csv";
const DEFAULT_GOLDEN_COVERAGE: &str = "target/decoder-tracker/golden-coverage.csv";

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Track decoder results across the raw.pixls.us corpus"
)]
struct Cli {
    #[command(subcommand)]
    command: Command,
    #[command(flatten)]
    verbose: clap_verbosity_flag::Verbosity<InfoLevel>,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Generate or verify the full corpus manifest.
    Manifest(ManifestArgs),
    /// Write and count the files matched by the requested filters.
    Select(SelectArgs),
    /// Materialize and run selected corpus files in isolated children.
    Run(RunArgs),
    /// Merge shard CSVs and reject missing or duplicate rows.
    Merge(MergeArgs),
    /// Compare two canonical baseline CSVs.
    Diff(DiffArgs),
    /// Update selected records in a full baseline.
    Update(UpdateArgs),
    /// Write or verify canonical decoder-output hashes.
    Hash(HashArgs),
    /// Report golden corpus coverage by source encoding.
    GoldenCoverage(GoldenCoverageArgs),
    /// Evaluate one materialized file for the run parent process.
    #[command(hide = true)]
    RunOne(RunOneArgs),
}

#[derive(Debug, Args)]
struct ManifestArgs {
    #[arg(long, default_value = DEFAULT_CORPUS)]
    corpus: PathBuf,
    #[arg(long)]
    out: Option<PathBuf>,
    #[arg(long)]
    check: bool,
}

#[derive(Debug, Args)]
struct SelectArgs {
    #[command(flatten)]
    filter: FilterArgs,
    #[arg(long, default_value = DEFAULT_MANIFEST)]
    manifest: PathBuf,
    #[arg(long, default_value = "target/decoder-tracker/selected-manifest.csv")]
    out: PathBuf,
}

#[derive(Clone, Debug, Args)]
struct FilterArgs {
    #[arg(long, default_value = DEFAULT_CORPUS)]
    corpus: PathBuf,
    #[arg(long = "camera", action = clap::ArgAction::Append)]
    cameras: Vec<String>,
    #[arg(long = "filename", action = clap::ArgAction::Append)]
    filenames: Vec<String>,
    #[arg(long = "glob", action = clap::ArgAction::Append)]
    globs: Vec<String>,
}

impl FilterArgs {
    fn selection(&self, shard: Option<Shard>) -> Selection {
        Selection {
            cameras: self.cameras.clone(),
            filenames: self.filenames.clone(),
            globs: self.globs.clone(),
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
    #[arg(long)]
    selected: PathBuf,
    #[arg(long, default_value = "target/decoder-tracker/diff.csv")]
    out: PathBuf,
}

#[derive(Debug, Args)]
struct UpdateArgs {
    #[arg(long)]
    baseline: Option<PathBuf>,
    #[arg(long)]
    candidate: PathBuf,
    #[arg(long, default_value = "target/decoder-tracker/baseline.csv")]
    out: PathBuf,
}

#[derive(Debug, Args)]
struct HashArgs {
    #[arg(long, default_value = DEFAULT_CORPUS)]
    corpus: PathBuf,
    #[arg(
        long,
        default_value = "crates/decoder-tracker/golden/selected-manifest.csv"
    )]
    manifest: PathBuf,
    #[arg(long, conflicts_with = "check")]
    out: Option<PathBuf>,
    #[arg(long, conflicts_with = "out")]
    check: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct GoldenCoverageArgs {
    #[arg(long, default_value = DEFAULT_BASELINE)]
    baseline: PathBuf,
    #[arg(long, default_value = DEFAULT_MANIFEST)]
    corpus_manifest: PathBuf,
    #[arg(long, default_value = DEFAULT_GOLDEN)]
    golden: PathBuf,
    #[arg(long, default_value = DEFAULT_GOLDEN_COVERAGE)]
    out: PathBuf,
    #[arg(long)]
    selected_out: Option<PathBuf>,
}

#[derive(Debug, Args)]
struct RunOneArgs {
    #[arg(long)]
    input: PathBuf,
}

fn main() -> ExitCode {
    let cli = Cli::parse();
    if let Err(error) = init_logging(&cli.verbose) {
        eprintln!("error: {error}");
        return ExitCode::FAILURE;
    }
    match execute(cli) {
        Ok(code) => code,
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::FAILURE
        }
    }
}

fn init_logging(verbose: &clap_verbosity_flag::Verbosity<InfoLevel>) -> Result<()> {
    let directive = format!("decoder_tracker={}", verbose.filter())
        .parse()
        .map_err(|error| TrackerError::new(format!("invalid log filter: {error}")))?;
    Registry::default()
        .with(
            clerk::terminal_layer(true).with_filter(
                EnvFilter::builder()
                    .with_default_directive(directive)
                    .from_env_lossy(),
            ),
        )
        .init();
    Ok(())
}

fn execute(cli: Cli) -> Result<ExitCode> {
    match cli.command {
        Command::Manifest(args) => manifest(args),
        Command::Select(args) => select(args),
        Command::Run(args) => run(args),
        Command::Merge(args) => merge(args),
        Command::Diff(args) => diff(args),
        Command::Update(args) => update(args),
        Command::Hash(args) => hash(args),
        Command::GoldenCoverage(args) => golden_coverage(args),
        Command::RunOne(args) => {
            println!("{}", serde_json::to_string(&evaluate_one(&args.input))?);
            Ok(ExitCode::SUCCESS)
        }
    }
}

fn select(args: SelectArgs) -> Result<ExitCode> {
    let rows = read_manifest_csv(read_file(&args.manifest)?)?;
    let selected = select_manifest(&rows, &args.filter.selection(None))?;
    if selected.is_empty() {
        return Err(TrackerError::new("filters matched no corpus files"));
    }
    let bytes = write_manifest_csv(&selected)?;
    write_file(&args.out, &bytes)?;
    clerk::info!("selected {} corpus files", selected.len());
    println!("selected={}", selected.len());
    Ok(ExitCode::SUCCESS)
}

fn manifest(args: ManifestArgs) -> Result<ExitCode> {
    let rows = generate_manifest(&args.corpus)?;
    let bytes = write_manifest_csv(&rows)?;
    let out = args.out.unwrap_or_else(|| PathBuf::from(DEFAULT_MANIFEST));
    if args.check {
        let committed = fs::read(&out).map_err(|error| {
            TrackerError::new(format!(
                "reading committed manifest {} failed: {error}",
                out.display()
            ))
        })?;
        ManifestCheck::against(&args.corpus, &committed)?;
        clerk::info!("manifest is current with {} rows", rows.len());
        println!("manifest: current ({} rows)", rows.len());
    } else {
        write_file(&out, &bytes)?;
        clerk::info!("wrote {} manifest rows to {}", rows.len(), out.display());
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
    let selection = args.filter.selection(args.shard);
    let selected = select_manifest(&rows, &selection)?;
    let ci = std::env::var("CI").is_ok_and(|value| value == "true");
    if !args.allow_full && !ci && selected.len() > args.max_files {
        return Err(TrackerError::new(format!(
            "selection has {} files but --max-files is {}; use filters, a shard, CI=true, or --allow-full",
            selected.len(),
            args.max_files
        )));
    }

    materialize(&args.filter.corpus, &selected)?;
    clerk::info!("executing {} selected corpus files", selected.len());
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
        if stage.error_kind.is_empty() {
            clerk::info!("decoder tracked {} successfully", row.path);
        } else {
            clerk::warn!("decoder tracked {} as {}", row.path, stage.error_kind);
        }
        output.push(baseline_row(row, stage));
    }
    let bytes = write_baseline_csv(&output)?;
    write_file(&args.out, &bytes)?;
    clerk::info!(
        "recorded {} shard rows in {}",
        output.len(),
        args.out.display()
    );
    println!(
        "run: recorded {} rows in {}",
        output.len(),
        args.out.display()
    );
    Ok(ExitCode::SUCCESS)
}

fn merge(args: MergeArgs) -> Result<ExitCode> {
    let rows = duckdb_merge(&args.manifest, &args.shards, &args.out)?;
    clerk::info!("DuckDB merged {rows} shard rows");
    println!(
        "merge: wrote {rows} complete rows to {}",
        args.out.display()
    );
    Ok(ExitCode::SUCCESS)
}

fn diff(args: DiffArgs) -> Result<ExitCode> {
    let report = duckdb_diff(&args.baseline, &args.candidate, &args.selected, &args.out)?;
    clerk::info!(
        "DuckDB diff found {} missing, {} extra, {} changed rows",
        report.missing,
        report.extra,
        report.changed
    );
    println!(
        "diff: {} missing, {} extra, {} changed",
        report.missing, report.extra, report.changed
    );
    Ok(if report.is_empty() {
        ExitCode::SUCCESS
    } else {
        ExitCode::FAILURE
    })
}

fn update(args: UpdateArgs) -> Result<ExitCode> {
    let rows =
        decoder_tracker::duckdb::upsert(args.baseline.as_deref(), &args.candidate, &args.out)?;
    clerk::info!("DuckDB upserted {rows} baseline rows");
    println!("update: wrote {rows} rows to {}", args.out.display());
    Ok(ExitCode::SUCCESS)
}

fn hash(args: HashArgs) -> Result<ExitCode> {
    let rows = read_manifest_csv(read_file(&args.manifest)?)?;
    materialize(&args.corpus, &rows)?;
    let rows = rows
        .iter()
        .map(|row| {
            let path = fs::canonicalize(corpus_path(&args.corpus, &row.path)).map_err(|error| {
                TrackerError::new(format!(
                    "resolving golden corpus file {} failed: {error}",
                    row.path
                ))
            })?;
            decode_golden(&row.path, &path)
        })
        .collect::<Result<Vec<_>>>()?;
    let bytes = write_golden_csv(&rows)?;

    if let Some(expected) = args.check {
        let expected = read_golden_csv(read_file(&expected)?)?;
        check_golden_csv(&rows, &expected)?;
        clerk::info!("decoder golden hashes are unchanged");
        println!("golden: unchanged ({} rows)", rows.len());
    } else if let Some(output) = args.out {
        write_file(&output, &bytes)?;
        clerk::info!("wrote decoder golden hashes to {}", output.display());
        println!("golden: wrote {} rows to {}", rows.len(), output.display());
    } else {
        print!("{}", String::from_utf8_lossy(&bytes));
    }
    Ok(ExitCode::SUCCESS)
}

fn golden_coverage(args: GoldenCoverageArgs) -> Result<ExitCode> {
    let baseline = read_baseline_csv(read_file(&args.baseline)?)?;
    let corpus_manifest = read_manifest_csv(read_file(&args.corpus_manifest)?)?;
    let golden_manifest = read_manifest_csv(read_file(&args.golden)?)?;
    let report = build_golden_coverage(&baseline.rows, &corpus_manifest, &golden_manifest)?;
    let bytes = write_golden_coverage_csv(&report.rows)?;
    write_file(&args.out, &bytes)?;
    if let Some(path) = args.selected_out {
        let selected = write_golden_selection_csv(&report.rows)?;
        write_file(&path, &selected)?;
        println!(
            "golden-coverage: wrote {} selected inputs to {}",
            report.rows.len(),
            path.display()
        );
    }
    clerk::info!(
        "golden coverage has {} covered, {} missing, and {} redundant groups",
        report.summary.covered,
        report.summary.missing,
        report.summary.redundant
    );
    println!(
        "golden-coverage: {} covered, {} missing, {} redundant -> {}",
        report.summary.covered,
        report.summary.missing,
        report.summary.redundant,
        args.out.display()
    );
    Ok(ExitCode::SUCCESS)
}

fn baseline_row(manifest: &ManifestRow, stage: StageRow) -> BaselineRow {
    let passes = [
        stage.decode_pass,
        stage.raw_mosaic_pass,
        stage.process_metadata_pass,
        stage.handoff_pass,
    ];
    BaselineRow {
        status: status_for_stages(passes).to_owned(),
        camera: manifest.camera.clone(),
        format: normalize_repo_path(&manifest.format),
        path: manifest.path.clone(),
        decode_pass: stage.decode_pass,
        raw_mosaic_pass: stage.raw_mosaic_pass,
        raw_sha256: stage.raw_sha256,
        process_metadata_pass: stage.process_metadata_pass,
        handoff_pass: stage.handoff_pass,
        error_kind: stage.error_kind,
        width: stage.width,
        height: stage.height,
        cfa: stage.cfa,
        compression_id: stage.compression_id,
        compression_name: stage.compression_name,
        compression_lossless: stage.compression_lossless,
        storage: stage.storage,
        bits_per_sample: stage.bits_per_sample,
        sample_format: stage.sample_format,
        predictor: stage.predictor,
        rows_per_strip: stage.rows_per_strip,
        tile_width: stage.tile_width,
        tile_height: stage.tile_height,
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
