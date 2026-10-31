use std::io::Write;
use std::process::ExitCode;

use clap::{Parser, ValueEnum};
use compare_demosaic::check;
use compare_demosaic::error::{CompareError, Result};
use compare_demosaic::report::BASELINE_FILE;
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
    #[arg(long, conflicts_with_all = ["algorithm", "check"])]
    update: bool,
}

fn main() -> ExitCode {
    match run_cli(Args::parse()) {
        Ok(code) => ExitCode::from(code),
        Err(error) => {
            let _ = writeln!(std::io::stderr().lock(), "error: {error}");
            ExitCode::from(2)
        }
    }
}

fn run_cli(args: Args) -> Result<u8> {
    let algorithm_names = args
        .algorithm
        .iter()
        .map(|algorithm| algorithm.name().to_owned())
        .collect::<Vec<_>>();
    let request = RunRequest {
        algorithms: algorithm_names.clone(),
        input: None,
        border_px: 16,
    };
    let output = run::execute(&request).map_err(|error| CompareError::usage(error.to_string()))?;
    if args.update {
        output
            .report
            .write_baseline()
            .map_err(|error| CompareError::usage(error.to_string()))?;
        compare_demosaic::results_md::write(&output.report)
            .map_err(|error| CompareError::usage(error.to_string()))?;
        writeln!(std::io::stderr().lock(), "updated baseline")?;
    } else if args.check {
        let baseline = run::repo_path(BASELINE_FILE);
        let report = check::check_file(&baseline, &output.report, &algorithm_names)
            .map_err(|error| CompareError::usage(error.to_string()))?;
        report
            .append_github_summary()
            .map_err(|error| CompareError::usage(error.to_string()))?;
        std::io::stdout()
            .lock()
            .write_all(report.render_text().as_bytes())?;
        return Ok(report.exit_code());
    }
    writeln!(
        std::io::stderr().lock(),
        "wrote {} ({} rows)",
        output.path.display(),
        output.report.rows.len()
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
