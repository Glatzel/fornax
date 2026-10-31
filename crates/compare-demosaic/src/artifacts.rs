use std::path::Path;

use fornax_core::BayerPattern;
use schemars::JsonSchema;
use serde::{Deserialize, Serialize};

use crate::metrics::{Evaluation, MAX_ERROR};

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

pub struct ArtifactInput<'a> {
    pub actual: &'a [u16],
    pub reference: &'a [u16],
    pub width: u32,
    pub height: u32,
    pub pattern: BayerPattern,
    pub algorithm: &'a str,
    pub reference_name: &'a str,
    pub evaluation: &'a Evaluation,
    pub output_dir: &'a Path,
}

pub fn write(input: ArtifactInput<'_>) -> mischief::Result<ArtifactPaths> {
    let directory_name = format!(
        "{}-{}-{}",
        slug(input.algorithm),
        input.pattern,
        slug(input.reference_name)
    );
    let artifacts_root = input.output_dir.join("artifacts");
    let directory = artifacts_root.join(&directory_name);
    std::fs::create_dir_all(&directory)?;

    let error_map = directory.join("error-map.png");
    write_error_map(
        input.actual,
        input.reference,
        input.width,
        input.height,
        &error_map,
    )?;

    let tile_summary = directory.join("tiles.json");
    let mut bytes = serde_json::to_vec_pretty(&TileArtifact {
        algorithm: input.algorithm,
        pattern: input.pattern.to_string(),
        reference: input.reference_name,
        width: input.width,
        height: input.height,
        threshold: 1024,
        tiles: &input.evaluation.tiles,
    })
    .map_err(|error| mischief::mischief!("cannot serialize tile summary: {error}"))?;
    bytes.push(b'\n');
    std::fs::write(&tile_summary, bytes)?;

    Ok(ArtifactPaths {
        error_map: relative_path(&artifacts_root, &error_map),
        tile_summary: relative_path(&artifacts_root, &tile_summary),
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

fn relative_path(root: &Path, path: &Path) -> String {
    let relative = path
        .strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .replace('\\', "/");
    format!("artifacts/{relative}")
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn slug_is_stable_for_algorithm_names() {
        assert_eq!(slug("AMaZE"), "amaze");
        assert_eq!(slug("PPG / darktable"), "ppg-darktable");
    }
}
