use std::sync::LazyLock;

use fornax_postprocessor::Demosaic;

const ALGORITHMS_JSON: &str = include_str!("../algorithms.json");

static ALGORITHM_IDS: LazyLock<Vec<String>> = LazyLock::new(|| {
    serde_json::from_str(ALGORITHMS_JSON).expect("bench-demosaic algorithm registry must be valid")
});

/// Returns the stable algorithm identifiers registered for demosaic benchmarks.
///
/// The identifiers are loaded from `algorithms.json` and are ordered
/// alphabetically.
pub fn algorithm_ids() -> &'static [String] { ALGORITHM_IDS.as_slice() }

/// Resolves a registry identifier to its production demosaic implementation.
///
/// # Panics
///
/// Panics when the registry contains an identifier without a `Demosaic`
/// mapping. Registry consistency is covered by this crate's tests.
pub fn demosaic(algorithm: &str) -> Demosaic {
    match algorithm {
        "aahd" => Demosaic::Aahd,
        "ahd" => Demosaic::Ahd,
        "amaze" => Demosaic::Amaze,
        "dcb" => Demosaic::Dcb,
        "dht" => Demosaic::Dht,
        "linear" => Demosaic::Linear,
        "lmmse" => Demosaic::Lmmse,
        "ppg" => Demosaic::Ppg,
        "rcd" => Demosaic::Rcd,
        "vng" => Demosaic::Vng,
        other => panic!("benchmark registry contains unknown algorithm {other:?}"),
    }
}

/// Reads and validates the optional `FORNAX_BENCH_ALGORITHM` filter.
///
/// An unset or empty value selects every registered algorithm.
///
/// # Panics
///
/// Panics when the value is not present in `algorithms.json`.
pub fn selected_algorithm() -> Option<String> {
    parse_selected(std::env::var("FORNAX_BENCH_ALGORITHM").ok().as_deref())
}

pub fn parse_selected(value: Option<&str>) -> Option<String> {
    let selected = value.unwrap_or_default();
    let selected = selected.trim();
    if selected.is_empty() {
        return None;
    }
    assert!(
        algorithm_ids()
            .iter()
            .any(|algorithm| algorithm == selected),
        "unknown FORNAX_BENCH_ALGORITHM value {selected:?}; expected one of {}",
        algorithm_ids().join(", ")
    );
    Some(selected.to_owned())
}
