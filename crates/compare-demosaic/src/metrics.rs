use std::cmp::Reverse;
use std::collections::BinaryHeap;

use fornax_core::{BayerChannel, BayerPattern};
use serde::{Deserialize, Serialize};

use crate::case::Tolerance;

pub const MAX_ERROR: usize = u16::MAX as usize + 1;

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum Verdict {
    Exact,
    RoundingOnly,
    PassWithOutliers,
    Pass,
    Fail,
}

#[derive(Clone, Copy, Debug, Deserialize, Eq, PartialEq, Serialize)]
#[serde(rename_all = "snake_case")]
pub enum FailedLimit {
    Mae,
    P99,
    HighErrFrac,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct Metrics {
    pub total_samples: u64,
    pub mae: f64,
    pub rmse: f64,
    pub max: u16,
    pub mean_signed: f64,
    pub p99: u16,
    pub p999: u16,
    pub p9999: u16,
    pub count_gt1: u64,
    pub frac_gt1: f64,
    pub count_gt16: u64,
    pub frac_gt16: f64,
    pub count_gt1024: u64,
    pub frac_gt1024: f64,
    pub count_below_reference: u64,
    pub changed_count: u64,
    pub changed_frac: f64,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct WorstPixel {
    pub x: u32,
    pub y: u32,
    pub c: u8,
    pub fornax: u16,
    pub reference: u16,
}

#[derive(Clone, Debug, Deserialize, PartialEq, Serialize)]
pub struct TileSummary {
    pub x: u32,
    pub y: u32,
    pub width: u32,
    pub height: u32,
    pub count_gt1024: u64,
    pub total_samples: u64,
}

#[derive(Clone, Debug)]
#[allow(
    dead_code,
    reason = "detailed fields remain available to the investigation example"
)]
pub struct Evaluation {
    pub metrics: Metrics,
    pub metrics_interior: Metrics,
    pub per_channel: [Metrics; 3],
    pub per_site: [Metrics; 2],
    pub worst_pixels: Vec<WorstPixel>,
    pub tiles: Vec<TileSummary>,
}

#[derive(Clone, Debug)]
struct ErrorAccumulator {
    histogram: Box<[u64]>,
    total: u64,
    changed: u64,
    below_reference: u64,
    signed_sum: i64,
    squared_sum: u128,
    maximum: u16,
}

impl ErrorAccumulator {
    fn new() -> Self {
        Self {
            histogram: vec![0; MAX_ERROR].into_boxed_slice(),
            total: 0,
            changed: 0,
            below_reference: 0,
            signed_sum: 0,
            squared_sum: 0,
            maximum: 0,
        }
    }

    fn push(&mut self, actual: u16, reference: u16) {
        let error = actual.abs_diff(reference);
        self.total += 1;
        self.changed += u64::from(error != 0);
        self.below_reference += u64::from(actual < reference);
        self.signed_sum += i64::from(actual) - i64::from(reference);
        self.squared_sum += u128::from(error).pow(2);
        self.maximum = self.maximum.max(error);
        self.histogram[usize::from(error)] += 1;
    }

    fn percentile(&self, fraction: f64) -> u16 {
        if self.total == 0 {
            return 0;
        }
        let rank = ((self.total as f64 * fraction).ceil() as u64)
            .saturating_sub(1)
            .min(self.total - 1);
        let mut seen = 0_u64;
        for (error, count) in self.histogram.iter().copied().enumerate() {
            seen += count;
            if seen > rank {
                return error as u16;
            }
        }
        self.maximum
    }

    fn count_above(&self, threshold: u16) -> u64 {
        self.histogram[usize::from(threshold) + 1..].iter().sum()
    }

    fn finish(self) -> Metrics {
        let total = self.total;
        let divisor = total.max(1) as f64;
        let count_gt1 = self.count_above(1);
        let count_gt16 = self.count_above(16);
        let count_gt1024 = self.count_above(1024);
        Metrics {
            total_samples: total,
            mae: round6(
                self.histogram
                    .iter()
                    .enumerate()
                    .fold(0_u128, |sum, (error, count)| {
                        sum + error as u128 * u128::from(*count)
                    }) as f64
                    / divisor,
            ),
            rmse: round6(((self.squared_sum as f64) / divisor).sqrt()),
            max: self.maximum,
            mean_signed: round6(self.signed_sum as f64 / divisor),
            p99: self.percentile(0.99),
            p999: self.percentile(0.999),
            p9999: self.percentile(0.9999),
            count_gt1,
            frac_gt1: round6(count_gt1 as f64 / divisor),
            count_gt16,
            frac_gt16: round6(count_gt16 as f64 / divisor),
            count_gt1024,
            frac_gt1024: round6(count_gt1024 as f64 / divisor),
            count_below_reference: self.below_reference,
            changed_count: self.changed,
            changed_frac: round6(self.changed as f64 / divisor),
        }
    }
}

