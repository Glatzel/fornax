use std::cmp::Ordering;
use std::io::Read;
use std::path::Path;

use fornax_raw_core::{BayerChannel, BayerPattern, MonoImage};
use fornax_raw_pipeline::{Demosaic, IDemosaic, LazyDemosaic};
use serde::{Deserialize, Serialize};

use crate::cases;
use crate::run::repo_path;

pub const VERSION: u32 = 2;
const WIDTH: u32 = 128;
const HEIGHT: u32 = 128;
const BORDER_PX: u32 = 8;
const REFERENCE_RANGE_ERROR_FRAGMENT: &str = "requires finite [0,1] samples";
const EXCLUDED_SOURCE_REASON: &str =
    "not all algorithms accept the explicit bounded reference range";
const RELATIVE_NOISE_FLOOR: f64 = 1.0e-3;
const LOG_NOISE_FLOOR: f64 = 1.0e-3;
const EDGE_RELATIVE_THRESHOLD: f64 = 0.02;
const SUSPICIOUS_MARGIN: f64 = 0.01;
const CANONICAL_PATTERN: BayerPattern = BayerPattern::GBRG;
const CORPUS_FILES: [&str; 3] = [
    "Canon/EOS 5D Mark III/M24-1221_000202.dng",
    "Sigma/fp/A002_653_20240229_000001_2k_8bit.DNG",
    "Nikon/D80/DSC_0143.dng",
];

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct GroundTruthReport {
    pub version: u32,
    pub pattern: String,
    pub width: u32,
    pub height: u32,
    pub border_px: u32,
    pub relative_noise_floor: f64,
    pub log_noise_floor: f64,
    pub edge_threshold: f64,
    pub suspicious_margin: f64,
    pub synthetic_sources: Vec<String>,
    pub excluded_sources: Vec<ExcludedSource>,
    pub corpus_sources: Vec<CorpusSource>,
    pub algorithms: Vec<AlgorithmGroundTruth>,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct ExcludedSource {
    pub name: String,
    pub reason: String,
}

#[derive(Clone, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct CorpusSource {
    pub path: String,
    pub status: CorpusStatus,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum CorpusStatus {
    Evaluated,
    Missing,
    TooSmall,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct AlgorithmGroundTruth {
    pub algorithm: String,
    pub rank: u32,
    pub samples: u64,
    pub relative: MetricSummary,
    pub log2: MetricSummary,
    pub zipper: MetricSummary,
    pub false_color: MetricSummary,
    pub worst: WorstLocation,
    pub vs_linear: LinearComparison,
    pub suspicious_sources: Vec<String>,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct WorstLocation {
    pub source: String,
    pub x: u32,
    pub y: u32,
    pub channel: u8,
    pub location: PixelClass,
    pub ground_truth: f64,
    pub fornax: f64,
    pub relative_error: f64,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum PixelClass {
    Edge,
    Flat,
    Border,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct LinearComparison {
    pub relative_mean_ratio: f64,
    pub log2_mean_delta: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct OutlierEvidence {
    pub synthetic_sources: u32,
    pub divergent_samples: u64,
    pub fornax_mae: f64,
    pub reference_mae: f64,
    pub fornax_p99: f64,
    pub reference_p99: f64,
    pub fornax_not_worse: bool,
}

impl OutlierEvidence {
    #[cfg(test)]
    pub(crate) const fn test_fixture() -> Self {
        Self {
            synthetic_sources: 1,
            divergent_samples: 0,
            fornax_mae: 0.0,
            reference_mae: 0.0,
            fornax_p99: 0.0,
            reference_p99: 0.0,
            fornax_not_worse: true,
        }
    }
}

impl GroundTruthReport {
    pub fn validate(&self) -> mischief::Result<()> {
        if self.version != VERSION {
            return Err(mischief::mischief!(
                "unsupported ground-truth version {}",
                self.version
            ));
        }
        if self.pattern != CANONICAL_PATTERN.to_string() {
            return Err(mischief::mischief!(
                "ground-truth pattern must be {CANONICAL_PATTERN}, got {}",
                self.pattern
            ));
        }
        if self.width != WIDTH || self.height != HEIGHT {
            return Err(mischief::mischief!(
                "ground-truth dimensions must be {WIDTH}x{HEIGHT}"
            ));
        }
        if self.synthetic_sources.is_empty() || self.algorithms.is_empty() {
            return Err(mischief::mischief!(
                "ground truth requires synthetic sources and algorithm rows"
            ));
        }
        let mut excluded_names = self
            .excluded_sources
            .iter()
            .map(|source| source.name.as_str())
            .collect::<Vec<_>>();
        excluded_names.sort_unstable();
        excluded_names.dedup();
        if excluded_names.len() != self.excluded_sources.len()
            || self.excluded_sources.iter().any(|source| {
                source.name.is_empty()
                    || source.reason.is_empty()
                    || self.synthetic_sources.contains(&source.name)
            })
        {
            return Err(mischief::mischief!(
                "ground truth has invalid excluded sources"
            ));
        }
        for value in [
            self.relative_noise_floor,
            self.log_noise_floor,
            self.edge_threshold,
            self.suspicious_margin,
        ] {
            if !value.is_finite() || value <= 0.0 || round6(value) != value {
                return Err(mischief::mischief!(
                    "ground-truth parameters must be positive rounded floats"
                ));
            }
        }
        let mut ranks = self
            .algorithms
            .iter()
            .map(|row| row.rank)
            .collect::<Vec<_>>();
        ranks.sort_unstable();
        if ranks != (1..=self.algorithms.len() as u32).collect::<Vec<_>>() {
            return Err(mischief::mischief!(
                "ground-truth ranks must be contiguous and unique"
            ));
        }
        let mut names = self
            .algorithms
            .iter()
            .map(|row| row.algorithm.to_ascii_lowercase())
            .collect::<Vec<_>>();
        names.sort_unstable();
        names.dedup();
        if names.len() != self.algorithms.len() {
            return Err(mischief::mischief!(
                "ground truth contains duplicate algorithms"
            ));
        }
        if !self
            .algorithms
            .iter()
            .any(|row| row.algorithm.eq_ignore_ascii_case("Linear"))
        {
            return Err(mischief::mischief!(
                "ground truth requires the Linear floor"
            ));
        }
        for row in &self.algorithms {
            if row.samples == 0 {
                return Err(mischief::mischief!(
                    "ground truth {} has no samples",
                    row.algorithm
                ));
            }
            for metrics in [row.relative, row.log2, row.zipper, row.false_color] {
                validate_summary(&row.algorithm, metrics)?;
            }
            if row.worst.x >= WIDTH || row.worst.y >= HEIGHT || row.worst.channel > 2 {
                return Err(mischief::mischief!(
                    "ground truth {} has an invalid worst pixel",
                    row.algorithm
                ));
            }
            for value in [
                row.worst.ground_truth,
                row.worst.fornax,
                row.worst.relative_error,
                row.vs_linear.relative_mean_ratio,
                row.vs_linear.log2_mean_delta,
            ] {
                if !value.is_finite() || round6(value) != value {
                    return Err(mischief::mischief!(
                        "ground truth {} contains an invalid float",
                        row.algorithm
                    ));
                }
            }
        }
        Ok(())
    }

    pub(crate) fn matches_selection(&self, other: &Self, algorithms: &[String]) -> bool {
        if algorithms.is_empty() {
            return self == other;
        }
        if self.version != other.version
            || self.pattern != other.pattern
            || self.width != other.width
            || self.height != other.height
            || self.border_px != other.border_px
            || self.relative_noise_floor != other.relative_noise_floor
            || self.log_noise_floor != other.log_noise_floor
            || self.edge_threshold != other.edge_threshold
            || self.suspicious_margin != other.suspicious_margin
            || self.synthetic_sources != other.synthetic_sources
            || self.excluded_sources != other.excluded_sources
            || self.corpus_sources != other.corpus_sources
        {
            return false;
        }
        other.algorithms.iter().all(|candidate| {
            let mut candidate = candidate.clone();
            candidate.rank = 0;
            self.algorithms.iter().any(|baseline| {
                let mut baseline = baseline.clone();
                baseline.rank = 0;
                baseline == candidate
            })
        })
    }

    #[cfg(test)]
    pub(crate) fn test_fixture() -> Self {
        let metrics = MetricSummary {
            mean: 0.0,
            p99: 0.0,
            p999: 0.0,
            max: 0.0,
        };
        Self {
            version: VERSION,
            pattern: CANONICAL_PATTERN.to_string(),
            width: WIDTH,
            height: HEIGHT,
            border_px: BORDER_PX,
            relative_noise_floor: RELATIVE_NOISE_FLOOR,
            log_noise_floor: LOG_NOISE_FLOOR,
            edge_threshold: EDGE_RELATIVE_THRESHOLD,
            suspicious_margin: SUSPICIOUS_MARGIN,
            synthetic_sources: vec!["fixture".to_owned()],
            excluded_sources: Vec::new(),
            corpus_sources: Vec::new(),
            algorithms: vec![AlgorithmGroundTruth {
                algorithm: "Linear".to_owned(),
                rank: 1,
                samples: 1,
                relative: metrics,
                log2: metrics,
                zipper: metrics,
                false_color: metrics,
                worst: WorstLocation {
                    source: "fixture".to_owned(),
                    x: 0,
                    y: 0,
                    channel: 0,
                    location: PixelClass::Flat,
                    ground_truth: 0.0,
                    fornax: 0.0,
                    relative_error: 0.0,
                },
                vs_linear: LinearComparison {
                    relative_mean_ratio: 1.0,
                    log2_mean_delta: 0.0,
                },
                suspicious_sources: Vec::new(),
            }],
        }
    }
}

fn validate_summary(algorithm: &str, summary: MetricSummary) -> mischief::Result<()> {
    for value in [summary.mean, summary.p99, summary.p999, summary.max] {
        if !value.is_finite() || value < 0.0 || round6(value) != value {
            return Err(mischief::mischief!(
                "ground truth {algorithm} contains an invalid metric"
            ));
        }
    }
    if summary.p99 > summary.p999 || summary.p999 > summary.max {
        return Err(mischief::mischief!(
            "ground truth {algorithm} has non-monotonic metrics"
        ));
    }
    Ok(())
}

#[derive(Clone, Copy, Debug, Deserialize, PartialEq, Serialize)]
#[serde(deny_unknown_fields)]
pub struct MetricSummary {
    pub mean: f64,
    pub p99: f64,
    pub p999: f64,
    pub max: f64,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum SourceKind {
    ZonePlate,
    Edge,
    ColoredEdge,
    FineLines,
    Gradient,
    Hdr,
    Corpus,
}

impl SourceKind {
    const fn is_suspicious_scope(self) -> bool {
        matches!(
            self,
            Self::ZonePlate | Self::Edge | Self::ColoredEdge | Self::FineLines
        )
    }
}

#[derive(Clone, Debug, PartialEq)]
struct Source {
    name: String,
    kind: SourceKind,
    truth: Vec<[f32; 3]>,
    edge: Vec<bool>,
    mosaic: Vec<f32>,
}

#[derive(Clone, Debug)]
struct SourceErrors {
    relative: Vec<f64>,
    log2: Vec<f64>,
    zipper: Vec<f64>,
    false_color: Vec<f64>,
    worst: WorstCandidate,
}

#[derive(Clone, Debug)]
struct WorstCandidate {
    source: String,
    x: u32,
    y: u32,
    channel: u8,
    location: PixelClass,
    ground_truth: f64,
    fornax: f64,
    relative_error: f64,
}

fn summarize(mut values: Vec<f64>) -> MetricSummary {
    if values.is_empty() {
        return MetricSummary {
            mean: 0.0,
            p99: 0.0,
            p999: 0.0,
            max: 0.0,
        };
    }
    let mean = round6(values.iter().sum::<f64>() / values.len() as f64);
    values.sort_by(f64::total_cmp);
    let percentile = |fraction: f64| {
        let rank = ((values.len() as f64 * fraction).ceil() as usize)
            .saturating_sub(1)
            .min(values.len() - 1);
        round6(values[rank])
    };
    MetricSummary {
        mean,
        p99: percentile(0.99),
        p999: percentile(0.999),
        max: round6(*values.last().expect("non-empty values")),
    }
}

fn round6(value: f64) -> f64 { (value * 1_000_000.0).round() / 1_000_000.0 }

fn bin_bayer_quad(codes: &[u16; 4], pattern: BayerPattern) -> [f32; 3] {
    let mut sums = [0_u32; 3];
    let mut counts = [0_u32; 3];
    for y in 0..2 {
        for x in 0..2 {
            let index = y * 2 + x;
            let channel = match pattern.channel_at(x as u32, y as u32) {
                BayerChannel::R => 0,
                BayerChannel::GRedRow | BayerChannel::GBlueRow => 1,
                BayerChannel::B => 2,
            };
            sums[channel] += u32::from(codes[index]);
            counts[channel] += 1;
        }
    }
    std::array::from_fn(|channel| {
        sums[channel] as f32 / counts[channel] as f32 / f32::from(u16::MAX)
    })
}

fn synthetic_sources() -> Vec<Source> {
    vec![
        scene("zone_plate", SourceKind::ZonePlate, 0x7a_01, |x, y| {
            let nx = (x as f32 - 64.0) / 64.0;
            let ny = (y as f32 - 64.0) / 64.0;
            let value = 0.5 + 0.35 * (38.0 * (nx * nx + ny * ny)).cos();
            [value; 3]
        }),
        scene("slanted_edge", SourceKind::Edge, 0x7a_02, |x, y| {
            if (x as f32) + 0.37 * (y as f32) < 80.0 {
                [0.04, 0.08, 0.03]
            } else {
                [0.82, 0.66, 0.53]
            }
        }),
        scene(
            "colored_edges",
            SourceKind::ColoredEdge,
            0x7a_03,
            |x, y| match (x / 20 + y / 28) % 4 {
                0 => [0.85, 0.08, 0.05],
                1 => [0.05, 0.75, 0.12],
                2 => [0.04, 0.10, 0.88],
                _ => [0.83, 0.72, 0.08],
            },
        ),
        scene("fine_lines", SourceKind::FineLines, 0x7a_04, |x, y| {
            if (x + y * 3) & 1 == 0 {
                [0.88, 0.18, 0.72]
            } else {
                [0.06, 0.46, 0.08]
            }
        }),
        scene("smooth_gradient", SourceKind::Gradient, 0x7a_05, |x, y| {
            [
                0.03 + 0.72 * x as f32 / 127.0,
                0.05 + 0.58 * y as f32 / 127.0,
                0.02 + 0.32 * x as f32 / 127.0 + 0.28 * y as f32 / 127.0,
            ]
        }),
        scene("hdr_blocks", SourceKind::Hdr, 0x7a_06, |x, y| {
            let levels = [1.0 / 256.0, 1.0 / 16.0, 1.0, 2.0, 4.0, 16.0];
            let level = levels[((x / 32) + 4 * (y / 32)) as usize % levels.len()];
            [level, level * 0.8, level * 0.6]
        }),
    ]
}

fn scene(
    name: &'static str,
    kind: SourceKind,
    seed: u64,
    value: impl Fn(u32, u32) -> [f32; 3],
) -> Source {
    let mut truth = Vec::with_capacity((WIDTH * HEIGHT) as usize);
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let mut pixel = value(x, y);
            for (channel, sample) in pixel.iter_mut().enumerate() {
                *sample += 1.0e-4 * noise(seed, x, y, channel as u32);
            }
            truth.push(pixel);
        }
    }
    let edge = derive_edges(&truth);
    let mosaic = re_mosaic(&truth, CANONICAL_PATTERN);
    Source {
        name: name.to_owned(),
        kind,
        truth,
        edge,
        mosaic,
    }
}

fn noise(seed: u64, x: u32, y: u32, channel: u32) -> f32 {
    let mut value = seed
        ^ u64::from(x).wrapping_mul(0x9e37_79b9_7f4a_7c15)
        ^ u64::from(y).wrapping_mul(0xbf58_476d_1ce4_e5b9)
        ^ u64::from(channel).wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^= value >> 30;
    value = value.wrapping_mul(0xbf58_476d_1ce4_e5b9);
    value ^= value >> 27;
    value = value.wrapping_mul(0x94d0_49bb_1331_11eb);
    value ^= value >> 31;
    ((value >> 40) as u32) as f32 / ((1_u32 << 24) as f32) * 2.0 - 1.0
}

fn derive_edges(truth: &[[f32; 3]]) -> Vec<bool> {
    let mut edges = vec![false; truth.len()];
    for y in 0..HEIGHT as usize {
        for x in 0..WIDTH as usize {
            let center = truth[y * WIDTH as usize + x];
            let mut neighbour_max = [0.0_f64; 3];
            let mut neighbour = Vec::with_capacity(4);
            if x > 0 {
                neighbour.push(truth[y * WIDTH as usize + x - 1]);
            }
            if x + 1 < WIDTH as usize {
                neighbour.push(truth[y * WIDTH as usize + x + 1]);
            }
            if y > 0 {
                neighbour.push(truth[(y - 1) * WIDTH as usize + x]);
            }
            if y + 1 < HEIGHT as usize {
                neighbour.push(truth[(y + 1) * WIDTH as usize + x]);
            }
            let mut gradient = 0.0_f64;
            for pixel in neighbour {
                for channel in 0..3 {
                    gradient = gradient
                        .max((f64::from(center[channel]) - f64::from(pixel[channel])).abs());
                    neighbour_max[channel] = neighbour_max[channel]
                        .max(f64::from(center[channel]).abs())
                        .max(f64::from(pixel[channel]).abs());
                }
            }
            let scale = neighbour_max
                .into_iter()
                .fold(RELATIVE_NOISE_FLOOR, f64::max);
            edges[y * WIDTH as usize + x] = gradient > EDGE_RELATIVE_THRESHOLD * scale;
        }
    }
    edges
}

fn re_mosaic(truth: &[[f32; 3]], pattern: BayerPattern) -> Vec<f32> {
    truth
        .iter()
        .enumerate()
        .map(|(index, pixel)| {
            let x = index as u32 % WIDTH;
            let y = index as u32 / WIDTH;
            let channel = match pattern.channel_at(x, y) {
                BayerChannel::R => 0,
                BayerChannel::GRedRow | BayerChannel::GBlueRow => 1,
                BayerChannel::B => 2,
            };
            pixel[channel]
        })
        .collect()
}

pub(crate) fn evaluate(algorithms: &[String]) -> mischief::Result<GroundTruthReport> {
    evaluate_algorithms(algorithms, true)
}

pub(crate) fn evaluate_reference_evidence(
    case: &crate::cases::Case,
    pattern: BayerPattern,
) -> mischief::Result<OutlierEvidence> {
    let mut fornax_errors = Vec::new();
    let mut reference_errors = Vec::new();
    let sources = synthetic_sources();
    for source in &sources {
        let maximum = source
            .truth
            .iter()
            .flat_map(|pixel| pixel.iter().copied())
            .fold(0.0_f32, f32::max);
        let scale = (60_000.0_f32 / f32::from(u16::MAX)) / maximum.max(f32::EPSILON);
        let scaled_truth = source
            .truth
            .iter()
            .map(|pixel| pixel.map(|sample| sample * scale))
            .collect::<Vec<_>>();
        let mosaic = re_mosaic(&scaled_truth, pattern);
        let codes = mosaic
            .iter()
            .map(|sample| (sample * f32::from(u16::MAX)).round() as u16)
            .collect::<Vec<_>>();
        let truth_codes = scaled_truth
            .iter()
            .flat_map(|pixel| pixel.map(|sample| (sample * f32::from(u16::MAX)).round() as u16))
            .collect::<Vec<_>>();
        let reference = crate::reference::run(
            case.reference_backend,
            case.algorithm,
            &codes,
            WIDTH,
            HEIGHT,
            crate::reference::libraw_filters(pattern),
        )?;
        let fornax = (case.run_fornax)(
            &codes,
            WIDTH,
            HEIGHT,
            pattern,
            crate::cases::SampleType::F32,
        )?;
        for y in BORDER_PX..HEIGHT - BORDER_PX {
            for x in BORDER_PX..WIDTH - BORDER_PX {
                let pixel = y as usize * WIDTH as usize + x as usize;
                for channel in 0..3 {
                    let index = pixel * 3 + channel;
                    if fornax[index].abs_diff(reference[index]) > crate::metrics::TOLERANCE_CODES {
                        let truth = truth_codes[pixel * 3 + channel];
                        fornax_errors.push(u64::from(fornax[index].abs_diff(truth)));
                        reference_errors.push(u64::from(reference[index].abs_diff(truth)));
                    }
                }
            }
        }
    }
    let divergent_samples = u64::try_from(fornax_errors.len())
        .map_err(|_| mischief::mischief!("outlier evidence sample count exceeds u64"))?;
    let summary = |mut values: Vec<u64>| {
        if values.is_empty() {
            return (0.0, 0.0);
        }
        let mean = values.iter().sum::<u64>() as f64 / values.len() as f64;
        values.sort_unstable();
        let rank = ((values.len() as f64 * 0.99).ceil() as usize)
            .saturating_sub(1)
            .min(values.len() - 1);
        (round6(mean), round6(values[rank] as f64))
    };
    let (fornax_mae, fornax_p99) = summary(fornax_errors);
    let (reference_mae, reference_p99) = summary(reference_errors);
    Ok(OutlierEvidence {
        synthetic_sources: sources.len() as u32,
        divergent_samples,
        fornax_mae,
        reference_mae,
        fornax_p99,
        reference_p99,
        fornax_not_worse: fornax_mae <= reference_mae + 1.0 && fornax_p99 <= reference_p99 + 1.0,
    })
}

fn evaluate_algorithms(
    algorithms: &[String],
    include_corpus: bool,
) -> mischief::Result<GroundTruthReport> {
    let mut algorithms = algorithms
        .iter()
        .map(|algorithm| cases::display_name(algorithm).to_owned())
        .collect::<Vec<_>>();
    algorithms.sort_unstable_by_key(|algorithm| algorithm.to_ascii_lowercase());
    algorithms.dedup_by(|left, right| left.eq_ignore_ascii_case(right));
    if algorithms.is_empty() {
        return Err(mischief::mischief!(
            "ground truth requires at least one algorithm"
        ));
    }
    for algorithm in &algorithms {
        cases::demosaic_for(algorithm)
            .ok_or_else(|| mischief::mischief!("unknown ground-truth algorithm `{algorithm}`"))?;
    }

    let mut sources = synthetic_sources();
    let corpus_sources = if include_corpus {
        let (corpus, statuses) = corpus_sources()?;
        sources.extend(corpus);
        statuses
    } else {
        Vec::new()
    };

    let linear_algorithm = "Linear".to_owned();
    if !algorithms
        .iter()
        .any(|algorithm| algorithm.eq_ignore_ascii_case("Linear"))
    {
        algorithms.push("Linear".to_owned());
    }
    algorithms.sort_unstable_by_key(|algorithm| algorithm.to_ascii_lowercase());

    let mut excluded_sources = Vec::new();
    for source in sources
        .iter()
        .filter(|source| source.kind != SourceKind::Corpus)
    {
        let mut excluded = false;
        for algorithm in cases::algorithm_names() {
            let demosaic = cases::demosaic_for(algorithm).ok_or_else(|| {
                mischief::mischief!("unknown ground-truth algorithm `{algorithm}`")
            })?;
            match evaluate_source(demosaic, source) {
                Ok(_) => {}
                Err(error) if error.to_string().contains(REFERENCE_RANGE_ERROR_FRAGMENT) => {
                    excluded = true;
                }
                Err(error) => return Err(error),
            }
        }
        if excluded {
            excluded_sources.push(ExcludedSource {
                name: source.name.clone(),
                reason: EXCLUDED_SOURCE_REASON.into(),
            });
        }
    }
    let excluded_names = excluded_sources
        .iter()
        .map(|source| source.name.as_str())
        .collect::<Vec<_>>();
    sources.retain(|source| !excluded_names.contains(&source.name.as_str()));
    if sources.is_empty() {
        return Err(mischief::mischief!(
            "all ground-truth sources were excluded"
        ));
    }

    let mut evaluated = Vec::with_capacity(algorithms.len());
    let mut linear_means = Vec::new();
    for algorithm in &algorithms {
        let demosaic = cases::demosaic_for(algorithm)
            .ok_or_else(|| mischief::mischief!("unknown ground-truth algorithm `{algorithm}`"))?;
        let evaluation = evaluate_algorithm(algorithm, demosaic, &sources)?;
        if algorithm == &linear_algorithm {
            linear_means = evaluation
                .source_relative_means
                .iter()
                .map(|mean| mean.mean)
                .collect();
        }
        evaluated.push(evaluation);
    }
    // The alphabetical insertion above can move Linear, so resolve it by name.
    if linear_means.is_empty() {
        let linear = evaluated
            .iter()
            .find(|row| row.algorithm.eq_ignore_ascii_case("Linear"))
            .ok_or_else(|| mischief::mischief!("ground truth did not evaluate Linear"))?;
        linear_means = linear
            .source_relative_means
            .iter()
            .map(|mean| mean.mean)
            .collect();
    }
    let linear = evaluated
        .iter()
        .find(|row| row.algorithm.eq_ignore_ascii_case("Linear"))
        .ok_or_else(|| mischief::mischief!("ground truth did not evaluate Linear"))?;
    let linear_relative_mean = linear.relative.mean;
    let linear_log2_mean = linear.log2.mean;

    let mut rows = evaluated
        .into_iter()
        .map(|evaluation| {
            let suspicious_sources = if evaluation.algorithm.eq_ignore_ascii_case("Linear") {
                Vec::new()
            } else {
                sources
                    .iter()
                    .zip(&evaluation.source_relative_means)
                    .zip(&linear_means)
                    .filter(|((source, candidate), linear)| {
                        source.kind.is_suspicious_scope()
                            && candidate.mean > *linear * (1.0 + SUSPICIOUS_MARGIN)
                    })
                    .map(|((source, _), _)| source.name.clone())
                    .collect()
            };
            AlgorithmGroundTruth {
                algorithm: evaluation.algorithm,
                rank: 0,
                samples: evaluation.samples,
                relative: evaluation.relative,
                log2: evaluation.log2,
                zipper: evaluation.zipper,
                false_color: evaluation.false_color,
                worst: evaluation.worst,
                vs_linear: LinearComparison {
                    relative_mean_ratio: ratio(evaluation.relative.mean, linear_relative_mean),
                    log2_mean_delta: round6(evaluation.log2.mean - linear_log2_mean),
                },
                suspicious_sources,
            }
        })
        .collect::<Vec<_>>();
    rows.sort_unstable_by(|left, right| {
        left.relative
            .mean
            .total_cmp(&right.relative.mean)
            .then_with(|| left.algorithm.cmp(&right.algorithm))
    });
    for (index, row) in rows.iter_mut().enumerate() {
        row.rank = index as u32 + 1;
    }

    Ok(GroundTruthReport {
        version: VERSION,
        pattern: CANONICAL_PATTERN.to_string(),
        width: WIDTH,
        height: HEIGHT,
        border_px: BORDER_PX,
        relative_noise_floor: round6(RELATIVE_NOISE_FLOOR),
        log_noise_floor: round6(LOG_NOISE_FLOOR),
        edge_threshold: round6(EDGE_RELATIVE_THRESHOLD),
        suspicious_margin: round6(SUSPICIOUS_MARGIN),
        synthetic_sources: sources
            .iter()
            .filter(|source| source.kind != SourceKind::Corpus)
            .map(|source| source.name.clone())
            .collect(),
        excluded_sources,
        corpus_sources,
        algorithms: rows,
    })
}

struct AlgorithmEvaluation {
    algorithm: String,
    samples: u64,
    relative: MetricSummary,
    log2: MetricSummary,
    zipper: MetricSummary,
    false_color: MetricSummary,
    worst: WorstLocation,
    source_relative_means: Vec<MetricSummary>,
}

fn evaluate_algorithm(
    algorithm: &str,
    demosaic: Demosaic,
    sources: &[Source],
) -> mischief::Result<AlgorithmEvaluation> {
    let mut relative = Vec::new();
    let mut log2 = Vec::new();
    let mut zipper = Vec::new();
    let mut false_color = Vec::new();
    let mut source_relative_means = Vec::with_capacity(sources.len());
    let mut worst = None;
    for source in sources {
        let errors = evaluate_source(demosaic, source)?;
        source_relative_means.push(summarize(errors.relative.clone()));
        relative.extend(errors.relative);
        log2.extend(errors.log2);
        zipper.extend(errors.zipper);
        false_color.extend(errors.false_color);
        if worst.as_ref().is_none_or(|current: &WorstCandidate| {
            errors
                .worst
                .relative_error
                .total_cmp(&current.relative_error)
                == Ordering::Greater
        }) {
            worst = Some(errors.worst);
        }
    }
    let worst = worst.ok_or_else(|| mischief::mischief!("ground truth has no source samples"))?;
    let samples = u64::try_from(relative.len())
        .map_err(|_| mischief::mischief!("ground-truth sample count exceeds u64"))?;
    Ok(AlgorithmEvaluation {
        algorithm: algorithm.to_owned(),
        samples,
        relative: summarize(relative),
        log2: summarize(log2),
        zipper: summarize(zipper),
        false_color: summarize(false_color),
        worst: WorstLocation {
            source: worst.source,
            x: worst.x,
            y: worst.y,
            channel: worst.channel,
            location: worst.location,
            ground_truth: round6(worst.ground_truth),
            fornax: round6(worst.fornax),
            relative_error: round6(worst.relative_error),
        },
        source_relative_means,
    })
}

fn evaluate_source(demosaic: Demosaic, source: &Source) -> mischief::Result<SourceErrors> {
    let image = MonoImage::from_raw(WIDTH, HEIGHT, source.mosaic.clone())
        .ok_or_else(|| mischief::mischief!("cannot construct ground-truth mosaic"))?;
    let output = demosaic
        .demosaic(&image, CANONICAL_PATTERN)
        .map_err(|error| mischief::mischief!("ground truth {} failed: {error:?}", source.name))?
        .collect()
        .into_raw();
    let expected = source.truth.len() * 3;
    if output.len() != expected {
        return Err(mischief::mischief!(
            "ground truth {} produced {} samples, expected {expected}",
            source.name,
            output.len()
        ));
    }

    let mut relative = Vec::with_capacity(expected);
    let mut log2 = Vec::with_capacity(expected);
    let mut zipper = Vec::new();
    let mut false_color = Vec::new();
    let mut worst = WorstCandidate {
        source: source.name.clone(),
        x: 0,
        y: 0,
        channel: 0,
        location: PixelClass::Flat,
        ground_truth: 0.0,
        fornax: 0.0,
        relative_error: -1.0,
    };
    for y in 0..HEIGHT {
        for x in 0..WIDTH {
            let pixel = y as usize * WIDTH as usize + x as usize;
            let truth = source.truth[pixel];
            let actual = [
                output[pixel * 3],
                output[pixel * 3 + 1],
                output[pixel * 3 + 2],
            ];
            if actual.iter().any(|sample| !sample.is_finite()) {
                return Err(mischief::mischief!(
                    "ground truth {} produced a non-finite pixel at ({x}, {y})",
                    source.name
                ));
            }
            let location = pixel_class(source, x, y);
            for channel in 0..3 {
                let ground_truth = f64::from(truth[channel]);
                let fornax = f64::from(actual[channel]);
                let denominator = ground_truth
                    .abs()
                    .max(fornax.abs())
                    .max(RELATIVE_NOISE_FLOOR);
                let error = (fornax - ground_truth).abs() / denominator;
                if location != PixelClass::Border {
                    relative.push(error);
                    log2.push(
                        (fornax.abs().max(LOG_NOISE_FLOOR)
                            / ground_truth.abs().max(LOG_NOISE_FLOOR))
                        .log2()
                        .abs(),
                    );
                }
                if error > worst.relative_error {
                    worst = WorstCandidate {
                        source: source.name.clone(),
                        x,
                        y,
                        channel: channel as u8,
                        location,
                        ground_truth,
                        fornax,
                        relative_error: error,
                    };
                }
            }
            if location == PixelClass::Edge {
                let truth_luma = luma(truth);
                let actual_luma = luma(actual);
                let luma_denominator = truth_luma
                    .abs()
                    .max(actual_luma.abs())
                    .max(RELATIVE_NOISE_FLOOR);
                zipper.push((actual_luma - truth_luma).abs() / luma_denominator);
                let truth_chroma = chroma(truth);
                let actual_chroma = chroma(actual);
                false_color.push(
                    ((actual_chroma[0] - truth_chroma[0]).powi(2)
                        + (actual_chroma[1] - truth_chroma[1]).powi(2))
                    .sqrt()
                        / luma_denominator,
                );
            }
        }
    }
    Ok(SourceErrors {
        relative,
        log2,
        zipper,
        false_color,
        worst,
    })
}

fn pixel_class(source: &Source, x: u32, y: u32) -> PixelClass {
    if x < BORDER_PX || y < BORDER_PX || x >= WIDTH - BORDER_PX || y >= HEIGHT - BORDER_PX {
        PixelClass::Border
    } else if source.edge[y as usize * WIDTH as usize + x as usize] {
        PixelClass::Edge
    } else {
        PixelClass::Flat
    }
}

fn luma(pixel: [f32; 3]) -> f64 {
    0.2126 * f64::from(pixel[0]) + 0.7152 * f64::from(pixel[1]) + 0.0722 * f64::from(pixel[2])
}

fn chroma(pixel: [f32; 3]) -> [f64; 2] {
    [
        f64::from(pixel[0]) - f64::from(pixel[1]),
        f64::from(pixel[2]) - f64::from(pixel[1]),
    ]
}

fn ratio(value: f64, reference: f64) -> f64 {
    if reference == 0.0 {
        0.0
    } else {
        round6(value / reference)
    }
}

fn corpus_sources() -> mischief::Result<(Vec<Source>, Vec<CorpusSource>)> {
    let mut sources = Vec::new();
    let mut statuses = Vec::with_capacity(CORPUS_FILES.len());
    for relative in CORPUS_FILES {
        let path = repo_path(&format!("external/raw.pixls.us/{relative}"));
        if !is_materialized(&path)? {
            statuses.push(CorpusSource {
                path: relative.to_owned(),
                status: CorpusStatus::Missing,
            });
            continue;
        }
        let input = crate::reference::active_area_from(&path)?;
        if input.width < WIDTH || input.height < HEIGHT {
            statuses.push(CorpusSource {
                path: relative.to_owned(),
                status: CorpusStatus::TooSmall,
            });
            continue;
        }
        sources.push(corpus_source(relative, input));
        statuses.push(CorpusSource {
            path: relative.to_owned(),
            status: CorpusStatus::Evaluated,
        });
    }
    Ok((sources, statuses))
}

fn is_materialized(path: &Path) -> mischief::Result<bool> {
    let Ok(metadata) = std::fs::metadata(path) else {
        return Ok(false);
    };
    if metadata.len() < 1_024 {
        return Ok(false);
    }
    let mut prefix = [0_u8; 48];
    let count = std::fs::File::open(path)?.read(&mut prefix)?;
    Ok(!prefix[..count].starts_with(b"version https://git-lfs.github.com/spec/v1"))
}

fn corpus_source(relative: &str, input: crate::reference::ActiveArea) -> Source {
    let x = ((input.width - WIDTH) / 2) & !1;
    let y = ((input.height - HEIGHT) / 2) & !1;
    let half_width = WIDTH / 2;
    let half_height = HEIGHT / 2;
    let mut half_truth = Vec::with_capacity((half_width * half_height) as usize);
    for block_y in 0..half_height {
        for block_x in 0..half_width {
            let source_x = x + block_x * 2;
            let source_y = y + block_y * 2;
            let row = source_y as usize * input.width as usize + source_x as usize;
            let codes = [
                input.codes[row],
                input.codes[row + 1],
                input.codes[row + input.width as usize],
                input.codes[row + input.width as usize + 1],
            ];
            half_truth.push(bin_bayer_quad(&codes, input.native_pattern));
        }
    }
    let truth = (0..HEIGHT)
        .flat_map(|y| {
            let half_truth = &half_truth;
            (0..WIDTH).map(move |x| half_truth[(y / 2 * half_width + x / 2) as usize])
        })
        .collect::<Vec<_>>();
    let edge = derive_edges(&truth);
    let mosaic = re_mosaic(&truth, CANONICAL_PATTERN);
    Source {
        name: format!("corpus:{relative}"),
        kind: SourceKind::Corpus,
        truth,
        edge,
        mosaic,
    }
}

#[cfg(test)]
mod tests {
    use fornax_raw_core::BayerPattern;

    use super::*;

    #[test]
    fn bayer_binning_averages_both_green_samples() {
        let truth = bin_bayer_quad(&[10, 20, 30, 40], BayerPattern::RGGB);
        assert_eq!(truth, [10.0 / 65_535.0, 25.0 / 65_535.0, 40.0 / 65_535.0]);
    }

    #[test]
    fn synthetic_scene_set_is_deterministic_and_contains_hdr_and_shadows() {
        let first = synthetic_sources();
        let second = synthetic_sources();
        assert_eq!(first.len(), 6);
        assert_eq!(first, second);
        let hdr = first
            .iter()
            .find(|source| source.name == "hdr_blocks")
            .expect("HDR scene");
        let maximum = hdr
            .truth
            .iter()
            .flat_map(|pixel| pixel.iter().copied())
            .fold(f32::NEG_INFINITY, f32::max);
        let minimum = hdr
            .truth
            .iter()
            .flat_map(|pixel| pixel.iter().copied())
            .fold(f32::INFINITY, f32::min);
        assert!(maximum >= 16.0);
        assert!(minimum <= 1.0 / 256.0);
    }

    #[test]
    fn summaries_report_mean_and_high_percentiles() {
        let summary = summarize((1..=1_000).map(f64::from).collect());
        assert_eq!(
            summary,
            MetricSummary {
                mean: 500.5,
                p99: 990.0,
                p999: 999.0,
                max: 1_000.0,
            }
        );
    }

    #[test]
    fn linear_is_the_ground_truth_floor_and_is_its_own_reference() -> mischief::Result<()> {
        let report = evaluate_algorithms(&["Linear".to_owned()], false)?;
        assert_eq!(report.version, 2);
        assert_eq!(report.pattern, "GBRG");
        assert_eq!(report.synthetic_sources.len(), 5);
        assert_eq!(report.excluded_sources.len(), 1);
        assert_eq!(report.excluded_sources[0].name, "hdr_blocks");
        assert_eq!(report.algorithms.len(), 1);
        let linear = &report.algorithms[0];
        assert_eq!(linear.algorithm, "Linear");
        assert_eq!(linear.rank, 1);
        assert_eq!(linear.vs_linear.relative_mean_ratio, 1.0);
        assert_eq!(linear.vs_linear.log2_mean_delta, 0.0);
        assert!(linear.samples > 0);
        assert!(linear.suspicious_sources.is_empty());
        Ok(())
    }

    #[test]
    #[ignore = "ground-truth harness; run explicitly with --ignored"]
    fn all_algorithms_have_deterministic_contiguous_ranks() -> mischief::Result<()> {
        let algorithms = crate::cases::algorithm_names()
            .into_iter()
            .map(ToOwned::to_owned)
            .collect::<Vec<_>>();
        let first = evaluate_algorithms(&algorithms, false)?;
        let second = evaluate_algorithms(&algorithms, false)?;
        first.validate()?;
        assert_eq!(first, second);
        assert_eq!(first.algorithms.len(), 9);
        let mut ranks = first
            .algorithms
            .iter()
            .map(|row| row.rank)
            .collect::<Vec<_>>();
        ranks.sort_unstable();
        assert_eq!(ranks, (1..=9).collect::<Vec<_>>());
        assert!(
            first
                .algorithms
                .iter()
                .any(|row| !row.suspicious_sources.is_empty())
        );
        for row in &first.algorithms {
            println!(
                "{} rank={} relative_mean={:.6} vs_linear={:.3}x suspicious={}",
                row.algorithm,
                row.rank,
                row.relative.mean,
                row.vs_linear.relative_mean_ratio,
                row.suspicious_sources.join(",")
            );
        }
        Ok(())
    }

    #[test]
    fn partial_selection_ignores_subset_rank_changes_but_not_metrics() {
        let baseline = GroundTruthReport::test_fixture();
        let mut partial = baseline.clone();
        partial.algorithms[0].rank = 2;
        assert!(baseline.matches_selection(&partial, &["linear".to_owned()]));

        partial.algorithms[0].relative.mean = 0.5;
        assert!(!baseline.matches_selection(&partial, &["linear".to_owned()]));
        assert!(!baseline.matches_selection(&partial, &[]));
    }

    #[test]
    fn reference_evidence_uses_ground_truth_on_divergent_samples() -> mischief::Result<()> {
        let case = crate::cases::case_for("linear", "libraw").expect("Linear/LibRaw case");
        let evidence = evaluate_reference_evidence(case, BayerPattern::GBRG)?;
        assert_eq!(evidence.synthetic_sources, 6);
        assert_eq!(evidence.divergent_samples, 0);
        assert!(evidence.fornax_not_worse);
        assert_eq!(evidence.fornax_mae, 0.0);
        assert_eq!(evidence.reference_mae, 0.0);
        Ok(())
    }

    #[test]
    #[ignore = "loads fixed RAW corpus files; run explicitly with --ignored"]
    fn corpus_ground_truth_skips_missing_and_evaluates_available() -> mischief::Result<()> {
        let report = evaluate_algorithms(&["Linear".to_owned()], true)?;
        assert_eq!(report.corpus_sources.len(), CORPUS_FILES.len());
        assert!(report.corpus_sources.iter().all(|source| matches!(
            source.status,
            CorpusStatus::Evaluated | CorpusStatus::Missing | CorpusStatus::TooSmall
        )));
        Ok(())
    }
}
