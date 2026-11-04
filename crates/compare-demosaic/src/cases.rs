use fornax_core::{BayerPattern, MonoImage, Primitive};
use fornax_postprocessor::Demosaic;
use fornax_postprocessor::protocol::{IDemosaic, LazyDemosaic};

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
pub enum ReferenceBackend {
    LibRaw,
    Darktable,
    RawTherapee,
}

impl ReferenceBackend {
    pub const fn library(self) -> &'static str {
        match self {
            Self::LibRaw => "LibRaw",
            Self::Darktable => "darktable",
            Self::RawTherapee => "RawTherapee",
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
pub enum SampleType {
    F32,
    F64,
}

#[derive(Clone, Copy, Debug)]
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

const ROBUST: Tolerance = Tolerance::Robust {
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

const LIBRAW_AHD_URL: &str = concat!(
    "https://github.com/LibRaw/LibRaw/blob/",
    env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT"),
    "/src/demosaic/ahd_demosaic.cpp#L293"
);
const LIBRAW_LINEAR_URL: &str = concat!(
    "https://github.com/LibRaw/LibRaw/blob/",
    env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT"),
    "/src/demosaic/misc_demosaic.cpp#L146"
);
const LIBRAW_PPG_URL: &str = concat!(
    "https://github.com/LibRaw/LibRaw/blob/",
    env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT"),
    "/src/demosaic/misc_demosaic.cpp#L363"
);
const LIBRAW_VNG_URL: &str = concat!(
    "https://github.com/LibRaw/LibRaw/blob/",
    env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT"),
    "/src/demosaic/misc_demosaic.cpp#L199"
);
const LIBRAW_VNG4_URL: &str = concat!(
    "https://github.com/LibRaw/LibRaw/blob/",
    env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT"),
    "/src/demosaic/misc_demosaic.cpp#L84"
);
const DARKTABLE_AMAZE_URL: &str = concat!(
    "https://github.com/darktable-org/darktable/blob/",
    env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT"),
    "/src/iop/demosaicing/amaze.cc#L129"
);
const DARKTABLE_LMMSE_URL: &str = concat!(
    "https://github.com/darktable-org/darktable/blob/",
    env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT"),
    "/src/iop/demosaicing/lmmse.c#L120"
);
const DARKTABLE_PPG_URL: &str = concat!(
    "https://github.com/darktable-org/darktable/blob/",
    env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT"),
    "/src/iop/demosaicing/ppg.c#L20"
);
const DARKTABLE_RCD_URL: &str = concat!(
    "https://github.com/darktable-org/darktable/blob/",
    env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT"),
    "/src/iop/demosaicing/rcd.c#L119"
);
const DARKTABLE_VNG_URL: &str = concat!(
    "https://github.com/darktable-org/darktable/blob/",
    env!("COMPARE_DEMOSAIC_DARKTABLE_COMMIT"),
    "/src/iop/demosaicing/vng.c#L171"
);
const LIBRAW_AAHD_URL: &str = concat!(
    "https://github.com/LibRaw/LibRaw/blob/",
    env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT"),
    "/src/demosaic/aahd_demosaic.cpp#L771"
);
const LIBRAW_DCB_URL: &str = concat!(
    "https://github.com/LibRaw/LibRaw/blob/",
    env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT"),
    "/src/demosaic/dcb_demosaic.cpp#L849"
);
const LIBRAW_DHT_URL: &str = concat!(
    "https://github.com/LibRaw/LibRaw/blob/",
    env!("COMPARE_DEMOSAIC_LIBRAW_COMMIT"),
    "/src/demosaic/dht_demosaic.cpp#L1012"
);
const RAWTHERAPEE_AHD_URL: &str = concat!(
    "https://github.com/RawTherapee/RawTherapee/blob/",
    env!("COMPARE_DEMOSAIC_RAWTHERAPEE_COMMIT"),
    "/rtengine/ahd_demosaic_RT.cc#L44"
);
const RAWTHERAPEE_AMAZE_URL: &str = concat!(
    "https://github.com/RawTherapee/RawTherapee/blob/",
    env!("COMPARE_DEMOSAIC_RAWTHERAPEE_COMMIT"),
    "/rtengine/amaze_demosaic_RT.cc#L48"
);
const RAWTHERAPEE_DCB_URL: &str = concat!(
    "https://github.com/RawTherapee/RawTherapee/blob/",
    env!("COMPARE_DEMOSAIC_RAWTHERAPEE_COMMIT"),
    "/rtengine/demosaic_algos.cc#L1406"
);
const RAWTHERAPEE_LMMSE_URL: &str = concat!(
    "https://github.com/RawTherapee/RawTherapee/blob/",
    env!("COMPARE_DEMOSAIC_RAWTHERAPEE_COMMIT"),
    "/rtengine/lmmse_demosaic.cc#L42"
);
const RAWTHERAPEE_RCD_URL: &str = concat!(
    "https://github.com/RawTherapee/RawTherapee/blob/",
    env!("COMPARE_DEMOSAIC_RAWTHERAPEE_COMMIT"),
    "/rtengine/rcd_demosaic.cc#L53"
);
const RAWTHERAPEE_VNG4_URL: &str = concat!(
    "https://github.com/RawTherapee/RawTherapee/blob/",
    env!("COMPARE_DEMOSAIC_RAWTHERAPEE_COMMIT"),
    "/rtengine/vng4_demosaic_RT.cc#L64"
);

const AHD: Case = Case {
    name: "AHD",
    source_url: LIBRAW_AHD_URL,
    reference_backend: ReferenceBackend::LibRaw,
    algorithm: 2,
    tolerance: ROBUST,
    run_fornax: run_ahd,
};
const AMAZE: Case = Case {
    name: "AMaZE",
    source_url: DARKTABLE_AMAZE_URL,
    reference_backend: ReferenceBackend::Darktable,
    algorithm: 3,
    tolerance: ROBUST,
    run_fornax: run_amaze,
};
const AAHD: Case = Case {
    name: "AAHD",
    source_url: LIBRAW_AAHD_URL,
    reference_backend: ReferenceBackend::LibRaw,
    algorithm: 6,
    tolerance: ROBUST,
    run_fornax: run_aahd,
};
const DCB: Case = Case {
    name: "DCB",
    source_url: LIBRAW_DCB_URL,
    reference_backend: ReferenceBackend::LibRaw,
    algorithm: 4,
    tolerance: ROBUST,
    run_fornax: run_dcb,
};
const DHT: Case = Case {
    name: "DHT",
    source_url: LIBRAW_DHT_URL,
    reference_backend: ReferenceBackend::LibRaw,
    algorithm: 5,
    tolerance: ROBUST,
    run_fornax: run_dht,
};
const LINEAR: Case = Case {
    name: "Linear",
    source_url: LIBRAW_LINEAR_URL,
    reference_backend: ReferenceBackend::LibRaw,
    algorithm: 0,
    tolerance: Tolerance::RoundingUp,
    run_fornax: run_linear,
};
const LMMSE: Case = Case {
    name: "LMMSE",
    source_url: DARKTABLE_LMMSE_URL,
    reference_backend: ReferenceBackend::Darktable,
    algorithm: 4,
    tolerance: ROBUST,
    run_fornax: run_lmmse,
};
const PPG_LIBRAW: Case = Case {
    name: "PPG",
    source_url: LIBRAW_PPG_URL,
    reference_backend: ReferenceBackend::LibRaw,
    algorithm: 1,
    tolerance: ROBUST,
    run_fornax: run_ppg,
};
const PPG_DARKTABLE: Case = Case {
    name: "PPG",
    source_url: DARKTABLE_PPG_URL,
    reference_backend: ReferenceBackend::Darktable,
    algorithm: 0,
    tolerance: ROBUST,
    run_fornax: run_ppg,
};
const RCD_DARKTABLE: Case = Case {
    name: "RCD",
    source_url: DARKTABLE_RCD_URL,
    reference_backend: ReferenceBackend::Darktable,
    algorithm: 1,
    tolerance: ROBUST,
    run_fornax: run_rcd,
};
const VNG_DARKTABLE: Case = Case {
    name: "VNG",
    source_url: DARKTABLE_VNG_URL,
    reference_backend: ReferenceBackend::Darktable,
    algorithm: 2,
    tolerance: ROBUST,
    run_fornax: run_vng,
};
const VNG_LIBRAW: Case = Case {
    name: "VNG",
    source_url: LIBRAW_VNG_URL,
    reference_backend: ReferenceBackend::LibRaw,
    algorithm: 3,
    tolerance: ROBUST,
    run_fornax: run_vng,
};
const VNG4_LIBRAW: Case = Case {
    name: "VNG4",
    source_url: LIBRAW_VNG4_URL,
    reference_backend: ReferenceBackend::LibRaw,
    algorithm: 7,
    tolerance: ROBUST,
    run_fornax: run_vng4,
};
const AHD_RAWTHERAPEE: Case = Case {
    name: "AHD",
    source_url: RAWTHERAPEE_AHD_URL,
    reference_backend: ReferenceBackend::RawTherapee,
    algorithm: 1,
    tolerance: ROBUST,
    run_fornax: run_ahd,
};
const AMAZE_RAWTHERAPEE: Case = Case {
    name: "AMaZE",
    source_url: RAWTHERAPEE_AMAZE_URL,
    reference_backend: ReferenceBackend::RawTherapee,
    algorithm: 0,
    tolerance: ROBUST,
    run_fornax: run_amaze,
};
const DCB_RAWTHERAPEE: Case = Case {
    name: "DCB",
    source_url: RAWTHERAPEE_DCB_URL,
    reference_backend: ReferenceBackend::RawTherapee,
    algorithm: 2,
    tolerance: ROBUST,
    run_fornax: run_dcb,
};
const LMMSE_RAWTHERAPEE: Case = Case {
    name: "LMMSE",
    source_url: RAWTHERAPEE_LMMSE_URL,
    reference_backend: ReferenceBackend::RawTherapee,
    algorithm: 3,
    tolerance: ROBUST,
    run_fornax: run_lmmse,
};
const RCD_RAWTHERAPEE: Case = Case {
    name: "RCD",
    source_url: RAWTHERAPEE_RCD_URL,
    reference_backend: ReferenceBackend::RawTherapee,
    algorithm: 4,
    tolerance: ROBUST,
    run_fornax: run_rcd,
};
const VNG4_RAWTHERAPEE: Case = Case {
    name: "VNG4",
    source_url: RAWTHERAPEE_VNG4_URL,
    reference_backend: ReferenceBackend::RawTherapee,
    algorithm: 5,
    tolerance: ROBUST,
    run_fornax: run_vng4,
};

pub const CASES: &[Case] = &[
    AHD,
    AMAZE,
    AAHD,
    DCB,
    DHT,
    LINEAR,
    LMMSE,
    PPG_LIBRAW,
    PPG_DARKTABLE,
    RCD_DARKTABLE,
    VNG_DARKTABLE,
    VNG_LIBRAW,
    VNG4_LIBRAW,
    AHD_RAWTHERAPEE,
    AMAZE_RAWTHERAPEE,
    DCB_RAWTHERAPEE,
    LMMSE_RAWTHERAPEE,
    RCD_RAWTHERAPEE,
    VNG4_RAWTHERAPEE,
];

#[derive(Clone, Copy, Debug)]
pub struct CaseRow {
    pub case: &'static Case,
    pub pattern: BayerPattern,
}

pub fn selected_cases(algorithms: &[String]) -> Vec<CaseRow> {
    selected_cases_filtered(algorithms, &[], &BayerPattern::ALL)
}

pub fn selected_cases_filtered(
    algorithms: &[String],
    references: &[String],
    patterns: &[BayerPattern],
) -> Vec<CaseRow> {
    let algorithms = lowercase(algorithms);
    let references = lowercase(references);
    CASES
        .iter()
        .filter(|case| {
            algorithms.is_empty() || algorithms.contains(&case.name.to_ascii_lowercase())
        })
        .filter(|case| {
            references.is_empty()
                || references.contains(&case.reference_backend.library().to_ascii_lowercase())
        })
        .flat_map(|case| {
            patterns
                .iter()
                .copied()
                .map(move |pattern| CaseRow { case, pattern })
        })
        .collect()
}

pub fn algorithm_names() -> Vec<&'static str> {
    let mut names = CASES.iter().map(|case| case.name).collect::<Vec<_>>();
    names.sort_unstable_by_key(|name| name.to_ascii_lowercase());
    names.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    names
}

pub fn reference_names() -> Vec<&'static str> { vec!["darktable", "libraw", "rawtherapee"] }

pub fn case_for(algorithm: &str, reference: &str) -> Option<&'static Case> {
    CASES.iter().find(|case| {
        case.name.eq_ignore_ascii_case(algorithm)
            && case
                .reference_backend
                .library()
                .eq_ignore_ascii_case(reference)
    })
}

pub fn display_name(algorithm: &str) -> &str {
    CASES
        .iter()
        .find(|case| case.name.eq_ignore_ascii_case(algorithm))
        .map_or(algorithm, |case| case.name)
}

pub fn source_url(algorithm: &str, reference: &str) -> Option<&'static str> {
    case_for(algorithm, reference).map(|case| case.source_url)
}

