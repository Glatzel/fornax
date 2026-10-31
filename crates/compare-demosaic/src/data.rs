use std::path::Path;
#[cfg(test)]
use std::path::PathBuf;

use fornax_core::BayerPattern;
use fornax_core::protocol::{IDecoder, IPreprocessor};
use fornax_decoder_rawler::RawlerDecoder;
#[cfg(test)]
use fornax_devtool::synthetic_mono_image;
use fornax_preprocessor::Preprocessor;

pub const CANONICAL_WIDTH: u32 = 512;
pub const CANONICAL_HEIGHT: u32 = 512;
#[cfg(test)]
const RAW_CORPUS: &str = "external/raw.pixls.us";
const FALLBACK_FIXTURE: &str = "external/raw-images/images/colorchart-eos-7d.cr2";
#[cfg(test)]
const MAX_REAL_FILES: usize = 8;

#[derive(Clone, Debug)]
pub struct Input {
    pub label: String,
    pub source: String,
    pub pattern: Option<BayerPattern>,
    pub width: u32,
    pub height: u32,
    pub codes: Vec<u16>,
}

#[cfg(test)]
pub fn real_inputs() -> Vec<Input> {
    let root = fornax_devtool::root_dir();
    let mut paths = raw_paths(&root.join(RAW_CORPUS));
    if paths.is_empty() {
        paths.push(root.join(FALLBACK_FIXTURE));
    }
    paths.sort();
    paths.truncate(MAX_REAL_FILES);

    let mut inputs = Vec::new();
    for path in paths {
        match decode_input(&root, &path) {
            Ok(input) => inputs.push(input),
            Err(_) => continue,
        }
    }
    assert!(
        !inputs.is_empty(),
        "no decodable 2x2 Bayer RAW files found; run scripts/download-raw-corps.ps1"
    );
    inputs
}

pub fn canonical_input() -> Input {
    let root = fornax_devtool::root_dir();
    decode_input(&root, &root.join(FALLBACK_FIXTURE))
        .expect("decode the canonical raw-images fixture")
}

fn decode_input(root: &Path, path: &Path) -> Result<Input, String> {
    let (decoded, metadata) =
        <RawlerDecoder as IDecoder<Path, f32>>::decode_active(&RawlerDecoder, path)
            .map_err(|error| error.to_string())?;
    let normalized = Preprocessor
        .process(decoded, &metadata.levels)
        .map_err(|error| error.to_string())?
        .into_image();
    let (width, height) = normalized.dimensions();
    if width < CANONICAL_WIDTH || height < CANONICAL_HEIGHT {
        return Err(format!("active area too small: {width}x{height}"));
    }
    let x = ((width - CANONICAL_WIDTH) / 2) & !1;
    let y = ((height - CANONICAL_HEIGHT) / 2) & !1;
    let row_stride = width as usize;
    let mut crop = Vec::with_capacity((CANONICAL_WIDTH * CANONICAL_HEIGHT) as usize);
    for row in y..y + CANONICAL_HEIGHT {
        let start = row as usize * row_stride + x as usize;
        crop.extend_from_slice(&normalized.as_raw()[start..start + CANONICAL_WIDTH as usize]);
    }
    let source = relative(root, path);
    let label = if source == FALLBACK_FIXTURE {
        "real-colorchart-center".to_owned()
    } else {
        source
            .replace(['/', '\\', '.'], "-")
            .trim_matches('-')
            .to_owned()
    };
    Ok(Input {
        label,
        source,
        pattern: Some(metadata.bayer_pattern),
        width: CANONICAL_WIDTH,
        height: CANONICAL_HEIGHT,
        codes: crop
            .iter()
            .map(|sample| (sample.clamp(0.0, 1.0) * f32::from(u16::MAX)).round() as u16)
            .collect(),
    })
}

#[cfg(test)]
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
