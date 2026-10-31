use std::path::{Path, PathBuf};

use fornax_core::BayerPattern;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::metrics::{Evaluation, MAX_ERROR};

pub const ARTIFACT_DIR: &str = "target/compare-demosaic/artifacts";

#[derive(Clone, Debug, Deserialize, Eq, JsonSchema, PartialEq, Serialize)]
pub struct ArtifactPaths {
    pub error_map: String,
    pub tile_summary: String,
}

#[derive(Serialize)]
struct TileArtifact<'a> {
    algorithm: &'a str,
    pattern: String,
    reference: &'a str,
    width: u32,
    height: u32,
    threshold: u16,
    tiles: &'a [crate::metrics::TileSummary],
}

pub fn write(
    actual: &[u16],
    reference: &[u16],
    width: u32,
    height: u32,
    pattern: BayerPattern,
    algorithm: &str,
    reference_name: &str,
    evaluation: &Evaluation,
) -> mischief::Result<ArtifactPaths> {
    let directory_name = format!("{}-{}-{}", slug(algorithm), pattern, slug(reference_name));
    let directory = PathBuf::from(ARTIFACT_DIR).join(&directory_name);
    std::fs::create_dir_all(&directory)?;

    let error_map = directory.join("error-map.png");
    write_error_map(actual, reference, width, height, &error_map)?;

    let tile_summary = directory.join("tiles.json");
    let mut bytes = serde_json::to_vec_pretty(&TileArtifact {
        algorithm,
        pattern: pattern.to_string(),
        reference: reference_name,
        width,
        height,
        threshold: 1024,
        tiles: &evaluation.tiles,
    })
    .map_err(|error| mischief::mischief!("cannot serialize tile summary: {error}"))?;
    bytes.push(b'\n');
    std::fs::write(&tile_summary, bytes)?;

    Ok(ArtifactPaths {
        error_map: path_string(&error_map),
        tile_summary: path_string(&tile_summary),
    })
}

fn write_error_map(
    actual: &[u16],
    reference: &[u16],
    width: u32,
    height: u32,
    path: &Path,
) -> mischief::Result<()> {
    let scale = (MAX_ERROR - 1) as f64;
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

fn path_string(path: &Path) -> String { path.to_string_lossy().replace('\\', "/") }

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_stable_for_algorithm_names() {
        assert_eq!(slug("AMaZE"), "amaze");
        assert_eq!(slug("PPG / darktable"), "ppg-darktable");
    }
}