fn lowercase(values: &[String]) -> Vec<String> {
    values
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

fn run_with(
    demosaic: Demosaic,
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    sample_type: SampleType,
) -> mischief::Result<Vec<u16>> {
    match sample_type {
        SampleType::F32 => run_typed::<f32>(demosaic, codes, width, height, pattern),
        SampleType::F64 => run_typed::<f64>(demosaic, codes, width, height, pattern),
    }
}

fn run_typed<T>(
    demosaic: Demosaic,
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
) -> mischief::Result<Vec<u16>>
where
    T: Primitive,
{
    let expected = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| mischief::mischief!("synthetic image dimensions overflow"))?;
    if codes.len() != expected {
        return Err(mischief::mischief!(
            "synthetic image contains {} samples, expected {expected}",
            codes.len()
        ));
    }
    let samples = codes
        .iter()
        .map(|code| {
            let normalized = f64::from(*code) / f64::from(u16::MAX);
            num_traits::cast(normalized).ok_or_else(|| {
                mischief::mischief!(
                    "sample cannot be represented as {}",
                    std::any::type_name::<T>()
                )
            })
        })
        .collect::<mischief::Result<Vec<T>>>()?;
    let image = MonoImage::from_raw(width, height, samples)
        .ok_or_else(|| mischief::mischief!("synthetic image dimensions are inconsistent"))?;
    Ok(demosaic
        .demosaic(&image, pattern)?
        .collect()
        .as_raw()
        .iter()
        .map(|sample| {
            let maximum =
                num_traits::cast(f64::from(u16::MAX)).expect("sample supports 16-bit range");
            let normalized = sample.clamp(T::zero(), T::one());
            num_traits::cast((normalized * maximum).round()).expect("rounded sample fits u16")
        })
        .collect())
}

