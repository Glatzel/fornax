use std::io::Write;
use std::path::PathBuf;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use compare_demosaic::error::{CompareError, Result};
use compare_demosaic::run::{self, RunRequest};

#[derive(Clone, Copy, Debug, Eq, PartialEq, ValueEnum)]
enum AlgorithmArg {
    Aahd,
    Ahd,
    Amaze,
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
            Self::Aahd => "AAHD",
            Self::Ahd => "AHD",
            Self::Amaze => "AMaZE",
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

#[derive(Debug, Parser)]
#[command(
    version,
    about = "Compare Fornax demosaic implementations with reference implementations"
)]
struct Args {
    /// Restrict the comparison to an algorithm; repeat for multiple algorithms.
    #[arg(long, value_enum, ignore_case = true)]
    algorithm: Vec<AlgorithmArg>,

    /// Compare the run with the committed baseline.
    #[arg(long, conflicts_with = "update")]
    check: bool,

    /// Replace the committed baseline and RESULTS.md with this run.
    #[arg(long, conflicts_with = "check")]
    update: bool,
}

fn main() -> ExitCode {
    match run_cli(Args::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            let _ = writeln!(std::io::stderr().lock(), "error: {error}");
            ExitCode::from(error.class().exit_code())
        }
    }
}

fn run_cli(args: Args) -> Result<u8> {
    if args.check || args.update {
        let mode = if args.check { "--check" } else { "--update" };
        return Err(CompareError::usage(format!(
            "{mode} is implemented in the next simplification step"
        )));
    }

    let output_dir = PathBuf::from("target/compare-demosaic");
    let algorithms = args
        .algorithm
        .iter()
        .map(|algorithm| algorithm.name().to_owned())
        .collect::<Vec<_>>();
    let request = RunRequest {
        algorithms,
        references: Vec::new(),
        patterns: Vec::new(),
        input: None,
        output_dir,
        shard: (!args.algorithm.is_empty()).then(|| "filtered".to_owned()),
        artifacts: false,
        border_px: 16,
        jobs: std::thread::available_parallelism()
            .map_or(1, std::num::NonZeroUsize::get)
            .min(4),
    };
    let output = run::execute(&request)?;
    writeln!(
        std::io::stderr().lock(),
        "wrote {} ({} rows)",
        output.path.display(),
        output.results.results.len()
    )?;
    Ok(0)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn parses_repeatable_algorithm_filters() {
        let args = Args::try_parse_from([
            "compare-demosaic",
            "--algorithm",
            "ahd",
            "--algorithm",
            "AmAzE",
        ])
        .expect("valid filters");
        assert_eq!(args.algorithm, vec![AlgorithmArg::Ahd, AlgorithmArg::Amaze]);
    }

    #[test]
    fn check_and_update_are_mutually_exclusive() {
        assert!(Args::try_parse_from(["compare-demosaic", "--check", "--update"]).is_err());
    }
}
