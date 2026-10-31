use std::collections::BTreeSet;

use fornax_core::BayerPattern;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::case::ReferenceBackend;
use crate::cases;

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, Ord, PartialEq, PartialOrd, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExpectedRow {
    pub algorithm: String,
    pub reference: String,
    pub pattern: String,
}

#[derive(Clone, Copy, Debug)]
pub struct CaseRow {
    pub case: &'static crate::case::Case,
    pub pattern: BayerPattern,
}

pub fn all_cases() -> impl Iterator<Item = &'static crate::case::Case> { cases::CASES.iter() }

pub fn expected_rows() -> Vec<ExpectedRow> {
    let mut rows = all_cases()
        .flat_map(|case| {
            BayerPattern::ALL.iter().map(move |pattern| ExpectedRow {
                algorithm: case.name.to_owned(),
                reference: case.reference_backend.library().to_ascii_lowercase(),
                pattern: pattern.to_string(),
            })
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

pub fn selected_cases(
    algorithms: &[String],
    references: &[ReferenceBackend],
    patterns: &[BayerPattern],
) -> Vec<CaseRow> {
    let algorithms = normalized_set(algorithms);
    let references = references.iter().copied().collect::<BTreeSet<_>>();
    let patterns = if patterns.is_empty() {
        BayerPattern::ALL.to_vec()
    } else {
        patterns.to_vec()
    };
    all_cases()
        .filter(|case| {
            algorithms.is_empty() || algorithms.contains(&case.name.to_ascii_lowercase())
        })
        .filter(|case| references.is_empty() || references.contains(&case.reference_backend))
        .flat_map(|case| {
            patterns
                .iter()
                .copied()
                .map(move |pattern| CaseRow { case, pattern })
        })
        .collect()
}

pub fn expected_rows_for_selection(
    algorithms: &[String],
    references: &[ReferenceBackend],
    patterns: &[BayerPattern],
) -> Vec<ExpectedRow> {
    let mut rows = selected_cases(algorithms, references, patterns)
        .into_iter()
        .map(|row| ExpectedRow {
            algorithm: row.case.name.to_owned(),
            reference: row.case.reference_backend.library().to_ascii_lowercase(),
            pattern: row.pattern.to_string(),
        })
        .collect::<Vec<_>>();
    rows.sort();
    rows
}

pub fn canonical_algorithm_name(value: &str) -> Option<&'static str> {
    all_cases()
        .find(|case| case.name.eq_ignore_ascii_case(value))
        .map(|case| case.name)
}

pub fn parse_reference(value: &str) -> Option<ReferenceBackend> {
    match value.to_ascii_lowercase().as_str() {
        "libraw" => Some(ReferenceBackend::LibRaw),
        "darktable" => Some(ReferenceBackend::Darktable),
        _ => None,
    }
}

pub fn parse_pattern(value: &str) -> Option<BayerPattern> {
    BayerPattern::ALL
        .iter()
        .copied()
        .find(|pattern| pattern.to_string().eq_ignore_ascii_case(value))
}

fn normalized_set(values: &[String]) -> BTreeSet<String> {
    values
        .iter()
        .map(|value| value.to_ascii_lowercase())
        .collect()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn expected_matrix_is_unique_and_complete() {
        let rows = expected_rows();
        assert_eq!(rows.len(), cases::CASES.len() * BayerPattern::ALL.len());
        let unique = rows.iter().collect::<BTreeSet<_>>();
        assert_eq!(unique.len(), rows.len());
    }

    #[test]
    fn selection_defaults_to_the_full_matrix() {
        assert_eq!(expected_rows_for_selection(&[], &[], &[]), expected_rows());
    }
}
