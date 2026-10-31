use std::path::{Path, PathBuf};

use clap::Parser;
use compare_demosaic::case::SampleType;
use compare_demosaic::metrics::{Evaluation, calculate};
use fornax_core::BayerPattern;

#[derive(Debug, Parser)]
struct Args {
    #[arg(long)]
    algorithm: String,
}

fn main() -> mischief::Result<()> {
    let args = Args::parse();
    let input = compare_demosaic::data::active_area()?;
    let output_root = PathBuf::from("target/compare-demosaic/inspect");
    for case in compare_demosaic::cases::CASES
        .iter()
        .filter(|case| case.name.eq_ignore_ascii_case(&args.algorithm))
    {
        for pattern in BayerPattern::ALL {
            let reference = compare_demosaic::ffi::run(
                case.reference_backend,
                case.algorithm,
                &input.codes,
                input.width,
                input.height,
                compare_demosaic::ffi::libraw_filters(pattern),
            )?;
            let actual = (case.run_fornax)(
                &input.codes,
                input.width,
                input.height,
                pattern,
                SampleType::F32,
            )?;
            let evaluation =
                calculate(&actual, &reference, input.width, input.height, 16, pattern)?;
            let directory = output_root.join(format!(
                "{}-{}-{}",
                slug(case.name),
                pattern,
                slug(case.reference_backend.library())
            ));
            std::fs::create_dir_all(&directory)?;
            print_details(case.name, pattern, &evaluation);
            write_error_map(
                &actual,
                &reference,
                input.width,
                input.height,
                &directory.join("error-map.png"),
            )?;
            write_tiles(
                case.name,
                pattern,
                &evaluation,
                &directory.join("tiles.json"),
            )?;
        }
    }
    Ok(())
}

fn print_details(algorithm: &str, pattern: BayerPattern, evaluation: &Evaluation) {
    println!("{algorithm} {pattern}");
    println!("  whole: {:?}", evaluation.metrics);
    println!("  interior: {:?}", evaluation.metrics_interior);
    println!("  per channel: {:?}", evaluation.per_channel);
    println!("  per site: {:?}", evaluation.per_site);
    println!("  worst pixels:");
    for pixel in &evaluation.worst_pixels {
        println!(
            "    ({}, {}) c{}: {} -> {}",
            pixel.x, pixel.y, pixel.c, pixel.fornax, pixel.reference
        );
    }
}

fn write_error_map(
    actual: &[u16],
    reference: &[u16],
    width: u32,
    height: u32,
    path: &Path,
) -> mischief::Result<()> {
    let scale = (u16::MAX as f64).ln_1p();
    let pixels = actual
        .as_chunks::<3>()
        .0
        .iter()
        .zip(reference.as_chunks::<3>().0)
        .map(|(actual, reference)| {
            let error = (0..3)
                .map(|channel| actual[channel].abs_diff(reference[channel]))
                .max()
                .unwrap_or(0);
            (f64::from(error).ln_1p() / scale * 255.0)
                .round()
                .clamp(0.0, 255.0) as u8
        })
        .collect::<Vec<_>>();
    let image = image::ImageBuffer::<image::Luma<u8>, Vec<u8>>::from_raw(width, height, pixels)
        .ok_or_else(|| mischief::mischief!("error-map dimensions are inconsistent"))?;
    image.save(path)?;
    Ok(())
}

fn write_tiles(
    algorithm: &str,
    pattern: BayerPattern,
    evaluation: &Evaluation,
    path: &Path,
) -> mischief::Result<()> {
    let mut bytes = serde_json::to_vec_pretty(&serde_json::json!({
        "algorithm": algorithm,
        "pattern": pattern.to_string(),
        "tiles": &evaluation.tiles,
    }))?;
    bytes.push(b'\n');
    std::fs::write(path, bytes)?;
    Ok(())
}

fn slug(value: &str) -> String {
    value
        .chars()
        .map(|character| {
            if character.is_ascii_alphanumeric() {
                character.to_ascii_lowercase()
            } else {
                '-'
            }
        })
        .collect::<String>()
        .trim_matches('-')
        .to_owned()
}
