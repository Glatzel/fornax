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

pub const WAIVERS: &[Waiver] = &[
    Waiver {
        algorithm: "ahd",
        reference: ReferenceBackend::LibRaw,
        pattern: None,
        input: None,
        max_count_gt1024: 6767,
        max_error: 5222,
        reason: "Independent AHD tile, clipping, matrix, and quantization conventions change local directional decisions at high-contrast chart edges.",
        evidence: "PR #602's valid-crop experiment has the same outlier tail in all four CFA phases with exact sampled Bayer values; the former BGGR and RGGB failures were invalid green-phase test input.",
    },
    Waiver {
        algorithm: "rcd",
        reference: ReferenceBackend::Darktable,
        pattern: None,
        input: None,
        max_count_gt1024: 1,
        max_error: 1359,
        reason: "A near-tie RCD interpolation decision flips under independent floating-point evaluation at one high-contrast chart edge.",
        evidence: "Every CFA phase has one interior high-error sample at the same shifted red-site edge; sampled Bayer values are exact.",
    },
];