macro_rules! runner {
    ($name:ident, $demosaic:expr) => {
        fn $name(
            codes: &[u16],
            width: u32,
            height: u32,
            pattern: BayerPattern,
            sample_type: SampleType,
        ) -> mischief::Result<Vec<u16>> {
            run_with($demosaic, codes, width, height, pattern, sample_type)
        }
    };
}

runner!(run_ahd, Demosaic::Ahd);
runner!(run_amaze, Demosaic::Amaze);
runner!(run_aahd, Demosaic::Aahd);
runner!(run_dcb, Demosaic::Dcb);
runner!(run_dht, Demosaic::Dht);
runner!(run_linear, Demosaic::Linear);
runner!(run_lmmse, Demosaic::Lmmse);
runner!(run_ppg, Demosaic::Ppg);
runner!(run_rcd, Demosaic::Rcd);
runner!(run_vng, Demosaic::Vng);
runner!(run_vng4, Demosaic::Vng4);

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn filters_by_algorithm_across_all_patterns() {
        let rows = selected_cases(&["vng".to_owned()]);
        assert_eq!(rows.len(), 8);
        assert!(rows.iter().all(|row| row.case.name == "VNG"));
        assert!(
            BayerPattern::ALL
                .iter()
                .all(|pattern| rows.iter().any(|row| row.pattern == *pattern))
        );
    }

    #[test]
    fn rawtherapee_covers_shared_algorithms_without_dual() {
        for algorithm in ["ahd", "amaze", "dcb", "lmmse", "rcd", "vng4"] {
            assert!(
                case_for(algorithm, "rawtherapee").is_some(),
                "missing RawTherapee comparison for {algorithm}"
            );
        }
        assert!(
            algorithm_names()
                .iter()
                .all(|name| !name.to_ascii_lowercase().contains("dual"))
        );
    }
}
