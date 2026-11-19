use std::io::{BufWriter, Write};
use std::path::{Path, PathBuf};

use fornax_raw_core::BayerPattern;
use serde::{Deserialize, Serialize};

use crate::ground_truth::{GroundTruthReport, OutlierEvidence};
use crate::metrics::OutlierLocations;

pub const SCHEMA_VERSION: u32 = 3;
pub const REFERENCE_FLOOR_VERSION: u32 = 1;
pub const CANONICAL_FILE: &str = "target/compare-demosaic/canonical.json";
pub const BASELINE_FILE: &str = "crates/compare-demosaic/baseline/canonical.json";
pub const BORDER_PX: u32 = 16;
pub const HIGH_ERROR_THRESHOLD: u16 = 1024;
pub const LIMITS: Limits = Limits {
    mae: 2.0,
    p99: 64,
    high_err: HIGH_ERROR_THRESHOLD,
    high_err_frac: 0.001,
};

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Report {
    pub schema_version: u32,
    pub meta: Meta,
    pub ground_truth: GroundTruthReport,
    pub reference_floor: ReferenceFloorReport,
    pub rows: Vec<Row>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
/// Invariant comparison inputs. Volatile run details belong in Git or CI logs.
pub struct Meta {
    pub inputs: Vec<InputMeta>,
    pub references: References,
    pub limits: Limits,
    pub border_px: u32,
    pub pattern_source: PatternSource,
    pub range_scaling: RangeScaling,
    pub selection: Selection,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct RangeScaling {
    pub enabled: bool,
    pub ceiling_codes: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct InputMeta {
    pub name: String,
    pub sha256: String,
    pub width: u32,
    pub height: u32,
    pub black: Vec<f64>,
    pub black_width: u32,
    pub black_height: u32,
    pub white: f64,
    pub native_pattern: String,
    pub reference_scale: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PatternSource {
    ShiftedCrops,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct References {
    pub libraw: String,
    pub darktable: String,
    pub rawtherapee: String,
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Limits {
    pub mae: f64,
    pub p99: u16,
    pub high_err: u16,
    pub high_err_frac: f64,
}

#[derive(Clone, Debug, Default, Deserialize, PartialEq, Eq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Selection {
    pub algorithm: Vec<String>,
    pub reference: Vec<String>,
    pub pattern: Vec<String>,
    pub input: Vec<String>,
}

impl ReferenceFloorReport {
    pub fn validate(&self) -> mischief::Result<()> {
        if self.version != REFERENCE_FLOOR_VERSION {
            return Err(mischief::mischief!(
                "unsupported reference-floor version {}",
                self.version
            ));
        }
        if self.tolerance_codes != crate::metrics::TOLERANCE_CODES {
            return Err(mischief::mischief!(
                "reference-floor tolerance must be {}",
                crate::metrics::TOLERANCE_CODES
            ));
        }
        for row in &self.rows {
            if row.samples == 0 {
                return Err(mischief::mischief!(
                    "reference floor {} {} {} has no samples",
                    row.input,
                    row.algorithm,
                    row.pattern
                ));
            }
            if row
                .outlier_locations
                .border
                .saturating_add(row.outlier_locations.edge)
                .saturating_add(row.outlier_locations.flat)
                > row.samples
            {
                return Err(mischief::mischief!(
                    "reference floor {} {} {} has inconsistent counts",
                    row.input,
                    row.algorithm,
                    row.pattern
                ));
            }
            validate_metrics("reference-floor", &row.metrics, row.samples)?;
            if !row.within_tolerance.is_finite()
                || !(0.0..=1.0).contains(&row.within_tolerance)
                || round6(row.within_tolerance) != row.within_tolerance
            {
                return Err(mischief::mischief!(
                    "reference floor has an invalid tolerance fraction"
                ));
            }
        }
        Ok(())
    }

    pub(crate) fn matches_selection(&self, other: &Self, algorithms: &[String]) -> bool {
        algorithms.is_empty()
            || (self.version == other.version
                && self.border_px == other.border_px
                && self.tolerance_codes == other.tolerance_codes
                && other.rows.iter().all(|candidate| {
                    self.rows.iter().any(|baseline| {
                        baseline.input == candidate.input
                            && baseline.algorithm == candidate.algorithm
                            && baseline.pattern == candidate.pattern
                            && baseline.samples == candidate.samples
                            && baseline.metrics == candidate.metrics
                            && baseline.within_tolerance == candidate.within_tolerance
                            && baseline.outlier_locations == candidate.outlier_locations
                    })
                }))
    }

    #[cfg(test)]
    pub(crate) const fn test_fixture() -> Self {
        Self {
            version: REFERENCE_FLOOR_VERSION,
            border_px: BORDER_PX,
            tolerance_codes: crate::metrics::TOLERANCE_CODES,
            rows: Vec::new(),
        }
    }
}

impl Selection {
    pub const fn is_partial(&self) -> bool {
        !self.algorithm.is_empty()
            || !self.reference.is_empty()
            || !self.pattern.is_empty()
            || !self.input.is_empty()
    }
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Kind {
    Exact,
    Rounding,
    Robust,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Pass,
    Fail,
}

impl Verdict {
    pub const fn symbol(self) -> &'static str {
        match self {
            Self::Pass => "✅",
            Self::Fail => "❌",
        }
    }
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Row {
    pub input: String,
    pub algorithm: String,
    pub reference: String,
    pub pattern: String,
    /// `[x, y]` pixels dropped from the native mosaic before demosaicing.
    pub crop: [u32; 2],
    pub native: bool,
    pub kind: Kind,
    pub samples: u64,
    pub verdict: Verdict,
    pub failed_limits: Vec<String>,
    #[serde(rename = "all")]
    pub all: Metrics,
    pub interior: Metrics,
    pub channels: Channels,
    pub sites: Sites,
    pub worst: Vec<WorstSample>,
    pub within_tolerance: f64,
    pub outlier_locations: OutlierLocations,
    pub reference_floor_match: bool,
    pub outlier_evidence: OutlierEvidence,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceFloorReport {
    pub version: u32,
    pub border_px: u32,
    pub tolerance_codes: u16,
    pub rows: Vec<ReferenceFloorRow>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ReferenceFloorRow {
    pub input: String,
    pub algorithm: String,
    pub pattern: String,
    pub samples: u64,
    pub metrics: Metrics,
    pub within_tolerance: f64,
    pub outlier_locations: OutlierLocations,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Metrics {
    pub mae: f64,
    pub rmse: f64,
    pub max: u16,
    pub mean_signed: f64,
    pub p99: u16,
    pub p999: u16,
    pub p9999: u16,
    pub frac_gt1: f64,
    pub frac_gt16: f64,
    pub count_gt1024: u64,
    pub changed_frac: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Channels {
    pub r: Metrics,
    pub g: Metrics,
    pub b: Metrics,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct Sites {
    pub sampled: Metrics,
    pub interpolated: Metrics,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorstSample {
    pub x: u32,
    pub y: u32,
    pub c: u8,
    pub fornax: u16,
    #[serde(rename = "ref")]
    pub reference: u16,
}

impl Report {
    pub fn new(
        meta: Meta,
        ground_truth: GroundTruthReport,
        reference_floor: ReferenceFloorReport,
        mut rows: Vec<Row>,
    ) -> Self {
        rows.sort_unstable_by(row_order);
        Self {
            schema_version: SCHEMA_VERSION,
            meta,
            ground_truth,
            reference_floor,
            rows,
        }
    }

    pub fn read(path: &Path) -> mischief::Result<Self> {
        let bytes = std::fs::read(path)
            .map_err(|error| mischief::mischief!("cannot read {}: {error}", path.display()))?;
        let report = serde_json::from_slice::<Self>(&bytes)
            .map_err(|error| mischief::mischief!("cannot parse {}: {error}", path.display()))?;
        report
            .validate()
            .map_err(|error| mischief::mischief!("invalid {}: {error}", path.display()))?;
        Ok(report)
    }

    pub fn write(&self, path: &Path) -> mischief::Result<()> {
        self.validate()?;
        if let Some(parent) = path.parent() {
            std::fs::create_dir_all(parent)?;
        }
        let file = std::fs::File::create(path)?;
        let mut writer = BufWriter::new(file);
        let meta = serde_json::to_string(&self.meta)
            .map_err(|error| mischief::mischief!("cannot serialize metadata: {error}"))?;
        let ground_truth = serde_json::to_string(&self.ground_truth)
            .map_err(|error| mischief::mischief!("cannot serialize ground truth: {error}"))?;
        let reference_floor = serde_json::to_string(&self.reference_floor)
            .map_err(|error| mischief::mischief!("cannot serialize reference floor: {error}"))?;
        writeln!(writer, "{{")?;
        writeln!(writer, "  \"schema_version\": {},", self.schema_version)?;
        writeln!(writer, "  \"meta\": {meta},")?;
        writeln!(writer, "  \"ground_truth\": {ground_truth},")?;
        writeln!(writer, "  \"reference_floor\": {reference_floor},")?;
        writeln!(writer, "  \"rows\": [")?;
        for (index, row) in self.rows.iter().enumerate() {
            let row = serde_json::to_string(row)
                .map_err(|error| mischief::mischief!("cannot serialize row: {error}"))?;
            if index + 1 == self.rows.len() {
                writeln!(writer, "    {row}")?;
            } else {
                writeln!(writer, "    {row},")?;
            }
        }
        writeln!(writer, "  ]")?;
        writeln!(writer, "}}")?;
        writer.flush()?;
        Ok(())
    }

    pub fn write_canonical(&self) -> mischief::Result<PathBuf> {
        let path = crate::run::repo_path(CANONICAL_FILE);
        self.write(&path)?;
        Ok(path)
    }

    pub fn write_baseline(&self) -> mischief::Result<PathBuf> {
        let path = crate::run::repo_path(BASELINE_FILE);
        self.write(&path)?;
        Ok(path)
    }

    pub fn validate(&self) -> mischief::Result<()> {
        if self.schema_version != SCHEMA_VERSION {
            return Err(mischief::mischief!(
                "unsupported canonical schema version {}",
                self.schema_version
            ));
        }
        if self.rows.is_empty() {
            return Err(mischief::mischief!("report has no rows"));
        }
        self.ground_truth.validate()?;
        self.reference_floor.validate()?;
        if self.meta.inputs.is_empty() {
            return Err(mischief::mischief!("report has no inputs"));
        }
        validate_selection_sorted("algorithm", &self.meta.selection.algorithm)?;
        validate_selection_sorted("reference", &self.meta.selection.reference)?;
        validate_selection_sorted("pattern", &self.meta.selection.pattern)?;
        validate_selection_sorted("input", &self.meta.selection.input)?;
        if self.meta.range_scaling.enabled && self.meta.range_scaling.ceiling_codes == 0 {
            return Err(mischief::mischief!(
                "reference range ceiling must be positive"
            ));
        }

        let mut input_names = self
            .meta
            .inputs
            .iter()
            .map(|input| input.name.as_str())
            .collect::<Vec<_>>();
        input_names.sort_unstable();
        input_names.dedup();
        if input_names.len() != self.meta.inputs.len() {
            return Err(mischief::mischief!("metadata contains duplicate inputs"));
        }
        for input in &self.meta.inputs {
            if input.name.is_empty() {
                return Err(mischief::mischief!("metadata contains an empty input name"));
            }
            if input.width == 0 || input.height == 0 {
                return Err(mischief::mischief!(
                    "input {} has zero dimensions",
                    input.name
                ));
            }
            if !input.reference_scale.is_finite()
                || !(0.0..=1.0).contains(&input.reference_scale)
                || round6(input.reference_scale) != input.reference_scale
            {
                return Err(mischief::mischief!(
                    "input {} has an invalid reference scale",
                    input.name
                ));
            }
            if input.sha256.len() != 64
                || !input.sha256.bytes().all(|byte| byte.is_ascii_hexdigit())
            {
                return Err(mischief::mischief!(
                    "input {} has an invalid sha256",
                    input.name
                ));
            }
            let expected_black = (input.black_width as usize)
                .checked_mul(input.black_height as usize)
                .ok_or_else(|| mischief::mischief!("black-level dimensions overflow"))?;
            if input.black.len() != expected_black {
                return Err(mischief::mischief!(
                    "input {} has inconsistent black levels",
                    input.name
                ));
            }
            if !BayerPattern::ALL.iter().any(|pattern| {
                pattern
                    .to_string()
                    .eq_ignore_ascii_case(&input.native_pattern)
            }) {
                return Err(mischief::mischief!(
                    "input {} has an unknown native pattern {}",
                    input.name,
                    input.native_pattern
                ));
            }
        }

        for pair in self.rows.windows(2) {
            if row_order(&pair[0], &pair[1]) != std::cmp::Ordering::Less {
                return Err(mischief::mischief!(
                    "report rows are not strictly sorted at {} {} {} {}",
                    pair[1].input,
                    pair[1].algorithm,
                    pair[1].reference,
                    pair[1].pattern
                ));
            }
        }
        for row in &self.rows {
            row.validate(&self.meta)?;
        }
        Ok(())
    }
}

impl Row {
    fn validate(&self, meta: &Meta) -> mischief::Result<()> {
        let input = meta
            .inputs
            .iter()
            .find(|input| input.name == self.input)
            .ok_or_else(|| mischief::mischief!("row references unknown input {}", self.input))?;
        let native = BayerPattern::ALL
            .into_iter()
            .find(|pattern| pattern.to_string() == input.native_pattern)
            .ok_or_else(|| mischief::mischief!("input has an unknown native pattern"))?;
        let pattern = BayerPattern::ALL
            .into_iter()
            .find(|pattern| pattern.to_string() == self.pattern)
            .ok_or_else(|| {
                mischief::mischief!(
                    "row {} {} {} has an unknown pattern",
                    self.input,
                    self.algorithm,
                    self.pattern
                )
            })?;
        let expected_crop = crate::reference::pattern_offset(native, pattern)
            .ok_or_else(|| mischief::mischief!("Bayer pattern conversion is unavailable"))?;
        if self.crop != [expected_crop.0, expected_crop.1] {
            return Err(mischief::mischief!(
                "row {} {} {} has crop {:?}, expected {:?}",
                self.input,
                self.algorithm,
                self.pattern,
                self.crop,
                [expected_crop.0, expected_crop.1]
            ));
        }
        if self.native != (self.pattern == input.native_pattern) {
            return Err(mischief::mischief!(
                "row {} {} {} has an incorrect native flag",
                self.input,
                self.algorithm,
                self.pattern
            ));
        }
        let width = input
            .width
            .checked_sub(self.crop[0])
            .ok_or_else(|| mischief::mischief!("row {} crop exceeds input width", self.input))?;
        let height = input
            .height
            .checked_sub(self.crop[1])
            .ok_or_else(|| mischief::mischief!("row {} crop exceeds input height", self.input))?;
        let pixels = u64::from(width) * u64::from(height);
        let samples = pixels
            .checked_mul(3)
            .ok_or_else(|| mischief::mischief!("row sample count overflow"))?;
        if self.samples != samples {
            return Err(mischief::mischief!(
                "row {} {} {} has samples {}, expected {samples}",
                self.input,
                self.algorithm,
                self.pattern,
                self.samples
            ));
        }
        let border = u64::from(meta.border_px);
        if border * 2 >= u64::from(width) || border * 2 >= u64::from(height) {
            return Err(mischief::mischief!(
                "border {} leaves no interior for input {}",
                meta.border_px,
                input.name
            ));
        }
        let interior_pixels = (u64::from(width) - border * 2) * (u64::from(height) - border * 2);
        validate_metrics("all", &self.all, self.samples)?;
        validate_metrics("interior", &self.interior, interior_pixels * 3)?;
        validate_metrics("channels.r", &self.channels.r, self.samples / 3)?;
        validate_metrics("channels.g", &self.channels.g, self.samples / 3)?;
        validate_metrics("channels.b", &self.channels.b, self.samples / 3)?;
        validate_metrics("sites.sampled", &self.sites.sampled, samples / 3)?;
        validate_metrics(
            "sites.interpolated",
            &self.sites.interpolated,
            samples * 2 / 3,
        )?;
        if self.worst.len() > 20 {
            return Err(mischief::mischief!("worst sample list exceeds 20 entries"));
        }
        if self.worst.windows(2).any(|pair| {
            pair[0].fornax.abs_diff(pair[0].reference) < pair[1].fornax.abs_diff(pair[1].reference)
        }) {
            return Err(mischief::mischief!("worst samples are not sorted by error"));
        }
        if self.worst.iter().any(|sample| {
            sample.x >= width
                || sample.y >= height
                || sample.c > 2
                || sample.fornax == sample.reference
        }) {
            return Err(mischief::mischief!("worst sample is outside the input"));
        }
        if !self.within_tolerance.is_finite()
            || !(0.0..=1.0).contains(&self.within_tolerance)
            || round6(self.within_tolerance) != self.within_tolerance
        {
            return Err(mischief::mischief!(
                "row {} {} {} has an invalid tolerance fraction",
                self.input,
                self.algorithm,
                self.pattern
            ));
        }
        let located = self
            .outlier_locations
            .border
            .saturating_add(self.outlier_locations.edge)
            .saturating_add(self.outlier_locations.flat);
        if located > self.samples {
            return Err(mischief::mischief!(
                "row {} {} {} has more located outliers than samples",
                self.input,
                self.algorithm,
                self.pattern
            ));
        }
        let evidence = &self.outlier_evidence;
        if evidence.synthetic_sources == 0
            || [
                evidence.fornax_mae,
                evidence.reference_mae,
                evidence.fornax_p99,
                evidence.reference_p99,
            ]
            .into_iter()
            .any(|value| !value.is_finite() || value < 0.0 || round6(value) != value)
        {
            return Err(mischief::mischief!(
                "row {} {} {} has invalid outlier evidence",
                self.input,
                self.algorithm,
                self.pattern
            ));
        }
        let interior_samples = interior_pixels * 3;
        let (verdict, failed_limits) =
            verdict(self.kind, &self.interior, interior_samples, meta.limits);
        if self.verdict != verdict || self.failed_limits != failed_limits {
            return Err(mischief::mischief!(
                "row {} {} {} has a stale verdict",
                self.input,
                self.algorithm,
                self.pattern
            ));
        }
        Ok(())
    }
}

pub fn verdict(
    kind: Kind,
    metrics: &Metrics,
    samples: u64,
    limits: Limits,
) -> (Verdict, Vec<String>) {
    let pass = match kind {
        Kind::Exact => metrics.max == 0,
        Kind::Rounding => metrics.max <= 1 && metrics.mean_signed == metrics.changed_frac,
        Kind::Robust => {
            metrics.mae <= limits.mae
                && metrics.p99 <= limits.p99
                && fraction(metrics.count_gt1024, samples) <= limits.high_err_frac
        }
    };
    let mut failed = Vec::new();
    if matches!(kind, Kind::Robust) {
        if metrics.mae > limits.mae {
            failed.push("mae".to_owned());
        }
        if metrics.p99 > limits.p99 {
            failed.push("p99".to_owned());
        }
        if fraction(metrics.count_gt1024, samples) > limits.high_err_frac {
            failed.push("high_err_frac".to_owned());
        }
    }
    (if pass { Verdict::Pass } else { Verdict::Fail }, failed)
}

pub fn fraction(count: u64, total: u64) -> f64 {
    if total == 0 {
        0.0
    } else {
        count as f64 / total as f64
    }
}

pub fn baseline_path() -> PathBuf { crate::run::repo_path(BASELINE_FILE) }

fn validate_selection_sorted(name: &str, values: &[String]) -> mischief::Result<()> {
    if values.windows(2).any(|pair| pair[0] >= pair[1]) {
        return Err(mischief::mischief!(
            "selection.{name} must be sorted and unique"
        ));
    }
    Ok(())
}

fn validate_metrics(scope: &str, metrics: &Metrics, samples: u64) -> mischief::Result<()> {
    let invalid = |message: &str| mischief::mischief!("{scope} {message}");
    if metrics.max > HIGH_ERROR_THRESHOLD && metrics.count_gt1024 == 0 {
        return Err(invalid("has a high maximum but no high-error samples"));
    }
    if metrics.p99 > metrics.p999 || metrics.p999 > metrics.p9999 || metrics.p9999 > metrics.max {
        return Err(invalid("has non-monotonic percentiles"));
    }
    if metrics.count_gt1024 > samples {
        return Err(invalid("has more high-error samples than samples"));
    }
    if !metrics.mae.is_finite()
        || metrics.mae < 0.0
        || !metrics.rmse.is_finite()
        || metrics.rmse < 0.0
        || !metrics.mean_signed.is_finite()
        || !metrics.frac_gt1.is_finite()
        || !(0.0..=1.0).contains(&metrics.frac_gt1)
        || !metrics.frac_gt16.is_finite()
        || !(0.0..=1.0).contains(&metrics.frac_gt16)
        || !metrics.changed_frac.is_finite()
        || !(0.0..=1.0).contains(&metrics.changed_frac)
    {
        return Err(invalid("has an invalid statistic"));
    }
    if metrics.frac_gt1 < metrics.frac_gt16 {
        return Err(invalid("has inverted threshold fractions"));
    }
    if round6(metrics.frac_gt1) != metrics.frac_gt1
        || round6(metrics.frac_gt16) != metrics.frac_gt16
        || round6(metrics.changed_frac) != metrics.changed_frac
        || round6(metrics.mae) != metrics.mae
        || round6(metrics.rmse) != metrics.rmse
        || round6(metrics.mean_signed) != metrics.mean_signed
    {
        return Err(invalid("contains an unrounded float"));
    }
    Ok(())
}

pub const fn round6(value: f64) -> f64 { (value * 1_000_000.0).round() / 1_000_000.0 }

fn row_order(left: &Row, right: &Row) -> std::cmp::Ordering {
    (&left.input, &left.algorithm, &left.reference, &left.pattern).cmp(&(
        &right.input,
        &right.algorithm,
        &right.reference,
        &right.pattern,
    ))
}

#[cfg(test)]
mod tests {
    use super::*;

    fn metrics() -> Metrics {
        Metrics {
            mae: 0.5,
            rmse: 1.0,
            max: 2,
            mean_signed: 0.5,
            p99: 1,
            p999: 1,
            p9999: 2,
            frac_gt1: 0.5,
            frac_gt16: 0.0,
            count_gt1024: 0,
            changed_frac: 0.5,
        }
    }

    fn report(verdict: Verdict, mae: f64) -> Report {
        let mut values = metrics();
        values.mae = mae;
        let row = Row {
            input: "input.raw".to_owned(),
            algorithm: "rcd".to_owned(),
            reference: "libraw".to_owned(),
            pattern: "RGGB".to_owned(),
            crop: [0, 0],
            native: true,
            kind: Kind::Robust,
            samples: 12,
            verdict,
            failed_limits: if verdict == Verdict::Fail {
                vec!["mae".to_owned()]
            } else {
                Vec::new()
            },
            all: values.clone(),
            interior: values.clone(),
            channels: Channels {
                r: values.clone(),
                g: values.clone(),
                b: values,
            },
            sites: Sites {
                sampled: metrics(),
                interpolated: metrics(),
            },
            worst: vec![WorstSample {
                x: 1,
                y: 1,
                c: 2,
                fornax: 2,
                reference: 0,
            }],
            within_tolerance: 1.0,
            outlier_locations: OutlierLocations::default(),
            reference_floor_match: false,
            outlier_evidence: OutlierEvidence::test_fixture(),
        };
        Report::new(
            Meta {
                inputs: vec![InputMeta {
                    name: "input.raw".to_owned(),
                    sha256: "00".repeat(32),
                    width: 2,
                    height: 2,
                    black: vec![0.0; 4],
                    black_width: 2,
                    black_height: 2,
                    white: 65_535.0,
                    native_pattern: "RGGB".to_owned(),
                    reference_scale: 1.0,
                }],
                references: References {
                    libraw: "0.22.3".to_owned(),
                    darktable: "5.6.2".to_owned(),
                    rawtherapee: "5.13".to_owned(),
                },
                limits: LIMITS,
                border_px: 0,
                pattern_source: PatternSource::ShiftedCrops,
                range_scaling: RangeScaling {
                    enabled: true,
                    ceiling_codes: 60_000,
                },
                selection: Selection::default(),
            },
            GroundTruthReport::test_fixture(),
            ReferenceFloorReport::test_fixture(),
            vec![row],
        )
    }

    #[test]
    fn verdict_boundaries_are_inclusive() {
        let mut input = metrics();
        input.mae = 2.0;
        input.p99 = 64;
        assert_eq!(
            verdict(Kind::Robust, &input, 12, LIMITS),
            (Verdict::Pass, Vec::new())
        );
        input.p99 = 65;
        assert_eq!(
            verdict(Kind::Robust, &input, 12, LIMITS),
            (Verdict::Fail, vec!["p99".to_owned()])
        );
        assert_eq!(
            verdict(Kind::Exact, &metrics(), 12, LIMITS).0,
            Verdict::Fail
        );
    }

    #[test]
    fn write_then_read_is_strict_and_line_oriented() -> mischief::Result<()> {
        let directory = tempfile::tempdir().expect("temporary directory");
        let path = directory.path().join("canonical.json");
        let report = report(Verdict::Pass, 0.5);
        report.write(&path)?;
        let text = std::fs::read_to_string(&path)?;
        assert_eq!(text.lines().count(), 9);
        assert!(text.contains("\"schema_version\": 3"));
        assert!(text.contains("\"ground_truth\":"));
        assert_eq!(Report::read(&path)?, report);

        let tampered = text.replacen("\"border_px\":0", "\"border_px\":0,\"unexpected\":true", 1);
        std::fs::write(&path, tampered)?;
        assert!(Report::read(&path).is_err());

        let missing_schema = text.replacen("  \"schema_version\": 3,\n", "", 1);
        std::fs::write(&path, missing_schema)?;
        assert!(Report::read(&path).is_err());

        let missing_ground_truth =
            text.replacen("  \"ground_truth\": {", "  \"unused_ground_truth\": {", 1);
        std::fs::write(&path, missing_ground_truth)?;
        assert!(Report::read(&path).is_err());

        let missing_reference_floor = text.replacen(
            "  \"reference_floor\": {",
            "  \"unused_reference_floor\": {",
            1,
        );
        std::fs::write(&path, missing_reference_floor)?;
        assert!(Report::read(&path).is_err());
        Ok(())
    }

    #[test]
    fn committed_baseline_loads_strictly() -> mischief::Result<()> {
        Report::read(&baseline_path())?;
        Ok(())
    }

    #[test]
    fn stale_verdicts_and_unsorted_rows_are_rejected() {
        let mut bad = report(Verdict::Pass, 0.5);
        bad.rows[0].verdict = Verdict::Fail;
        assert!(bad.validate().is_err());

        let mut first = report(Verdict::Pass, 0.5).rows.remove(0);
        let mut second = first.clone();
        first.algorithm = "vng".to_owned();
        second.algorithm = "amaze".to_owned();
        let mut unsorted = report(Verdict::Pass, 0.5);
        unsorted.rows = vec![first, second];
        assert!(unsorted.validate().is_err());
    }
}
