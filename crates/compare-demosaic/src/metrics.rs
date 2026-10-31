use crate::case::Tolerance;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Exact,
    RoundingOnly,
    WithinTolerance(u16),
    WithinRobustTolerance,
    Differs,
}

#[derive(Clone, Copy, Debug)]
pub struct Metrics {
    pub mean_absolute_error: f64,
    pub root_mean_square_error: f64,
    pub maximum_error: u16,
    pub changed_percent: f64,
    pub peak_signal_to_noise_ratio: f64,
    pub below_reference: u64,
    pub p95_error: u16,
    pub p99_error: u16,
    pub p99_9_error: u16,
    pub over_64_percent: f64,
    pub over_1024_percent: f64,
    pub channel_mae: [f64; 3],
    pub luminance_rmse: f64,
    pub chroma_rmse: f64,
}

impl Metrics {
    pub const fn verdict(self, tolerance: Tolerance) -> Verdict {
        if self.maximum_error == 0 {
            return Verdict::Exact;
        }
        match tolerance {
            Tolerance::RoundingUp if self.maximum_error <= 1 && self.below_reference == 0 => {
                Verdict::RoundingOnly
            }
            Tolerance::MaxError(maximum) if self.maximum_error <= maximum => {
                Verdict::WithinTolerance(maximum)
            }
            Tolerance::Robust {
                max_mae,
                max_p99,
                max_over_1024_percent,
            } if self.mean_absolute_error <= max_mae
                && self.p99_error <= max_p99
                && self.over_1024_percent <= max_over_1024_percent =>
            {
                Verdict::WithinRobustTolerance
            }
            Tolerance::Exact
            | Tolerance::RoundingUp
            | Tolerance::MaxError(_)
            | Tolerance::Robust { .. } => Verdict::Differs,
        }
    }
}

