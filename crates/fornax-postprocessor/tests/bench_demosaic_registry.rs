#[path = "../benches/support/demosaic_registry.rs"]
mod demosaic_registry;

use std::collections::BTreeSet;

use demosaic_registry::{algorithm_ids, demosaic, parse_selected};

#[test]
fn registry_is_sorted_unique_and_maps_to_every_algorithm() {
    let algorithms = algorithm_ids();
    assert_eq!(
        algorithms.iter().cloned().collect::<BTreeSet<_>>().len(),
        algorithms.len()
    );
    assert!(algorithms.windows(2).all(|pair| pair[0] < pair[1]));
    for algorithm in algorithms {
        let _ = demosaic(algorithm);
    }
}

#[test]
fn algorithm_filter_accepts_registry_ids_and_rejects_unknown_values() {
    assert_eq!(parse_selected(None), None);
    assert_eq!(parse_selected(Some("")), None);
    assert_eq!(parse_selected(Some(" ahd ")).as_deref(), Some("ahd"));
    assert!(std::panic::catch_unwind(|| parse_selected(Some("unknown"))).is_err());
}
