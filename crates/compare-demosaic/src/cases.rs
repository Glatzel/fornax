use fornax_core::protocol::{IDemosaic, LazyDemosaic};
use fornax_core::{BayerPattern, MonoImage, Primitive};
use fornax_postprocessor::Demosaic;

#[derive(Clone, Copy, Debug, Eq, PartialEq, PartialOrd, Ord)]
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
];

#[derive(Clone, Copy, Debug)]
pub struct CaseRow {
    pub case: &'static Case,
    pub pattern: BayerPattern,
}

pub fn selected_cases(algorithms: &[String]) -> Vec<CaseRow> {
    let algorithms = algorithms
        .iter()
        .map(|algorithm| algorithm.to_ascii_lowercase())
        .collect::<Vec<_>>();
    CASES
        .iter()
        .filter(|case| {
            algorithms.is_empty() || algorithms.contains(&case.name.to_ascii_lowercase())
        })
        .flat_map(|case| {
            BayerPattern::ALL
                .iter()
                .copied()
                .map(move |pattern| CaseRow { case, pattern })
        })
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
}