pub fn calculate(
    actual: &[u16],
    reference: &[u16],
    width: u32,
    height: u32,
    border: u32,
) -> mischief::Result<Metrics> {
    let expected = (width as usize)
        .checked_mul(height as usize)
        .and_then(|pixels| pixels.checked_mul(3))
        .ok_or_else(|| mischief::mischief!("metric dimensions overflow"))?;
    if actual.len() != expected || reference.len() != expected {
        return Err(mischief::mischief!(
            "comparison buffers have inconsistent lengths"
        ));
    }
    if border.saturating_mul(2) >= width || border.saturating_mul(2) >= height {
        return Err(mischief::mischief!(
            "comparison border leaves no interior pixels"
        ));
    }

    let mut count = 0_u64;
    let mut changed = 0_u64;
    let mut below_reference = 0_u64;
    let mut maximum_error = 0_u16;
    let mut absolute_sum = 0_f64;
    let mut squared_sum = 0_f64;
    let mut channel_sum = [0_f64; 3];
    let mut luma_squared_sum = 0_f64;
    let mut chroma_squared_sum = 0_f64;
    let mut errors = Vec::with_capacity(
        (width as usize - 2 * border as usize) * (height as usize - 2 * border as usize) * 3,
    );
    for y in border..height - border {
        for x in border..width - border {
            let offset = ((y as usize * width as usize) + x as usize) * 3;
            for channel in 0..3 {
                let actual = actual[offset + channel];
                let reference = reference[offset + channel];
                let difference = actual.abs_diff(reference);
                count += 1;
                changed += u64::from(difference != 0);
                below_reference += u64::from(actual < reference);
                maximum_error = maximum_error.max(difference);
                absolute_sum += f64::from(difference);
                squared_sum += f64::from(difference).powi(2);
                channel_sum[channel] += f64::from(difference);
                errors.push(difference);
            }
            let actual_luma = 0.114_f64.mul_add(
                f64::from(actual[offset + 2]),
                0.587_f64.mul_add(
                    f64::from(actual[offset + 1]),
                    0.299 * f64::from(actual[offset]),
                ),
            );
            let reference_luma = 0.114_f64.mul_add(
                f64::from(reference[offset + 2]),
                0.587_f64.mul_add(
                    f64::from(reference[offset + 1]),
                    0.299 * f64::from(reference[offset]),
                ),
            );
            luma_squared_sum += (actual_luma - reference_luma).powi(2);
            let actual_chroma = [
                f64::from(actual[offset + 2]) - actual_luma,
                f64::from(actual[offset]) - actual_luma,
            ];
            let reference_chroma = [
                f64::from(reference[offset + 2]) - reference_luma,
                f64::from(reference[offset]) - reference_luma,
            ];
            chroma_squared_sum += (actual_chroma[1] - reference_chroma[1]).mul_add(
                actual_chroma[1] - reference_chroma[1],
                (actual_chroma[0] - reference_chroma[0]).powi(2),
            );
        }
    }

    let mean_squared_error = squared_sum / count as f64;
    errors.sort_unstable();
    let percentile = |fraction: f64| {
        let index = ((errors.len() as f64 * fraction).ceil() as usize)
            .saturating_sub(1)
            .min(errors.len() - 1);
        errors[index]
    };
    let percent_above = |threshold| {
        errors.iter().filter(|error| **error > threshold).count() as f64 * 100.0
            / errors.len() as f64
    };
    let pixels = (width as usize - 2 * border as usize) * (height as usize - 2 * border as usize);
    let peak = f64::from(u16::MAX);
    Ok(Metrics {
        mean_absolute_error: absolute_sum / count as f64,
        root_mean_square_error: mean_squared_error.sqrt(),
        maximum_error,
        changed_percent: changed as f64 * 100.0 / count as f64,
        peak_signal_to_noise_ratio: if mean_squared_error == 0.0 {
            f64::INFINITY
        } else {
            10.0_f64.mul_add(-mean_squared_error.log10(), 20.0 * peak.log10())
        },
        below_reference,
        p95_error: percentile(0.95),
        p99_error: percentile(0.99),
        p99_9_error: percentile(0.999),
        over_64_percent: percent_above(64),
        over_1024_percent: percent_above(1024),
        channel_mae: channel_sum.map(|sum| sum * 3.0 / count as f64),
        luminance_rmse: (luma_squared_sum / pixels as f64).sqrt(),
        chroma_rmse: (chroma_squared_sum / pixels as f64).sqrt(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn exact_and_rounding_verdicts_are_distinct() {
        let exact = Metrics {
            mean_absolute_error: 0.0,
            root_mean_square_error: 0.0,
            maximum_error: 0,
            changed_percent: 0.0,
            peak_signal_to_noise_ratio: f64::INFINITY,
            below_reference: 0,
            p95_error: 0,
            p99_error: 0,
            p99_9_error: 0,
            over_64_percent: 0.0,
            over_1024_percent: 0.0,
            channel_mae: [0.0; 3],
            luminance_rmse: 0.0,
            chroma_rmse: 0.0,
        };
        assert_eq!(exact.verdict(Tolerance::Exact), Verdict::Exact);

        let rounding = Metrics {
            maximum_error: 1,
            below_reference: 0,
            ..exact
        };
        assert_eq!(
            rounding.verdict(Tolerance::RoundingUp),
            Verdict::RoundingOnly
        );

        let backward = Metrics {
            below_reference: 1,
            ..rounding
        };
        assert_eq!(backward.verdict(Tolerance::RoundingUp), Verdict::Differs);

        let approximate = Metrics {
            maximum_error: 2,
            ..exact
        };
        assert_eq!(
            approximate.verdict(Tolerance::MaxError(2)),
            Verdict::WithinTolerance(2)
        );
        assert_eq!(
            approximate.verdict(Tolerance::MaxError(1)),
            Verdict::Differs
        );
    }

    #[test]
    fn robust_verdict_requires_all_distribution_limits() {
        let metrics = Metrics {
            mean_absolute_error: 1.5,
            p99_error: 32,
            over_1024_percent: 0.05,
            ..Metrics {
                mean_absolute_error: 0.0,
                root_mean_square_error: 0.0,
                maximum_error: 4096,
                changed_percent: 0.0,
                peak_signal_to_noise_ratio: 0.0,
                below_reference: 0,
                p95_error: 0,
                p99_error: 0,
                p99_9_error: 0,
                over_64_percent: 0.0,
                over_1024_percent: 0.0,
                channel_mae: [0.0; 3],
                luminance_rmse: 0.0,
                chroma_rmse: 0.0,
            }
        };
        assert_eq!(
            metrics.verdict(Tolerance::Robust {
                max_mae: 2.0,
                max_p99: 64,
                max_over_1024_percent: 0.1,
            }),
            Verdict::WithinRobustTolerance
        );
        assert_eq!(
            Metrics {
                maximum_error: 65535,
                ..metrics
            }
            .verdict(Tolerance::Robust {
                max_mae: 2.0,
                max_p99: 64,
                max_over_1024_percent: 0.1,
            }),
            Verdict::WithinRobustTolerance
        );
    }
}
