use std::collections::BTreeSet;
use std::path::Path;

use crate::case::{ReferenceBackend, Tolerance};
use crate::error::{CompareError, Result};
use crate::metrics::{self, Metrics};
use crate::registry::{self, ExpectedRow};
use crate::results::{CanonicalResults, ComparisonResult, Coverage};

#[derive(Clone, Debug)]
pub struct ValidationReport {
    pub rows: usize,
}

pub fn validate_file(
    path: &Path,
    partial: bool,
    check_artifacts: bool,
) -> Result<ValidationReport> {
    let results =
        CanonicalResults::read(path).map_err(|error| CompareError::invalid(error.to_string()))?;
    validate_results(&results, partial, check_artifacts, path.parent())
}

pub fn validate_results(
    results: &CanonicalResults,
    partial: bool,
    check_artifacts: bool,
    artifact_base: Option<&Path>,
) -> Result<ValidationReport> {
    validate_stable_metadata(results)?;
    validate_rows(results, partial)?;
    if check_artifacts {
        validate_artifacts(results, artifact_base)?;
    }
    Ok(ValidationReport {
        rows: results.results.len(),
    })
}

fn validate_stable_metadata(results: &CanonicalResults) -> Result<()> {
    let stable = &results.meta.stable;
    if stable.border_px == 0 {
        return Err(CompareError::invalid(
            "meta.stable.border_px must be positive",
        ));
    }
    if stable.input.sha256.len() != 64
        || !stable
            .input
            .sha256
            .bytes()
            .all(|byte| byte.is_ascii_hexdigit())
    {
        return Err(CompareError::invalid(
            "meta.stable.input.sha256 must contain 64 hexadecimal characters",
        ));
    }
    if registry::parse_pattern(&stable.input.native_pattern).is_none() {
        return Err(CompareError::invalid(format!(
            "unknown native pattern `{}`",
            stable.input.native_pattern
        )));
    }
    let expected = crate::results::robust_limits();
    if stable.limits != expected {
        return Err(CompareError::invalid(
            "meta.stable.limits do not match the registered robust limits",
        ));
    }
    Ok(())
}

fn validate_rows(results: &CanonicalResults, partial: bool) -> Result<()> {
    if results.results.is_empty() {
        return Err(CompareError::invalid("results must not be empty"));
    }
    if partial {
        if results.meta.run.coverage != Coverage::Partial
            && results.meta.run.coverage != Coverage::Full
        {
            return Err(CompareError::invalid("run coverage is not recognized"));
        }
    } else if results.meta.run.coverage != Coverage::Full {
        return Err(CompareError::invalid(
            "full validation requires meta.run.coverage = `full`; pass --partial for shards",
        ));
    }

    let mut keys = BTreeSet::new();
    let mut previous = None::<&ComparisonResult>;
    for row in &results.results {
        if let Some(before) = previous
            && crate::results::comparison_order(before, row) == std::cmp::Ordering::Greater
        {
            return Err(CompareError::invalid(format!(
                "results are not sorted at {} {} {}",
                row.algorithm, row.pattern, row.reference
            )));
        }
        previous = Some(row);
        let key = row_key(row)?;
        if !keys.insert(key.clone()) {
            return Err(CompareError::invalid(format!("duplicate result row {key}")));
        }
        validate_row(row, &results.meta.stable.limits)?;
    }

    if !partial {
        let expected = registry::expected_rows()
            .into_iter()
            .map(expected_key)
            .collect::<BTreeSet<_>>();
        if keys != expected {
            let missing = expected.difference(&keys).cloned().collect::<Vec<_>>();
            let extra = keys.difference(&expected).cloned().collect::<Vec<_>>();
            return Err(CompareError::invalid(format!(
                "full coverage mismatch; missing: [{}]; unexpected: [{}]",
                missing.join(", "),
                extra.join(", ")
            )));
        }
    }
    Ok(())
}

fn validate_row(row: &ComparisonResult, limits: &crate::results::Limits) -> Result<()> {
    validate_metric_set(&row.metrics, "metrics")?;
    validate_metric_set(&row.metrics_interior, "metrics_interior")?;
    validate_metric_set(&row.per_channel.r, "per_channel.r")?;
    validate_metric_set(&row.per_channel.g, "per_channel.g")?;
    validate_metric_set(&row.per_channel.b, "per_channel.b")?;
    validate_metric_set(&row.per_site.sampled, "per_site.sampled")?;
    validate_metric_set(&row.per_site.interpolated, "per_site.interpolated")?;
    if row.total_samples != row.metrics.total_samples {
        return Err(CompareError::invalid(format!(
            "{} {} {} total_samples disagrees with metrics.total_samples",
            row.algorithm, row.pattern, row.reference
        )));
    }
    if row.metrics_interior.total_samples > row.total_samples {
        return Err(CompareError::invalid(format!(
            "{} {} {} interior sample count exceeds total",
            row.algorithm, row.pattern, row.reference
        )));
    }

    let tolerance = match row.kind {
        crate::results::ToleranceKind::Exact => Tolerance::Exact,
        crate::results::ToleranceKind::Rounding => Tolerance::RoundingUp,
        crate::results::ToleranceKind::Robust => Tolerance::Robust {
            max_mae: limits.mae,
            max_p99: limits.p99,
            max_high_error_frac: limits.high_err_frac,
        },
        crate::results::ToleranceKind::MaxError => {
            return Err(CompareError::invalid(format!(
                "{} {} {} uses max_error without a persisted threshold",
                row.algorithm, row.pattern, row.reference
            )));
        }
    };
    let (verdict, failed_limits) = metrics::verdict(&row.metrics, tolerance);
    if verdict != row.verdict || failed_limits != row.failed_limits {
        return Err(CompareError::invalid(format!(
            "{} {} {} verdict/failed_limits do not recompute from metrics",
            row.algorithm, row.pattern, row.reference
        )));
    }
    Ok(())
}

