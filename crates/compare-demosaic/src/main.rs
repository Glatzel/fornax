use std::fmt;

use fornax_core::{BayerChannel, BayerPattern, IDemosaic, MonoImage, Primitive};
use fornax_demosaic::Demosaic;
use fornax_devtool::synthetic_mono_image;

const WIDTH: u32 = 512;
const HEIGHT: u32 = 512;
const SOURCE_URL: &str = "https://github.com/LibRaw/LibRaw/blob/cf02d66eb5c19d4e46df7aee0582e47a4918509a/src/demosaic/misc_demosaic.cpp#L146";

unsafe extern "C" {
    fn compare_demosaic_libraw_linear(
        mosaic: *const u16,
        width: u32,
        height: u32,
        filters: u32,
        output: *mut u16,
    ) -> i32;
}

/// Float sample types under test, converted to/from 16-bit codes.
trait Sample: Primitive {
    fn from_code(code: u16) -> Self;
    fn to_code(self) -> u16;
}

impl Sample for f32 {
    fn from_code(code: u16) -> Self { f32::from(code) / f32::from(u16::MAX) }

    fn to_code(self) -> u16 { (self.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16 }
}

impl Sample for f64 {
    fn from_code(code: u16) -> Self { f64::from(code) / f64::from(u16::MAX) }

    fn to_code(self) -> u16 { (self.clamp(0.0, 1.0) * f64::from(u16::MAX)).round() as u16 }
}

#[derive(Clone, Copy, Debug)]
struct Comparison {
    pattern: BayerPattern,
    metrics: Metrics,
}

#[derive(Clone, Copy, Debug)]
struct Metrics {
    mean_absolute_error: f64,
    root_mean_square_error: f64,
    maximum_error: u16,
    changed_percent: f64,
    peak_signal_to_noise_ratio: f64,
    /// Samples where Fornax is lower than the reference.
    below_reference: u64,
}

/// Whether Fornax reproduces the upstream algorithm.
#[derive(Clone, Copy, Debug, PartialEq, Eq)]
enum Verdict {
    /// Bit-identical on the compared pixels.
    Exact,
    /// Every difference is +1 code: Fornax rounds to nearest, LibRaw
    /// truncates.
    RoundingOnly,
    /// Anything else: a real algorithmic difference.
    Differs,
}

impl fmt::Display for Verdict {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        f.write_str(match self {
            Self::Exact => "✅ exact",
            Self::RoundingOnly => "✅ rounding only",
            Self::Differs => "❌ differs",
        })
    }
}

impl Metrics {
    fn verdict(&self) -> Verdict {
        if self.maximum_error == 0 {
            Verdict::Exact
        } else if self.maximum_error <= 1 && self.below_reference == 0 {
            Verdict::RoundingOnly
        } else {
            Verdict::Differs
        }
    }
}

fn main() -> mischief::Result<()> {
    print!("{}", render_markdown_table(&compare_all()?));
    Ok(())
}

/// README case: 512x512 synthetic mosaic, `f32`, all patterns.
fn compare_all() -> mischief::Result<Vec<Comparison>> {
    let synthetic = synthetic_mono_image::<f32>((WIDTH, HEIGHT), 65_535.0);
    let codes: Vec<u16> = synthetic
        .as_raw()
        .iter()
        .map(|sample| sample.round().clamp(0.0, 65_535.0) as u16)
        .collect();

    BayerPattern::ALL
        .into_iter()
        .map(|pattern| {
            compare_linear::<f32>(&codes, WIDTH, HEIGHT, pattern)
                .map(|metrics| Comparison { pattern, metrics })
        })
        .collect()
}

/// Runs Fornax linear and LibRaw `lin_interpolate` on the same 16-bit codes
/// and compares the interiors.
fn compare_linear<T>(
    codes: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
) -> mischief::Result<Metrics>
where
    T: Sample,
    Demosaic: IDemosaic<T>,
{
    let pixels = width as usize * height as usize;
    if codes.len() != pixels {
        return Err(mischief::mischief!(
            "mosaic has {} samples, expected {pixels}",
            codes.len()
        ));
    }

    let normalized = MonoImage::from_raw(width, height, codes.iter().map(|c| T::from_code(*c)).collect())
        .ok_or_else(|| mischief::mischief!("mosaic dimensions are inconsistent"))?;
    let expected = Demosaic::Linear.demosaic(&normalized, pattern)?;

    let mut reference = vec![0_u16; pixels * 3];
    let status = unsafe {
        compare_demosaic_libraw_linear(
            codes.as_ptr(),
            width,
            height,
            libraw_filters(pattern),
            reference.as_mut_ptr(),
        )
    };
    if status != 0 {
        return Err(mischief::mischief!(
            "LibRaw linear reference failed with status {status}"
        ));
    }

    let actual: Vec<u16> = expected.as_raw().iter().map(|s| s.to_code()).collect();
    metrics(&actual, &reference, width, height)
}

