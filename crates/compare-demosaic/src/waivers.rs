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
        algorithm: "ppg",
        reference: ReferenceBackend::LibRaw,
        pattern: None,
        input: None,
        max_count_gt1024: 53,
        max_error: 2221,
        reason: "LibRaw selects PPG directions with integer shifts and truncating arithmetic while Fornax follows darktable's normalized f32 path; near ties at high-contrast chart edges flip a small local set of directional choices.",
        evidence: "The committed baseline matches darktable exactly in all CFA phases, preserves sampled Bayer sites exactly, and has the same 53 interior high-error samples and maximum error 2221 in every phase; each row's top-20 worst samples align after its crop offset, and the error map is sparse. LibRaw 0.22.3 ppg_interpolate uses right shifts and integer division for its predictions and diagonal tie.",
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
