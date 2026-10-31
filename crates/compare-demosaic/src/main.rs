use clap::{Parser, ValueEnum};
use compare_demosaic::results;
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
}

impl Args {
    const fn is_filtered(&self) -> bool { self.pattern.is_some() || self.algorithm.is_some() }
}

fn main() -> mischief::Result<()> {
    let args = Args::parse();
    let filtered = args.is_filtered();
    let summary = if filtered {
        results::generate_filtered(
            args.pattern.map(BayerPattern::from),
            args.algorithm.map(AlgorithmArg::case_name),
        )?
    } else {
        results::generate()?
    };
    let path = if filtered {
        summary.write_filtered()?
    } else {
        summary.write_default()?
    };
    println!("wrote {}", path.display());
    Ok(())
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn defaults_to_unfiltered_summary() {
        let args = Args::try_parse_from(["compare-demosaic"]).expect("default arguments");
        assert!(!args.is_filtered());
        assert!(args.pattern.is_none());
        assert!(args.algorithm.is_none());
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
}
