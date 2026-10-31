use std::fs::OpenOptions;
use std::io::Write;
use std::path::{Path, PathBuf};
use std::process::ExitCode;

use clap::{Args as ClapArgs, Parser, Subcommand, ValueEnum};
use compare_demosaic::diff::{self, FailOn};
use compare_demosaic::error::{CompareError, ErrorClass, Result};
use compare_demosaic::registry;
use compare_demosaic::results::{self, SCHEMA_FILE};
use compare_demosaic::run::{self, RunRequest};

#[derive(Clone, Copy, Debug, ValueEnum)]
enum PatternArg {
    Rggb,
    Bggr,
    Grbg,
    Gbrg,
}

impl PatternArg {
    const fn name(self) -> &'static str {
        match self {
            Self::Rggb => "RGGB",
            Self::Bggr => "BGGR",
            Self::Grbg => "GRBG",
            Self::Gbrg => "GBRG",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum AlgorithmArg {
    Ahd,
    Amaze,
    Aahd,
    Dcb,
    Dht,
    Linear,
    Lmmse,
    Ppg,
    Rcd,
    Vng,
}

impl AlgorithmArg {
    const fn name(self) -> &'static str {
        match self {
            Self::Ahd => "AHD",
            Self::Amaze => "AMaZE",
            Self::Aahd => "AAHD",
            Self::Dcb => "DCB",
            Self::Dht => "DHT",
            Self::Linear => "Linear",
            Self::Lmmse => "LMMSE",
            Self::Ppg => "PPG",
            Self::Rcd => "RCD",
            Self::Vng => "VNG",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum ReferenceArg {
    Libraw,
    Darktable,
}

impl ReferenceArg {
    const fn name(self) -> &'static str {
        match self {
            Self::Libraw => "libraw",
            Self::Darktable => "darktable",
        }
    }
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum FailOnArg {
    #[default]
    Regression,
    Any,
    Never,
}

impl From<FailOnArg> for FailOn {
    fn from(value: FailOnArg) -> Self {
        match value {
            FailOnArg::Regression => Self::Regression,
            FailOnArg::Any => Self::Any,
            FailOnArg::Never => Self::Never,
        }
    }
}

#[derive(Clone, Debug, ClapArgs)]
struct RunArgs {
    /// Restrict the run to an algorithm; repeat for multiple algorithms.
    #[arg(long = "algorithm", value_enum, ignore_case = true)]
    algorithms: Vec<AlgorithmArg>,

    /// Restrict the run to a reference; repeat for multiple references.
    #[arg(long = "reference", value_enum, ignore_case = true)]
    references: Vec<ReferenceArg>,

    /// Restrict the run to a Bayer pattern; repeat for multiple patterns.
    #[arg(long = "pattern", value_enum, ignore_case = true)]
    patterns: Vec<PatternArg>,

    /// Raw input file. Defaults to the repository-managed canonical fixture.
    #[arg(long)]
    input: Option<PathBuf>,

    /// Directory for canonical.json, shards, timings, and artifacts.
    #[arg(long, default_value = "target/compare-demosaic")]
    out: PathBuf,

    /// Write a partial evidence file named <shard>.shard.json.
    #[arg(long)]
    shard: Option<String>,

    /// Write error maps and tile summaries for every selected row.
    #[arg(long)]
    artifacts: bool,

    /// Border excluded from metrics_interior.
    #[arg(long, default_value_t = results::default_border_px())]
    border_px: u32,

    /// Maximum number of rows computed concurrently.
    #[arg(long, default_value_t = default_jobs())]
    jobs: usize,

    /// Output format for command status.
    #[arg(long, value_enum, default_value_t)]
    format: OutputFormat,
}

impl RunArgs {
    fn request(&self) -> RunRequest {
        RunRequest {
            algorithms: self
                .algorithms
                .iter()
                .map(|value| value.name().to_owned())
                .collect(),
            references: self
                .references
                .iter()
                .map(|value| value.name().to_owned())
                .collect(),
            patterns: self
                .patterns
                .iter()
                .map(|value| value.name().to_owned())
                .collect(),
            input: self.input.clone(),
            output_dir: self.out.clone(),
            shard: self.shard.clone(),
            artifacts: self.artifacts,
            border_px: self.border_px,
            jobs: self.jobs,
        }
    }
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Compute comparison rows into canonical.json or one shard.
    Run(RunArgs),
    /// Combine partial shards into canonical evidence.
    Merge {
        shards: Vec<PathBuf>,
        #[arg(long)]
        out: PathBuf,
        #[arg(long)]
        allow_partial: bool,
        #[arg(long)]
        check_artifacts: bool,
        #[arg(long, value_enum, default_value_t)]
        format: OutputFormat,
    },
    /// Compare two evidence files and fail on verdict regressions.
    Diff {
        old: PathBuf,
        new: PathBuf,
        #[arg(long, value_enum, default_value_t)]
        fail_on: FailOnArg,
        #[arg(long, value_enum, default_value_t)]
        format: OutputFormat,
        /// Append a Markdown report to this path.
        #[arg(long)]
        md_out: Option<PathBuf>,
        /// Restrict changes to rows native to the input CFA.
        #[arg(long)]
        only_native: bool,
    },
    /// Validate schema shape and semantic consistency.
    Validate {
        path: PathBuf,
        #[arg(long)]
        partial: bool,
        #[arg(long)]
        check_artifacts: bool,
        #[arg(long, value_enum, default_value_t)]
        format: OutputFormat,
    },
    /// Generate or verify the committed JSON Schema.
    Schema {
        #[arg(long)]
        check: bool,
    },
    /// Generate or verify README tables from the committed baseline.
    Readme {
        #[arg(long)]
        check: bool,
        /// Canonical evidence to render instead of the committed baseline.
        #[arg(long)]
        from: Option<PathBuf>,
    },
    /// Accept a full, clean, valid run as the new baseline.
    Bless {
        path: PathBuf,
        #[arg(long)]
        yes: bool,
    },
    /// Print the expected algorithm/reference/pattern matrix.
    List {
        #[arg(long, value_enum, default_value_t)]
        format: OutputFormat,
    },
}

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Compare Fornax demosaic implementations with reference implementations",
    args_conflicts_with_subcommands = true
)]
struct Args {
    #[command(flatten)]
    run: RunArgs,

    /// Legacy alias for running and blessing in one command.
    #[arg(long, hide = true)]
    bless: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

fn main() -> ExitCode {
    let args = match Args::try_parse() {
        Ok(args) => args,
        Err(error)
            if matches!(
                error.kind(),
                clap::error::ErrorKind::DisplayHelp | clap::error::ErrorKind::DisplayVersion
            ) =>
        {
            let _ = error.print();
            return ExitCode::SUCCESS;
        }
        Err(error) => {
            let _ = error.print();
            return ExitCode::from(ErrorClass::Usage.exit_code());
        }
    };
    match dispatch(args) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            eprintln!("error: {error}");
            ExitCode::from(error.class().exit_code())
        }
    }
}

fn dispatch(args: Args) -> Result<u8> {
    match args.command {
        Some(Command::Run(run_args)) => run_command(run_args),
        Some(Command::Merge {
            shards,
            out,
            allow_partial,
            check_artifacts,
            format,
        }) => merge_command(&shards, &out, allow_partial, check_artifacts, format),
        Some(Command::Diff {
            old,
            new,
            fail_on,
            format,
            md_out,
            only_native,
        }) => diff_command(
            &old,
            &new,
            fail_on.into(),
            format,
            md_out.as_deref(),
            only_native,
        ),
        Some(Command::Validate {
            path,
            partial,
            check_artifacts,
            format,
        }) => validate_command(&path, partial, check_artifacts, format),
        Some(Command::Schema { check }) => schema_command(check),
        Some(Command::Readme { check, from }) => readme_command(check, from.as_deref()),
        Some(Command::Bless { path, yes }) => bless_command(&path, yes),
        Some(Command::List { format }) => list_command(format),
        None => {
            let run_args = args.run;
            if args.bless {
                let output = run::execute(&run_args.request())?;
                emit_run(&output, run_args.format);
                bless_command(&output.path, true)
            } else {
                run_command(run_args)
            }
        }
    }
}

fn run_command(args: RunArgs) -> Result<u8> {
    let format = args.format;
    let output = run::execute(&args.request())?;
    emit_run(&output, format);
    Ok(0)
}

fn emit_run(output: &compare_demosaic::run::RunOutput, format: OutputFormat) {
    match format {
        OutputFormat::Text => {
            eprintln!(
                "wrote {} ({} rows, {} coverage)",
                output.path.display(),
                output.results.results.len(),
                match output.results.meta.run.coverage {
                    results::Coverage::Full => "full",
                    results::Coverage::Partial => "partial",
                }
            );
        }
        OutputFormat::Json => {
            println!(
                "{}",
                serde_json::json!({
                    "path": output.path,
                    "rows": output.results.results.len(),
                    "coverage": output.results.meta.run.coverage,
                    "total_ms": output.timings.total_ms,
                })
            );
        }
    }
}

fn merge_command(
    shards: &[PathBuf],
    out: &Path,
    allow_partial: bool,
    check_artifacts: bool,
    format: OutputFormat,
) -> Result<u8> {
    let report = compare_demosaic::merge::merge(shards, out, allow_partial, check_artifacts)?;
    match format {
        OutputFormat::Text => eprintln!(
            "merged {} rows into {} ({})",
            report.rows,
            report.path.display(),
            if report.complete { "full" } else { "partial" }
        ),
        OutputFormat::Json => println!(
            "{}",
            serde_json::json!({
                "path": report.path,
                "rows": report.rows,
                "coverage": if report.complete { "full" } else { "partial" },
            })
        ),
    }
    Ok(0)
}

fn diff_command(
    old: &Path,
    new: &Path,
    fail_on: FailOn,
    format: OutputFormat,
    md_out: Option<&Path>,
    only_native: bool,
) -> Result<u8> {
    let report = diff::compare_files(old, new, fail_on, only_native)?;
    match format {
        OutputFormat::Text => eprintln!("{}", report.render_text()),
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string(&report)
                .map_err(|error| CompareError::usage(error.to_string()))?
        ),
    }
    if let Some(path) = md_out {
        let mut file = OpenOptions::new().create(true).append(true).open(path)?;
        writeln!(file, "{}", report.render_markdown())?;
    }
    Ok(report.exit_code)
}

