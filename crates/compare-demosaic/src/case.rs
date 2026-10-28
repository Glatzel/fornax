use fornax_core::BayerPattern;

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
}

pub type FornaxRunner =
    fn(&[u16], u32, u32, BayerPattern, SampleType) -> mischief::Result<Vec<u16>>;

#[derive(Clone, Copy, Debug)]
pub struct Case {
    pub name: &'static str,
    pub reference: &'static str,
    pub source_url: &'static str,
    pub algorithm: u32,
    pub border: u32,
    pub tolerance: Tolerance,
    pub run_fornax: FornaxRunner,
}
