#![allow(
    dead_code,
    reason = "the generic scale and transfer helpers are consumed by Phase C kernels"
)]

use std::cmp::Ordering;

use fornax_raw_core::Primitive;
use num_traits::{Float, cast};

const SCALE_PERCENTILE_NUMERATOR: usize = 995;
const SCALE_PERCENTILE_DENOMINATOR: usize = 1_000;

const SRGB_SLOPE: f64 = 12.92;
const SRGB_DISPLAY_GAMMA: f64 = 1.0 / 2.4;
const SRGB_ALPHA: f64 = 1.055;
const SRGB_OFFSET: f64 = 0.055;
const SRGB_LINEAR_THRESHOLD: f64 = 0.003_130_8;
const SRGB_ENCODED_THRESHOLD: f64 = 0.040_45;
const SRGB_DECODE_GAMMA: f64 = 2.4;

pub(super) fn normalization_scale<T: Primitive>(samples: &[T]) -> T {
    let mut magnitudes = samples
        .iter()
        .copied()
        .filter(|sample| sample.is_finite())
        .map(Float::abs)
        .filter(|sample| *sample > T::zero())
        .collect::<Vec<_>>();
    if magnitudes.is_empty() {
        return T::one();
    }

    let rank = (SCALE_PERCENTILE_NUMERATOR * (magnitudes.len() - 1))
        .div_ceil(SCALE_PERCENTILE_DENOMINATOR);
    let (_, &mut scale, _) = magnitudes.select_nth_unstable_by(rank, |left, right| {
        left.partial_cmp(right).unwrap_or(Ordering::Equal)
    });
    scale
}

pub(super) struct SrgbTransfer<T: Primitive> {
    slope: T,
    display_gamma: T,
    alpha: T,
    offset: T,
    linear_threshold: T,
    encoded_threshold: T,
    decode_gamma: T,
}

impl<T: Primitive> SrgbTransfer<T> {
    pub(super) fn new() -> Self {
        let from_f64 = |value| cast::<f64, T>(value).expect("the transfer constants fit in T");
        Self {
            slope: from_f64(SRGB_SLOPE),
            display_gamma: from_f64(SRGB_DISPLAY_GAMMA),
            alpha: from_f64(SRGB_ALPHA),
            offset: from_f64(SRGB_OFFSET),
            linear_threshold: from_f64(SRGB_LINEAR_THRESHOLD),
            encoded_threshold: from_f64(SRGB_ENCODED_THRESHOLD),
            decode_gamma: from_f64(SRGB_DECODE_GAMMA),
        }
    }

    pub(super) fn encode(&self, value: T) -> T {
        let sign = if value < T::zero() {
            -T::one()
        } else {
            T::one()
        };
        let magnitude = value.abs();
        let encoded = if magnitude > T::one() {
            magnitude.powf(self.display_gamma)
        } else if magnitude <= self.linear_threshold {
            self.slope * magnitude
        } else {
            self.alpha
                .mul_add(magnitude.powf(self.display_gamma), -self.offset)
        };
        sign * encoded
    }

    pub(super) fn decode(&self, value: T) -> T {
        let sign = if value < T::zero() {
            -T::one()
        } else {
            T::one()
        };
        let magnitude = value.abs();
        let decoded = if magnitude > T::one() {
            magnitude.powf(self.decode_gamma)
        } else if magnitude <= self.encoded_threshold {
            magnitude / self.slope
        } else {
            ((magnitude + self.offset) / self.alpha).powf(self.decode_gamma)
        };
        sign * decoded
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn assert_close(actual: f64, expected: f64, tolerance: f64) {
        assert!(
            (actual - expected).abs() <= tolerance,
            "actual {actual:?}, expected {expected:?}"
        );
    }

    #[test]
    fn normalization_scale_is_homogeneous_and_ignores_zero() {
        let samples = [-4.0_f64, -1.0, 2.0, 3.0, 0.0];
        let scale = normalization_scale(&samples);
        assert_close(scale, 4.0, 1.0e-12);
        assert_close(normalization_scale(&[0.0_f64; 8]), 1.0, 0.0);
        assert_close(f64::from(normalization_scale::<f32>(&[0.0; 8])), 1.0, 0.0);
    }

    #[test]
    fn srgb_transfer_round_trips_signed_and_hdr_values() {
        for &value in &[-4.0_f64, -1.0, -0.25, 0.0, 0.003, 0.25, 1.0, 2.0, 1.0e4] {
            let transfer = SrgbTransfer::<f64>::new();
            let encoded = transfer.encode(value);
            assert_close(transfer.decode(encoded), value, 2.0e-12);
        }
        let transfer = SrgbTransfer::<f64>::new();
        assert_close(transfer.encode(-0.5), -transfer.encode(0.5), 2.0e-12);
        assert!(transfer.encode(2.0) > 1.0);
    }
}