fn validate_command(
    path: &Path,
    partial: bool,
    check_artifacts: bool,
    format: OutputFormat,
) -> Result<u8> {
    let report = compare_demosaic::validate::validate_file(path, partial, check_artifacts)?;
    match format {
        OutputFormat::Text => eprintln!("valid {} ({} rows)", path.display(), report.rows),
        OutputFormat::Json => println!(
            "{}",
            serde_json::json!({
                "valid": true,
                "path": path,
                "rows": report.rows,
            })
        ),
    }
    Ok(0)
}

fn schema_command(check: bool) -> Result<u8> {
    let path = results::repo_path(SCHEMA_FILE);
    let expected =
        results::schema_json().map_err(|error| CompareError::usage(error.to_string()))?;
    if check {
        let actual = std::fs::read_to_string(&path)?;
        if actual != expected {
            return Err(CompareError::check(format!(
                "{} is out of date; run `compare-demosaic schema`",
                path.display()
            )));
        }
        eprintln!("schema up to date");
    } else {
        std::fs::write(&path, expected)?;
        eprintln!("schema written to {}", path.display());
    }
    Ok(0)
}

fn readme_command(check: bool, from: Option<&Path>) -> Result<u8> {
    if check {
        compare_demosaic::render::check(from)
            .map_err(|error| CompareError::check(error.to_string()))?;
        eprintln!("README up to date");
    } else {
        compare_demosaic::render::write(from)
            .map_err(|error| CompareError::usage(error.to_string()))?;
        eprintln!("README written");
    }
    Ok(0)
}