fn validate_metric_set(metrics: &Metrics, name: &str) -> Result<()> {
    if metrics.total_samples == 0 {
        return Err(CompareError::invalid(format!(
            "{name}.total_samples must be positive"
        )));
    }
    if !metrics.mae.is_finite()
        || !metrics.rmse.is_finite()
        || !metrics.mean_signed.is_finite()
        || metrics.mae < 0.0
        || metrics.rmse < 0.0
    {
        return Err(CompareError::invalid(format!(
            "{name} contains a non-finite or negative aggregate"
        )));
    }
    if !(metrics.p99 <= metrics.p999
        && metrics.p999 <= metrics.p9999
        && metrics.p9999 <= metrics.max)
    {
        return Err(CompareError::invalid(format!(
            "{name} percentiles are not monotonic"
        )));
    }
    for (count, fraction, label) in [
        (metrics.count_gt1, metrics.frac_gt1, "gt1"),
        (metrics.count_gt16, metrics.frac_gt16, "gt16"),
        (metrics.count_gt1024, metrics.frac_gt1024, "gt1024"),
    ] {
        if count > metrics.total_samples {
            return Err(CompareError::invalid(format!(
                "{name}.count_{label} exceeds total_samples"
            )));
        }
        let expected = count as f64 / metrics.total_samples as f64;
        if (fraction - expected).abs() > 1e-6 {
            return Err(CompareError::invalid(format!(
                "{name}.frac_{label} does not match count_{label}"
            )));
        }
    }
    if !(metrics.count_gt1 >= metrics.count_gt16 && metrics.count_gt16 >= metrics.count_gt1024) {
        return Err(CompareError::invalid(format!(
            "{name} threshold counts are not monotonic"
        )));
    }
    Ok(())
}

fn validate_artifacts(results: &CanonicalResults, base: Option<&Path>) -> Result<()> {
    let base = base.unwrap_or_else(|| Path::new("."));
    for row in &results.results {
        let Some(artifacts) = &row.artifacts else {
            continue;
        };
        for path in [&artifacts.error_map, &artifacts.tile_summary] {
            if Path::new(path).is_absolute()
                || Path::new(path)
                    .components()
                    .any(|component| component.as_os_str() == "..")
            {
                return Err(CompareError::invalid(format!(
                    "artifact path `{path}` is not a relative child of the evidence directory"
                )));
            }
            if !base.join(path).is_file() {
                return Err(CompareError::invalid(format!(
                    "referenced artifact `{path}` does not exist relative to {}",
                    base.display()
                )));
            }
        }
    }
    Ok(())
}

pub fn row_key(row: &ComparisonResult) -> Result<String> {
    let reference = registry::parse_reference(&row.reference).ok_or_else(|| {
        CompareError::invalid(format!(
            "{} {} has unknown reference `{}`",
            row.algorithm, row.pattern, row.reference
        ))
    })?;
    let pattern = registry::parse_pattern(&row.pattern).ok_or_else(|| {
        CompareError::invalid(format!(
            "{} has unknown Bayer pattern `{}`",
            row.algorithm, row.pattern
        ))
    })?;
    let algorithm = registry::canonical_algorithm_name(&row.algorithm)
        .ok_or_else(|| CompareError::invalid(format!("unknown algorithm `{}`", row.algorithm)))?;
    let case = registry::all_cases()
        .find(|case| {
            case.name.eq_ignore_ascii_case(algorithm) && case.reference_backend == reference
        })
        .ok_or_else(|| {
            CompareError::invalid(format!(
                "algorithm `{algorithm}` is not registered for reference `{}`",
                match reference {
                    ReferenceBackend::LibRaw => "libraw",
                    ReferenceBackend::Darktable => "darktable",
                }
            ))
        })?;
    Ok(format!(
        "{} {} {}",
        case.name,
        pattern,
        case.reference_backend.library().to_ascii_lowercase()
    ))
}

fn expected_key(row: ExpectedRow) -> String {
    format!("{} {} {}", row.algorithm, row.pattern, row.reference)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn fraction_check_rejects_a_tampered_count() {
        let metrics = Metrics {
            total_samples: 10,
            mae: 0.0,
            rmse: 0.0,
            max: 0,
            mean_signed: 0.0,
            p99: 0,
            p999: 0,
            p9999: 0,
            count_gt1: 2,
            frac_gt1: 0.1,
            count_gt16: 1,
            frac_gt16: 0.2,
            count_gt1024: 0,
            frac_gt1024: 0.0,
            count_below_reference: 0,
            changed_count: 2,
            changed_frac: 0.2,
        };
        let error = validate_metric_set(&metrics, "metrics").unwrap_err();
        assert!(error.to_string().contains("frac_gt1"));
    }
}