#[derive(Clone, Copy, Debug, Eq, Ord, PartialEq, PartialOrd)]
struct WorstCandidate {
    error: u16,
    x: u32,
    y: u32,
    c: u8,
    fornax: u16,
    reference: u16,
}

pub fn verdict(metrics: &Metrics, tolerance: Tolerance) -> (Verdict, Vec<FailedLimit>) {
    if metrics.max == 0 {
        return (Verdict::Exact, Vec::new());
    }
    match tolerance {
        Tolerance::RoundingUp if metrics.max <= 1 && metrics.count_below_reference == 0 => {
            (Verdict::RoundingOnly, Vec::new())
        }
        Tolerance::MaxError(maximum) if metrics.max <= maximum => (Verdict::Pass, Vec::new()),
        Tolerance::Exact | Tolerance::RoundingUp | Tolerance::MaxError(_) => {
            (Verdict::Fail, Vec::new())
        }
        Tolerance::Robust {
            max_mae,
            max_p99,
            max_high_error_frac,
        } => {
            let mut failed = Vec::new();
            if metrics.mae > max_mae {
                failed.push(FailedLimit::Mae);
            }
            if metrics.p99 > max_p99 {
                failed.push(FailedLimit::P99);
            }
            if metrics.frac_gt1024 > max_high_error_frac {
                failed.push(FailedLimit::HighErrFrac);
            }
            if !failed.is_empty() {
                (Verdict::Fail, failed)
            } else if metrics.count_gt1024 > 0 {
                (Verdict::PassWithOutliers, failed)
            } else {
                (Verdict::Pass, failed)
            }
        }
    }
}

pub fn calculate(
    actual: &[u16],
    reference: &[u16],
    width: u32,
    height: u32,
    border: u32,
    pattern: BayerPattern,
) -> mischief::Result<Evaluation> {
    let pixels = (width as usize)
        .checked_mul(height as usize)
        .ok_or_else(|| mischief::mischief!("metric dimensions overflow"))?;
    let expected = pixels
        .checked_mul(3)
        .ok_or_else(|| mischief::mischief!("metric sample count overflow"))?;
    if actual.len() != expected || reference.len() != expected {
        return Err(mischief::mischief!(
            "comparison buffers have inconsistent lengths"
        ));
    }
    if u64::from(border) * 2 >= u64::from(width) || u64::from(border) * 2 >= u64::from(height) {
        return Err(mischief::mischief!(
            "comparison border leaves no interior pixels"
        ));
    }

    let mut whole = ErrorAccumulator::new();
    let mut interior = ErrorAccumulator::new();
    let mut per_channel = [
        ErrorAccumulator::new(),
        ErrorAccumulator::new(),
        ErrorAccumulator::new(),
    ];
    let mut per_site = [ErrorAccumulator::new(), ErrorAccumulator::new()];
    let mut worst = BinaryHeap::<Reverse<WorstCandidate>>::new();
    let tile_columns = width.div_ceil(64) as usize;
    let tile_rows = height.div_ceil(64) as usize;
    let mut tiles = vec![(0_u64, 0_u64); tile_columns * tile_rows];

    for y in 0..height {
        for x in 0..width {
            let offset = (y as usize * width as usize + x as usize) * 3;
            let is_interior =
                x >= border && y >= border && x < width - border && y < height - border;
            let sampled_channel = match pattern.channel_at(x, y) {
                BayerChannel::R => 0,
                BayerChannel::GRedRow | BayerChannel::GBlueRow => 1,
                BayerChannel::B => 2,
            };
            let tile = y as usize / 64 * tile_columns + x as usize / 64;
            for channel in 0..3 {
                let actual = actual[offset + channel];
                let reference = reference[offset + channel];
                let error = actual.abs_diff(reference);
                whole.push(actual, reference);
                per_channel[channel].push(actual, reference);
                per_site[usize::from(channel != sampled_channel)].push(actual, reference);
                if is_interior {
                    interior.push(actual, reference);
                }
                if error > 1024 {
                    tiles[tile].0 += 1;
                }
                tiles[tile].1 += 1;

                if error > 0 {
                    let candidate = WorstCandidate {
                        error,
                        x,
                        y,
                        c: channel as u8,
                        fornax: actual,
                        reference,
                    };
                    if worst.len() < 20 {
                        worst.push(Reverse(candidate));
                    } else if worst.peek().is_some_and(|minimum| candidate > minimum.0) {
                        worst.pop();
                        worst.push(Reverse(candidate));
                    }
                }
            }
        }
    }

    let mut worst_pixels = worst
        .into_iter()
        .map(|Reverse(candidate)| WorstPixel {
            x: candidate.x,
            y: candidate.y,
            c: candidate.c,
            fornax: candidate.fornax,
            reference: candidate.reference,
        })
        .collect::<Vec<_>>();
    worst_pixels.sort_unstable_by(|left, right| {
        right
            .fornax
            .abs_diff(right.reference)
            .cmp(&left.fornax.abs_diff(left.reference))
            .then_with(|| left.y.cmp(&right.y))
            .then_with(|| left.x.cmp(&right.x))
            .then_with(|| left.c.cmp(&right.c))
    });

    let tiles = tiles
        .into_iter()
        .enumerate()
        .filter_map(|(index, (count_gt1024, total_samples))| {
            let tile_x = (index % tile_columns) as u32 * 64;
            let tile_y = (index / tile_columns) as u32 * 64;
            (count_gt1024 > 0).then_some(TileSummary {
                x: tile_x,
                y: tile_y,
                width: 64.min(width - tile_x),
                height: 64.min(height - tile_y),
                count_gt1024,
                total_samples,
            })
        })
        .collect();

    let [red, green, blue] = per_channel;
    let [sampled, interpolated] = per_site;
    Ok(Evaluation {
        metrics: whole.finish(),
        metrics_interior: interior.finish(),
        per_channel: [red.finish(), green.finish(), blue.finish()],
        per_site: [sampled.finish(), interpolated.finish()],
        worst_pixels,
        tiles,
    })
}

