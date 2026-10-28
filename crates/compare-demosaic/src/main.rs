use fornax_core::{BayerChannel, BayerPattern, IDemosaic, MonoImage};
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
}

fn main() -> mischief::Result<()> {
    print!("{}", render_markdown_table(&compare_all()?));
    Ok(())
}

fn compare_all() -> mischief::Result<Vec<Comparison>> {
    let synthetic = synthetic_mono_image::<f32>((WIDTH, HEIGHT), 65_535.0);
    let codes: Vec<u16> = synthetic
        .as_raw()
        .iter()
        .map(|sample| sample.round().clamp(0.0, 65_535.0) as u16)
        .collect();
    let normalized = MonoImage::from_raw(
        WIDTH,
        HEIGHT,
        codes
            .iter()
            .map(|code| f32::from(*code) / f32::from(u16::MAX))
            .collect(),
    )
    .ok_or_else(|| mischief::mischief!("synthetic mosaic dimensions are inconsistent"))?;

    BayerPattern::ALL
        .into_iter()
        .map(|pattern| {
            compare_linear(&normalized, &codes, pattern)
                .map(|metrics| Comparison { pattern, metrics })
        })
        .collect()
}

fn compare_linear(
    mosaic: &MonoImage<f32>,
    codes: &[u16],
    pattern: BayerPattern,
) -> mischief::Result<Metrics> {
    let expected = Demosaic::Linear.demosaic(mosaic, pattern)?;
    let mut reference = vec![0_u16; codes.len() * 3];
    let status = unsafe {
        compare_demosaic_libraw_linear(
            codes.as_ptr(),
            WIDTH,
            HEIGHT,
            libraw_filters(pattern),
            reference.as_mut_ptr(),
        )
    };
    if status != 0 {
        return Err(mischief::mischief!(
            "LibRaw linear reference failed with status {status}"
        ));
    }

    let actual: Vec<u16> = expected
        .as_raw()
        .iter()
        .map(|sample| (sample.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16)
        .collect();
    metrics(&actual, &reference)
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

fn metrics(actual: &[u16], reference: &[u16]) -> mischief::Result<Metrics> {
    if actual.len() != reference.len() || actual.len() != (WIDTH * HEIGHT * 3) as usize {
        return Err(mischief::mischief!(
            "comparison buffers have inconsistent lengths"
        ));
    }

    let mut count = 0_u64;
    let mut changed = 0_u64;
    let mut maximum_error = 0_u16;
    let mut absolute_sum = 0_f64;
    let mut squared_sum = 0_f64;
    for y in 1..HEIGHT - 1 {
        for x in 1..WIDTH - 1 {
            let offset = ((y as usize * WIDTH as usize) + x as usize) * 3;
            for channel in 0..3 {
                let difference = actual[offset + channel].abs_diff(reference[offset + channel]);
                count += 1;
                changed += u64::from(difference != 0);
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
    })
}

fn render_markdown_table(comparisons: &[Comparison]) -> String {
    let mut output = String::from(
        "| Fornax algorithm | Reference algorithm | Source | Bayer pattern | MAE (codes) | RMSE (codes) | Max difference (codes) | PSNR (dB) | Changed |\n\
         | --- | --- | --- | --- | ---: | ---: | ---: | ---: | ---: |\n",
    );
    for comparison in comparisons {
        let metrics = comparison.metrics;
        let psnr = if metrics.peak_signal_to_noise_ratio.is_infinite() {
            "∞".to_owned()
        } else {
            format!("{:.2}", metrics.peak_signal_to_noise_ratio)
        };
        output.push_str(&format!(
            "| Linear | `LibRaw::lin_interpolate` | [`misc_demosaic.cpp`]({SOURCE_URL}) | {} | {:.3} | {:.3} | {} | {} | {:.3}% |\n",
            comparison.pattern,
            metrics.mean_absolute_error,
            metrics.root_mean_square_error,
            metrics.maximum_error,
            psnr,
            metrics.changed_percent,
        ));
    }
    output
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn linear_matches_libraw_for_all_bayer_patterns() -> mischief::Result<()> {
        for comparison in compare_all()? {
            assert!(
                comparison.metrics.maximum_error <= 1,
                "{} maximum error is {}",
                comparison.pattern,
                comparison.metrics.maximum_error
            );
            assert!(
                comparison.metrics.mean_absolute_error <= 0.5,
                "{} mean absolute error is {}",
                comparison.pattern,
                comparison.metrics.mean_absolute_error
            );
        }
        Ok(())
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
        assert!(include_str!("../README.md").contains(&table));
    }

    #[test]
    fn perfect_match_has_infinite_psnr() -> mischief::Result<()> {
        let samples = vec![0_u16; (WIDTH * HEIGHT * 3) as usize];
        let result = metrics(&samples, &samples)?;
        assert!(result.peak_signal_to_noise_ratio.is_infinite());
        Ok(())
    }
}
