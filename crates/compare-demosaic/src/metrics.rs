use crate::case::Tolerance;

#[derive(Clone, Copy, Debug, PartialEq, Eq)]
pub enum Verdict {
    Exact,
    RoundingOnly,
    WithinTolerance(u16),
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
}

impl Metrics {
    pub fn verdict(self, tolerance: Tolerance) -> Verdict {
        if self.maximum_error == 0 {
            return Verdict::Exact;
        }
        match tolerance {
            Tolerance::Exact => Verdict::Differs,
            Tolerance::RoundingUp if self.maximum_error <= 1 && self.below_reference == 0 => {
                Verdict::RoundingOnly
            }
            Tolerance::RoundingUp => Verdict::Differs,
            Tolerance::MaxError(maximum) if self.maximum_error <= maximum => {
                Verdict::WithinTolerance(maximum)
            }
            Tolerance::MaxError(_) => Verdict::Differs,
        }
    }

    #[cfg(test)]
    pub fn summary(self) -> String {
        format!(
            "max={}, mae={:.3}, below_reference={}",
            self.maximum_error, self.mean_absolute_error, self.below_reference
        )
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
            }
        }
    }

    let mean_squared_error = squared_sum / count as f64;
    let peak = f64::from(u16::MAX);
    Ok(Metrics {
        mean_absolute_error: absolute_sum / count as f64,
        root_mean_square_error: mean_squared_error.sqrt(),
        maximum_error,
        changed_percent: changed as f64 * 100.0 / count as f64,
        peak_signal_to_noise_ratio: if mean_squared_error == 0.0 {
            f64::INFINITY
        } else {
            20.0 * peak.log10() - 10.0 * mean_squared_error.log10()
        },
        below_reference,
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
}