fn round6(value: f64) -> f64 { (value * 1_000_000.0).round() / 1_000_000.0 }

#[cfg(test)]
mod tests {
    use super::*;

    fn one_error_of_two(total_samples: u64) -> Metrics {
        let divisor = total_samples as f64;
        Metrics {
            total_samples,
            mae: round6(2.0 / divisor),
            rmse: round6((4.0 / divisor).sqrt()),
            max: 2,
            mean_signed: round6(2.0 / divisor),
            p99: 2,
            p999: 2,
            p9999: 2,
            count_gt1: 1,
            frac_gt1: round6(1.0 / divisor),
            count_gt16: 0,
            frac_gt16: 0.0,
            count_gt1024: 0,
            frac_gt1024: 0.0,
            count_below_reference: 0,
            changed_count: 1,
            changed_frac: round6(1.0 / divisor),
        }
    }

    fn unchanged(total_samples: u64) -> Metrics {
        Metrics {
            total_samples,
            mae: 0.0,
            rmse: 0.0,
            max: 0,
            mean_signed: 0.0,
            p99: 0,
            p999: 0,
            p9999: 0,
            count_gt1: 0,
            frac_gt1: 0.0,
            count_gt16: 0,
            frac_gt16: 0.0,
            count_gt1024: 0,
            frac_gt1024: 0.0,
            count_below_reference: 0,
            changed_count: 0,
            changed_frac: 0.0,
        }
    }

