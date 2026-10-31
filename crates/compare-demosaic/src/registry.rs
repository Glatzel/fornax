use fornax_core::BayerPattern;

use crate::case::Case;
use crate::cases;

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
    cases::CASES
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