fn bless_command(path: &Path, yes: bool) -> Result<u8> {
    let report = compare_demosaic::bless::bless(path, yes)?;
    eprintln!(
        "blessed {} and regenerated README tables",
        report.baseline.display()
    );
    Ok(0)
}

fn list_command(format: OutputFormat) -> Result<u8> {
    let rows = registry::expected_rows();
    match format {
        OutputFormat::Text => {
            for row in &rows {
                eprintln!("{:<8} {:<10} {}", row.algorithm, row.reference, row.pattern);
            }
            eprintln!("{} expected rows", rows.len());
        }
        OutputFormat::Json => println!(
            "{}",
            serde_json::to_string(&rows).map_err(|error| CompareError::usage(error.to_string()))?
        ),
    }
    Ok(0)
}

fn default_jobs() -> usize {
    std::thread::available_parallelism()
        .map_or(1, std::num::NonZeroUsize::get)
        .min(4)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_unfiltered_canonical_run() {
        let args = Args::try_parse_from(["compare-demosaic"]).expect("default arguments");
        assert!(args.run.algorithms.is_empty());
        assert!(args.run.references.is_empty());
        assert!(args.run.patterns.is_empty());
        assert!(args.command.is_none());
    }

    #[test]
    fn parses_repeatable_filters() {
        let args = Args::try_parse_from([
            "compare-demosaic",
            "run",
            "--pattern",
            "gbRg",
            "--algorithm",
            "AmAzE",
            "--algorithm",
            "ahd",
            "--reference",
            "libraw",
            "--shard",
            "scratch",
        ])
        .expect("valid filters");
        let Some(Command::Run(run)) = args.command else {
            panic!("expected run");
        };
        assert_eq!(run.patterns[0].name(), "GBRG");
        assert_eq!(run.algorithms.len(), 2);
        assert_eq!(run.references[0].name(), "libraw");
    }
}