fn libraw_filters(pattern: BayerPattern) -> u32 {
    let mut filters = 0_u32;
    for y in 0..8 {
        for x in 0..2 {
            let code = match pattern.channel_at(x, y) {
                BayerChannel::R => 0_u32,
                BayerChannel::GRedRow => 1,
                BayerChannel::B => 2,
                BayerChannel::GBlueRow => 3,
            };
            filters |= code << ((y * 2 + x) * 2);
        }
    }
    filters
}

/// Metrics over interior pixels only (`1..w-1`, `1..h-1`); borders differ by
/// design between the two implementations.
fn metrics(actual: &[u16], reference: &[u16], width: u32, height: u32) -> mischief::Result<Metrics> {
    if actual.len() != reference.len() || actual.len() != (width * height * 3) as usize {
        return Err(mischief::mischief!(
            "comparison buffers have inconsistent lengths"
        ));
    }

    let mut count = 0_u64;
    let mut changed = 0_u64;
    let mut below = 0_u64;
    let mut maximum_error = 0_u16;
    let mut absolute_sum = 0_f64;
    let mut squared_sum = 0_f64;
    for y in 1..height - 1 {
        for x in 1..width - 1 {
            let offset = ((y as usize * width as usize) + x as usize) * 3;
            for channel in 0..3 {
                let a = actual[offset + channel];
                let r = reference[offset + channel];
                let difference = a.abs_diff(r);
                count += 1;
                changed += u64::from(difference != 0);
                below += u64::from(a < r);
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
        below_reference: below,
    })
}

fn render_markdown_table(comparisons: &[Comparison]) -> String {
    let mut output = String::from(
        "| Fornax algorithm | Reference algorithm | Source | Bayer pattern | MAE (codes) | RMSE (codes) | Max difference (codes) | PSNR (dB) | Changed | Matches upstream |\n\
         | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: | --- |\n",
    );
    for comparison in comparisons {
        let metrics = comparison.metrics;
        let psnr = if metrics.peak_signal_to_noise_ratio.is_infinite() {
            "∞".to_owned()
        } else {
            format!("{:.2}", metrics.peak_signal_to_noise_ratio)
        };
        output.push_str(&format!(
            "| Linear | `LibRaw::lin_interpolate` | [`misc_demosaic.cpp`]({SOURCE_URL}) | {} | {:.3} | {:.3} | {} | {} | {:.3}% | {} |\n",
            comparison.pattern,
            metrics.mean_absolute_error,
            metrics.root_mean_square_error,
            metrics.maximum_error,
            psnr,
            metrics.changed_percent,
            metrics.verdict(),
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    /// Deterministic noise (splitmix64), no extra dependency.
    fn noise_codes(width: u32, height: u32, seed: u64) -> Vec<u16> {
        let mut state = seed;
        (0..width as usize * height as usize)
            .map(|_| {
                state = state.wrapping_add(0x9E37_79B9_7F4A_7C15);
                let mut z = state;
                z = (z ^ (z >> 30)).wrapping_mul(0xBF58_476D_1CE4_E5B9);
                z = (z ^ (z >> 27)).wrapping_mul(0x94D0_49BB_1331_11EB);
                (z ^ (z >> 31)) as u16
            })
            .collect()
    }

    fn assert_close(label: &str, metrics: Metrics) {
        assert!(
            metrics.verdict().is_match(),
            "{label}: {} (max error {}, {} samples below reference)",
            metrics.verdict(),
            metrics.maximum_error,
            metrics.below_reference
        );
        assert!(
            metrics.mean_absolute_error <= 0.5,
            "{label}: mean absolute error is {}",
            metrics.mean_absolute_error
        );
    }

    fn check_codes<T>(label: &str, codes: &[u16], width: u32, height: u32)
    where
        T: Sample,
        Demosaic: IDemosaic<T>,
    {
        for pattern in BayerPattern::ALL {
            let metrics = compare_linear::<T>(codes, width, height, pattern)
                .unwrap_or_else(|e| panic!("{label} {pattern}: {e:?}"));
            assert_close(&format!("{label} {pattern} {width}x{height}"), metrics);
        }
    }

    #[test]
    fn linear_matches_libraw_for_all_bayer_patterns() -> mischief::Result<()> {
        for comparison in compare_all()? {
            assert_close(&comparison.pattern.to_string(), comparison.metrics);
        }
        Ok(())
    }

    /// Random data exposes neighbour/weight mix-ups that smooth gradients hide.
    /// Odd and tiny sizes exercise the `interior_row` tail and the last rayon
    /// chunk.
    #[test]
    fn noise_matches_libraw_for_f32_and_f64_across_sizes() {
        for (i, (w, h)) in [(512, 512), (38, 30), (37, 29), (17, 16), (16, 17), (5, 4), (3, 3)]
            .into_iter()
            .enumerate()
        {
            let codes = noise_codes(w, h, 0xF0A1 + i as u64);
            check_codes::<f32>("noise f32", &codes, w, h);
            check_codes::<f64>("noise f64", &codes, w, h);
        }
    }

    /// Row count not divisible by `ROWS_PER_TASK` (16) and more than one chunk.
    #[test]
    fn partial_last_rayon_chunk_matches_libraw() {
        let (w, h) = (33, 16 * 3 + 5);
        let codes = noise_codes(w, h, 7);
        check_codes::<f32>("chunks f32", &codes, w, h);
        check_codes::<f64>("chunks f64", &codes, w, h);
    }

    #[test]
    fn saturated_input_stays_saturated() {
        let (w, h) = (31, 27);
        let codes = vec![u16::MAX; w as usize * h as usize];
        for pattern in BayerPattern::ALL {
            let metrics = compare_linear::<f32>(&codes, w, h, pattern).unwrap();
            assert_eq!(metrics.verdict(), Verdict::Exact, "{pattern}");
            let metrics = compare_linear::<f64>(&codes, w, h, pattern).unwrap();
            assert_eq!(metrics.verdict(), Verdict::Exact, "{pattern}");
        }
    }

    #[test]
    fn mismatched_mosaic_length_is_rejected() {
        let codes = vec![0_u16; 10];
        assert!(compare_linear::<f32>(&codes, 8, 8, BayerPattern::ALL[0]).is_err());
    }

    #[test]
    fn libraw_filter_masks_match_all_bayer_tiles() {
        for pattern in BayerPattern::ALL {
            let filters = libraw_filters(pattern);
            for y in 0..8 {
                for x in 0..2 {
                    let shift = (y * 2 + x) * 2;
                    let code = (filters >> shift) & 3;
                    let channel = match code {
                        0 => BayerChannel::R,
                        1 => BayerChannel::GRedRow,
                        2 => BayerChannel::B,
                        _ => BayerChannel::GBlueRow,
                    };
                    assert_eq!(channel, pattern.channel_at(x, y));
                }
            }
        }
    }

    #[test]
    fn markdown_has_one_row_per_pattern_and_a_permanent_source_link() {
        let table = render_markdown_table(&compare_all().expect("comparison runs"));
        assert_eq!(table.lines().count(), 6);
        assert!(table.contains("LibRaw::lin_interpolate"));
        assert!(table.contains(SOURCE_URL));
        for pattern in BayerPattern::ALL {
            assert!(table.contains(&pattern.to_string()));
        }
    }

    #[test]
    fn readme_results_match_generated_table() {
        let table = render_markdown_table(&compare_all().expect("comparison runs"));
        assert!(
            include_str!("../README.md").contains(&table),
            "README results are stale; regenerate with `cargo run` in this crate and paste the table:\n{table}"
        );
    }

    #[test]
    fn perfect_match_has_infinite_psnr() -> mischief::Result<()> {
        let samples = vec![0_u16; (WIDTH * HEIGHT * 3) as usize];
        let result = metrics(&samples, &samples, WIDTH, HEIGHT)?;
        assert!(result.peak_signal_to_noise_ratio.is_infinite());
        assert_eq!(result.verdict(), Verdict::Exact);
        Ok(())
    }

    #[test]
    fn verdict_classifies_rounding_and_real_differences() -> mischief::Result<()> {
        let (w, h) = (4_u32, 4_u32);
        let reference = vec![100_u16; (w * h * 3) as usize];
        let probe = (w as usize + 1) * 3; // an interior sample

        let mut plus_one = reference.clone();
        plus_one[probe] += 1;
        assert_eq!(metrics(&plus_one, &reference, w, h)?.verdict(), Verdict::RoundingOnly);

        let mut minus_one = reference.clone();
        minus_one[probe] -= 1;
        assert_eq!(metrics(&minus_one, &reference, w, h)?.verdict(), Verdict::Differs);

        let mut plus_two = reference.clone();
        plus_two[probe] += 2;
        assert_eq!(metrics(&plus_two, &reference, w, h)?.verdict(), Verdict::Differs);
        Ok(())
    }
}
