use std::path::{Path, PathBuf};

use fornax_core::BayerPattern;
use fornax_core::protocol::{IDecoder, IPreprocessor};
use fornax_decoder_rawler::RawlerDecoder;
#[cfg(test)]
use fornax_devtool::synthetic_mono_image;
use fornax_preprocessor::Preprocessor;

#[cfg(test)]
pub const CANONICAL_WIDTH: u32 = 512;
#[cfg(test)]
pub const CANONICAL_HEIGHT: u32 = 512;
const RAW_CORPUS: &str = "external/raw.pixls.us";

#[derive(Clone, Debug)]
pub struct Input {
    pub label: String,
    pub source: String,
    pub pattern: Option<BayerPattern>,
    pub width: u32,
    pub height: u32,
    pub codes: Vec<u16>,
}

pub fn real_inputs() -> Vec<Input> {
    let root = fornax_devtool::root_dir();
    let paths = one_per_make(raw_paths(&root.join(RAW_CORPUS)), &root);

    let mut inputs = Vec::new();
    for path in paths {
        match decode_input(&root, &path) {
            Ok(input) => inputs.push(input),
            Err(_) => continue,
        }
    }
    assert!(
        !inputs.is_empty(),
        "no decodable 2x2 Bayer RAW files found; run scripts/download-raw-corpus.ps1"
    );
    inputs
}

fn decode_input(root: &Path, path: &Path) -> Result<Input, String> {
    let (decoded, metadata) =
        <RawlerDecoder as IDecoder<Path, f32>>::decode_active(&RawlerDecoder, path)
            .map_err(|error| error.to_string())?;
    let normalized = Preprocessor
        .process(decoded, &metadata.levels)
        .map_err(|error| error.to_string())?
        .into_image();
    let source = relative(root, path);
    let label = source
        .replace(['/', '\\', '.'], "-")
        .trim_matches('-')
        .to_owned();
    let (width, height) = normalized.dimensions();
    Ok(Input {
        label,
        source,
        pattern: Some(metadata.bayer_pattern),
        width,
        height,
        codes: normalized
            .as_raw()
            .iter()
            .map(|sample| (sample.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16)
            .collect(),
    })
}

fn raw_paths(root: &Path) -> Vec<PathBuf> {
    let mut pending = vec![root.to_path_buf()];
    let mut paths = Vec::new();
    while let Some(directory) = pending.pop() {
        let Ok(entries) = std::fs::read_dir(directory) else {
            continue;
        };
        for entry in entries.flatten() {
            let path = entry.path();
            if path.is_dir() {
                pending.push(path);
            } else if path
                .extension()
                .and_then(|extension| extension.to_str())
                .is_some_and(|extension| {
                    matches!(
                        extension.to_ascii_lowercase().as_str(),
                        "arw" | "cr2" | "cr3" | "dng" | "nef" | "orf" | "raf" | "rw2"
                    )
                })
            {
                paths.push(path);
            }
        }
    }
    paths
}

fn one_per_make(mut paths: Vec<PathBuf>, root: &Path) -> Vec<PathBuf> {
    paths.sort();
    let mut seen = std::collections::BTreeSet::new();
    paths.retain(|path| {
        let make = path
            .strip_prefix(root.join(RAW_CORPUS))
            .ok()
            .and_then(|path| path.components().next())
            .map(|component| component.as_os_str().to_owned());
        make.is_some_and(|make| seen.insert(make))
    });
    paths
}

fn relative(root: &Path, path: &Path) -> String {
    path.strip_prefix(root)
        .unwrap_or(path)
        .to_string_lossy()
        .into_owned()
}

#[cfg(test)]
pub fn test_inputs() -> Vec<Input> {
    vec![
        seeded("canonical", CANONICAL_WIDTH, CANONICAL_HEIGHT),
        seeded("odd", 513, 511),
        seeded("partial-row-chunk", 34, 513),
        seeded("tiny", 3, 3),
        saturated("saturated", CANONICAL_WIDTH, CANONICAL_HEIGHT),
        saturated("saturated-tiny", 3, 3),
    ]
}

#[cfg(test)]
fn seeded(label: &str, width: u32, height: u32) -> Input {
    let image = synthetic_mono_image::<f32>((width, height), 65_535.0);
    Input {
        label: label.to_owned(),
        source: "fornax-devtool::synthetic_mono_image".to_owned(),
        pattern: None,
        width,
        height,
        codes: image
            .as_raw()
            .iter()
            .map(|sample| sample.round().clamp(0.0, 65_535.0) as u16)
            .collect(),
    }
}

#[cfg(test)]
fn saturated(label: &str, width: u32, height: u32) -> Input {
    Input {
        label: label.to_owned(),
        source: "constant".to_owned(),
        pattern: None,
        width,
        height,
        codes: vec![u16::MAX; width as usize * height as usize],
    }
}
