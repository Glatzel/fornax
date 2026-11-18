use crate::cases::ReferenceBackend;

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub struct Waiver {
    pub algorithm: &'static str,
    pub reference: ReferenceBackend,
    pub pattern: Option<&'static str>,
    pub input: Option<&'static str>,
    pub max_count_gt1024: u64,
    pub max_error: u16,
    pub reason: &'static str,
    pub evidence: &'static str,
}

pub const WAIVERS: &[Waiver] = &[];
