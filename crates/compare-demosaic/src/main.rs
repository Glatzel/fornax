use std::path::{Path, PathBuf};

use clap::{Parser, Subcommand, ValueEnum};
use compare_demosaic::results::{self, CanonicalResults, SCHEMA_FILE};
use compare_demosaic::{diff, render};
use fornax_core::BayerPattern;

#[derive(Clone, Copy, Debug, ValueEnum)]
enum PatternArg {
    Rggb,
    Bggr,
    Grbg,
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
    const fn case_name(self) -> &'static str {
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

#[derive(Debug, Subcommand)]
enum Command {
    /// Compare two canonical JSON files and fail on verdict regressions.
    Diff { old: PathBuf, new: PathBuf },
    /// Validate a canonical JSON file against the Rust schema contract.
    Validate { path: PathBuf },
    /// Generate or verify the committed JSON Schema.
    Schema {
        /// Fail if the committed schema differs instead of updating it.
        #[arg(long)]
        check: bool,
    },
    /// Generate or verify README tables from the committed baseline.
    Readme {
        /// Fail if the committed README differs instead of updating it.
        #[arg(long)]
        check: bool,
    },
}

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Compare Fornax demosaic implementations with reference implementations"
)]
struct Args {
    /// Restrict the comparison to one Bayer pattern.
    #[arg(long, value_enum, ignore_case = true)]
    pattern: Option<PatternArg>,

    /// Restrict the comparison to one algorithm name.
    #[arg(long, value_enum, ignore_case = true)]
    algorithm: Option<AlgorithmArg>,

    /// Write error-map and tile artifacts for every row.
    #[arg(long)]
    artifacts: bool,

    /// Replace the committed baseline with this canonical run.
    #[arg(long)]
    bless: bool,

    #[command(subcommand)]
    command: Option<Command>,
}

impl Args {
    const fn is_filtered(&self) -> bool { self.pattern.is_some() || self.algorithm.is_some() }
}

fn main() -> mischief::Result<()> {
    let args = Args::parse();
    match args.command {
        Some(Command::Diff { old, new }) => run_diff(&old, &new),
        Some(Command::Validate { path }) => {
            CanonicalResults::read(&path)?;
            println!("valid {}", path.display());
            Ok(())
        }
        Some(Command::Schema { check }) => run_schema(check),
        Some(Command::Readme { check }) => run_readme(check),
        None => run_generation(args),
    }
}

fn run_generation(args: Args) -> mischief::Result<()> {
    if args.bless && args.is_filtered() {
        return Err(mischief::mischief!(
            "--bless requires an unfiltered canonical run"
        ));
    }
    let results = results::generate_filtered(
        args.pattern.map(BayerPattern::from),
        args.algorithm.map(AlgorithmArg::case_name),
        args.artifacts,
    )?;
    let path = if args.bless {
        let canonical = results.write_default()?;
        let baseline = results.write_baseline()?;
        println!("blessed {}", baseline.display());
        canonical
    } else if args.is_filtered() {
        results.write_filtered()?
    } else {
        results.write_default()?
    };
    println!("wrote {}", path.display());
    Ok(())
}

fn run_diff(old: &Path, new: &Path) -> mischief::Result<()> {
    let old = CanonicalResults::read(old)?;
    let new = CanonicalResults::read(new)?;
    let report = diff::compare(&old, &new);
    for line in &report.lines {
        println!("{line}");
    }
    println!(
        "{} regressions, {} improvements",
        report.regressions, report.improvements
    );
    if report.regressions > 0 {
        std::process::exit(1);
    }
    Ok(())
}

fn run_schema(check: bool) -> mischief::Result<()> {
    let path = results::repo_path(SCHEMA_FILE);
    let expected = results::schema_json()?;
    if check {
        let actual = std::fs::read_to_string(&path)?;
        if actual != expected {
            return Err(mischief::mischief!(
                "{} is out of date; run `cargo run -p compare-demosaic -- schema`",
                path.display()
            ));
        }
    } else {
        std::fs::write(&path, expected)?;
    }
    println!("schema {}", if check { "up to date" } else { "written" });
    Ok(())
}

fn run_readme(check: bool) -> mischief::Result<()> {
    if check {
        render::check_baseline()?;
        println!("README up to date");
    } else {
        render::write_baseline()?;
        println!("README written");
    }
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_unfiltered_canonical_run() {
        let args = Args::try_parse_from(["compare-demosaic"]).expect("default arguments");
        assert!(!args.is_filtered());
        assert!(!args.artifacts);
        assert!(!args.bless);
        assert!(args.command.is_none());
    }

    #[test]
    fn parses_case_insensitive_filters() {
        let args = Args::try_parse_from([
            "compare-demosaic",
            "--pattern",
            "gbRg",
            "--algorithm",
            "AmAzE",
        ])
        .expect("valid filters");
        assert!(args.is_filtered());
        assert_eq!(
            args.pattern.map(BayerPattern::from),
            Some(BayerPattern::GBRG)
        );
        assert_eq!(args.algorithm, Some(AlgorithmArg::Amaze));
    }

    #[test]
    fn parses_diff_subcommand() {
        let args =
            Args::try_parse_from(["compare-demosaic", "diff", "old.json", "new.json"]).unwrap();
        assert!(matches!(
            args.command,
            Some(Command::Diff { old: _, new: _ })
        ));
    }
}
