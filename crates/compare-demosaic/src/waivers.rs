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

pub const WAIVERS: &[Waiver] = &[Waiver {
    algorithm: "rcd",
    reference: ReferenceBackend::Darktable,
    pattern: None,
    input: None,
    max_count_gt1024: 1,
    max_error: 1359,
    reason: "A near-tie RCD interpolation decision flips under independent floating-point evaluation at one high-contrast chart edge.",
    evidence: "Every CFA phase has one interior high-error sample at the same shifted red-site edge; sampled Bayer values are exact.",
}];
