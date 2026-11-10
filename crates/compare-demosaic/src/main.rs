use std::io::Write;
use std::path::PathBuf;

use clap::{CommandFactory, Parser, Subcommand, ValueEnum};
use compare_demosaic::report::{self, Report};
use compare_demosaic::run::{self, RunOutput, RunRequest};
use compare_demosaic::{diff, readme};
use fornax_raw_core::BayerPattern;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum PatternArg {
    #[value(name = "RGGB")]
    Rggb,
    #[value(name = "BGGR")]
    Bggr,
    #[value(name = "GRBG")]
    Grbg,
    #[value(name = "GBRG")]
    Gbrg,
}

impl From<PatternArg> for BayerPattern {
    fn from(pattern: PatternArg) -> Self {
        match pattern {
            PatternArg::Rggb => Self::RGGB,
            PatternArg::Bggr => Self::BGGR,
            PatternArg::Grbg => Self::GRBG,
            PatternArg::Gbrg => Self::GBRG,
        }
    }
}

#[derive(Clone, Copy, Debug, Default, ValueEnum)]
enum OutputFormat {
    #[default]
    Text,
    Json,
}

#[derive(Debug, Subcommand)]
enum Command {
    /// Compute comparison rows and write canonical or shard JSON.
    Run(RunArgs),
    /// Compare a baseline JSON document with a new run.
    Diff(DiffArgs),
    /// Generate or verify the README regions and notes.
    Readme(ReadmeArgs),
}

#[derive(Debug, clap::Args)]
struct RunArgs {
    /// Restrict rows to an algorithm; repeat for more than one.
    #[arg(long)]
    algorithm: Vec<String>,
    /// Restrict rows to a reference; repeat for more than one.
    #[arg(long)]
    reference: Vec<String>,
    /// Restrict rows to a Bayer pattern; repeat for more than one.
    #[arg(long, value_enum, ignore_case = true)]
    pattern: Vec<PatternArg>,
    /// Use a RAW input; repeat for more than one. Defaults to the devtool
    /// fixture.
    #[arg(long)]
    input: Vec<PathBuf>,
    /// Directory for canonical JSON and artifacts.
    #[arg(long, default_value = "target/compare-demosaic")]
    out: PathBuf,
    /// Write <name>.shard.json instead of canonical.json.
    #[arg(long)]
    shard: Option<String>,
    /// Write a log-scaled error map and high-error tile summary for every row.
    #[arg(long)]
    artifacts: bool,
    /// Border width excluded from interior statistics.
    #[arg(long, default_value_t = report::BORDER_PX)]
    border_px: u32,
    /// Number of rows computed in parallel.
    #[arg(long, default_value_t = run::default_jobs())]
    jobs: usize,
    /// Output format written to stdout.
    #[arg(long, value_enum, default_value_t)]
    format: OutputFormat,
}

#[derive(Debug, clap::Args)]
struct DiffArgs {
    /// Accepted baseline JSON. It must be a full run.
    baseline: PathBuf,
    /// New run JSON. A partial run compares only its selected rows.
    new: PathBuf,
    /// Ignore numeric-only changes when choosing the exit code.
    #[arg(long)]
    verdict_only: bool,
    /// Output format written to stdout.
    #[arg(long, value_enum, default_value_t)]
    format: OutputFormat,
}

#[derive(Debug, clap::Args)]
struct ReadmeArgs {
    /// Fail instead of updating when the README is out of date.
    #[arg(long)]
    check: bool,
    /// Read a different full JSON report instead of the committed baseline.
    #[arg(long)]
    from: Option<PathBuf>,
}

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Compare Fornax demosaic implementations with LibRaw, darktable, and RawTherapee"
)]
struct Args {
    #[command(subcommand)]
    command: Command,
}

fn main() {
    match run() {
        Ok(0) => {}
        Ok(code) => std::process::exit(code),
        Err(error) => {
            eprintln!("error: {error}");
            std::process::exit(2);
        }
    }
}

