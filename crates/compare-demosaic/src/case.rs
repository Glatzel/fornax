use fornax_core::BayerPattern;

#[derive(Clone, Copy, Debug, PartialEq, Eq, PartialOrd, Ord)]
pub enum ReferenceBackend {
    LibRaw,
    Darktable,
}

impl ReferenceBackend {
    pub const fn library(self) -> &'static str {
        match self {
            Self::LibRaw => "LibRaw",
            Self::Darktable => "darktable",
        }
    }
}

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
#[allow(dead_code, reason = "f64 cases are exercised by comparison tests")]
pub enum SampleType {
    F32,
    F64,
}

#[derive(Clone, Copy, Debug)]
#[allow(
    dead_code,
    reason = "case metadata supports exact and bounded tolerances"
)]
pub enum Tolerance {
    Exact,
    RoundingUp,
    MaxError(u16),
    Robust {
        max_mae: f64,
        max_p99: u16,
        max_high_error_frac: f64,
    },
}

pub const ROBUST: Tolerance = Tolerance::Robust {
    max_mae: 2.0,
    max_p99: 64,
    max_high_error_frac: 0.001,
};

pub type FornaxRunner =
    fn(&[u16], u32, u32, BayerPattern, SampleType) -> mischief::Result<Vec<u16>>;

#[derive(Clone, Copy, Debug)]
pub struct Case {
    pub name: &'static str,
    pub source_url: &'static str,
    pub reference_backend: ReferenceBackend,
    pub algorithm: u32,
    pub tolerance: Tolerance,
    pub run_fornax: FornaxRunner,
}
