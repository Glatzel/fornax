use fornax_raw_core::RawMosaic;

pub(crate) enum CanonicalMosaic {
    U8 {
        width: u32,
        height: u32,
        samples: Vec<u8>,
    },
    U16 {
        width: u32,
        height: u32,
        samples: Vec<u16>,
    },
    U32 {
        width: u32,
        height: u32,
        samples: Vec<u32>,
    },
    F32 {
        width: u32,
        height: u32,
        samples: Vec<u32>,
    },
}

impl CanonicalMosaic {
    fn storage(&self) -> &'static str {
        match self {
            Self::U8 { .. } => "U8",
            Self::U16 { .. } => "U16",
            Self::U32 { .. } => "U32",
            Self::F32 { .. } => "F32",
        }
    }

    fn dimensions(&self) -> (u32, u32) {
        match self {
            Self::U8 { width, height, .. }
            | Self::U16 { width, height, .. }
            | Self::U32 { width, height, .. }
            | Self::F32 { width, height, .. } => (*width, *height),
        }
    }

    fn sample_count(&self) -> usize {
        match self {
            Self::U8 { samples, .. } => samples.len(),
            Self::U16 { samples, .. } => samples.len(),
            Self::U32 { samples, .. } | Self::F32 { samples, .. } => samples.len(),
        }
    }
}

pub(crate) fn canonical_mosaic(mosaic: &RawMosaic) -> CanonicalMosaic {
    let (width, height) = mosaic.dimensions();
    match mosaic {
        RawMosaic::U8(image) => CanonicalMosaic::U8 {
            width,
            height,
            samples: image.as_raw().clone(),
        },
        RawMosaic::U16(image) => CanonicalMosaic::U16 {
            width,
            height,
            samples: image.as_raw().clone(),
        },
        RawMosaic::U32(image) => CanonicalMosaic::U32 {
            width,
            height,
            samples: image.as_raw().clone(),
        },
        RawMosaic::F32(image) => CanonicalMosaic::F32 {
            width,
            height,
            samples: image.as_raw().iter().map(|value| value.to_bits()).collect(),
        },
    }
}

pub(crate) fn assert_mosaic_eq(
    case: &str,
    reference_name: &str,
    expected: &CanonicalMosaic,
    actual: &CanonicalMosaic,
) {
    assert_eq!(
        actual.storage(),
        expected.storage(),
        "decoder equivalence mismatch for {case}: Fornax storage != {reference_name}"
    );
    assert_eq!(
        actual.dimensions(),
        expected.dimensions(),
        "decoder equivalence mismatch for {case}: Fornax dimensions != {reference_name}"
    );
    assert_eq!(
        actual.sample_count(),
        expected.sample_count(),
        "decoder equivalence mismatch for {case}: Fornax sample count != {reference_name}"
    );

    match (expected, actual) {
        (
            CanonicalMosaic::U8 {
                samples: expected, ..
            },
            CanonicalMosaic::U8 {
                samples: actual, ..
            },
        ) => assert_samples(case, reference_name, expected, actual),
        (
            CanonicalMosaic::U16 {
                samples: expected, ..
            },
            CanonicalMosaic::U16 {
                samples: actual, ..
            },
        ) => assert_samples(case, reference_name, expected, actual),
        (
            CanonicalMosaic::U32 {
                samples: expected, ..
            },
            CanonicalMosaic::U32 {
                samples: actual, ..
            },
        ) => assert_samples(case, reference_name, expected, actual),
        (
            CanonicalMosaic::F32 {
                samples: expected, ..
            },
            CanonicalMosaic::F32 {
                samples: actual, ..
            },
        ) => assert_samples(case, reference_name, expected, actual),
        _ => unreachable!("storage equality was checked above"),
    }
}

fn assert_samples<T>(case: &str, reference_name: &str, expected: &[T], actual: &[T])
where
    T: Copy + Eq + std::fmt::Debug,
{
    if let Some(index) = expected
        .iter()
        .zip(actual)
        .position(|(expected, actual)| expected != actual)
    {
        panic!(
            "decoder equivalence mismatch for {case}: Fornax sample {index} is {:?}, \
             {reference_name} sample is {:?}",
            actual[index], expected[index]
        );
    }
}