fn run() -> mischief::Result<i32> {
    match Args::parse().command {
        Command::Run(arguments) => run_command(arguments),
        Command::Diff(arguments) => diff_command(arguments),
        Command::Readme(arguments) => readme_command(arguments),
    }
}

fn run_command(arguments: RunArgs) -> mischief::Result<i32> {
    let request = RunRequest {
        algorithms: arguments.algorithm,
        references: arguments.reference,
        patterns: arguments
            .pattern
            .into_iter()
            .map(BayerPattern::from)
            .collect(),
        inputs: arguments.input,
        out: arguments.out,
        shard: arguments.shard,
        artifacts: arguments.artifacts,
        border_px: arguments.border_px,
        jobs: arguments.jobs,
    };
    let output = run::execute(&request)?;
    match arguments.format {
        OutputFormat::Text => print!("{}", run::render_text(&output)),
        OutputFormat::Json => print_document(&output)?,
    }
    Ok(0)
}

fn diff_command(arguments: DiffArgs) -> mischief::Result<i32> {
    let baseline = Report::read(&arguments.baseline)?;
    let new = Report::read(&arguments.new)?;
    let report = diff::compare(&baseline, &new, arguments.verdict_only)?;
    report.append_github_summary()?;
    match arguments.format {
        OutputFormat::Text => print!("{}", report.render_text()),
        OutputFormat::Json => {
            let json = serde_json::to_string_pretty(&report)
                .map_err(|error| mischief::mischief!("cannot serialize diff: {error}"))?;
            println!("{json}");
        }
    }
    Ok(i32::from(report.exit_code()))
}

fn readme_command(arguments: ReadmeArgs) -> mischief::Result<i32> {
    let source = arguments.from.unwrap_or_else(report::baseline_path);
    let report = readme::load(&source)?;
    let usage = readme::render_clap_usage(&Args::command());
    if arguments.check {
        let result = readme::check(&report, &usage, None)?;
        print!("{}", result.render());
        Ok(i32::from(result.exit_code()))
    } else {
        let result = readme::write(&report, &usage, None)?;
        println!("README written");
        if !result.issues.is_empty() {
            print!("{}", result.render());
        }
        Ok(i32::from(result.exit_code()))
    }
}

fn print_document(output: &RunOutput) -> mischief::Result<()> {
    let document = std::fs::read(&output.path)?;
    std::io::stdout().write_all(&document)?;
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exposes_only_run_diff_and_readme() {
        let command = Args::command();
        let names = command
            .get_subcommands()
            .map(clap::Command::get_name)
            .collect::<Vec<_>>();
        assert_eq!(names, ["run", "diff", "readme"]);
    }

    #[test]
    fn run_options_are_repeatable_and_case_insensitive() {
        let args = Args::try_parse_from([
            "compare-demosaic",
            "run",
            "--algorithm",
            "ahd",
            "--algorithm",
            "rcd",
            "--reference",
            "LibRaw",
            "--pattern",
            "rGgB",
            "--input",
            "one.raw",
            "--input",
            "two.raw",
            "--jobs",
            "2",
        ])
        .expect("valid run");
        let Command::Run(run) = args.command else {
            panic!("expected run");
        };
        assert_eq!(run.algorithm, ["ahd", "rcd"]);
        assert_eq!(run.reference, ["LibRaw"]);
        assert_eq!(run.input.len(), 2);
        assert_eq!(run.jobs, 2);
    }

    #[test]
    fn committed_readme_matches_the_baseline_and_cli() -> mischief::Result<()> {
        let report = readme::load(&report::baseline_path())?;
        let usage = readme::render_clap_usage(&Args::command());
        let result = readme::check(&report, &usage, None)?;
        assert_eq!(result.exit_code(), 0, "{}", result.render());
        Ok(())
    }
}
