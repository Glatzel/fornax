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
        algorithm: "aahd",
        reference: ReferenceBackend::LibRaw,
        pattern: None,
        input: Some("colorchart-5D2-6000K.dng"),
        max_count_gt1024: 4,
        max_error: 1_246,
        reason: "Independent floating-point homogeneity and gradient comparisons can select the other AAHD directional candidate at sparse high-contrast chart details.",
        evidence: "The full-resolution DNG error maps contain at most four interior high-error samples in any CFA phase, with a maximum error of 1246. Sampled Bayer sites are exact, every high-error sample is interpolated, each 64x64 tile contains at most one such sample, and the same physical chart sites recur after pattern-crop alignment without border or full-frame structure.",
    },
    Waiver {
        algorithm: "ahd",
        reference: ReferenceBackend::LibRaw,
        pattern: None,
        input: Some("colorchart-5D2-6000K.dng"),
        max_count_gt1024: 1,
        max_error: 1_220,
        reason: "Independent AHD tile, clipping, matrix, and quantization conventions can change a local directional decision at a high-contrast chart edge.",
        evidence: "Each DNG CFA phase has one interior high-error sample with a maximum error of 1220. Sampled Bayer sites are exact, the high-error sample is interpolated, the same physical chart site recurs after pattern-crop alignment, and the full error map has no border strip or global structure.",
    },
];