    #[test]
    fn synthetic_single_error_checks_every_metric() {
        let width = 4;
        let height = 4;
        let mut actual = vec![100_u16; width * height * 3];
        let reference = actual.clone();
        let error_offset = (width + 1) * 3 + 2;
        actual[error_offset] = 102;

        let evaluation =
            calculate(&actual, &reference, 4, 4, 1, BayerPattern::RGGB).expect("valid metrics");
        assert_eq!(
            evaluation.metrics,
            Metrics {
                total_samples: 48,
                mae: 0.041667,
                rmse: 0.288675,
                max: 2,
                mean_signed: 0.041667,
                p99: 2,
                p999: 2,
                p9999: 2,
                count_gt1: 1,
                frac_gt1: 0.020833,
                count_gt16: 0,
                frac_gt16: 0.0,
                count_gt1024: 0,
                frac_gt1024: 0.0,
                count_below_reference: 0,
                changed_count: 1,
                changed_frac: 0.020833,
            }
        );
        assert_eq!(
            evaluation.metrics_interior,
            Metrics {
                total_samples: 12,
                mae: 0.166667,
                rmse: 0.57735,
                max: 2,
                mean_signed: 0.166667,
                p99: 2,
                p999: 2,
                p9999: 2,
                count_gt1: 1,
                frac_gt1: 0.083333,
                count_gt16: 0,
                frac_gt16: 0.0,
                count_gt1024: 0,
                frac_gt1024: 0.0,
                count_below_reference: 0,
                changed_count: 1,
                changed_frac: 0.083333,
            }
        );
        assert_eq!(
            evaluation.per_channel,
            [unchanged(16), unchanged(16), one_error_of_two(16)]
        );
        assert_eq!(evaluation.per_site, [one_error_of_two(16), unchanged(32)]);
        assert_eq!(evaluation.worst_pixels.len(), 1);
        assert_eq!(
            evaluation.worst_pixels[0],
            WorstPixel {
                x: 1,
                y: 1,
                c: 2,
                fornax: 102,
                reference: 100,
            }
        );
        assert!(evaluation.tiles.is_empty());
    }

    #[test]
    fn histogram_percentiles_and_high_error_tiles_are_exact() {
        let width = 128;
        let height = 64;
        let mut actual = vec![0_u16; width * height * 3];
        let reference = actual.clone();
        actual[0] = 1025;
        actual[1] = 1026;
        actual[2] = 1027;
        let evaluation = calculate(
            &actual,
            &reference,
            width as u32,
            height as u32,
            16,
            BayerPattern::RGGB,
        )
        .expect("valid metrics");
        assert_eq!(evaluation.metrics.count_gt1024, 3);
        assert_eq!(evaluation.metrics.p9999, 1025);
        assert_eq!(evaluation.tiles.len(), 1);
        assert_eq!(evaluation.tiles[0].count_gt1024, 3);
        assert_eq!(evaluation.tiles[0].total_samples, 64 * 64 * 3);
    }

    #[test]
    fn verdicts_distinguish_exact_rounding_outliers_and_failure() {
        assert_eq!(
            verdict(&unchanged(16), Tolerance::RoundingUp),
            (Verdict::Exact, Vec::new())
        );
        let rounding = Metrics {
            mae: 0.25,
            rmse: 0.25,
            max: 1,
            mean_signed: 0.25,
            p99: 1,
            p999: 1,
            p9999: 1,
            count_gt1: 0,
            frac_gt1: 0.0,
            changed_count: 4,
            changed_frac: 0.25,
            ..unchanged(16)
        };
        assert_eq!(
            verdict(&rounding, Tolerance::RoundingUp),
            (Verdict::RoundingOnly, Vec::new())
        );
        assert_eq!(
            verdict(
                &Metrics {
                    count_below_reference: 1,
                    ..rounding.clone()
                },
                Tolerance::RoundingUp
            ),
            (Verdict::Fail, Vec::new())
        );

        let robust = Tolerance::Robust {
            max_mae: 2.0,
            max_p99: 64,
            max_high_error_frac: 0.001,
        };
        assert_eq!(
            verdict(&one_error_of_two(16), robust),
            (Verdict::Pass, Vec::new())
        );
        assert_eq!(
            verdict(
                &Metrics {
                    count_gt1024: 1,
                    frac_gt1024: 0.0005,
                    ..one_error_of_two(16)
                },
                robust
            ),
            (Verdict::PassWithOutliers, Vec::new())
        );
    }

    #[test]
    fn robust_failure_lists_each_failed_limit() {
        let metrics = Metrics {
            total_samples: 1_000,
            mae: 3.0,
            rmse: 3.0,
            max: 2048,
            mean_signed: 3.0,
            p99: 65,
            p999: 65,
            p9999: 2048,
            count_gt1: 2,
            frac_gt1: 0.002,
            count_gt16: 1,
            frac_gt16: 0.001,
            count_gt1024: 2,
            frac_gt1024: 0.002,
            count_below_reference: 0,
            changed_count: 2,
            changed_frac: 0.002,
        };
        assert_eq!(
            verdict(
                &metrics,
                Tolerance::Robust {
                    max_mae: 2.0,
                    max_p99: 64,
                    max_high_error_frac: 0.001,
                }
            ),
            (
                Verdict::Fail,
                vec![FailedLimit::Mae, FailedLimit::P99, FailedLimit::HighErrFrac]
            )
        );
    }
}
