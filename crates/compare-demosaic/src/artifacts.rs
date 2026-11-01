use std::path::{Path, PathBuf};

use fornax_core::BayerPattern;
use serde::Serialize;

use crate::metrics::{Evaluation, MAX_ERROR, TileSummary};

#[derive(Serialize)]
struct TileArtifact<'a> {
    algorithm: &'a str,
    reference: &'a str,
    pattern: String,
    width: u32,
    height: u32,
    threshold: u16,
    tiles: &'a [TileSummary],
}

/// Artifact paths are deliberately absent from the canonical report: the
/// committed JSON must not point at files that are only produced locally.
pub fn write(
    out: &Path,
    input_name: &str,
    algorithm: &str,
    reference: &str,
    pattern: BayerPattern,
    actual: &[u16],
    expected: &[u16],
    width: u32,
    height: u32,
    evaluation: &Evaluation,
) -> mischief::Result<(PathBuf, PathBuf)> {
    let directory = out.join("artifacts").join(slug(input_name)).join(format!(
        "{}-{}-{pattern}",
        slug(algorithm),
        slug(reference)
    ));
    std::fs::create_dir_all(&directory)?;
    let error_map = directory.join("error.png");
    write_error_map(actual, expected, width, height, &error_map)?;
    let tiles = directory.join("tiles.json");
    let mut bytes = serde_json::to_vec(&TileArtifact {
        algorithm,
        reference,
        pattern: pattern.to_string(),
        width,
        height,
        threshold: crate::report::HIGH_ERROR_THRESHOLD,
        tiles: &evaluation.tiles,
    })
    .map_err(|error| mischief::mischief!("cannot serialize tile summary: {error}"))?;
    bytes.push(b'\n');
    std::fs::write(&tiles, bytes)?;
    Ok((error_map, tiles))
}

fn write_error_map(
    actual: &[u16],
    expected: &[u16],
    width: u32,
    height: u32,
    path: &Path,
) -> mischief::Result<()> {
    let scale = (MAX_ERROR - 1) as f64;
    let pixels = actual
        .as_chunks::<3>()
        .0
        .iter()
        .zip(expected.as_chunks::<3>().0)
        .map(|(actual, expected)| {
            let error = (0..3)
                .map(|channel| actual[channel].abs_diff(expected[channel]))
                .max()
                .unwrap_or(0);
            (f64::from(error).ln_1p() / scale.ln_1p() * 255.0)
                .round()
                .clamp(0.0, 255.0) as u8
        })
        .collect::<Vec<_>>();
    let image = image::ImageBuffer::<image::Luma<u8>, Vec<u8>>::from_raw(width, height, pixels)
        .ok_or_else(|| mischief::mischief!("error-map dimensions are inconsistent"))?;
    image.save(path)?;
    Ok(())
}

fn slug(value: &str) -> String {
    let mut output = String::new();
    for character in value.chars() {
        if character.is_ascii_alphanumeric() {
            output.push(character.to_ascii_lowercase());
        } else if !output.ends_with('-') {
            output.push('-');
        }
    }
    output.trim_matches('-').to_owned()
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn artifact_names_are_stable() {
        assert_eq!(slug("AMaZE"), "amaze");
        assert_eq!(slug("colorchart-eos-7d.cr2"), "colorchart-eos-7d-cr2");
    }
}
